//! Forwarding loop: grabbed source -> [`Passthrough`] -> virtual mouse.

use std::io::ErrorKind;

use super::grab::Grabbed;
use super::virtual_mouse::VirtualMouse;
use crate::error::Error;
use crate::passthrough::{Output, Passthrough};
use crate::stroke_loop::LoopParams;

/// Forward every event of `src` to `out`, holding trigger presses back until stroke-vs-click is decided, and
/// call `on_stroke` for each recognized stroke. Returns on the first read or write error (e.g. unplug).
pub fn forward(
    src: &mut Grabbed,
    out: &mut VirtualMouse,
    params: &LoopParams,
    mut on_stroke: impl FnMut(String),
) -> Result<(), Error> {
    let mut engine = Passthrough::new(params);
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
                    Output::Stroke(s) => on_stroke(s),
                }
            }
        }
    }
}
