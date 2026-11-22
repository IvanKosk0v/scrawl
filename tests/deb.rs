//! M6.2: `.deb` metadata for cargo-deb and the release workflow (static checks; the build runs in CI).
#![allow(clippy::expect_used)] // test helpers outside #[test] fns

use std::process::Command;

const MANIFEST: &str = include_str!("../Cargo.toml");
const USER_UNIT: &str = include_str!("../packaging/scrawl.service");
const DEB_UNIT: &str = include_str!("../packaging/scrawl-deb.service");
const POSTINST: &str = include_str!("../packaging/debian/postinst");
const RELEASE: &str = include_str!("../.github/workflows/release.yml");

#[test]
fn version_flag_prints_the_package_version() {
    let out = Command::new(env!("CARGO_BIN_EXE_scrawl"))
        .arg("--version")
        .output()
        .expect("run");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(text.trim(), format!("scrawl {}", env!("CARGO_PKG_VERSION")));
}

#[test]
fn manifest_has_deb_metadata_with_all_assets() {
    assert!(MANIFEST.contains("[package.metadata.deb]"));
    for needle in [
        "target/release/scrawl",
        "usr/bin/",
        "packaging/scrawl-deb.service",
        "usr/lib/systemd/user/scrawl.service",
        "packaging/70-scrawl.rules",
        "usr/lib/udev/rules.d/70-scrawl.rules",
        "packaging/config.example.toml",
        "usr/share/doc/scrawl/",
        "maintainer-scripts = \"packaging/debian\"",
    ] {
        assert!(MANIFEST.contains(needle), "Cargo.toml lacks {needle}");
    }
}

#[test]
fn deb_unit_differs_from_the_user_unit_only_in_the_binary_path() {
    let swap = USER_UNIT.replace("%h/.local/bin/scrawl", "/usr/bin/scrawl");
    assert_eq!(swap, DEB_UNIT);
}

#[test]
fn postinst_reloads_udev_without_failing_the_install_or_using_root_tools() {
    assert!(POSTINST.starts_with("#!/bin/sh"));
    assert!(POSTINST.contains("udevadm control --reload"));
    assert!(POSTINST.contains("|| true"));
    assert!(POSTINST.contains("#DEBHELPER#"));
    assert!(!POSTINST.contains("sudo"));
}

#[test]
fn release_workflow_builds_the_deb_on_version_tags_and_checks_it() {
    assert!(RELEASE.contains("tags:") && RELEASE.contains("'v*'"));
    assert!(RELEASE.contains("cargo install cargo-deb --locked"));
    assert!(RELEASE.contains("cargo deb"));
    assert!(RELEASE.contains("dpkg-deb --info"));
    assert!(
        RELEASE.contains("--version"),
        "workflow must run the installed binary's --version"
    );
    assert!(
        RELEASE.contains("gh release"),
        "workflow must publish the .deb"
    );
    assert!(RELEASE.contains("contents: write"));
}
