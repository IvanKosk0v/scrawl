//! M2.2: event loop driven by a fake event source.

use strokebind::config::{Config, Trigger};
use strokebind::device::InputEvent;
use strokebind::error::Error;
use strokebind::stroke_loop::{run_events, LoopParams};

fn press(t: Trigger) -> InputEvent {
    InputEvent::Button {
        button: t,
        pressed: true,
    }
}

fn release(t: Trigger) -> InputEvent {
    InputEvent::Button {
        button: t,
        pressed: false,
    }
}

fn mv(dx: i32, dy: i32) -> InputEvent {
    InputEvent::Motion { dx, dy }
}

fn params() -> LoopParams {
    LoopParams {
        trigger: Trigger::Right,
        min_step_len: 10.0,
        min_run_len: 30.0,
    }
}

fn run(events: Vec<InputEvent>) -> Vec<Option<String>> {
    let mut out = Vec::new();
    if let Err(e) = run_events(events.into_iter().map(Ok), &params(), |s| out.push(s)) {
        panic!("run failed: {e}");
    }
    out
}

fn s(x: &str) -> Option<String> {
    Some(x.to_owned())
}

#[test]
fn horizontal_drag_is_stroke_r() {
    let ev = vec![
        press(Trigger::Right),
        mv(20, 0),
        mv(20, 0),
        mv(20, 0),
        release(Trigger::Right),
    ];
    assert_eq!(run(ev), vec![s("R")]);
}

#[test]
fn right_then_down_is_r_d() {
    let ev = vec![
        press(Trigger::Right),
        mv(20, 0),
        mv(20, 0),
        mv(0, 20),
        mv(0, 20),
        release(Trigger::Right),
    ];
    assert_eq!(run(ev), vec![s("R-D")]);
}

#[test]
fn motion_outside_a_press_is_ignored() {
    let ev = vec![
        mv(500, 500),
        press(Trigger::Right),
        mv(-20, 0),
        mv(-20, 0),
        release(Trigger::Right),
        mv(500, 500),
    ];
    assert_eq!(run(ev), vec![s("L")]);
}

#[test]
fn press_without_movement_reports_no_stroke() {
    let ev = vec![press(Trigger::Right), release(Trigger::Right)];
    assert_eq!(run(ev), vec![None]);
}

#[test]
fn tiny_wiggle_reports_no_stroke() {
    let ev = vec![
        press(Trigger::Right),
        mv(2, 1),
        mv(-3, 2),
        mv(1, -2),
        release(Trigger::Right),
    ];
    assert_eq!(run(ev), vec![None]);
}

#[test]
fn other_buttons_are_ignored() {
    let ev = vec![press(Trigger::Middle), mv(50, 0), release(Trigger::Middle)];
    assert!(run(ev).is_empty());
}

#[test]
fn two_strokes_are_reported_in_order() {
    let ev = vec![
        press(Trigger::Right),
        mv(40, 0),
        release(Trigger::Right),
        press(Trigger::Right),
        mv(0, -40),
        release(Trigger::Right),
    ];
    assert_eq!(run(ev), vec![s("R"), s("U")]);
}

#[test]
fn stroke_unfinished_at_end_of_stream_is_discarded() {
    let ev = vec![press(Trigger::Right), mv(40, 0)];
    assert!(run(ev).is_empty());
}

#[test]
fn repeated_press_and_stray_release_are_ignored() {
    let ev = vec![
        release(Trigger::Right),
        press(Trigger::Right),
        mv(20, 0),
        press(Trigger::Right),
        mv(20, 0),
        release(Trigger::Right),
    ];
    assert_eq!(run(ev), vec![s("R")]);
}

#[test]
fn source_error_stops_the_loop() {
    let events = vec![Ok(press(Trigger::Right)), Err(Error::Unsupported("boom"))];
    let mut seen = 0;
    let r = run_events(events, &params(), |_| seen += 1);
    assert!(matches!(r, Err(Error::Unsupported("boom"))));
    assert_eq!(seen, 0);
}

#[test]
fn params_derive_from_config() {
    let cfg = Config::parse("trigger = \"side\"\nmin_stroke_px = 40\n").expect("config");
    let p = LoopParams::from_config(&cfg);
    assert_eq!(p.trigger, Trigger::Side);
    assert_eq!(p.min_run_len, 40.0);
    assert_eq!(p.min_step_len, 10.0);
}

#[test]
fn default_params_match_default_config() {
    let cfg = Config::parse("trigger = \"right\"").unwrap();
    assert_eq!(LoopParams::default(), LoopParams::from_config(&cfg));
}
