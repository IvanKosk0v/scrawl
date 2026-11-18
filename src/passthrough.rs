//! Click passthrough: raw reports are forwarded unchanged, except that a trigger press is held back until its
//! release decides between a plain click and a stroke. [`Mode::Frozen`] buffers everything until then (click =
//! replayed verbatim, stroke = swallowed and reported); [`Mode::Live`] forwards motion immediately and withholds
//! only the press (click within the slop, stroke reported, anything else swallowed).
//!
//! Pure logic over `evdev::InputEvent` values; no I/O.

use evdev::{EventSummary, EventType, InputEvent, KeyCode, RelativeAxisCode, SynchronizationCode};

use crate::config::Escape;
use crate::device::source::{button_code, trigger_code};
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
    /// The escape chord is held: the caller must stop forwarding and exit.
    Escape,
}

/// How the pointer behaves while the trigger is held.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// `freeze_pointer = true`: everything is buffered until release (M3.3).
    #[default]
    Frozen,
    /// `freeze_pointer = false`: motion and other events are forwarded live; only the press is withheld.
    Live {
        /// A release within this distance of the press point is a click.
        click_slop_px: u32,
    },
}

/// A trigger press waiting for its release.
#[derive(Debug)]
struct Pending {
    /// Frozen: every report since the press. Live: unused.
    reports: Vec<Vec<InputEvent>>,
    /// Live: the withheld trigger press (with its MSC events).
    press: Vec<InputEvent>,
    points: Vec<Point>,
    /// Largest squared distance from the press point so far.
    max_dist2: i64,
}

/// Stroke-vs-click state machine, fed one raw event at a time.
#[derive(Debug)]
pub struct Passthrough {
    params: LoopParams,
    trigger: KeyCode,
    report: Vec<InputEvent>,
    pending: Option<Pending>,
    escape: Vec<KeyCode>,
    held: Vec<KeyCode>,
    mode: Mode,
}

impl Passthrough {
    /// New engine for the trigger and thresholds in `params`, with the default escape chord.
    pub fn new(params: &LoopParams) -> Self {
        Passthrough {
            params: *params,
            trigger: trigger_code(params.trigger),
            report: Vec::new(),
            pending: None,
            escape: Escape::default()
                .buttons()
                .iter()
                .map(|&b| button_code(b))
                .collect(),
            held: Vec::new(),
            mode: Mode::Frozen,
        }
    }

    /// Replace the escape chord: holding all of `buttons` at once yields [`Output::Escape`].
    pub fn with_escape(mut self, buttons: Vec<KeyCode>) -> Self {
        self.escape = buttons;
        self
    }

    /// Choose frozen (default) or live pointer motion while the trigger is held.
    pub fn with_mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    /// Track button state; true once every escape button is held.
    fn escape_held(&mut self, report: &[InputEvent]) -> bool {
        for e in report {
            if let EventSummary::Key(_, code, v) = e.destructure() {
                match v {
                    1 if !self.held.contains(&code) => self.held.push(code),
                    0 => self.held.retain(|&c| c != code),
                    _ => {}
                }
            }
        }
        !self.escape.is_empty() && self.escape.iter().all(|c| self.held.contains(c))
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
        if self.escape_held(&report) {
            return vec![Output::Escape];
        }
        let trigger = self.trigger_value(&report);
        let Some(pending) = self.pending.as_mut() else {
            if trigger != Some(1) {
                return vec![Output::Emit(report)];
            }
            let origin = vec![Point { x: 0, y: 0 }];
            return match self.mode {
                Mode::Frozen => {
                    self.pending = Some(Pending {
                        reports: vec![report],
                        press: Vec::new(),
                        points: origin,
                        max_dist2: 0,
                    });
                    Vec::new()
                }
                Mode::Live { .. } => {
                    let (press, rest) = split_trigger(report, self.trigger);
                    self.pending = Some(Pending {
                        reports: Vec::new(),
                        press,
                        points: origin,
                        max_dist2: 0,
                    });
                    emit_nonempty(rest)
                }
            };
        };
        let (dx, dy) = motion(&report);
        if dx != 0 || dy != 0 {
            let last = pending.points[pending.points.len() - 1];
            let p = Point {
                x: last.x.saturating_add(dx),
                y: last.y.saturating_add(dy),
            };
            let (px, py) = (i64::from(p.x), i64::from(p.y));
            // Each square is at most 2^62, so only the sum can overflow (both at i32::MIN).
            pending.max_dist2 = pending.max_dist2.max((px * px).saturating_add(py * py));
            pending.points.push(p);
        }
        if let Mode::Live { click_slop_px } = self.mode {
            match trigger {
                Some(0) => {}
                // A repeated press or autorepeat while held: forwarding it would unpair the withheld press.
                Some(_) => return emit_nonempty(split_trigger(report, self.trigger).1),
                None => return vec![Output::Emit(report)],
            }
            let Some(done) = self.pending.take() else {
                return Vec::new();
            };
            return self.live_release(done, report, click_slop_px);
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

    /// Live mode release: the rest of the report first (pointer ends at the release position), then a
    /// click within the slop, else the stroke if one is recognized, else nothing.
    fn live_release(&self, done: Pending, report: Vec<InputEvent>, slop: u32) -> Vec<Output> {
        let (release, rest) = split_trigger(report, self.trigger);
        let mut out = emit_nonempty(rest);
        let slop = i64::from(slop);
        if done.max_dist2 <= slop * slop {
            out.push(Output::Emit(done.press));
            out.push(Output::Emit(release));
        } else if let Some(stroke) = recognize(
            &done.points,
            self.params.min_step_len,
            self.params.min_run_len,
        ) {
            out.push(Output::Stroke(stroke));
        }
        out
    }
}

/// Split a report into (trigger key + MSC events, everything else).
fn split_trigger(report: Vec<InputEvent>, trigger: KeyCode) -> (Vec<InputEvent>, Vec<InputEvent>) {
    report.into_iter().partition(|e| match e.destructure() {
        EventSummary::Key(_, code, _) => code == trigger,
        EventSummary::Misc(..) => true,
        _ => false,
    })
}

/// `[Emit(report)]`, or nothing for an empty report.
fn emit_nonempty(report: Vec<InputEvent>) -> Vec<Output> {
    if report.is_empty() {
        Vec::new()
    } else {
        vec![Output::Emit(report)]
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
