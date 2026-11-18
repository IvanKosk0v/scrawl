//! M5.4: window lookups are bounded (<= 50 ms); a lookup that never answers falls back to global
//! bindings and never stalls forwarding or the escape chord.
#![allow(clippy::expect_used)] // test helpers outside #[test] fns

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use evdev::{EventType, InputEvent, KeyCode, RelativeAxisCode, SynchronizationCode};
use strokebind::config::Trigger;
use strokebind::focus::Focused;
use strokebind::passthrough::{Mode, Output, Passthrough};
use strokebind::stroke_loop::LoopParams;
use strokebind::window::{AppId, Bounded, NeverAnswers, WindowAdapter, ADAPTER_TIMEOUT};

/// Generous slack over the bound for slow CI schedulers; the bound itself is asserted separately.
const SLACK: Duration = Duration::from_millis(50);

/// Counts calls; call n (0-based) sleeps `delays[n]` then answers `answers[n]`.
struct Scripted {
    calls: Arc<AtomicUsize>,
    delays: Vec<Duration>,
    answers: Vec<&'static str>,
}

impl WindowAdapter for Scripted {
    fn active_window(&self) -> Option<AppId> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(self.delays.get(n).copied().unwrap_or_default());
        self.answers.get(n).and_then(|a| AppId::new(a))
    }
}

fn scripted(delays: &[u64], answers: &[&'static str]) -> (Bounded, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let adapter = Scripted {
        calls: Arc::clone(&calls),
        delays: delays.iter().map(|&ms| Duration::from_millis(ms)).collect(),
        answers: answers.to_vec(),
    };
    (Bounded::new(adapter), calls)
}

fn timed<T>(f: impl FnOnce() -> T) -> (T, Duration) {
    let start = Instant::now();
    let out = f();
    (out, start.elapsed())
}

#[test]
fn bound_is_at_most_50_ms() {
    assert!(ADAPTER_TIMEOUT <= Duration::from_millis(50));
}

#[test]
fn fast_answer_is_passed_through() {
    let (b, _) = scripted(&[0], &["firefox"]);
    assert_eq!(
        b.active_window().as_ref().map(AppId::as_str),
        Some("firefox")
    );
}

#[test]
fn never_answering_lookup_falls_back_within_the_bound() {
    let b = Bounded::new(NeverAnswers);
    let (app, took) = timed(|| b.active_window());
    assert_eq!(app, None);
    assert!(took <= ADAPTER_TIMEOUT + SLACK, "took {took:?}");
}

#[test]
fn while_a_lookup_hangs_later_ones_return_at_once_without_new_threads() {
    let calls = Arc::new(AtomicUsize::new(0));
    struct Hang(Arc<AtomicUsize>);
    impl WindowAdapter for Hang {
        fn active_window(&self) -> Option<AppId> {
            self.0.fetch_add(1, Ordering::SeqCst);
            NeverAnswers.active_window()
        }
    }
    let b = Bounded::new(Hang(Arc::clone(&calls)));
    assert_eq!(b.active_window(), None);
    for _ in 0..20 {
        let (app, took) = timed(|| b.active_window());
        assert_eq!(app, None);
        assert!(took < Duration::from_millis(10), "took {took:?}");
    }
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "no lookup piles up behind the hung one"
    );
}

#[test]
fn late_answer_is_dropped_and_the_next_lookup_is_fresh() {
    // First lookup answers after the bound ("stale"); the next one answers at once ("fresh").
    let (b, calls) = scripted(&[120, 0], &["stale", "fresh"]);
    assert_eq!(b.active_window(), None);
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(b.active_window().as_ref().map(AppId::as_str), Some("fresh"));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

// ---- the engine keeps running with a hung adapter ----

fn hung_engine() -> Focused {
    let engine = Passthrough::new(&LoopParams {
        trigger: Trigger::Right,
        min_step_len: 10.0,
        min_run_len: 30.0,
    })
    .with_mode(Mode::Live { click_slop_px: 6 })
    .with_escape(vec![KeyCode::BTN_LEFT, KeyCode::BTN_RIGHT]);
    Focused::new(engine, Box::new(Bounded::new(NeverAnswers)))
}

fn report(f: &mut Focused, evs: &[(EventType, u16, i32)]) -> Vec<Output> {
    let syn = (
        EventType::SYNCHRONIZATION,
        SynchronizationCode::SYN_REPORT.0,
        0,
    );
    evs.iter()
        .chain(std::iter::once(&syn))
        .flat_map(|&(t, c, v)| f.push(InputEvent::new(t.0, c, v)))
        .collect()
}

fn button(code: KeyCode, v: i32) -> (EventType, u16, i32) {
    (EventType::KEY, code.0, v)
}

#[test]
fn hung_adapter_does_not_stop_forwarding() {
    let mut f = hung_engine();
    let (_, took) = timed(|| report(&mut f, &[button(KeyCode::BTN_RIGHT, 1)]));
    assert!(took <= ADAPTER_TIMEOUT + SLACK, "press took {took:?}");
    // Live motion is forwarded promptly, presses after the first do not wait again.
    let (out, took) = timed(|| {
        let mut out = report(
            &mut f,
            &[(EventType::RELATIVE, RelativeAxisCode::REL_X.0, 3)],
        );
        out.extend(report(&mut f, &[button(KeyCode::BTN_RIGHT, 0)]));
        out.extend(report(&mut f, &[button(KeyCode::BTN_RIGHT, 1)]));
        out.extend(report(&mut f, &[button(KeyCode::BTN_RIGHT, 0)]));
        out
    });
    assert!(took < Duration::from_millis(10), "took {took:?}");
    let emitted = out.iter().filter(|o| matches!(o, Output::Emit(_))).count();
    assert!(emitted >= 3, "motion and both clicks forwarded: {out:?}");
    assert_eq!(f.press_app(), None, "global bindings");
}

#[test]
fn hung_adapter_does_not_block_the_escape_chord() {
    let mut f = hung_engine();
    report(&mut f, &[button(KeyCode::BTN_RIGHT, 1)]);
    let out = report(&mut f, &[button(KeyCode::BTN_LEFT, 1)]);
    assert_eq!(out, vec![Output::Escape]);
}
