//! Point stream to 8-direction segments.

use std::f64::consts::FRAC_PI_4;

/// A point of the pointer path, in pixels. Screen coordinates: `y` grows downward.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    /// Horizontal position.
    pub x: i32,
    /// Vertical position.
    pub y: i32,
}

/// One of eight quantized directions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Right.
    R,
    /// Down-right.
    DR,
    /// Down.
    D,
    /// Down-left.
    DL,
    /// Left.
    L,
    /// Up-left.
    UL,
    /// Up.
    U,
    /// Up-right.
    UR,
}

impl Direction {
    /// Directions in order of increasing counter-clockwise angle from +x.
    const BY_SECTOR: [Direction; 8] = [
        Direction::R,
        Direction::UR,
        Direction::U,
        Direction::UL,
        Direction::L,
        Direction::DL,
        Direction::D,
        Direction::DR,
    ];

    /// The token used in stroke strings, e.g. `DR`.
    pub fn as_str(self) -> &'static str {
        match self {
            Direction::R => "R",
            Direction::DR => "DR",
            Direction::D => "D",
            Direction::DL => "DL",
            Direction::L => "L",
            Direction::UL => "UL",
            Direction::U => "U",
            Direction::UR => "UR",
        }
    }

    /// Parse a stroke-string token such as `DR`.
    pub fn from_token(token: &str) -> Option<Self> {
        Self::BY_SECTOR.into_iter().find(|d| d.as_str() == token)
    }

    /// Number of 45-degree steps between two directions, going the short way round (0..=4).
    pub fn steps_to(self, other: Self) -> u32 {
        let index = |d: Self| Self::BY_SECTOR.iter().position(|&x| x == d).unwrap_or(0) as u32;
        let diff = index(self).abs_diff(index(other));
        diff.min(8 - diff)
    }

    fn from_delta(dx: f64, dy: f64) -> Self {
        // Flip y so that "up" on screen is a positive angle.
        let sector = ((-dy).atan2(dx) / FRAC_PI_4).round() as i64;
        Self::BY_SECTOR[sector.rem_euclid(8) as usize]
    }
}

/// A straight step of the path with a quantized direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    /// Quantized direction of the step.
    pub dir: Direction,
    /// Euclidean length of the step in pixels.
    pub len: f64,
}

/// Turn a point stream into direction segments.
///
/// Points closer than `min_len` pixels to the last accepted point are treated as jitter and skipped.
/// Consecutive segments may share a direction; merging is the caller's job.
pub fn segments(points: &[Point], min_len: f64) -> Vec<Segment> {
    let mut out = Vec::new();
    let Some(&first) = points.first() else {
        return out;
    };
    let mut anchor = first;
    for &p in &points[1..] {
        let dx = f64::from(p.x) - f64::from(anchor.x);
        let dy = f64::from(p.y) - f64::from(anchor.y);
        let len = dx.hypot(dy);
        if len > 0.0 && len >= min_len {
            out.push(Segment {
                dir: Direction::from_delta(dx, dy),
                len,
            });
            anchor = p;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn line(dx: i32, dy: i32, n: i32) -> Vec<Point> {
        (0..=n)
            .map(|i| Point {
                x: dx * i,
                y: dy * i,
            })
            .collect()
    }

    #[test]
    fn straight_lines_in_eight_directions() {
        use Direction::*;
        let cases = [
            (1, 0, R),
            (-1, 0, L),
            (0, -1, U),
            (0, 1, D),
            (1, -1, UR),
            (-1, -1, UL),
            (1, 1, DR),
            (-1, 1, DL),
        ];
        for (dx, dy, want) in cases {
            let segs = segments(&line(dx * 10, dy * 10, 5), 15.0);
            assert!(!segs.is_empty(), "{want:?}: no segments");
            assert!(segs.iter().all(|s| s.dir == want), "{want:?}: {segs:?}");
        }
    }

    #[test]
    fn off_axis_angles_snap_to_nearest_direction() {
        // ~20 degrees below +x axis snaps to R; ~70 degrees snaps to D.
        assert_eq!(
            segments(&[Point { x: 0, y: 0 }, Point { x: 94, y: 34 }], 10.0)[0].dir,
            Direction::R
        );
        assert_eq!(
            segments(&[Point { x: 0, y: 0 }, Point { x: 34, y: 94 }], 10.0)[0].dir,
            Direction::D
        );
    }

    #[test]
    fn jitter_below_min_len_is_ignored() {
        let pts = [
            Point { x: 0, y: 0 },
            Point { x: 3, y: -2 },
            Point { x: -2, y: 4 },
            Point { x: 1, y: 1 },
        ];
        assert!(segments(&pts, 10.0).is_empty());
    }

    #[test]
    fn corner_yields_two_directions() {
        let mut pts = line(10, 0, 5);
        pts.extend((1..=5).map(|i| Point { x: 50, y: 10 * i }));
        let dirs: Vec<_> = segments(&pts, 15.0).iter().map(|s| s.dir).collect();
        assert_eq!(dirs.first(), Some(&Direction::R));
        assert_eq!(dirs.last(), Some(&Direction::D));
    }

    #[test]
    fn empty_and_single_point_give_no_segments() {
        assert!(segments(&[], 10.0).is_empty());
        assert!(segments(&[Point { x: 5, y: 5 }], 10.0).is_empty());
    }

    proptest! {
        #[test]
        fn never_panics_and_segments_respect_min_len(
            pts in proptest::collection::vec((any::<i32>(), any::<i32>()), 0..64),
            min_len in 0.0f64..1000.0,
        ) {
            let pts: Vec<Point> = pts.into_iter().map(|(x, y)| Point { x, y }).collect();
            for s in segments(&pts, min_len) {
                prop_assert!(s.len >= min_len);
            }
        }
    }
}
