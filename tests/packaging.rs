//! M6.1: systemd user unit, udev rule, `install.sh` (nothing runs as root).
#![allow(clippy::expect_used)] // test helpers outside #[test] fns

use std::path::{Path, PathBuf};
use std::process::Command;

const UNIT: &str = include_str!("../packaging/strokebind.service");
const RULE: &str = include_str!("../packaging/70-strokebind.rules");
const INSTALL: &str = include_str!("../packaging/install.sh");

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn unit_is_a_user_service_running_the_daemon() {
    assert!(
        UNIT.contains("ExecStart=%h/.local/bin/strokebind run --config %E/strokebind/config.toml")
    );
    assert!(UNIT.contains("WantedBy=graphical-session.target"));
    assert!(UNIT.contains("PartOf=graphical-session.target"));
    assert!(UNIT.contains("Restart=on-failure"));
    // A user unit must never switch user or ask for privileges.
    for banned in ["User=", "Group=", "AmbientCapabilities", "sudo"] {
        assert!(!UNIT.contains(banned), "unit must not contain {banned}");
    }
}

#[test]
fn rule_grants_the_seat_user_uinput_and_mice_via_uaccess() {
    let lines: Vec<&str> = RULE
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect();
    let uinput = lines
        .iter()
        .find(|l| l.contains("uinput"))
        .expect("uinput rule");
    assert!(uinput.contains("uaccess"), "{uinput}");
    let mouse = lines
        .iter()
        .find(|l| l.contains("ID_INPUT_MOUSE"))
        .expect("mouse rule");
    assert!(
        mouse.contains("event*") && mouse.contains("uaccess"),
        "{mouse}"
    );
    assert!(!RULE.contains("MODE=\"0666\"") && !RULE.contains("GROUP=\"root\""));
}

#[test]
fn install_script_never_uses_root_tools() {
    for banned in ["sudo", "su -", "pkexec", "doas"] {
        let code: Vec<&str> = INSTALL
            .lines()
            .filter(|l| !l.trim_start().starts_with('#'))
            .collect();
        assert!(
            code.iter().all(|l| !l.contains(banned)),
            "install.sh runs {banned}"
        );
    }
}

#[test]
fn install_script_parses() {
    let st = Command::new("bash")
        .arg("-n")
        .arg(root().join("packaging/install.sh"))
        .status()
        .expect("bash");
    assert!(st.success());
}

#[test]
fn install_copies_files_into_the_user_dirs_and_keeps_an_existing_config() {
    let tmp = std::env::temp_dir().join(format!("strokebind-install-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("tmp");
    let bin = tmp.join("fake-bin");
    std::fs::write(&bin, "#!/bin/sh\n").expect("fake binary");
    let run = |home: &Path| {
        Command::new("bash")
            .arg(root().join("packaging/install.sh"))
            .env("HOME", home)
            .env_remove("XDG_CONFIG_HOME")
            .env("STROKEBIND_BIN", &bin)
            .env("STROKEBIND_NO_SYSTEMCTL", "1")
            .output()
            .expect("run install.sh")
    };
    let home = tmp.join("home");
    let out = run(&home);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(home.join(".local/bin/strokebind").is_file());
    assert!(home
        .join(".config/systemd/user/strokebind.service")
        .is_file());
    let cfg = home.join(".config/strokebind/config.toml");
    assert!(cfg.is_file());
    // The root-only step is printed, never executed.
    assert!(
        stdout.contains("70-strokebind.rules") && stdout.contains("/etc/udev/rules.d"),
        "{stdout}"
    );

    // A second run must not clobber the user's edited config.
    std::fs::write(&cfg, "trigger = \"middle\"\n").expect("edit config");
    assert!(run(&home).status.success());
    assert_eq!(
        std::fs::read_to_string(&cfg).expect("read"),
        "trigger = \"middle\"\n"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn install_fails_clearly_without_a_binary() {
    let out = Command::new("bash")
        .arg(root().join("packaging/install.sh"))
        .env("STROKEBIND_BIN", "/nonexistent/strokebind")
        .env("STROKEBIND_NO_SYSTEMCTL", "1")
        .output()
        .expect("run");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("cargo build --release"));
}

#[test]
fn starter_config_passes_check_config() {
    let out = Command::new(env!("CARGO_BIN_EXE_strokebind"))
        .arg("check-config")
        .arg(root().join("packaging/config.example.toml"))
        .output()
        .expect("run");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// P9: the `package` CI job stands in for a fresh Ubuntu VM.
#[test]
fn ci_package_job_checks_deb_install_script_unit_and_rule_on_every_push() {
    let ci = std::fs::read_to_string(root().join(".github/workflows/ci.yml")).expect("ci.yml");
    let job = ci
        .split("\n  package:")
        .nth(1)
        .expect("ci.yml must define a `package` job");
    for needed in [
        "runs-on: ubuntu-latest",
        "cargo deb --locked",
        "dpkg-deb --info",
        "dpkg -i",
        "strokebind --version",
        "STROKEBIND_NO_SYSTEMCTL=1",
        ".local/bin/strokebind",
        ".config/systemd/user/strokebind.service",
        "systemd-analyze --user verify",
        "udevadm verify",
    ] {
        assert!(job.contains(needed), "package job misses `{needed}`");
    }
    assert!(
        !ci.contains("tags:"),
        "package job must run on every push/PR"
    );
}
