//! M7.1: overlay backend picker and the shared tiny-skia trail renderer.

use std::ffi::OsString;

use proptest::prelude::*;
use scrawl::overlay::trail::{render, Trail, TrailStyle, MAX_TRAIL_POINTS};
use scrawl::overlay::{overlay_backend, OverlayBackend};

/// A fake environment holding exactly `vars`.
fn env(vars: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<OsString> {
    move |k| {
        vars.iter()
            .find(|(n, _)| *n == k)
            .map(|(_, v)| OsString::from(v))
    }
}

#[test]
fn backend_nothing_set_is_none() {
    assert_eq!(overlay_backend(env(&[])), OverlayBackend::None);
}

#[test]
fn backend_display_only_is_x11() {
    assert_eq!(
        overlay_backend(env(&[("DISPLAY", ":0")])),
        OverlayBackend::X11
    );
}

#[test]
fn backend_wayland_only_is_wayland() {
    assert_eq!(
        overlay_backend(env(&[("WAYLAND_DISPLAY", "wayland-0")])),
        OverlayBackend::Wayland
    );
}

#[test]
fn backend_xwayland_both_set_is_wayland() {
    assert_eq!(
        overlay_backend(env(&[("DISPLAY", ":0"), ("WAYLAND_DISPLAY", "wayland-0")])),
        OverlayBackend::Wayland
    );
}

#[test]
fn backend_empty_wayland_display_falls_back_to_x11() {
    assert_eq!(
        overlay_backend(env(&[("WAYLAND_DISPLAY", ""), ("DISPLAY", ":0")])),
        OverlayBackend::X11
    );
}

#[test]
fn trail_accumulates_deltas_from_start() {
    let mut t = Trail::new((10.0, 20.0));
    t.push_delta(5, 0);
    t.push_delta(0, -3);
    assert_eq!(t.points(), &[(10.0, 20.0), (15.0, 20.0), (15.0, 17.0)]);
    t.clear();
    assert!(t.points().is_empty());
}

#[test]
fn push_point_appends_absolute_and_caps_at_max() {
    let mut t = Trail::new((10.0, 20.0));
    t.push_point((300.0, 5.0));
    t.push_point((300.0, 5.0));
    assert_eq!(t.points(), &[(10.0, 20.0), (300.0, 5.0)]);
    for i in 0..MAX_TRAIL_POINTS {
        t.push_point((i as f32, 0.0));
    }
    assert_eq!(t.points().len(), MAX_TRAIL_POINTS);
    assert_eq!(
        t.points().last(),
        Some(&((MAX_TRAIL_POINTS - 1) as f32, 0.0))
    );
}

const W: u32 = 64;
const H: u32 = 64;

fn buffer() -> Vec<u8> {
    vec![0; (W * H * 4) as usize]
}

/// Premultiplied RGBA of pixel (x, y).
fn px(buf: &[u8], x: u32, y: u32) -> [u8; 4] {
    let i = ((y * W + x) * 4) as usize;
    [buf[i], buf[i + 1], buf[i + 2], buf[i + 3]]
}

fn style() -> TrailStyle {
    TrailStyle {
        width_px: 4.0,
        rgba: [255, 0, 0, 255],
    }
}

fn trail(points: &[(i32, i32)]) -> Trail {
    let (first, rest) = points
        .split_first()
        .map_or(((0, 0), &[][..]), |(f, r)| (*f, r));
    let mut t = Trail::new((first.0 as f32, first.1 as f32));
    let mut prev = first;
    for &p in rest {
        t.push_delta(p.0 - prev.0, p.1 - prev.1);
        prev = p;
    }
    t
}

#[test]
fn horizontal_trail_paints_line_pixels_only() {
    let mut buf = buffer();
    render(&trail(&[(8, 32), (56, 32)]), &style(), &mut buf, W, H).unwrap();
    assert_eq!(px(&buf, 32, 32), [255, 0, 0, 255]);
    assert_eq!(px(&buf, 32, 5), [0, 0, 0, 0]);
    assert_eq!(px(&buf, 60, 60), [0, 0, 0, 0]);
}

#[test]
fn l_shaped_trail_covers_both_legs() {
    let mut buf = buffer();
    render(
        &trail(&[(10, 10), (10, 50), (50, 50)]),
        &style(),
        &mut buf,
        W,
        H,
    )
    .unwrap();
    assert_eq!(px(&buf, 10, 30)[3], 255);
    assert_eq!(px(&buf, 30, 50)[3], 255);
    assert_eq!(px(&buf, 40, 20), [0, 0, 0, 0]);
}

#[test]
fn empty_and_single_point_trails_leave_buffer_transparent() {
    for t in [Trail::new((32.0, 32.0)), {
        let mut t = Trail::new((32.0, 32.0));
        t.clear();
        t
    }] {
        let mut buf = vec![7; (W * H * 4) as usize];
        render(&t, &style(), &mut buf, W, H).unwrap();
        assert!(buf.iter().all(|&b| b == 0));
    }
}

#[test]
fn rerender_clears_previous_frame() {
    let mut buf = buffer();
    render(&trail(&[(8, 10), (56, 10)]), &style(), &mut buf, W, H).unwrap();
    render(&trail(&[(8, 50), (56, 50)]), &style(), &mut buf, W, H).unwrap();
    assert_eq!(px(&buf, 32, 10), [0, 0, 0, 0]);
    assert_eq!(px(&buf, 32, 50)[3], 255);
}

#[test]
fn wrong_buffer_size_is_an_error() {
    let mut buf = vec![0; 10];
    assert!(render(&trail(&[(0, 0), (5, 5)]), &style(), &mut buf, W, H).is_err());
    let mut empty = Vec::new();
    assert!(render(&trail(&[(0, 0), (5, 5)]), &style(), &mut empty, 0, 0).is_err());
}

#[test]
fn invalid_width_is_an_error() {
    for width_px in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        let mut buf = buffer();
        let bad = TrailStyle {
            width_px,
            ..style()
        };
        assert!(
            render(&trail(&[(8, 32), (56, 32)]), &bad, &mut buf, W, H).is_err(),
            "width {width_px} accepted"
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    fn render_never_panics(points in prop::collection::vec((-100_000i32..100_000, -100_000i32..100_000), 0..40)) {
        let mut buf = buffer();
        prop_assert!(render(&trail(&points), &style(), &mut buf, W, H).is_ok());
    }
}

// M7.5: HiDPI. The trail lives in logical pixels; a scale-`s` buffer gets every point and the
// line width multiplied by `s`.

#[test]
fn scaled_trail_multiplies_every_point() {
    let t = trail(&[(1, 2), (5, -3)]).scaled(2.0);
    assert_eq!(t.points(), &[(2.0, 4.0), (10.0, -6.0)]);
}

#[test]
fn scaled_style_multiplies_the_width_only() {
    let s = style().scaled(3.0);
    assert_eq!(s.width_px, 12.0);
    assert_eq!(s.rgba, style().rgba);
}

#[test]
fn scale_two_draws_at_doubled_position_and_twice_as_thick() {
    // Logical 32x32 surface, line at logical y = 16, width 4: physical y = 32, width 8 (28..36).
    let mut hi = buffer();
    let logical = trail(&[(4, 16), (28, 16)]);
    render(&logical.scaled(2.0), &style().scaled(2.0), &mut hi, W, H).unwrap();
    assert_eq!(px(&hi, 32, 32), [255, 0, 0, 255], "doubled position");
    assert_eq!(px(&hi, 32, 16)[3], 0, "the unscaled position stays empty");
    assert_ne!(
        px(&hi, 32, 35)[3],
        0,
        "3 px off the center: inside an 8 px line"
    );

    // The same line unscaled (width 4: 30..34) leaves that pixel empty.
    let mut lo = buffer();
    render(&trail(&[(8, 32), (56, 32)]), &style(), &mut lo, W, H).unwrap();
    assert_eq!(px(&lo, 32, 35)[3], 0);
}

// Review fix: a trigger held for a long time must not make every frame slower forever.

#[test]
fn trail_keeps_only_the_newest_points() {
    let mut t = Trail::new((0.0, 0.0));
    for _ in 0..MAX_TRAIL_POINTS * 3 {
        t.push_delta(1, 0);
    }
    assert_eq!(t.points().len(), MAX_TRAIL_POINTS);
    let n = (MAX_TRAIL_POINTS * 3) as f32;
    assert_eq!(t.points().last(), Some(&(n, 0.0)), "newest point kept");
    assert_eq!(
        t.points().first(),
        Some(&(n - (MAX_TRAIL_POINTS - 1) as f32, 0.0)),
        "oldest points dropped"
    );
}
