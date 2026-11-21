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

// ---- M6.6: Arch PKGBUILD shipped in the repo ----

const PKGBUILD: &str = include_str!("../packaging/arch/PKGBUILD");

fn pkgbuild_var(name: &str) -> &'static str {
    PKGBUILD
        .lines()
        .find_map(|l| l.strip_prefix(&format!("{name}=")))
        .map(|v| v.trim_matches(|c| c == '\'' || c == '"'))
        .unwrap_or_else(|| panic!("PKGBUILD has no {name}="))
}

#[test]
fn pkgbuild_version_matches_cargo_and_fetches_the_tag() {
    let manifest = include_str!("../Cargo.toml");
    let cargo_version = manifest
        .lines()
        .find_map(|l| l.strip_prefix("version = "))
        .expect("Cargo.toml version")
        .trim_matches('"');
    assert_eq!(pkgbuild_var("pkgver"), cargo_version);
    assert_eq!(pkgbuild_var("pkgname"), "strokebind");
    let source = PKGBUILD
        .lines()
        .find(|l| l.starts_with("source="))
        .expect("source=");
    assert!(
        source.contains("/archive/refs/tags/v$pkgver.tar.gz"),
        "{source}"
    );
}

#[test]
fn pkgbuild_installs_files_that_exist_into_the_right_places() {
    let installs: Vec<Vec<&str>> = PKGBUILD
        .lines()
        .filter(|l| l.trim_start().starts_with("install -Dm"))
        .map(|l| l.split_whitespace().collect())
        .collect();
    assert!(
        installs.len() >= 6,
        "expected 6 install lines, got {}",
        installs.len()
    );
    for words in &installs {
        let src = words[2];
        if src != "target/release/strokebind" {
            assert!(root().join(src).exists(), "{src} does not exist");
        }
    }
    for dest in [
        "usr/bin/strokebind",
        "usr/lib/udev/rules.d/70-strokebind.rules",
        "usr/lib/systemd/user/strokebind.service",
        "usr/share/doc/strokebind/config.example.toml",
        "usr/share/licenses/$pkgname/LICENSE-MIT",
        "usr/share/licenses/$pkgname/LICENSE-APACHE",
    ] {
        assert!(PKGBUILD.contains(dest), "PKGBUILD does not install {dest}");
    }
    // The unit shipped to /usr/lib is the deb one (absolute /usr/bin path).
    assert!(PKGBUILD.contains("packaging/strokebind-deb.service"));
}

#[test]
fn pkgbuild_never_asks_for_root_and_builds_locked() {
    assert!(!PKGBUILD.contains("sudo"));
    assert!(PKGBUILD.contains("cargo fetch --locked"));
    assert!(PKGBUILD.contains("cargo build --release --frozen"));
    assert!(
        pkgbuild_var("license").contains("MIT") && pkgbuild_var("license").contains("Apache-2.0")
    );
}

#[test]
fn readme_documents_the_arch_install() {
    let readme = include_str!("../README.md");
    assert!(readme.contains("cd packaging/arch && makepkg -si"));
}

#[test]
fn pkgbuild_checksum_is_a_real_sha256() {
    let sums = PKGBUILD
        .lines()
        .find(|l| l.starts_with("sha256sums="))
        .expect("sha256sums=");
    let sum = sums
        .trim_start_matches("sha256sums=(")
        .trim_end_matches(')')
        .trim_matches('\'');
    assert!(
        sum.len() == 64 && sum.chars().all(|c| c.is_ascii_hexdigit()),
        "sha256sums must be the tag tarball's checksum, not {sum}"
    );
}
