//! Segments to stroke string.

use super::segment::{Direction, Segment};

/// Merge adjacent runs of the same direction, summing their lengths.
fn merge(runs: impl IntoIterator<Item = (Direction, f64)>) -> Vec<(Direction, f64)> {
    let mut out: Vec<(Direction, f64)> = Vec::new();
    for (dir, len) in runs {
        match out.last_mut() {
            Some((d, l)) if *d == dir => *l += len,
            _ => out.push((dir, len)),
        }
    }
    out
}

/// Build a stroke string such as `R`, `DR` or `L-U` from segments.
///
/// Same-direction neighbours are merged, merged runs shorter than `min_run_len` are dropped as noise,
/// and runs that become adjacent after dropping are merged again. Returns `None` if nothing is left.
pub fn stroke_string(segments: &[Segment], min_run_len: f64) -> Option<String> {
    let runs = merge(segments.iter().map(|s| (s.dir, s.len)));
    let runs = merge(runs.into_iter().filter(|&(_, len)| len >= min_run_len));
    if runs.is_empty() {
        return None;
    }
    let tokens: Vec<&str> = runs.iter().map(|(d, _)| d.as_str()).collect();
    Some(tokens.join("-"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Stroke;
    use crate::recognizer::segment::{Direction, Segment};
    use proptest::prelude::*;

    fn seg(dir: Direction, len: f64) -> Segment {
        Segment { dir, len }
    }

    #[test]
    fn single_direction_merges_repeats() {
        let segs = [
            seg(Direction::R, 20.0),
            seg(Direction::R, 20.0),
            seg(Direction::R, 20.0),
        ];
        assert_eq!(stroke_string(&segs, 30.0).as_deref(), Some("R"));
    }

    #[test]
    fn diagonal_is_one_token() {
        assert_eq!(
            stroke_string(&[seg(Direction::DR, 50.0)], 30.0).as_deref(),
            Some("DR")
        );
    }

    #[test]
    fn l_shape_is_two_tokens() {
        let segs = [
            seg(Direction::L, 25.0),
            seg(Direction::L, 25.0),
            seg(Direction::U, 25.0),
            seg(Direction::U, 25.0),
        ];
        assert_eq!(stroke_string(&segs, 30.0).as_deref(), Some("L-U"));
    }

    #[test]
    fn zigzag_noise_is_dropped() {
        let segs = [
            seg(Direction::R, 30.0),
            seg(Direction::U, 5.0),
            seg(Direction::D, 5.0),
            seg(Direction::R, 30.0),
            seg(Direction::U, 4.0),
        ];
        assert_eq!(stroke_string(&segs, 30.0).as_deref(), Some("R"));
    }

    #[test]
    fn tiny_wiggle_is_no_stroke() {
        let segs = [
            seg(Direction::R, 5.0),
            seg(Direction::U, 5.0),
            seg(Direction::L, 5.0),
            seg(Direction::D, 5.0),
        ];
        assert_eq!(stroke_string(&segs, 30.0), None);
    }

    #[test]
    fn empty_is_no_stroke() {
        assert_eq!(stroke_string(&[], 30.0), None);
    }

    fn any_direction() -> impl Strategy<Value = Direction> {
        prop::sample::select(vec![
            Direction::R,
            Direction::DR,
            Direction::D,
            Direction::DL,
            Direction::L,
            Direction::UL,
            Direction::U,
            Direction::UR,
        ])
    }

    proptest! {
        #[test]
        fn output_is_valid_stroke_without_adjacent_repeats(
            raw in prop::collection::vec((any_direction(), 0.0f64..200.0), 0..64),
            min_len in 0.0f64..100.0,
        ) {
            let segs: Vec<Segment> = raw.into_iter().map(|(dir, len)| seg(dir, len)).collect();
            if let Some(s) = stroke_string(&segs, min_len) {
                prop_assert!(s.parse::<Stroke>().is_ok(), "invalid stroke {s}");
                let toks: Vec<&str> = s.split('-').collect();
                prop_assert!(toks.windows(2).all(|w| w[0] != w[1]), "repeat in {s}");
            }
        }
    }
}
