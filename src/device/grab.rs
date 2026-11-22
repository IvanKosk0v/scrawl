//! Exclusive grab (EVIOCGRAB) of a source mouse that can never outlive the process.
//!
//! The kernel drops a grab when its fd closes, and process exit closes every fd. So [`Grabbed`] ungrabs on
//! Drop, and [`install_failsafes`] makes a panic in any thread and SIGINT/SIGTERM exit the process promptly.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use evdev::Device;
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;

use super::source::open_readonly;
use crate::error::Error;

/// An evdev device held with an exclusive grab; released on drop.
pub struct Grabbed {
    dev: Device,
    path: PathBuf,
}

impl std::fmt::Debug for Grabbed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Grabbed")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl Grabbed {
    /// Open `path` read-only and grab it exclusively.
    pub fn grab(path: &Path) -> Result<Self, Error> {
        let io_err = |source: std::io::Error| {
            if source.kind() == ErrorKind::PermissionDenied {
                Error::PermissionDenied(path.to_path_buf())
            } else {
                Error::Io {
                    path: path.to_path_buf(),
                    source,
                }
            }
        };
        let mut dev = open_readonly(path).map_err(io_err)?;
        dev.grab().map_err(io_err)?;
        Ok(Grabbed {
            dev,
            path: path.to_path_buf(),
        })
    }

    /// Path of the grabbed node.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The grabbed device, for reading its events.
    pub fn device_mut(&mut self) -> &mut Device {
        &mut self.dev
    }
}

impl Drop for Grabbed {
    fn drop(&mut self) {
        // Closing the fd right after releases the grab anyway; an error here changes nothing.
        let _ = self.dev.ungrab();
    }
}

/// Exit code after a panic in any thread.
pub const EXIT_PANIC: i32 = 101;

/// Make a panic in any thread and SIGINT/SIGTERM terminate the process, which releases every grab.
///
/// Panics print as usual and then exit with [`EXIT_PANIC`]; a signal exits with `128 + signal` (130, 143).
/// Calling it again does nothing.
pub fn install_failsafes() -> Result<(), Error> {
    static INSTALLED: OnceLock<()> = OnceLock::new();
    if INSTALLED.get().is_some() {
        return Ok(());
    }
    let mut signals = Signals::new([SIGINT, SIGTERM]).map_err(Error::Signals)?;
    std::thread::Builder::new()
        .name("scrawl-signals".into())
        .spawn(move || {
            if let Some(sig) = signals.forever().next() {
                std::process::exit(128 + sig);
            }
        })
        .map_err(Error::Signals)?;
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        previous(info);
        std::process::exit(EXIT_PANIC);
    }));
    let _ = INSTALLED.set(());
    Ok(())
}
