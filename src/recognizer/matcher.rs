//! Stroke string to binding.

use super::segment::Direction;
use crate::config::Binding;

/// Outcome of matching a stroke against bindings.
#[derive(Debug, Clone, Copy)]
pub enum MatchResult<'a> {
    /// A single closest binding within tolerance.
    Found(&'a Binding),
    /// Two or more bindings are equally close; no action should run.
    Ambiguous,
    /// No binding is within tolerance.
    NoMatch,
}

/// Distance between two stroke strings: the sum of 45-degree steps between corresponding tokens.
/// `None` if either string is invalid or the token counts differ.
fn distance(a: &str, b: &str) -> Option<u32> {
    let mut a = a.split('-');
    let mut b = b.split('-');
    let mut total = 0;
    loop {
        match (a.next(), b.next()) {
            (None, None) => return Some(total),
            (Some(x), Some(y)) => {
                total += Direction::from_token(x)?.steps_to(Direction::from_token(y)?);
            }
            _ => return None,
        }
    }
}

/// Find the binding closest to `stroke`, allowing a total distance of at most `tolerance`
/// 45-degree steps (0 = exact). Ties for the best distance are reported as ambiguous.
pub fn match_stroke<'a>(stroke: &str, bindings: &'a [Binding], tolerance: u32) -> MatchResult<'a> {
    let mut best: Option<(u32, &Binding)> = None;
    let mut tied = false;
    for binding in bindings {
        let Some(d) = distance(stroke, binding.stroke.as_str()) else {
            continue;
        };
        if d > tolerance {
            continue;
        }
        match best {
            Some((bd, _)) if d > bd => {}
            Some((bd, _)) if d == bd => tied = true,
            _ => {
                best = Some((d, binding));
                tied = false;
            }
        }
    }
    match best {
        Some(_) if tied => MatchResult::Ambiguous,
        Some((_, b)) => MatchResult::Found(b),
        None => MatchResult::NoMatch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Action, Binding, Stroke};
    use proptest::prelude::*;

    fn b(stroke: &str) -> Binding {
        Binding {
            stroke: stroke.parse::<Stroke>().expect("valid stroke"),
            app: None,
            action: Action::Key(stroke.to_lowercase()),
        }
    }

    fn found<'a>(r: MatchResult<'a>) -> &'a str {
        match r {
            MatchResult::Found(b) => b.stroke.as_str(),
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn exact_match_with_zero_tolerance() {
        let bs = [b("L"), b("DR"), b("L-U")];
        assert_eq!(found(match_stroke("L-U", &bs, 0)), "L-U");
        assert_eq!(found(match_stroke("DR", &bs, 0)), "DR");
    }

    #[test]
    fn zero_tolerance_rejects_neighbouring_direction() {
        assert!(matches!(
            match_stroke("R", &[b("DR")], 0),
            MatchResult::NoMatch
        ));
    }

    #[test]
    fn tolerance_one_accepts_adjacent_direction() {
        assert_eq!(found(match_stroke("DR", &[b("R")], 1)), "R");
    }

    #[test]
    fn tolerance_counts_around_the_circle() {
        // UR and R are adjacent through the +x axis; UR and DR are two steps apart.
        assert_eq!(found(match_stroke("UR", &[b("R")], 1)), "R");
        assert!(matches!(
            match_stroke("UR", &[b("DR")], 1),
            MatchResult::NoMatch
        ));
        assert_eq!(found(match_stroke("UR", &[b("DR")], 2)), "DR");
    }

    #[test]
    fn different_token_count_never_matches() {
        assert!(matches!(
            match_stroke("R", &[b("R-U")], 8),
            MatchResult::NoMatch
        ));
    }

    #[test]
    fn distances_sum_over_tokens() {
        let bs = [b("UL-U")];
        assert!(matches!(match_stroke("L-U", &bs, 0), MatchResult::NoMatch));
        assert_eq!(found(match_stroke("L-U", &bs, 1)), "UL-U");
    }

    #[test]
    fn equally_close_bindings_are_ambiguous() {
        let bs = [b("UR"), b("DR")];
        assert!(matches!(match_stroke("R", &bs, 1), MatchResult::Ambiguous));
    }

    #[test]
    fn exact_match_beats_near_match() {
        let bs = [b("DR"), b("R")];
        assert_eq!(found(match_stroke("R", &bs, 1)), "R");
    }

    #[test]
    fn duplicate_bindings_are_ambiguous() {
        let bs = [b("R"), b("R")];
        assert!(matches!(match_stroke("R", &bs, 0), MatchResult::Ambiguous));
    }

    #[test]
    fn invalid_input_never_matches() {
        assert!(matches!(
            match_stroke("", &[b("R")], 8),
            MatchResult::NoMatch
        ));
        assert!(matches!(
            match_stroke("R--U", &[b("R")], 8),
            MatchResult::NoMatch
        ));
    }

    fn valid_stroke() -> impl Strategy<Value = String> {
        prop::collection::vec(
            prop::sample::select(vec!["U", "D", "L", "R", "UL", "UR", "DL", "DR"]),
            1..5,
        )
        .prop_map(|t| t.join("-"))
    }

    proptest! {
        #[test]
        fn arbitrary_input_never_panics(input in ".*", tol in 0u32..20) {
            let _ = match_stroke(&input, &[b("R"), b("L-U")], tol);
        }

        #[test]
        fn own_stroke_matches_itself_at_any_tolerance(s in valid_stroke(), tol in 0u32..10) {
            let bs = [b(&s)];
            prop_assert_eq!(found(match_stroke(&s, &bs, tol)), s.as_str());
        }

        #[test]
        fn match_implies_distance_within_tolerance(
            input in valid_stroke(), cand in valid_stroke(), tol in 0u32..10,
        ) {
            if let MatchResult::Found(f) = match_stroke(&input, &[b(&cand)], tol) {
                let d = distance(&input, f.stroke.as_str());
                prop_assert!(d.is_some_and(|d| d <= tol));
            }
        }
    }
}
