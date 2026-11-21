//! M7.3: non-protocol logic of the Wayland (wlr-layer-shell) overlay backend.
//! The surface itself: `cargo test --test overlay_wayland -- --ignored` on Sway, Hyprland or KDE
//! (shows a stroke from screen center for about a second, no grab).

use smithay_client_toolkit::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer};
use strokebind::overlay::trail::TrailStyle;
use strokebind::overlay::wayland::{
    center, input_region_rects, layer_settings, no_layer_shell_error, rgba_to_argb8888,
    WaylandOverlay, LAYER_NAMESPACE,
};
use strokebind::overlay::Overlay;

#[test]
fn center_of_even_and_odd_sizes() {
    assert_eq!(center((1920, 1080)), (960.0, 540.0));
    assert_eq!(center((1365, 767)), (682.5, 383.5));
    assert_eq!(center((0, 0)), (0.0, 0.0));
}

#[test]
fn rgba_becomes_little_endian_argb8888() {
    let mut buf = vec![0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd];
    rgba_to_argb8888(&mut buf);
    // u32 0xAARRGGBB in little-endian = B, G, R, A
    assert_eq!(buf, vec![0x33, 0x22, 0x11, 0x44, 0xcc, 0xbb, 0xaa, 0xdd]);
    assert_eq!(
        u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
        0x4411_2233
    );
}

#[test]
fn rgba_conversion_leaves_a_partial_tail_alone() {
    let mut buf = vec![1, 2, 3, 4, 5, 6];
    rgba_to_argb8888(&mut buf);
    assert_eq!(buf, vec![3, 2, 1, 4, 5, 6]);
}

#[test]
fn layer_surface_covers_the_output_above_everything_without_focus() {
    let s = layer_settings();
    assert_eq!(s.layer, Layer::Overlay);
    assert_eq!(
        s.anchor,
        Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT
    );
    assert_eq!(s.exclusive_zone, -1);
    assert_eq!(s.keyboard, KeyboardInteractivity::None);
    assert_eq!(s.namespace, LAYER_NAMESPACE);
    assert_eq!(LAYER_NAMESPACE, "strokebind-overlay");
}

#[test]
fn input_region_is_empty_so_clicks_pass_through() {
    assert!(input_region_rects().is_empty());
}

#[test]
fn missing_layer_shell_points_to_notify_feedback() {
    let msg = no_layer_shell_error().to_string();
    assert!(msg.contains("wlr-layer-shell"), "{msg}");
    assert!(msg.contains("GNOME"), "{msg}");
    assert!(msg.contains("feedback = \"notify\""), "{msg}");
}

#[test]
#[ignore = "needs a Wayland compositor with wlr-layer-shell; shows a stroke for about a second"]
fn wayland_overlay_draws_demo_stroke() {
    let mut o = WaylandOverlay::new(TrailStyle::default()).unwrap();
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
    std::thread::sleep(std::time::Duration::from_millis(200));
}
