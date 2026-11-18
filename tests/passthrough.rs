//! M3.3 (`freeze_pointer = true`, the engine default) and M4.6 live-motion click passthrough.

use evdev::{EventType, InputEvent, KeyCode, MiscCode, RelativeAxisCode, SynchronizationCode};
use proptest::prelude::*;
use strokebind::config::Trigger;
use strokebind::passthrough::{Mode, Output, Passthrough};
use strokebind::stroke_loop::LoopParams;

type Ev = (u16, u16, i32);

#[derive(Debug, Clone, PartialEq)]
enum Out {
    Emit(Vec<Ev>),
    Stroke(String),
    Escape,
}

fn key(code: KeyCode, v: i32) -> Ev {
    (EventType::KEY.0, code.0, v)
}
fn rel(code: RelativeAxisCode, v: i32) -> Ev {
    (EventType::RELATIVE.0, code.0, v)
}
fn scan(v: i32) -> Ev {
    (EventType::MISC.0, MiscCode::MSC_SCAN.0, v)
}
fn x(v: i32) -> Ev {
    rel(RelativeAxisCode::REL_X, v)
}
fn right(v: i32) -> Ev {
    key(KeyCode::BTN_RIGHT, v)
}

fn engine() -> Passthrough {
    Passthrough::new(&LoopParams {
        trigger: Trigger::Right,
        min_step_len: 10.0,
        min_run_len: 30.0,
    })
}

/// Push one report (SYN_REPORT appended) and return normalized outputs.
fn push_report(pt: &mut Passthrough, report: &[Ev]) -> Vec<Out> {
    let syn = (
        EventType::SYNCHRONIZATION.0,
        SynchronizationCode::SYN_REPORT.0,
        0,
    );
    let mut out = Vec::new();
    for &(t, c, v) in report.iter().chain(std::iter::once(&syn)) {
        for o in pt.push(InputEvent::new(t, c, v)) {
            out.push(match o {
                Output::Emit(evs) => Out::Emit(
                    evs.iter()
                        .map(|e| (e.event_type().0, e.code(), e.value()))
                        .collect(),
                ),
                Output::Stroke(s) => Out::Stroke(s),
                Output::Escape => Out::Escape,
            });
        }
    }
    out
}

fn feed(reports: &[Vec<Ev>]) -> Vec<Out> {
    let mut pt = engine();
    reports
        .iter()
        .flat_map(|r| push_report(&mut pt, r))
        .collect()
}

#[test]
fn plain_click_is_replayed_unchanged() {
    let mut pt = engine();
    let press = vec![scan(0x90001), right(1)];
    let release = vec![scan(0x90001), right(0)];
    assert_eq!(push_report(&mut pt, &press), vec![], "press is held");
    assert_eq!(
        push_report(&mut pt, &release),
        vec![Out::Emit(press), Out::Emit(release)]
    );
}

#[test]
fn stroke_is_swallowed_and_reported() {
    let mut reports = vec![vec![scan(1), right(1)]];
    reports.extend(std::iter::repeat_n(vec![x(20)], 5));
    reports.push(vec![scan(1), right(0)]);
    assert_eq!(feed(&reports), vec![Out::Stroke("R".into())]);
}

#[test]
fn press_move_return_click_is_replayed() {
    let reports = vec![vec![right(1)], vec![x(8)], vec![x(-8)], vec![right(0)]];
    let expected: Vec<Out> = reports.iter().cloned().map(Out::Emit).collect();
    assert_eq!(feed(&reports), expected);
}

#[test]
fn idle_events_pass_through_immediately() {
    let mut pt = engine();
    for r in [
        vec![key(KeyCode::BTN_LEFT, 1)],
        vec![key(KeyCode::BTN_LEFT, 0)],
        vec![rel(RelativeAxisCode::REL_WHEEL, -1)],
        vec![x(5), rel(RelativeAxisCode::REL_Y, 3)],
    ] {
        assert_eq!(push_report(&mut pt, &r), vec![Out::Emit(r.clone())]);
    }
}

#[test]
fn wheel_during_stroke_is_kept() {
    let wheel = rel(RelativeAxisCode::REL_WHEEL, 1);
    let mut reports = vec![vec![right(1)], vec![wheel]];
    reports.extend(std::iter::repeat_n(vec![x(20)], 5));
    reports.push(vec![right(0)]);
    assert_eq!(
        feed(&reports),
        vec![Out::Emit(vec![wheel]), Out::Stroke("R".into())]
    );
}

#[test]
fn syn_dropped_discards_partial_report() {
    let mut pt = engine();
    let dropped = InputEvent::new(
        EventType::SYNCHRONIZATION.0,
        SynchronizationCode::SYN_DROPPED.0,
        0,
    );
    assert!(pt.push(InputEvent::new(x(7).0, x(7).1, 7)).is_empty());
    assert!(pt.push(dropped).is_empty());
    assert_eq!(
        push_report(&mut pt, &[x(3)]),
        vec![Out::Emit(vec![x(3)])],
        "the partial report before SYN_DROPPED is gone"
    );
}

fn non_trigger_event() -> impl Strategy<Value = Ev> {
    prop_oneof![
        (-50i32..50).prop_map(x),
        (-50i32..50).prop_map(|v| rel(RelativeAxisCode::REL_Y, v)),
        (-3i32..3).prop_map(|v| rel(RelativeAxisCode::REL_WHEEL, v)),
        (0i32..2).prop_map(|v| key(KeyCode::BTN_LEFT, v)),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    fn without_trigger_output_equals_input(
        reports in prop::collection::vec(prop::collection::vec(non_trigger_event(), 1..4), 0..20)
    ) {
        let expected: Vec<Out> = reports.iter().cloned().map(Out::Emit).collect();
        prop_assert_eq!(feed(&reports), expected);
    }
}

#[test]
fn escape_chord_requests_exit_even_while_trigger_is_held() {
    let mut pt = engine();
    assert_eq!(
        push_report(&mut pt, &[key(KeyCode::BTN_LEFT, 1)]),
        vec![Out::Emit(vec![key(KeyCode::BTN_LEFT, 1)])]
    );
    assert_eq!(push_report(&mut pt, &[right(1)]), vec![], "trigger held");
    assert_eq!(
        push_report(&mut pt, &[key(KeyCode::BTN_MIDDLE, 1)]),
        vec![Out::Escape]
    );
}

#[test]
fn escape_chord_needs_all_buttons_down_at_once() {
    let mut pt = engine().with_escape(vec![KeyCode::BTN_SIDE, KeyCode::BTN_EXTRA]);
    let side = |v| key(KeyCode::BTN_SIDE, v);
    let extra = |v| key(KeyCode::BTN_EXTRA, v);
    for r in [vec![side(1)], vec![side(0)], vec![extra(1)]] {
        assert_eq!(push_report(&mut pt, &r), vec![Out::Emit(r.clone())]);
    }
    assert_eq!(push_report(&mut pt, &[side(1)]), vec![Out::Escape]);
}

// ---- M4.6 live motion (`freeze_pointer = false`) ----

fn live_engine() -> Passthrough {
    engine().with_mode(Mode::Live { click_slop_px: 6 })
}

fn live_feed(reports: &[Vec<Ev>]) -> Vec<Out> {
    let mut pt = live_engine();
    reports
        .iter()
        .flat_map(|r| push_report(&mut pt, r))
        .collect()
}

#[test]
fn live_click_within_slop_is_replayed_at_release_position() {
    let mut pt = live_engine();
    let press = vec![scan(0x90001), right(1)];
    assert_eq!(
        push_report(&mut pt, &press),
        vec![],
        "only the press is withheld"
    );
    assert_eq!(push_report(&mut pt, &[x(3)]), vec![Out::Emit(vec![x(3)])]);
    assert_eq!(push_report(&mut pt, &[x(-1)]), vec![Out::Emit(vec![x(-1)])]);
    assert_eq!(
        push_report(&mut pt, &[x(2), scan(0x90001), right(0)]),
        vec![
            Out::Emit(vec![x(2)]),
            Out::Emit(press),
            Out::Emit(vec![scan(0x90001), right(0)]),
        ]
    );
}

#[test]
fn live_stroke_moves_pointer_and_reports_stroke() {
    let mut reports = vec![vec![scan(1), right(1)]];
    reports.extend(std::iter::repeat_n(vec![x(20)], 5));
    reports.push(vec![scan(1), right(0)]);
    let mut expected: Vec<Out> = std::iter::repeat_n(Out::Emit(vec![x(20)]), 5).collect();
    expected.push(Out::Stroke("R".into()));
    assert_eq!(live_feed(&reports), expected);
}

#[test]
fn live_swallow_band_emits_no_click() {
    let reports = vec![vec![right(1)], vec![x(10)], vec![right(0)]];
    assert_eq!(live_feed(&reports), vec![Out::Emit(vec![x(10)])]);
}

#[test]
fn live_press_move_return_beyond_slop_is_swallowed() {
    let reports = vec![vec![right(1)], vec![x(10)], vec![x(-10)], vec![right(0)]];
    assert_eq!(
        live_feed(&reports),
        vec![Out::Emit(vec![x(10)]), Out::Emit(vec![x(-10)])]
    );
}

#[test]
fn live_other_buttons_and_wheel_pass_through_while_held() {
    let wheel = rel(RelativeAxisCode::REL_WHEEL, 1);
    let left = |v| key(KeyCode::BTN_LEFT, v);
    let mut pt = live_engine();
    assert_eq!(
        push_report(&mut pt, &[x(1), right(1)]),
        vec![Out::Emit(vec![x(1)])]
    );
    for r in [vec![wheel], vec![left(1)], vec![left(0)]] {
        assert_eq!(push_report(&mut pt, &r), vec![Out::Emit(r.clone())]);
    }
}

#[test]
fn live_escape_chord_still_exits() {
    let mut pt = live_engine();
    let left = vec![key(KeyCode::BTN_LEFT, 1)];
    assert_eq!(push_report(&mut pt, &left), vec![Out::Emit(left.clone())]);
    assert_eq!(push_report(&mut pt, &[right(1)]), vec![]);
    assert_eq!(
        push_report(&mut pt, &[key(KeyCode::BTN_MIDDLE, 1)]),
        vec![Out::Escape]
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    fn live_without_trigger_output_equals_input(
        reports in prop::collection::vec(prop::collection::vec(non_trigger_event(), 1..4), 0..20)
    ) {
        let expected: Vec<Out> = reports.iter().cloned().map(Out::Emit).collect();
        prop_assert_eq!(live_feed(&reports), expected);
    }
}
