//! M7.2: pure geometry / shape / input-region logic of the X11 overlay backend.
//! The window itself: `cargo test --test overlay_x11 -- --ignored` on an X11 session (shows a
//! stroke for about a second, no grab).

use strokebind::overlay::trail::{Trail, TrailStyle};
use strokebind::overlay::x11::{
    input_region, mask_rects, rgb_pixel, trail_bbox, window_attributes, Rect, X11Overlay,
    OVERLAY_WM_CLASS,
};
use strokebind::overlay::Overlay;

fn rect(x: i16, y: i16, width: u16, height: u16) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn translated_trail_shifts_every_point() {
    let mut t = Trail::new((10.0, 20.0));
    t.push_delta(5, 5);
    assert_eq!(
        t.translated(-10.0, -20.0).points(),
        &[(0.0, 0.0), (5.0, 5.0)]
    );
}

#[test]
fn bbox_pads_points_by_half_width_plus_one() {
    let mut t = Trail::new((100.0, 100.0));
    t.push_delta(50, 20);
    // width 4 => pad ceil(2) + 1 = 3
    assert_eq!(
        trail_bbox(&t, 4.0, (1920, 1080)),
        Some(rect(97, 97, 57, 27))
    );
}

#[test]
fn bbox_is_clipped_to_the_screen() {
    let mut t = Trail::new((5.0, 1075.0));
    t.push_delta(-50, 50);
    assert_eq!(trail_bbox(&t, 4.0, (1920, 1080)), Some(rect(0, 1072, 9, 8)));
}

#[test]
fn bbox_none_for_empty_or_offscreen_trail() {
    let mut empty = Trail::new((0.0, 0.0));
    empty.clear();
    assert_eq!(trail_bbox(&empty, 4.0, (1920, 1080)), None);
    let mut off = Trail::new((-500.0, -500.0));
    off.push_delta(10, 10);
    assert_eq!(trail_bbox(&off, 4.0, (1920, 1080)), None);
}

/// A 4x3 RGBA buffer with the given (x, y) pixels opaque.
fn buf_with(opaque: &[(usize, usize)], half: &[(usize, usize)]) -> Vec<u8> {
    let mut b = vec![0u8; 4 * 3 * 4];
    for &(x, y) in opaque {
        b[(y * 4 + x) * 4 + 3] = 255;
    }
    for &(x, y) in half {
        b[(y * 4 + x) * 4 + 3] = 100;
    }
    b
}

#[test]
fn mask_rects_merge_row_runs_and_offset_by_origin() {
    // rows 0 and 1: x 1..3 opaque; row 2: x 0 opaque, x 3 below threshold
    let b = buf_with(&[(1, 0), (2, 0), (1, 1), (2, 1), (0, 2)], &[(3, 2)]);
    assert_eq!(
        mask_rects(&b, 4, 3, (10, 20)),
        vec![rect(11, 20, 2, 2), rect(10, 22, 1, 1)]
    );
}

#[test]
fn mask_rects_empty_for_transparent_or_wrong_size_buffer() {
    assert!(mask_rects(&buf_with(&[], &[(0, 0)]), 4, 3, (0, 0)).is_empty());
    assert!(mask_rects(&[255; 7], 4, 3, (0, 0)).is_empty());
}

#[test]
fn rgb_pixel_packs_truecolor() {
    assert_eq!(rgb_pixel([0x12, 0x34, 0x56, 0x78]), 0x0012_3456);
}

#[test]
fn window_is_override_redirect_with_trail_color() {
    let aux = window_attributes(&TrailStyle {
        width_px: 4.0,
        rgba: [1, 2, 3, 255],
    });
    assert_eq!(aux.override_redirect, Some(1));
    assert_eq!(aux.background_pixel, Some(0x0001_0203));
    assert_eq!(aux.event_mask, None);
}

#[test]
fn input_region_is_empty_so_clicks_pass_through() {
    assert!(input_region().is_empty());
}

#[test]
#[ignore = "needs an X11 display; shows a stroke for about a second"]
fn x11_overlay_draws_demo_stroke() {
    let mut o = X11Overlay::new(TrailStyle::default()).unwrap();
    o.begin().unwrap();
    for _ in 0..40 {
        o.motion(6, 2).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    for _ in 0..40 {
        o.motion(0, 6).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    std::thread::sleep(std::time::Duration::from_millis(300));
    o.end().unwrap();
}

#[test]
#[ignore = "needs an X11 display; flashes a short stroke"]
fn x11_overlay_end_hides_by_shape_without_unmapping() {
    use x11rb::protocol::shape::{ConnectionExt as _, SK};
    use x11rb::protocol::xproto::{ConnectionExt as _, MapState};

    let mut o = X11Overlay::new(TrailStyle::default()).unwrap();
    o.begin().unwrap();
    o.motion(40, 40).unwrap();
    o.end().unwrap();
    // A second client sees the server state after the overlay's requests (they were flushed).
    let (conn, _) = x11rb::connect(None).unwrap();
    let attrs = conn
        .get_window_attributes(o.window())
        .unwrap()
        .reply()
        .unwrap();
    // Unmapping would play a compositor's close animation on a full-screen window.
    assert_eq!(attrs.map_state, MapState::VIEWABLE);
    let shape = conn
        .shape_get_rectangles(o.window(), SK::BOUNDING)
        .unwrap()
        .reply()
        .unwrap();
    assert!(shape.rectangles.is_empty());
}

#[test]
fn overlay_wm_class_is_strokebind_overlay() {
    let id = strokebind::x11::parse_wm_class(OVERLAY_WM_CLASS).unwrap();
    assert_eq!(id.as_str(), "strokebind-overlay");
}

#[test]
#[ignore = "needs an X11 display"]
fn x11_overlay_window_is_tagged_for_compositor_rules() {
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _};

    let o = X11Overlay::new(TrailStyle::default()).unwrap();
    let (conn, _) = x11rb::connect(None).unwrap();
    let class = conn
        .get_property(
            false,
            o.window(),
            AtomEnum::WM_CLASS,
            AtomEnum::STRING,
            0,
            64,
        )
        .unwrap()
        .reply()
        .unwrap();
    assert_eq!(class.value, OVERLAY_WM_CLASS);
    let atom = |name: &[u8]| conn.intern_atom(false, name).unwrap().reply().unwrap().atom;
    let ty = conn
        .get_property(
            false,
            o.window(),
            atom(b"_NET_WM_WINDOW_TYPE"),
            AtomEnum::ATOM,
            0,
            8,
        )
        .unwrap()
        .reply()
        .unwrap();
    let types: Vec<u32> = ty.value32().unwrap().collect();
    assert_eq!(types, vec![atom(b"_NET_WM_WINDOW_TYPE_DND")]);
}
