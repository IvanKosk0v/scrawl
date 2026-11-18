//! Forwarding loop: grabbed source -> [`Focused`] engine -> virtual mouse.

use std::io::ErrorKind;

use super::grab::Grabbed;
use super::virtual_mouse::VirtualMouse;
use crate::error::Error;
use crate::focus::Focused;
use crate::passthrough::Output;
use crate::window::AppId;

/// Forward every event of `src` to `out` through `engine` (trigger presses held until stroke-vs-click is
/// decided) and call `on_stroke` for each recognized stroke with the app focused at its trigger press. Returns `Ok` when the escape chord is held, or the
/// first read or write error (e.g. unplug).
pub fn forward(
    src: &mut Grabbed,
    out: &mut VirtualMouse,
    mut engine: Focused,
    mut on_stroke: impl FnMut(String, Option<AppId>),
) -> Result<(), Error> {
    let path = src.path().to_path_buf();
    loop {
        let batch: Vec<_> = match src.device_mut().fetch_events() {
            Ok(events) => events.collect(),
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(source) => return Err(Error::Io { path, source }),
        };
        for raw in batch {
            for o in engine.push(raw) {
                match o {
                    Output::Emit(report) => out.emit(&report)?,
                    Output::Stroke(s) => on_stroke(s, engine.press_app().cloned()),
                    Output::Escape => return Ok(()),
                }
            }
        }
    }
}
