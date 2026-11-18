//! M0.2: the CLI lists its subcommands in `--help`.

use std::process::Command;

#[test]
fn help_lists_subcommands() {
    let out = Command::new(env!("CARGO_BIN_EXE_strokebind"))
        .arg("--help")
        .output()
        .expect("run strokebind");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    for cmd in ["record", "run", "check-config", "list-devices"] {
        assert!(text.contains(cmd), "--help is missing `{cmd}`:\n{text}");
    }
}

#[test]
fn check_config_requires_a_file() {
    let out = Command::new(env!("CARGO_BIN_EXE_strokebind"))
        .arg("check-config")
        .output()
        .expect("run strokebind");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("<FILE>"));
}

#[test]
fn record_help_lists_options() {
    let out = Command::new(env!("CARGO_BIN_EXE_strokebind"))
        .args(["record", "--help"])
        .output()
        .expect("run strokebind");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    for opt in ["--device", "--config"] {
        assert!(
            text.contains(opt),
            "record --help is missing `{opt}`:\n{text}"
        );
    }
}

#[test]
fn run_help_lists_options() {
    let out = Command::new(env!("CARGO_BIN_EXE_strokebind"))
        .args(["run", "--help"])
        .output()
        .expect("run strokebind");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    for opt in ["--device", "--config", "--timeout"] {
        assert!(text.contains(opt), "run --help is missing `{opt}`:\n{text}");
    }
}

#[test]
fn run_rejects_zero_timeout() {
    let out = Command::new(env!("CARGO_BIN_EXE_strokebind"))
        .args(["run", "--timeout", "0"])
        .output()
        .expect("run strokebind");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("--timeout"));
}
