//! M0.3: config parsing and validation.

use strokebind::config::{Action, Config, Feedback, MouseButton, Trigger};
use strokebind::error::Error;

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
