//! Real event source: raw evdev reports mapped to [`InputEvent`]. Reads only; never grabs.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use evdev::{Device, EventSummary, KeyCode, RelativeAxisCode, SynchronizationCode};

use super::InputEvent;
use crate::config::Trigger;
use crate::error::Error;

/// Folds raw evdev events into [`InputEvent`]s, one batch per `SYN_REPORT`.
///
/// Per report, the summed motion is emitted first (if non-zero), then button changes in arrival
/// order. Buttons other than the trigger candidates and key auto-repeat are ignored.
#[derive(Debug, Default)]
pub struct ReportMapper {
    dx: i32,
    dy: i32,
    buttons: Vec<InputEvent>,
}

impl ReportMapper {
    /// Feed one raw event; returns the mapped events when it completes a report.
    pub fn push(&mut self, raw: evdev::InputEvent) -> Vec<InputEvent> {
        match raw.destructure() {
            EventSummary::RelativeAxis(_, RelativeAxisCode::REL_X, v) => {
                self.dx = self.dx.saturating_add(v);
            }
            EventSummary::RelativeAxis(_, RelativeAxisCode::REL_Y, v) => {
                self.dy = self.dy.saturating_add(v);
            }
            EventSummary::Key(_, code, v) if v == 0 || v == 1 => {
                if let Some(button) = trigger_for(code) {
                    self.buttons.push(InputEvent::Button {
                        button,
                        pressed: v == 1,
                    });
                }
            }
            EventSummary::Synchronization(_, SynchronizationCode::SYN_DROPPED, _) => {
                // Events were lost: whatever was buffered for this report is unreliable.
                self.dx = 0;
                self.dy = 0;
                self.buttons.clear();
            }
            EventSummary::Synchronization(_, SynchronizationCode::SYN_REPORT, _) => {
                let mut out = Vec::new();
                if self.dx != 0 || self.dy != 0 {
                    out.push(InputEvent::Motion {
                        dx: self.dx,
                        dy: self.dy,
                    });
                }
                out.append(&mut self.buttons);
                self.dx = 0;
                self.dy = 0;
                return out;
            }
            _ => {}
        }
        Vec::new()
    }
}

fn trigger_for(code: KeyCode) -> Option<Trigger> {
    match code {
        KeyCode::BTN_RIGHT => Some(Trigger::Right),
        KeyCode::BTN_MIDDLE => Some(Trigger::Middle),
        KeyCode::BTN_SIDE => Some(Trigger::Side),
        KeyCode::BTN_EXTRA => Some(Trigger::Extra),
        _ => None,
    }
}

/// The evdev button code of a trigger (inverse of the mapping used by [`ReportMapper`]).
pub fn trigger_code(trigger: Trigger) -> KeyCode {
    match trigger {
        Trigger::Right => KeyCode::BTN_RIGHT,
        Trigger::Middle => KeyCode::BTN_MIDDLE,
        Trigger::Side => KeyCode::BTN_SIDE,
        Trigger::Extra => KeyCode::BTN_EXTRA,
    }
}

/// Open an evdev node strictly read-only (`Device::open` would try read-write first).
pub fn open_readonly(path: &Path) -> std::io::Result<Device> {
    let file = std::fs::OpenOptions::new().read(true).open(path)?;
    Device::from_fd(file.into())
}

/// Blocking iterator over the events of one evdev node, opened with read-only access and no grab.
pub struct EvdevSource {
    dev: Device,
    path: PathBuf,
    mapper: ReportMapper,
    queue: VecDeque<InputEvent>,
}

impl std::fmt::Debug for EvdevSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EvdevSource")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl EvdevSource {
    /// Open `path` for reading. Permission problems yield [`Error::PermissionDenied`].
    pub fn open(path: &Path) -> Result<Self, Error> {
        let dev = open_readonly(path).map_err(|source| {
            if source.kind() == std::io::ErrorKind::PermissionDenied {
                Error::PermissionDenied(path.to_path_buf())
            } else {
                Error::Io {
                    path: path.to_path_buf(),
                    source,
                }
            }
        })?;
        Ok(EvdevSource {
            dev,
            path: path.to_path_buf(),
            mapper: ReportMapper::default(),
            queue: VecDeque::new(),
        })
    }
}

impl Iterator for EvdevSource {
    type Item = Result<InputEvent, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(ev) = self.queue.pop_front() {
                return Some(Ok(ev));
            }
            match self.dev.fetch_events() {
                Ok(batch) => {
                    for raw in batch {
                        self.queue.extend(self.mapper.push(raw));
                    }
                }
                Err(source) if source.kind() == std::io::ErrorKind::Interrupted => {}
                Err(source) => {
                    return Some(Err(Error::Io {
                        path: self.path.clone(),
                        source,
                    }))
                }
            }
        }
    }
}
