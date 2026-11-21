//! The stroke trail and its tiny-skia renderer (pure: draws into a caller-owned buffer).

use tiny_skia::{Color, LineCap, LineJoin, Paint, PathBuilder, PixmapMut, Stroke, Transform};

use crate::error::Error;

/// Most points a [`Trail`] keeps; older ones are dropped so each frame's cost stays bounded
/// while the trigger is held for a long time.
pub const MAX_TRAIL_POINTS: usize = 2000;

/// Points of the stroke being drawn, in surface pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct Trail {
    points: Vec<(f32, f32)>,
}

impl Trail {
    /// A trail holding only its start point.
    pub fn new(start: (f32, f32)) -> Self {
        Self {
            points: vec![start],
        }
    }

    /// Appends the point `(dx, dy)` away from the last one (a relative mouse motion). An empty
    /// trail starts at the origin. Beyond [`MAX_TRAIL_POINTS`] the oldest point is dropped.
    pub fn push_delta(&mut self, dx: i32, dy: i32) {
        let (x, y) = self.points.last().copied().unwrap_or((0.0, 0.0));
        if self.points.len() >= MAX_TRAIL_POINTS {
            self.points.remove(0);
        }
        self.points.push((x + dx as f32, y + dy as f32));
    }

    /// Removes every point.
    pub fn clear(&mut self) {
        self.points.clear();
    }

    /// A copy with every point moved by `(dx, dy)` (e.g. into a smaller buffer's coordinates).
    pub fn translated(&self, dx: f32, dy: f32) -> Trail {
        Trail {
            points: self.points.iter().map(|&(x, y)| (x + dx, y + dy)).collect(),
        }
    }

    /// A copy with every point multiplied by `s` (logical pixels into a scale-`s` buffer).
    pub fn scaled(&self, s: f32) -> Trail {
        Trail {
            points: self.points.iter().map(|&(x, y)| (x * s, y * s)).collect(),
        }
    }

    /// The points, oldest first.
    pub fn points(&self) -> &[(f32, f32)] {
        &self.points
    }
}

/// How the trail is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrailStyle {
    /// Line width in pixels.
    pub width_px: f32,
    /// Straight (not premultiplied) RGBA color.
    pub rgba: [u8; 4],
}

impl TrailStyle {
    /// The same style for a scale-`s` buffer: the line width multiplied by `s`, same color.
    pub fn scaled(&self, s: f32) -> TrailStyle {
        TrailStyle {
            width_px: self.width_px * s,
            rgba: self.rgba,
        }
    }
}

impl Default for TrailStyle {
    fn default() -> Self {
        Self {
            width_px: 4.0,
            rgba: [0x33, 0x99, 0xff, 0xdd],
        }
    }
}

/// Clears `buf` (premultiplied RGBA, `width * height * 4` bytes, row-major) to transparent and
/// draws `trail` as an anti-aliased polyline with round caps and joins. Fewer than two points
/// draw nothing. Errors when the buffer size does not match or `style.width_px` is not a finite
/// positive number (tiny-skia would silently draw nothing).
pub fn render(
    trail: &Trail,
    style: &TrailStyle,
    buf: &mut [u8],
    width: u32,
    height: u32,
) -> Result<(), Error> {
    if !(style.width_px.is_finite() && style.width_px > 0.0) {
        return Err(Error::Overlay(format!(
            "trail width must be a positive number, got {}",
            style.width_px
        )));
    }
    let len = buf.len();
    let mut pixmap = PixmapMut::from_bytes(buf, width, height).ok_or_else(|| {
        Error::Overlay(format!(
            "pixel buffer of {len} bytes does not fit {width}x{height} RGBA"
        ))
    })?;
    pixmap.fill(Color::TRANSPARENT);

    let Some(((x0, y0), rest)) = trail.points().split_first() else {
        return Ok(());
    };
    if rest.is_empty() {
        return Ok(());
    }
    let mut pb = PathBuilder::new();
    pb.move_to(*x0, *y0);
    for &(x, y) in rest {
        pb.line_to(x, y);
    }
    // A path of identical points has no bounds; nothing to draw.
    let Some(path) = pb.finish() else {
        return Ok(());
    };

    let [r, g, b, a] = style.rgba;
    let mut paint = Paint::default();
    paint.set_color_rgba8(r, g, b, a);
    paint.anti_alias = true;
    let stroke = Stroke {
        width: style.width_px,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };
    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    Ok(())
}
