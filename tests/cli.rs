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
fn stub_subcommand_reports_not_implemented() {
    let out = Command::new(env!("CARGO_BIN_EXE_strokebind"))
        .arg("list-devices")
        .output()
        .expect("run strokebind");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("not implemented"));
}
