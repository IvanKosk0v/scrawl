//! M0.3: config parsing and validation.

use scrawl::config::{Action, Config, Feedback, MouseButton, Trigger};
use scrawl::error::Error;

const VALID: &str = r#"
trigger = "right"
min_stroke_px = 50

[[binding]]
stroke = "L"
action = { key = "alt+left" }

[[binding]]
stroke = "DR"
action = { run = ["notify-send", "hello"] }

[[binding]]
stroke = "L-U"
action = { key = "ctrl+w" }
"#;

fn config_error(src: &str) -> (Option<usize>, String) {
    match Config::parse(src) {
        Err(Error::Config { line, message }) => (line, message),
        other => panic!("expected config error, got {other:?}"),
    }
}

#[test]
fn valid_config_parses() {
    let cfg = Config::parse(VALID).expect("valid config");
    assert_eq!(cfg.trigger, Trigger::Right);
    assert_eq!(cfg.min_stroke_px, 50);
    assert_eq!(cfg.binding.len(), 3);
    assert_eq!(cfg.binding[0].stroke.as_str(), "L");
    assert_eq!(cfg.binding[0].action, Action::Key("alt+left".into()));
    assert_eq!(
        cfg.binding[1].action,
        Action::Run(vec!["notify-send".into(), "hello".into()])
    );
    assert_eq!(cfg.binding[2].stroke.as_str(), "L-U");
}

#[test]
fn min_stroke_px_defaults_to_40() {
    let cfg = Config::parse("trigger = \"middle\"\n").expect("minimal config");
    assert_eq!(cfg.min_stroke_px, 40);
    assert!(cfg.binding.is_empty());
}

#[test]
fn missing_field_reports_line_and_name() {
    let src = "trigger = \"right\"\n\n[[binding]]\nstroke = \"L\"\n";
    let (line, msg) = config_error(src);
    assert_eq!(line, Some(3));
    assert!(msg.contains("action"), "message: {msg}");
}

#[test]
fn missing_trigger_is_an_error() {
    let (line, msg) = config_error("min_stroke_px = 40\n");
    assert!(msg.contains("trigger"), "message: {msg}");
    // The whole document is at fault, not its first line.
    assert_eq!(line, None);
}

#[test]
fn unknown_stroke_string_reports_line() {
    let src = "trigger = \"right\"\n\n[[binding]]\nstroke = \"LX\"\naction = { key = \"a\" }\n";
    let (line, msg) = config_error(src);
    assert_eq!(line, Some(4));
    assert!(msg.contains("LX"), "message: {msg}");
}

#[test]
fn syntax_error_reports_line() {
    let (line, _) = config_error("trigger = \"right\"\nmin_stroke_px = = 3\n");
    assert_eq!(line, Some(2));
}

#[test]
fn typo_in_key_is_rejected_with_line() {
    let (line, msg) = config_error("trigger = \"right\"\nmin_strok_px = 10\n");
    assert_eq!(line, Some(2));
    assert!(msg.contains("min_strok_px"), "message: {msg}");
}

#[test]
fn typo_in_table_name_is_rejected_with_line() {
    let src = "trigger = \"right\"\n\n[[bindings]]\nstroke = \"L\"\naction = { key = \"a\" }\n";
    let (line, msg) = config_error(src);
    assert_eq!(line, Some(3));
    assert!(msg.contains("bindings"), "message: {msg}");
}

#[test]
fn typo_in_binding_key_is_rejected() {
    let src = "trigger = \"right\"\n\n[[binding]]\nstroke = \"L\"\naction = { key = \"a\" }\nstrok = \"R\"\n";
    let (line, msg) = config_error(src);
    assert_eq!(line, Some(6));
    assert!(msg.contains("strok"), "message: {msg}");
}

#[test]
fn escape_defaults_to_left_right_middle() {
    let cfg = Config::parse("trigger = \"right\"\n").expect("valid");
    assert_eq!(
        cfg.escape.buttons(),
        &[MouseButton::Left, MouseButton::Right, MouseButton::Middle]
    );
}

#[test]
fn escape_parses_custom_chord() {
    let cfg = Config::parse("trigger = \"right\"\nescape = \"side+extra\"\n").expect("valid");
    assert_eq!(
        cfg.escape.buttons(),
        &[MouseButton::Side, MouseButton::Extra]
    );
}

#[test]
fn escape_rejects_unknown_button_with_line() {
    let (line, msg) = config_error("trigger = \"right\"\nescape = \"left+banana\"\n");
    assert_eq!(line, Some(2));
    assert!(msg.contains("banana"), "{msg}");
}

#[test]
fn escape_rejects_fewer_than_two_distinct_buttons() {
    for chord in ["left", "left+left", ""] {
        let (line, msg) = config_error(&format!("trigger = \"right\"\nescape = \"{chord}\"\n"));
        assert_eq!(line, Some(2), "{chord}");
        assert!(msg.contains("two"), "{chord}: {msg}");
    }
}

#[test]
fn invalid_key_combo_is_rejected_at_load_with_line() {
    let src = "trigger = \"right\"\n\n[[binding]]\nstroke = \"L\"\naction = { key = \"ctrl+nosuchkey\" }\n";
    let (line, msg) = config_error(src);
    assert_eq!(line, Some(5));
    assert!(msg.contains("nosuchkey"), "message: {msg}");
}

#[test]
fn empty_run_command_is_rejected_at_load_with_line() {
    for run in ["[]", "[\"\"]"] {
        let src = format!(
            "trigger = \"right\"\n\n[[binding]]\nstroke = \"L\"\naction = {{ run = {run} }}\n"
        );
        let (line, msg) = config_error(&src);
        assert_eq!(line, Some(5), "run = {run}");
        assert!(msg.contains("program"), "message: {msg}");
    }
}

const PROFILES: &str = r#"
trigger = "right"
min_stroke_px = 40

[[binding]]
stroke = "L"
action = { key = "alt+left" }

[[binding]]
stroke = "R"
action = { key = "alt+right" }

[[profile]]
device = "trackball"
trigger = "middle"
min_stroke_px = 80

[[profile.binding]]
stroke = "L"
action = { key = "ctrl+w" }
"#;

#[test]
fn unmatched_device_gets_the_global_settings() {
    let cfg = Config::parse(PROFILES).expect("valid");
    let p = cfg.resolve("Logitech G502");
    assert_eq!(p.trigger, Trigger::Right);
    assert_eq!(p.min_stroke_px, 40);
    assert_eq!(p.bindings.len(), 2);
}

#[test]
fn profile_matches_device_name_substring_case_insensitively() {
    let cfg = Config::parse(PROFILES).expect("valid");
    let p = cfg.resolve("Kensington TrackBall Pro");
    assert_eq!(p.trigger, Trigger::Middle);
    assert_eq!(p.min_stroke_px, 80);
}

#[test]
fn profile_bindings_override_global_ones_for_the_same_stroke() {
    let cfg = Config::parse(PROFILES).expect("valid");
    let p = cfg.resolve("trackball");
    let strokes: Vec<(&str, &Action)> = p
        .bindings
        .iter()
        .map(|b| (b.stroke.as_str(), &b.action))
        .collect();
    assert_eq!(
        strokes,
        vec![
            ("L", &Action::Key("ctrl+w".into())),
            ("R", &Action::Key("alt+right".into())),
        ]
    );
}

#[test]
fn profile_without_overrides_inherits_global_settings() {
    let src = "trigger = \"side\"\nmin_stroke_px = 60\n\n[[profile]]\ndevice = \"x\"\n";
    let cfg = Config::parse(src).expect("valid");
    let p = cfg.resolve("x");
    assert_eq!(p.trigger, Trigger::Side);
    assert_eq!(p.min_stroke_px, 60);
}

#[test]
fn profile_requires_device_and_rejects_bad_min_stroke() {
    let (line, msg) = config_error("trigger = \"right\"\n\n[[profile]]\ntrigger = \"middle\"\n");
    assert_eq!(line, Some(3));
    assert!(msg.contains("device"), "message: {msg}");
}

#[test]
fn feedback_defaults_to_off() {
    let cfg = Config::parse("trigger = \"right\"\n").expect("valid");
    assert_eq!(cfg.feedback, Feedback::Off);
}

#[test]
fn feedback_notify_parses() {
    let cfg = Config::parse("trigger = \"right\"\nfeedback = \"notify\"\n").expect("valid");
    assert_eq!(cfg.feedback, Feedback::Notify);
    let cfg = Config::parse("trigger = \"right\"\nfeedback = \"off\"\n").expect("valid");
    assert_eq!(cfg.feedback, Feedback::Off);
}

#[test]
fn feedback_rejects_unknown_value_with_line() {
    let (line, msg) = config_error("trigger = \"right\"\nfeedback = \"popup\"\n");
    assert_eq!(line, Some(2));
    assert!(msg.contains("notify"), "message: {msg}");
}

#[test]
fn resolve_carries_feedback() {
    let src = format!("feedback = \"notify\"\n{PROFILES}");
    let cfg = Config::parse(&src).expect("valid");
    assert_eq!(cfg.resolve("trackball").feedback, Feedback::Notify);
    assert_eq!(cfg.resolve("other").feedback, Feedback::Notify);
}

#[test]
fn overlay_defaults_to_off_and_parses() {
    let cfg = Config::parse("trigger = \"right\"\n").expect("valid");
    assert!(!cfg.overlay);
    let cfg = Config::parse("trigger = \"right\"\noverlay = true\n").expect("valid");
    assert!(cfg.overlay);
}

#[test]
fn resolve_carries_overlay() {
    let src = format!("overlay = true\n{PROFILES}");
    let cfg = Config::parse(&src).expect("valid");
    assert!(cfg.resolve("trackball").overlay);
    assert!(cfg.resolve("other").overlay);
}

#[test]
fn overlay_is_global_only() {
    let (line, msg) =
        config_error("trigger = \"right\"\n\n[[profile]]\ndevice = \"x\"\noverlay = true\n");
    assert!(line.is_some(), "message: {msg}");
    assert!(msg.contains("overlay"), "message: {msg}");
}

#[test]
fn live_motion_defaults() {
    let cfg = Config::parse("trigger = \"right\"\n").expect("valid");
    assert!(!cfg.freeze_pointer);
    assert_eq!(cfg.click_slop_px, 6);
}

#[test]
fn freeze_pointer_and_slop_parse() {
    let cfg = Config::parse("trigger = \"right\"\nfreeze_pointer = true\nclick_slop_px = 3\n")
        .expect("valid");
    assert!(cfg.freeze_pointer);
    assert_eq!(cfg.click_slop_px, 3);
}

#[test]
fn slop_must_be_below_min_stroke_px_with_line() {
    let (line, msg) = config_error("trigger = \"right\"\n\nclick_slop_px = 40\n");
    assert_eq!(line, Some(3));
    assert_eq!(
        msg,
        "click_slop_px (40) must be less than min_stroke_px (40)"
    );
    // Default slop against a too-small min_stroke_px points at min_stroke_px.
    let (line, msg) = config_error("trigger = \"right\"\nmin_stroke_px = 5\n");
    assert_eq!(line, Some(2));
    assert!(msg.contains("(6)") && msg.contains("(5)"), "message: {msg}");
}

#[test]
fn slop_checked_against_profile_min_stroke_px() {
    let src = "trigger = \"right\"\n[[profile]]\ndevice = \"trackball\"\nmin_stroke_px = 5\n";
    let (line, msg) = config_error(src);
    assert_eq!(
        msg,
        "click_slop_px (6) must be less than min_stroke_px (5) in profile \"trackball\""
    );
    assert_eq!(line, Some(4));
}

#[test]
fn slop_error_points_at_the_offending_profile_not_global_keys() {
    let src = "trigger = \"right\"\nmin_stroke_px = 40\nclick_slop_px = 8\n\n\
               [[profile]]\ndevice = \"a\"\nmin_stroke_px = 20\n\n\
               [[profile]]\ndevice = \"b\"\nmin_stroke_px = 8\n";
    let (line, msg) = config_error(src);
    assert!(msg.contains("profile \"b\""), "message: {msg}");
    assert_eq!(line, Some(11));
}

#[test]
fn slop_is_not_checked_when_the_pointer_is_frozen() {
    let src = "trigger = \"right\"\nfreeze_pointer = true\nmin_stroke_px = 5\n";
    let cfg = Config::parse(src).expect("slop is unused in frozen mode");
    assert_eq!(cfg.min_stroke_px, 5);
}

#[test]
fn resolve_carries_live_settings() {
    let src = format!("freeze_pointer = true\nclick_slop_px = 4\n{PROFILES}");
    let cfg = Config::parse(&src).expect("valid");
    let r = cfg.resolve("trackball");
    assert!(r.freeze_pointer);
    assert_eq!(r.click_slop_px, 4);
}

#[test]
fn empty_profile_device_is_rejected_with_line() {
    for device in ["\"\"", "\"  \""] {
        let src = format!("trigger = \"right\"\n\n[[profile]]\ndevice = {device}\n");
        let (line, msg) = config_error(&src);
        assert_eq!(line, Some(4), "device = {device}");
        assert!(msg.contains("device"), "message: {msg}");
    }
}

// ---- M5.2: per-app bindings and profiles ----

use scrawl::window::AppId;

const APPS: &str = r#"
trigger = "right"

[[binding]]
stroke = "L"
action = { key = "alt+left" }

[[binding]]
stroke = "L"
app = "Firefox"
action = { key = "ctrl+shift+tab" }

[[profile]]
device = "trackball"

[[profile.binding]]
stroke = "R"
action = { key = "ctrl+w" }

[[profile.binding]]
stroke = "R"
app = "kitty"
action = { key = "ctrl+shift+w" }

[[profile]]
app = "org.gnome.Nautilus"

[[profile.binding]]
stroke = "U"
action = { key = "alt+up" }

[[profile]]
app = "kitty"
device = "trackball"

[[profile.binding]]
stroke = "R"
action = { key = "ctrl+d" }

[[profile.binding]]
stroke = "D"
action = { key = "ctrl+shift+d" }
"#;

#[allow(clippy::expect_used)] // test helper outside #[test] fns
fn app(s: &str) -> AppId {
    AppId::new(s).expect("non-blank")
}

/// (stroke, key) pairs that apply to `device` while `focused` has focus.
fn keys(cfg: &Config, device: &str, focused: Option<&str>) -> Vec<(String, String)> {
    let id = focused.map(app);
    cfg.resolve(device)
        .bindings_for(id.as_ref())
        .into_iter()
        .map(|b| match b.action {
            Action::Key(k) => (b.stroke.as_str().to_string(), k),
            Action::Run(argv) => (b.stroke.as_str().to_string(), argv.join(" ")),
        })
        .collect()
}

fn pairs(v: &[(&str, &str)]) -> Vec<(String, String)> {
    v.iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

#[test]
fn app_binding_applies_only_to_its_app_case_insensitively() {
    let cfg = Config::parse(APPS).expect("valid");
    assert_eq!(
        keys(&cfg, "mouse", Some("firefox")),
        pairs(&[("L", "ctrl+shift+tab")])
    );
    assert_eq!(
        keys(&cfg, "mouse", Some("FIREFOX")),
        pairs(&[("L", "ctrl+shift+tab")])
    );
    // A substring is not a match: app ids are compared whole.
    assert_eq!(
        keys(&cfg, "mouse", Some("firefox-esr")),
        pairs(&[("L", "alt+left")])
    );
}

#[test]
fn unknown_or_absent_app_falls_back_to_global_bindings() {
    let cfg = Config::parse(APPS).expect("valid");
    assert_eq!(keys(&cfg, "mouse", None), pairs(&[("L", "alt+left")]));
    assert_eq!(
        keys(&cfg, "mouse", Some("gimp")),
        pairs(&[("L", "alt+left")])
    );
}

#[test]
fn app_profile_without_device_applies_to_every_mouse() {
    let cfg = Config::parse(APPS).expect("valid");
    let want = pairs(&[("U", "alt+up"), ("L", "alt+left")]);
    assert_eq!(keys(&cfg, "mouse", Some("org.gnome.nautilus")), want);
    let want = pairs(&[("U", "alt+up"), ("R", "ctrl+w"), ("L", "alt+left")]);
    assert_eq!(keys(&cfg, "trackball", Some("org.gnome.Nautilus")), want);
}

#[test]
fn app_profile_with_device_needs_both_to_match() {
    let cfg = Config::parse(APPS).expect("valid");
    // Other mouse: the kitty+trackball profile does not apply.
    assert_eq!(
        keys(&cfg, "mouse", Some("kitty")),
        pairs(&[("L", "alt+left")])
    );
    // Precedence for R on the trackball in kitty: device-profile app binding beats the app profile.
    let want = pairs(&[
        ("R", "ctrl+shift+w"),
        ("D", "ctrl+shift+d"),
        ("L", "alt+left"),
    ]);
    assert_eq!(keys(&cfg, "Kensington Trackball", Some("kitty")), want);
}

#[test]
fn app_specific_bindings_rank_above_device_profile_ones() {
    let src = r#"
trigger = "right"
[[binding]]
stroke = "L"
app = "kitty"
action = { key = "a" }
[[profile]]
device = "trackball"
[[profile.binding]]
stroke = "L"
action = { key = "b" }
"#;
    let cfg = Config::parse(src).expect("valid");
    assert_eq!(keys(&cfg, "trackball", Some("kitty")), pairs(&[("L", "a")]));
    assert_eq!(keys(&cfg, "trackball", None), pairs(&[("L", "b")]));
}

#[test]
fn all_bindings_include_app_ones_so_the_keyboard_is_prepared() {
    let src = "trigger = \"right\"\n[[profile]]\napp = \"kitty\"\n[[profile.binding]]\nstroke = \"L\"\naction = { key = \"a\" }\n";
    let cfg = Config::parse(src).expect("valid");
    let r = cfg.resolve("mouse");
    assert_eq!(r.bindings.len(), 1);
    assert!(r.bindings_for(None).is_empty());
    assert_eq!(r.bindings_for(Some(&app("kitty"))).len(), 1);
}

#[test]
fn configs_without_app_behave_as_before() {
    let cfg = Config::parse(PROFILES).expect("valid");
    let r = cfg.resolve("trackball");
    assert_eq!(r.bindings_for(None), r.bindings);
    assert_eq!(r.bindings_for(Some(&app("anything"))), r.bindings);
}

#[test]
fn blank_app_is_rejected_with_line() {
    for (src, want) in [
        ("trigger = \"right\"\n[[binding]]\nstroke = \"L\"\napp = \" \"\naction = { key = \"a\" }\n", 4),
        ("trigger = \"right\"\n\n[[profile]]\napp = \"\"\n", 4),
    ] {
        let (line, msg) = config_error(src);
        assert_eq!(line, Some(want), "{src}");
        assert!(msg.contains("app"), "message: {msg}");
    }
}

#[test]
fn profile_needs_device_or_app() {
    let (line, msg) = config_error("trigger = \"right\"\n\n[[profile]]\n\n[[profile.binding]]\nstroke = \"L\"\naction = { key = \"a\" }\n");
    assert_eq!(line, Some(3));
    assert!(
        msg.contains("device") && msg.contains("app"),
        "message: {msg}"
    );
}

#[test]
fn app_profile_rejects_restart_only_settings() {
    for key in [
        "trigger = \"middle\"",
        "min_stroke_px = 80",
        "escape = \"left+right\"",
    ] {
        let src = format!("trigger = \"right\"\n\n[[profile]]\napp = \"kitty\"\n{key}\n");
        let (line, msg) = config_error(&src);
        assert_eq!(line, Some(3), "{key}");
        assert!(msg.contains("app"), "message: {msg}");
    }
}

#[test]
fn app_on_both_profile_and_its_binding_is_rejected() {
    let src = "trigger = \"right\"\n\n[[profile]]\napp = \"kitty\"\n[[profile.binding]]\nstroke = \"L\"\napp = \"kitty\"\naction = { key = \"a\" }\n";
    let (line, msg) = config_error(src);
    assert_eq!(line, Some(3));
    assert!(msg.contains("app"), "message: {msg}");
}
