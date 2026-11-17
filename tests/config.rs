//! M0.3: config parsing and validation.

use strokebind::config::{Action, Config, Trigger};
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
    let (_, msg) = config_error("min_stroke_px = 40\n");
    assert!(msg.contains("trigger"), "message: {msg}");
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
