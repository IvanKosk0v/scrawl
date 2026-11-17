//! M1.4: end-to-end property tests of the recognizer pipeline (points -> stroke string).

use proptest::prelude::*;
use strokebind::recognizer::recognize;
use strokebind::recognizer::segment::Point;

const MIN_STEP: f64 = 10.0;
const MIN_RUN: f64 = 30.0;
const STEP_PX: i32 = 20;

/// (token, unit vector in screen coordinates, y grows down)
const DIRS: [(&str, i32, i32); 8] = [
    ("R", 1, 0),
    ("DR", 1, 1),
    ("D", 0, 1),
    ("DL", -1, 1),
    ("L", -1, 0),
    ("UL", -1, -1),
    ("U", 0, -1),
    ("UR", 1, -1),
];

/// Legs of a path: (direction index, number of points in the leg).
fn legs() -> impl Strategy<Value = Vec<(usize, i32)>> {
    prop::collection::vec((0..8usize, 3..7i32), 1..5)
}

fn build_points(legs: &[(usize, i32)]) -> Vec<Point> {
    let mut pts = vec![Point { x: 0, y: 0 }];
    for &(d, n) in legs {
        let (_, dx, dy) = DIRS[d];
        for _ in 0..n {
            let last = pts[pts.len() - 1];
            pts.push(Point {
                x: last.x + dx * STEP_PX,
                y: last.y + dy * STEP_PX,
            });
        }
    }
    pts
}

/// Expected stroke string, computed independently: adjacent equal directions collapse.
fn expected(legs: &[(usize, i32)]) -> String {
    let mut toks: Vec<&str> = Vec::new();
    for &(d, _) in legs {
        if toks.last() != Some(&DIRS[d].0) {
            toks.push(DIRS[d].0);
        }
    }
    toks.join("-")
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn never_panics_on_arbitrary_points(
        pts in prop::collection::vec((any::<i32>(), any::<i32>()), 0..64),
        min_step in 0.0f64..1000.0,
        min_run in 0.0f64..1000.0,
    ) {
        let pts: Vec<Point> = pts.into_iter().map(|(x, y)| Point { x, y }).collect();
        let _ = recognize(&pts, min_step, min_run);
    }

    #[test]
    fn empty_and_single_point_give_no_stroke(x in any::<i32>(), y in any::<i32>()) {
        prop_assert_eq!(recognize(&[], MIN_STEP, MIN_RUN), None);
        prop_assert_eq!(recognize(&[Point { x, y }], MIN_STEP, MIN_RUN), None);
    }

    #[test]
    fn clean_path_yields_expected_stroke(l in legs()) {
        prop_assert_eq!(
            recognize(&build_points(&l), MIN_STEP, MIN_RUN),
            Some(expected(&l))
        );
    }

    #[test]
    fn same_stroke_under_uniform_scaling(l in legs(), k in 1..5i32) {
        let scaled: Vec<Point> = build_points(&l)
            .into_iter()
            .map(|p| Point { x: p.x * k, y: p.y * k })
            .collect();
        let kf = f64::from(k);
        prop_assert_eq!(
            recognize(&scaled, MIN_STEP * kf, MIN_RUN * kf),
            Some(expected(&l))
        );
    }

    #[test]
    fn same_stroke_under_small_jitter(
        l in legs(),
        jitter in prop::collection::vec((-2..=2i32, -2..=2i32), 64),
    ) {
        let noisy: Vec<Point> = build_points(&l)
            .into_iter()
            .zip(jitter)
            .map(|(p, (jx, jy))| Point { x: p.x + jx, y: p.y + jy })
            .collect();
        prop_assert_eq!(recognize(&noisy, MIN_STEP, MIN_RUN), Some(expected(&l)));
    }
}
