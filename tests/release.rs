//! M6.4: release hygiene for 0.1.0 (version, changelog, licenses, package contents, security contact).

use std::process::Command;

const MANIFEST: &str = include_str!("../Cargo.toml");
const CHANGELOG: &str = include_str!("../CHANGELOG.md");
const SECURITY: &str = include_str!("../SECURITY.md");
const README: &str = include_str!("../README.md");
const LICENSE_MIT: &str = include_str!("../LICENSE-MIT");
const LICENSE_APACHE: &str = include_str!("../LICENSE-APACHE");

const REPO: &str = "https://github.com/IvanKosk0v/scrawl";

fn manifest_has_line(line: &str) -> bool {
    MANIFEST.lines().any(|l| l.trim() == line)
}

#[test]
fn manifest_version_is_0_1_0() {
    assert!(manifest_has_line("version = \"0.1.0\""));
}

#[test]
fn manifest_points_at_the_repository() {
    assert!(manifest_has_line(&format!("repository = \"{REPO}\"")));
}

#[test]
fn changelog_has_dated_0_1_0_section_below_unreleased() {
    let unreleased = CHANGELOG
        .find("## [Unreleased]")
        .expect("Unreleased heading");
    let release = CHANGELOG.find("## [0.1.0] - ").expect("0.1.0 heading");
    assert!(unreleased < release, "Unreleased must come first");

    // Unreleased was empty when v0.1.0 was tagged; later changes are logged there, so a change must
    // not be listed under both headings.
    let unreleased_text = &CHANGELOG[unreleased + "## [Unreleased]".len()..release];
    let released_text = CHANGELOG[release..]
        .split("\n## [")
        .next()
        .unwrap_or_default();
    for bullet in unreleased_text.lines().filter(|l| l.starts_with("- ")) {
        assert!(
            !released_text.lines().any(|l| l == bullet),
            "listed under both Unreleased and 0.1.0: {bullet}"
        );
    }

    let heading = CHANGELOG[release..].lines().next().unwrap_or_default();
    let date = heading.trim_start_matches("## [0.1.0] - ");
    let parts: Vec<&str> = date.split('-').collect();
    assert!(
        parts.len() == 3
            && parts[0].len() == 4
            && parts[1].len() == 2
            && parts[2].len() == 2
            && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit())),
        "0.1.0 heading needs a YYYY-MM-DD date: {heading}"
    );

    let body = &CHANGELOG[release + heading.len()..];
    assert!(body.contains("- "), "0.1.0 section must list changes");
}

#[test]
fn changelog_bullets_are_not_fused() {
    for (n, line) in CHANGELOG.lines().enumerate() {
        assert!(
            !line.contains(".- `") && !line.contains(". - `"),
            "CHANGELOG line {} has two bullets on one line",
            n + 1
        );
    }
}

#[test]
fn changelog_has_no_claims_that_later_entries_made_false() {
    assert!(!CHANGELOG.contains("does not track the focused window yet"));
    assert!(!CHANGELOG.contains("slot is still empty"));
}

#[test]
fn license_files_and_expression_agree() {
    assert!(LICENSE_MIT.starts_with("MIT License"));
    assert!(LICENSE_MIT.contains("Copyright (c) 2026"));
    assert!(LICENSE_APACHE.contains("Apache License"));
    assert!(LICENSE_APACHE.contains("Version 2.0, January 2004"));
    assert!(manifest_has_line("license = \"MIT OR Apache-2.0\""));
    let license_section = README.split("## License").nth(1).unwrap_or_default();
    assert!(license_section.contains("MIT") && license_section.contains("Apache-2.0"));
}

#[test]
fn cargo_package_list_ships_license_files() {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let out = Command::new(cargo)
        .args(["package", "--list", "--allow-dirty", "--offline"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run cargo package --list");
    assert!(
        out.status.success(),
        "cargo package --list failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let listing = String::from_utf8_lossy(&out.stdout);
    let files: Vec<&str> = listing.lines().collect();
    for want in ["LICENSE-MIT", "LICENSE-APACHE", "README.md", "SECURITY.md"] {
        assert!(
            files.contains(&want),
            "{want} missing from package: {files:?}"
        );
    }
    assert!(
        files.iter().all(|f| !f.starts_with("target/")),
        "target/ leaked into the package"
    );
}

#[test]
fn security_policy_links_to_private_reporting() {
    assert!(SECURITY.contains(&format!("{REPO}/security/advisories/new")));
    assert!(SECURITY.contains("private"));
    assert!(!SECURITY.contains("once the repository is public"));
}
