//! M6.5: the launch checklist exists, has final post texts for every channel and makes no unbacked claims.

use std::path::Path;

const LAUNCH: &str = include_str!("../docs/LAUNCH.md");
const REPO: &str = "https://github.com/IvanKosk0v/strokebind";

fn section(title: &str) -> &'static str {
    let marker = format!("## {title}");
    let start = LAUNCH
        .find(&marker)
        .unwrap_or_else(|| panic!("missing section: {title}"));
    let rest = &LAUNCH[start + marker.len()..];
    rest.split("\n## ").next().unwrap_or_default()
}

#[test]
fn has_a_section_per_channel() {
    for title in [
        "Before posting",
        "Show HN",
        "r/linux",
        "r/unixporn",
        "r/rust",
        "Compositor subreddits",
        "AUR",
        "After posting",
    ] {
        assert!(
            !section(title).trim().is_empty(),
            "section {title} is empty"
        );
    }
}

#[test]
fn checklists_use_unticked_boxes() {
    let boxes = LAUNCH.lines().filter(|l| l.starts_with("- [ ] ")).count();
    assert!(boxes >= 8, "expected a real checklist, found {boxes} boxes");
    assert!(!LAUNCH.contains("- [x] "), "nothing is posted yet");
}

#[test]
fn show_hn_title_follows_the_rules() {
    let title = LAUNCH
        .lines()
        .find_map(|l| l.strip_prefix("Title: Show HN: "))
        .expect("a `Title: Show HN: ...` line");
    assert!(
        title.len() + "Show HN: ".len() <= 80,
        "HN title too long: {title}"
    );
    assert!(!title.contains('!'), "no exclamation marks in HN titles");
}

#[test]
fn posts_are_final_text_without_placeholders() {
    for bad in ["TODO", "TBD", "XXX", "<insert", "[link]", "lorem"] {
        assert!(!LAUNCH.contains(bad), "placeholder left in: {bad}");
    }
}

#[test]
fn links_point_at_the_real_repo_and_release() {
    assert!(LAUNCH.contains(REPO));
    assert!(LAUNCH.contains(&format!("{REPO}/releases/tag/v0.1.0")));
}

#[test]
fn commands_in_the_posts_exist() {
    let subcommands = ["record", "run", "check-config", "list-devices"];
    for line in LAUNCH.lines().filter(|l| l.contains("strokebind ")) {
        for word in line.split("strokebind ").skip(1) {
            let first = word.split_whitespace().next().unwrap_or_default();
            let first = first.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-');
            if first.is_empty() || !first.chars().all(|c| c.is_ascii_lowercase() || c == '-') {
                continue;
            }
            let prose = [
                "is", "reads", "works", "needs", "does", "runs", "has", "the", "and",
            ];
            if prose.contains(&first) || !line.contains('`') {
                continue;
            }
            assert!(
                subcommands.contains(&first) || first.starts_with('-'),
                "unknown subcommand `{first}` in: {line}"
            );
        }
    }
}

#[test]
fn does_not_claim_what_research_says_not_to() {
    let lower = LAUNCH.to_lowercase();
    for bad in [
        "key remapping tool",
        "touchpad gestures work",
        "per-app on kde",
        "per-app on gnome",
    ] {
        assert!(!lower.contains(bad), "unbacked claim: {bad}");
    }
}

#[test]
fn pkgbuild_installs_files_that_exist() {
    let aur = section("AUR");
    for file in [
        "packaging/70-strokebind.rules",
        "packaging/strokebind-deb.service",
        "packaging/config.example.toml",
        "LICENSE-MIT",
        "LICENSE-APACHE",
    ] {
        assert!(aur.contains(file), "PKGBUILD does not install {file}");
        assert!(
            Path::new(env!("CARGO_MANIFEST_DIR")).join(file).exists(),
            "{file} does not exist in the repo"
        );
    }
}
