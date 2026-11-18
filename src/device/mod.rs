//! evdev enumeration (opened read-only; nothing here grabs a device).

pub mod event;
pub mod forward;
pub mod grab;
pub mod source;
pub mod virtual_keyboard;
pub mod virtual_mouse;

pub use event::InputEvent;

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use evdev::{AttributeSetRef, KeyCode, RelativeAxisCode};

use crate::error::Error;

/// Directory holding evdev nodes.
const INPUT_DIR: &str = "/dev/input";

/// A mouse found under `/dev/input`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    /// Device name reported by the kernel.
    pub name: String,
    /// Path of the evdev node, e.g. `/dev/input/event5`.
    pub path: PathBuf,
}

/// True if the capabilities look like a mouse: a left button plus relative X and Y motion.
pub fn is_mouse(
    keys: Option<&AttributeSetRef<KeyCode>>,
    rel: Option<&AttributeSetRef<RelativeAxisCode>>,
) -> bool {
    keys.is_some_and(|k| k.contains(KeyCode::BTN_LEFT))
        && rel.is_some_and(|r| {
            r.contains(RelativeAxisCode::REL_X) && r.contains(RelativeAxisCode::REL_Y)
        })
}

/// List mice under `/dev/input`.
pub fn list_mice() -> Result<Vec<DeviceInfo>, Error> {
    list_mice_in(Path::new(INPUT_DIR))
}

/// List mice among the `event*` nodes of `dir`, sorted by path. strokebind's own virtual devices are skipped.
///
/// Entries and devices that cannot be opened for reasons other than permissions are skipped. If some node was
/// unreadable and no mouse was found, the permission error is returned so the user gets a hint.
pub fn list_mice_in(dir: &Path) -> Result<Vec<DeviceInfo>, Error> {
    let io_err = |source| Error::Io {
        path: dir.to_path_buf(),
        source,
    };
    let mut nodes = Vec::new();
    for entry in fs::read_dir(dir).map_err(io_err)? {
        // An unreadable directory entry must not hide the other devices.
        let Ok(entry) = entry else { continue };
        if entry.file_name().to_string_lossy().starts_with("event") {
            nodes.push(entry.path());
        }
    }
    nodes.sort();

    let mut mice = Vec::new();
    let mut denied = None;
    for path in nodes {
        match source::open_readonly(&path) {
            Ok(dev) => {
                let name = dev.name().unwrap_or("unknown");
                // Our own virtual mouse must never become a source (feedback loop).
                if is_mouse(dev.supported_keys(), dev.supported_relative_axes())
                    && !virtual_mouse::is_virtual_name(name)
                {
                    mice.push(DeviceInfo {
                        name: name.to_owned(),
                        path,
                    });
                }
            }
            Err(e) if e.kind() == ErrorKind::PermissionDenied => {
                denied.get_or_insert(path);
            }
            Err(_) => {}
        }
    }
    match denied {
        Some(path) if mice.is_empty() => Err(Error::PermissionDenied(path)),
        _ => Ok(mice),
    }
}
