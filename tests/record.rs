//! M2.3: raw evdev reports -> InputEvent -> stroke, using a fake raw source (no hardware).

use evdev::{EventType, InputEvent as Raw, KeyCode, RelativeAxisCode, SynchronizationCode};
use scrawl::config::Trigger;
use scrawl::device::source::ReportMapper;
use scrawl::device::InputEvent;
use scrawl::stroke_loop::{run_events, LoopParams};

fn rel(axis: RelativeAxisCode, v: i32) -> Raw {
    Raw::new(EventType::RELATIVE.0, axis.0, v)
}
fn key(k: KeyCode, v: i32) -> Raw {
    Raw::new(EventType::KEY.0, k.0, v)
}
fn syn() -> Raw {
    Raw::new(
        EventType::SYNCHRONIZATION.0,
        SynchronizationCode::SYN_REPORT.0,
        0,
    )
}

fn map_all(raw: &[Raw]) -> Vec<InputEvent> {
    let mut m = ReportMapper::default();
    raw.iter().flat_map(|e| m.push(*e)).collect()
}

#[test]
fn motion_is_folded_per_report() {
    let out = map_all(&[
        rel(RelativeAxisCode::REL_X, 3),
        rel(RelativeAxisCode::REL_Y, -2),
        rel(RelativeAxisCode::REL_X, 1),
        syn(),
    ]);
    assert_eq!(out, vec![InputEvent::Motion { dx: 4, dy: -2 }]);
}

#[test]
fn empty_report_emits_nothing() {
    assert!(map_all(&[syn(), syn()]).is_empty());
}

#[test]
fn buttons_map_and_repeat_is_ignored() {
    let out = map_all(&[
        key(KeyCode::BTN_RIGHT, 1),
        syn(),
        key(KeyCode::BTN_RIGHT, 2),
        syn(),
        key(KeyCode::BTN_SIDE, 1),
        key(KeyCode::BTN_LEFT, 1),
        syn(),
        key(KeyCode::BTN_RIGHT, 0),
        syn(),
    ]);
    assert_eq!(
        out,
        vec![
            InputEvent::Button {
                button: Trigger::Right,
                pressed: true
            },
            InputEvent::Button {
                button: Trigger::Side,
                pressed: true
            },
            InputEvent::Button {
                button: Trigger::Right,
                pressed: false
            },
        ]
    );
}

#[test]
fn raw_drag_yields_stroke() {
    let mut raw = vec![key(KeyCode::BTN_RIGHT, 1), syn()];
    for _ in 0..10 {
        raw.push(rel(RelativeAxisCode::REL_X, 10));
        raw.push(syn());
    }
    raw.push(key(KeyCode::BTN_RIGHT, 0));
    raw.push(syn());

    let params = LoopParams {
        trigger: Trigger::Right,
        min_step_len: 5.0,
        min_run_len: 30.0,
    };
    let mut strokes = Vec::new();
    run_events(map_all(&raw).into_iter().map(Ok), &params, |s| {
        strokes.push(s)
    })
    .unwrap();
    assert_eq!(strokes, vec![Some("R".to_owned())]);
}

#[test]
fn syn_dropped_discards_partial_report() {
    let out = map_all(&[
        rel(RelativeAxisCode::REL_X, 50),
        key(KeyCode::BTN_RIGHT, 1),
        Raw::new(
            EventType::SYNCHRONIZATION.0,
            SynchronizationCode::SYN_DROPPED.0,
            0,
        ),
        rel(RelativeAxisCode::REL_Y, 7),
        syn(),
    ]);
    assert_eq!(out, vec![InputEvent::Motion { dx: 0, dy: 7 }]);
}
