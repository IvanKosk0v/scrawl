//! Pure stroke recognition logic, no I/O.

/// Turn a point stream into a stroke string such as `R` or `L-U`.
///
/// Steps shorter than `min_step_len` pixels are jitter; direction runs shorter than `min_run_len`
/// pixels are noise. Returns `None` when no stroke remains.
pub fn recognize(points: &[segment::Point], min_step_len: f64, min_run_len: f64) -> Option<String> {
    stroke::stroke_string(&segment::segments(points, min_step_len), min_run_len)
}

/// Module marker, replaced as the module grows.
pub mod matcher;
pub mod segment;
pub mod stroke;

pub const MODULE: &str = "recognizer";
