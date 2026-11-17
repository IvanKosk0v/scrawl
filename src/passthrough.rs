//! Click passthrough: raw reports are forwarded unchanged, except that a trigger press is held back until its
//! release decides between a plain click (replayed verbatim) and a stroke (swallowed and reported).
//!
//! Pure logic over `evdev::InputEvent` values; no I/O.

use evdev::{EventSummary, EventType, InputEvent, KeyCode, RelativeAxisCode, SynchronizationCode};

use crate::device::source::trigger_code;
use crate::recognizer::recognize;
use crate::recognizer::segment::Point;
use crate::stroke_loop::LoopParams;

/// What the caller must do after an event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output {
    /// Write this report to the virtual mouse (it does not include the final `SYN_REPORT`).
    Emit(Vec<InputEvent>),
    /// A stroke was recognized.
    Stroke(String),
}

/// A trigger press waiting for its release.
#[derive(Debug)]
struct Pending {
    reports: Vec<Vec<InputEvent>>,
    points: Vec<Point>,
}

/// Stroke-vs-click state machine, fed one raw event at a time.
#[derive(Debug)]
pub struct Passthrough {
    params: LoopParams,
    trigger: KeyCode,
    report: Vec<InputEvent>,
    pending: Option<Pending>,
}

impl Passthrough {
    /// New engine for the trigger and thresholds in `params`.
    pub fn new(params: &LoopParams) -> Self {
        Passthrough {
            params: *params,
            trigger: trigger_code(params.trigger),
            report: Vec::new(),
            pending: None,
        }
    }

    /// Feed one raw event; outputs are produced only when a `SYN_REPORT` completes a report.
    pub fn push(&mut self, raw: InputEvent) -> Vec<Output> {
        match raw.destructure() {
            EventSummary::Synchronization(_, SynchronizationCode::SYN_REPORT, _) => {
                let report = std::mem::take(&mut self.report);
                self.on_report(report)
            }
            EventSummary::Synchronization(_, SynchronizationCode::SYN_DROPPED, _) => {
                // evdev's synced Device re-injects the lost state; the partial report is unreliable.
                self.report.clear();
                Vec::new()
            }
            EventSummary::Synchronization(..) => Vec::new(),
            _ => {
                self.report.push(raw);
                Vec::new()
            }
        }
    }

    fn trigger_value(&self, report: &[InputEvent]) -> Option<i32> {
        report
            .iter()
            .rev()
            .find(|e| e.event_type() == EventType::KEY && e.code() == self.trigger.0)
            .map(InputEvent::value)
    }

    fn on_report(&mut self, report: Vec<InputEvent>) -> Vec<Output> {
        if report.is_empty() {
            return Vec::new();
        }
        let trigger = self.trigger_value(&report);
        let Some(pending) = self.pending.as_mut() else {
            if trigger == Some(1) {
                self.pending = Some(Pending {
                    reports: vec![report],
                    points: vec![Point { x: 0, y: 0 }],
                });
                return Vec::new();
            }
            return vec![Output::Emit(report)];
        };
        let (dx, dy) = motion(&report);
        if dx != 0 || dy != 0 {
            let last = pending.points[pending.points.len() - 1];
            pending.points.push(Point {
                x: last.x.saturating_add(dx),
                y: last.y.saturating_add(dy),
            });
        }
        pending.reports.push(report);
        if trigger != Some(0) {
            return Vec::new();
        }
        let Some(done) = self.pending.take() else {
            return Vec::new();
        };
        match recognize(
            &done.points,
            self.params.min_step_len,
            self.params.min_run_len,
        ) {
            None => done.reports.into_iter().map(Output::Emit).collect(),
            Some(stroke) => {
                let trigger = self.trigger;
                let mut out: Vec<Output> = done
                    .reports
                    .into_iter()
                    .map(|r| {
                        r.into_iter()
                            .filter(|e| keep_in_stroke(e, trigger))
                            .collect()
                    })
                    .filter(|r: &Vec<InputEvent>| !r.is_empty())
                    .map(Output::Emit)
                    .collect();
                out.push(Output::Stroke(stroke));
                out
            }
        }
    }
}

/// Summed REL_X/REL_Y of a report.
fn motion(report: &[InputEvent]) -> (i32, i32) {
    report
        .iter()
        .fold((0i32, 0i32), |(dx, dy), e| match e.destructure() {
            EventSummary::RelativeAxis(_, RelativeAxisCode::REL_X, v) => (dx.saturating_add(v), dy),
            EventSummary::RelativeAxis(_, RelativeAxisCode::REL_Y, v) => (dx, dy.saturating_add(v)),
            _ => (dx, dy),
        })
}

/// After a stroke, buffered pointer motion, the trigger and MSC events are dropped; the rest is replayed.
fn keep_in_stroke(e: &InputEvent, trigger: KeyCode) -> bool {
    match e.destructure() {
        EventSummary::RelativeAxis(_, RelativeAxisCode::REL_X | RelativeAxisCode::REL_Y, _) => {
            false
        }
        EventSummary::Key(_, code, _) => code != trigger,
        EventSummary::Misc(..) => false,
        _ => true,
    }
}
