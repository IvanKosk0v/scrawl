//! M5.9: the README support matrix has one line per window backend.

const README: &str = include_str!("../README.md");

fn row(name: &str) -> &'static str {
    README
        .lines()
        .find(|l| l.starts_with('|') && l.contains(name))
        .unwrap_or_else(|| panic!("no support-matrix row for {name}"))
}

#[test]
fn matrix_has_a_row_per_backend() {
    assert!(row("X11").contains("`_NET_ACTIVE_WINDOW`"));
    assert!(row("Sway").contains("`SWAYSOCK`"));
    assert!(row("Hyprland").contains("`HYPRLAND_INSTANCE_SIGNATURE`"));
}

#[test]
fn kde_and_gnome_are_global_only() {
    for name in ["KDE Plasma", "GNOME"] {
        assert!(row(name).contains("global bindings only"), "{name}");
    }
}
