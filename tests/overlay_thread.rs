//! M7.4: the overlay renders on its own thread; sending never waits for rendering, and motions
//! queued while a frame is drawn are merged so the trail catches up.
#![allow(clippy::expect_used)]

mod fake_overlay;

use std::time::{Duration, Instant};

use proptest::prelude::*;
use scrawl::error::Error;
use scrawl::overlay::thread::{coalesce, spawn, OverlayEvent};
use scrawl::overlay::Overlay;

use OverlayEvent::{Begin, End, Motion};

fn coalesced(events: &[OverlayEvent]) -> Vec<OverlayEvent> {
    let mut q = Vec::new();
    for &e in events {
        coalesce(&mut q, e);
    }
    q
}

#[test]
fn adjacent_motions_merge() {
    assert_eq!(
        coalesced(&[Begin, Motion(1, 2), Motion(3, -4), Motion(0, 1)]),
        [Begin, Motion(4, -1)]
    );
}

#[test]
fn begin_drops_everything_queued_before_it() {
    assert_eq!(
        coalesced(&[Begin, Motion(5, 5), End, Begin, Motion(1, 0)]),
        [Begin, Motion(1, 0)]
    );
}

#[test]
fn end_is_kept_after_motion() {
    assert_eq!(
        coalesced(&[Motion(1, 1), End, Motion(2, 2)]),
        [Motion(1, 1), End, Motion(2, 2)]
    );
}

#[test]
fn merged_motion_saturates() {
    assert_eq!(
        coalesced(&[Motion(i32::MAX, i32::MIN), Motion(1, -1)]),
        [Motion(i32::MAX, i32::MIN)]
    );
}

fn event() -> impl Strategy<Value = OverlayEvent> {
    prop_oneof![
        Just(Begin),
        Just(End),
        (-50i32..50, -50i32..50).prop_map(|(x, y)| Motion(x, y)),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// The queue stays tiny whatever the input, so a hung renderer cannot grow memory.
    #[test]
    fn queue_never_exceeds_three(events in prop::collection::vec(event(), 0..400)) {
        let mut q = Vec::new();
        for e in events {
            coalesce(&mut q, e);
            prop_assert!(q.len() <= 3, "{q:?}");
        }
    }

    /// Merging keeps the total motion of the stroke being drawn (Begin, motions, maybe End),
    /// whatever was queued before it.
    #[test]
    fn motion_of_the_current_stroke_is_preserved(
        before in prop::collection::vec(event(), 0..100),
        moves in prop::collection::vec((-50i32..50, -50i32..50), 0..100),
        ended in any::<bool>(),
    ) {
        let mut events = before;
        events.push(Begin);
        let start = events.len();
        events.extend(moves.into_iter().map(|(x, y)| Motion(x, y)));
        if ended {
            events.push(End);
        }
        let q = coalesced(&events);
        prop_assert_eq!(q.first(), Some(&Begin));
        prop_assert_eq!(q.last() == Some(&End), ended);
        prop_assert_eq!(
            fake_overlay::total_motion(&q),
            fake_overlay::total_motion(&events[start..])
        );
    }
}

#[test]
fn sending_never_waits_for_a_blocked_renderer() {
    let (fake, gate, log) = fake_overlay::blocking();
    let handle = spawn(move || Ok(Box::new(fake) as Box<dyn Overlay>));
    let t = Instant::now();
    handle.send(Begin);
    for _ in 0..100_000 {
        handle.send(Motion(1, 0));
    }
    handle.send(End);
    let took = t.elapsed();
    assert!(
        took < Duration::from_secs(1),
        "100k sends took {took:?} while the renderer was blocked"
    );
    gate.send(()).expect("renderer alive");
    let seen = fake_overlay::wait_for_end(&log);
    assert_eq!(seen.first(), Some(&Begin), "{seen:?}");
    assert_eq!(seen.last(), Some(&End), "{seen:?}");
    assert_eq!(fake_overlay::total_motion(&seen), (100_000, 0));
    assert!(
        seen.len() < 100,
        "motions were merged, got {} calls",
        seen.len()
    );
}

#[test]
fn failed_overlay_creation_disables_it_and_send_still_works() {
    let handle = spawn(|| Err(Error::Overlay("no display".into())));
    let deadline = Instant::now() + Duration::from_secs(5);
    while handle.is_running() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!handle.is_running());
    let t = Instant::now();
    handle.send(Begin);
    for _ in 0..1000 {
        handle.send(Motion(1, 1));
    }
    handle.send(End);
    assert!(t.elapsed() < Duration::from_secs(1));
}
