//! M5.3: the focused app is sampled once per trigger press (not at release) and fed to binding
//! selection; no answer from the adapter falls back to the global bindings.
#![allow(clippy::expect_used)] // test helpers outside #[test] fns

mod fake_overlay;

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

use evdev::{EventType, InputEvent, KeyCode, MiscCode, RelativeAxisCode, SynchronizationCode};
use scrawl::actions::select;
use scrawl::config::{Action, Config, Trigger};
use scrawl::focus::Focused;
use scrawl::overlay::thread::{spawn, OverlayEvent};
use scrawl::overlay::Overlay;
use scrawl::passthrough::{Mode, Output, Passthrough};
use scrawl::stroke_loop::LoopParams;
use scrawl::window::{AppId, WindowAdapter};

/// Scripted adapter: each call pops the next answer (then `None`) and counts the call.
struct Mock {
    script: RefCell<VecDeque<Option<&'static str>>>,
    calls: Rc<Cell<usize>>,
}

impl WindowAdapter for Mock {
    fn active_window(&self) -> Option<AppId> {
        self.calls.set(self.calls.get() + 1);
        self.script
            .borrow_mut()
            .pop_front()
            .flatten()
            .and_then(AppId::new)
    }
}

fn focused(mode: Mode, script: &[Option<&'static str>]) -> (Focused, Rc<Cell<usize>>) {
    let calls = Rc::new(Cell::new(0));
    let engine = Passthrough::new(&LoopParams {
        trigger: Trigger::Right,
        min_step_len: 10.0,
        min_run_len: 30.0,
    })
    .with_mode(mode);
    let mock = Mock {
        script: RefCell::new(script.iter().copied().collect()),
        calls: Rc::clone(&calls),
    };
    (Focused::new(engine, Box::new(mock)), calls)
}

const LIVE: Mode = Mode::Live { click_slop_px: 6 };

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

fn press(f: &mut Focused) -> Vec<Output> {
    report(
        f,
        &[
            (EventType::MISC, MiscCode::MSC_SCAN.0, 1),
            (EventType::KEY, KeyCode::BTN_RIGHT.0, 1),
        ],
    )
}
fn release(f: &mut Focused) -> Vec<Output> {
    report(
        f,
        &[
            (EventType::MISC, MiscCode::MSC_SCAN.0, 1),
            (EventType::KEY, KeyCode::BTN_RIGHT.0, 0),
        ],
    )
}
fn move_x(f: &mut Focused, dx: i32) -> Vec<Output> {
    report(f, &[(EventType::RELATIVE, RelativeAxisCode::REL_X.0, dx)])
}

/// Press, move right, release; returns the recognized stroke.
fn stroke_right(f: &mut Focused) -> String {
    let mut out = press(f);
    for _ in 0..5 {
        out.extend(move_x(f, 20));
    }
    out.extend(release(f));
    out.into_iter()
        .find_map(|o| match o {
            Output::Stroke(s) => Some(s),
            _ => None,
        })
        .expect("a stroke")
}

fn press_app(f: &Focused) -> Option<String> {
    f.press_app().map(|a| a.to_string())
}

#[test]
fn app_is_sampled_at_press_not_at_release() {
    for mode in [Mode::Frozen, LIVE] {
        // Focus changes to kitty mid-stroke (e.g. focus-follows-mouse): the press-time app wins.
        let (mut f, calls) = focused(mode, &[Some("firefox"), Some("kitty")]);
        press(&mut f);
        assert_eq!(calls.get(), 1, "{mode:?}: sampled when the press arrives");
        for _ in 0..5 {
            move_x(&mut f, 20);
        }
        release(&mut f);
        assert_eq!(
            calls.get(),
            1,
            "{mode:?}: not sampled again during the stroke or at release"
        );
        assert_eq!(press_app(&f).as_deref(), Some("firefox"), "{mode:?}");
    }
}

#[test]
fn each_press_samples_again() {
    let (mut f, calls) = focused(LIVE, &[Some("firefox"), Some("kitty")]);
    assert_eq!(stroke_right(&mut f), "R");
    assert_eq!(press_app(&f).as_deref(), Some("firefox"));
    assert_eq!(stroke_right(&mut f), "R");
    assert_eq!(press_app(&f).as_deref(), Some("kitty"));
    assert_eq!(calls.get(), 2);
}

#[test]
fn motion_and_other_buttons_never_query_the_adapter() {
    let (mut f, calls) = focused(LIVE, &[Some("firefox")]);
    move_x(&mut f, 50);
    report(&mut f, &[(EventType::KEY, KeyCode::BTN_LEFT.0, 1)]);
    report(&mut f, &[(EventType::KEY, KeyCode::BTN_LEFT.0, 0)]);
    assert_eq!(calls.get(), 0);
    assert_eq!(press_app(&f), None);
}

#[test]
fn press_app_selects_app_bindings_and_unknown_falls_back_to_global() {
    let cfg = Config::parse(
        "trigger = \"right\"\n\
         [[binding]]\nstroke = \"R\"\naction = { key = \"alt+right\" }\n\
         [[binding]]\nstroke = \"R\"\napp = \"firefox\"\naction = { key = \"ctrl+tab\" }\n",
    )
    .expect("valid");
    let resolved = cfg.resolve("mouse");
    // firefox at press; the adapter then fails (None) for the second press.
    let (mut f, _) = focused(LIVE, &[Some("Firefox"), None]);
    let mut chosen = Vec::new();
    for _ in 0..2 {
        let stroke = stroke_right(&mut f);
        let bindings = resolved.bindings_for(f.press_app());
        chosen.push(select(&stroke, &bindings).cloned());
    }
    assert_eq!(
        chosen,
        vec![
            Some(Action::Key("ctrl+tab".into())),
            Some(Action::Key("alt+right".into()))
        ]
    );
}

/// M7.4: with an overlay attached, the engine still answers at once while the renderer is blocked,
/// and the renderer gets Begin, the stroke's motion, then End.
#[test]
fn overlay_is_fed_without_blocking_the_engine() {
    for mode in [Mode::Frozen, LIVE] {
        let (fake, gate, log) = fake_overlay::blocking();
        let (f, _) = focused(mode, &[]);
        let mut f = f.with_overlay(spawn(move || Ok(Box::new(fake) as Box<dyn Overlay>)));
        let t = std::time::Instant::now();
        assert_eq!(stroke_right(&mut f), "R", "{mode:?}");
        assert!(t.elapsed() < std::time::Duration::from_secs(1), "{mode:?}");
        gate.send(()).expect("renderer alive");
        let seen = fake_overlay::wait_for_end(&log);
        assert_eq!(
            seen.first(),
            Some(&OverlayEvent::Begin),
            "{mode:?}: {seen:?}"
        );
        assert_eq!(seen.last(), Some(&OverlayEvent::End), "{mode:?}: {seen:?}");
        assert_eq!(fake_overlay::total_motion(&seen), (100, 0), "{mode:?}");
    }
}

#[test]
fn overlay_sees_a_click_as_begin_then_end() {
    let (fake, gate, log) = fake_overlay::blocking();
    drop(gate);
    let (f, _) = focused(LIVE, &[]);
    let mut f = f.with_overlay(spawn(move || Ok(Box::new(fake) as Box<dyn Overlay>)));
    press(&mut f);
    release(&mut f);
    assert_eq!(
        fake_overlay::wait_for_end(&log),
        [OverlayEvent::Begin, OverlayEvent::End]
    );
}

#[test]
fn motion_without_the_trigger_never_reaches_the_overlay() {
    let (fake, gate, log) = fake_overlay::blocking();
    drop(gate);
    let (f, _) = focused(LIVE, &[]);
    let mut f = f.with_overlay(spawn(move || Ok(Box::new(fake) as Box<dyn Overlay>)));
    move_x(&mut f, 50);
    press(&mut f);
    move_x(&mut f, 7);
    release(&mut f);
    move_x(&mut f, 50);
    assert_eq!(
        fake_overlay::wait_for_end(&log),
        [
            OverlayEvent::Begin,
            OverlayEvent::Motion(7, 0),
            OverlayEvent::End
        ]
    );
}
