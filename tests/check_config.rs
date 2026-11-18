//! M4.4: `check-config` output is compared exactly against golden files in `tests/check_config/`.
//!
//! `ok_<name>.toml` must exit 0 and print `ok_<name>.out` on stdout; `err_<name>.toml` must exit 1,
//! print nothing on stdout and `err_<name>.out` on stderr.
#![allow(clippy::expect_used)] // test helpers outside #[test] fns

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/check_config")
}

/// Run `check-config <file>` from the fixture directory so printed paths are stable.
fn check_config(file: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_strokebind"))
        .current_dir(fixtures())
        .args(["check-config", file])
        .output()
        .expect("run strokebind")
}

fn golden(name: &str) -> String {
    let path = fixtures().join(format!("{name}.out"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn assert_golden(name: &str) {
    let out = check_config(&format!("{name}.toml"));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    if name.starts_with("ok_") {
        assert_eq!(out.status.code(), Some(0), "{name}: stderr:\n{stderr}");
        assert_eq!(stdout, golden(name), "{name}: stdout");
        assert_eq!(stderr, "", "{name}: stderr");
    } else {
        assert_eq!(out.status.code(), Some(1), "{name}: stdout:\n{stdout}");
        assert_eq!(stderr, golden(name), "{name}: stderr");
        assert_eq!(stdout, "", "{name}: stdout");
    }
}

#[test]
fn every_fixture_has_a_golden_file() {
    let mut n = 0;
    for entry in std::fs::read_dir(fixtures()).expect("fixture dir") {
        let path = entry.expect("dir entry").path();
        if path.extension().is_some_and(|e| e == "toml") {
            assert!(path.with_extension("out").exists(), "{}", path.display());
            n += 1;
        }
    }
    assert_eq!(n, 9, "update the test list below when adding fixtures");
}

#[test]
fn ok_minimal() {
    assert_golden("ok_minimal");
}

#[test]
fn ok_full() {
    assert_golden("ok_full");
}

#[test]
fn err_syntax() {
    assert_golden("err_syntax");
}

#[test]
fn err_unknown_stroke() {
    assert_golden("err_unknown_stroke");
}

#[test]
fn err_unknown_field() {
    assert_golden("err_unknown_field");
}

#[test]
fn err_missing_trigger() {
    assert_golden("err_missing_trigger");
}

#[test]
fn err_bad_key_combo() {
    assert_golden("err_bad_key_combo");
}

#[test]
fn err_empty_run() {
    assert_golden("err_empty_run");
}

#[test]
fn err_bad_escape() {
    assert_golden("err_bad_escape");
}

#[test]
fn missing_file_is_a_clear_error() {
    let out = check_config("nope.toml");
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.starts_with("error: nope.toml: "), "{stderr}");
}
