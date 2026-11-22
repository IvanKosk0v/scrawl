//! M10.1: the old project name survives only where history or migration needs it.
#![allow(clippy::expect_used)] // test helpers outside #[test] fns

use std::process::Command;

/// The old name, split so this file does not match itself.
const OLD: &str = concat!("stroke", "bind");

/// Files that may mention the old name anywhere: history (STATE/PLAN), the migration hint and
/// its test, and this test.
const ALLOWED_FILES: &[&str] = &[
    "STATE.md",
    "PLAN.md",
    "src/migrate.rs",
    "tests/migrate.rs",
    "tests/rename.rs",
];

/// README heading of the migration steps, which must name the old paths.
const README_MIGRATION: &str = "## Migrating from";

fn tracked_files() -> Vec<String> {
    let out = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run git ls-files");
    assert!(out.status.success(), "git ls-files failed");
    String::from_utf8(out.stdout)
        .expect("utf-8 paths")
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Lines of `text` that may not mention the old name: for CHANGELOG.md everything above the
/// `## [0.1.0]` section except the `- Renamed` entry, for README.md everything outside the
/// migration section, for other files every line.
fn checked_lines<'a>(path: &str, text: &'a str) -> Vec<(usize, &'a str)> {
    let lines = text.lines().enumerate().map(|(i, l)| (i + 1, l));
    match path {
        "CHANGELOG.md" => lines
            .take_while(|(_, l)| !l.starts_with("## [0.1.0]"))
            .filter(|(_, l)| !l.starts_with("- Renamed"))
            .collect(),
        "README.md" => {
            let mut in_migration = false;
            lines
                .filter(|(_, l)| {
                    if l.starts_with("## ") {
                        in_migration = l.starts_with(README_MIGRATION);
                    }
                    !in_migration
                })
                .collect()
        }
        _ => lines.collect(),
    }
}

fn offenders() -> Vec<String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut bad = Vec::new();
    for path in tracked_files() {
        if ALLOWED_FILES.contains(&path.as_str()) {
            continue;
        }
        if path.to_lowercase().contains(OLD) {
            bad.push(format!("{path}: file name"));
        }
        let Ok(text) = std::fs::read_to_string(root.join(&path)) else {
            continue; // binary file (e.g. the demo GIF)
        };
        for (n, line) in checked_lines(&path, &text) {
            if line.to_lowercase().contains(OLD) {
                bad.push(format!("{path}:{n}: {line}"));
            }
        }
    }
    bad
}

#[test]
fn old_name_appears_only_in_the_allowlist() {
    let bad = offenders();
    assert!(bad.is_empty(), "old name still used:\n{}", bad.join("\n"));
}

#[test]
fn changelog_history_is_allowed_but_unreleased_is_checked() {
    let text = "## [Unreleased]\n- x\n## [0.1.0] - 2026-11-21\n- old\n";
    let lines = checked_lines("CHANGELOG.md", text);
    assert_eq!(lines.len(), 2);
    assert_eq!(checked_lines("src/lib.rs", text).len(), 4);
    let changelog = "## [Unreleased]\n- Renamed to x\n- other\n";
    assert_eq!(checked_lines("CHANGELOG.md", changelog).len(), 2);
}

#[test]
fn readme_migration_section_is_allowed_and_nothing_else() {
    let text = "# x\n## Install\na\n## Migrating from old\nb\nc\n## License\nd\n";
    let kept: Vec<_> = checked_lines("README.md", text)
        .into_iter()
        .map(|(_, l)| l)
        .collect();
    assert_eq!(kept, ["# x", "## Install", "a", "## License", "d"]);
}

#[test]
fn binary_and_package_names() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("name = \"scrawld\""), "package name");
    let bin = std::path::Path::new(env!("CARGO_BIN_EXE_scrawl"));
    assert_eq!(bin.file_name().and_then(|n| n.to_str()), Some("scrawl"));
}
