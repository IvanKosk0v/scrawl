//! M5.9: the README support matrix has one line per window backend.

const README: &str = include_str!("../README.md");

fn row(name: &str) -> &'static str {
    README
        .lines()
        .find(|l| l.starts_with('|') && l.contains(name))
        .unwrap_or_else(|| panic!("no support-matrix row for {name}"))
}

#[test]
fn matrix_has_a_row_per_backend() {
    assert!(row("X11").contains("`_NET_ACTIVE_WINDOW`"));
    assert!(row("Sway").contains("`SWAYSOCK`"));
    assert!(row("Hyprland").contains("`HYPRLAND_INSTANCE_SIGNATURE`"));
}

#[test]
fn kde_and_gnome_are_global_only() {
    for name in ["KDE Plasma", "GNOME"] {
        assert!(row(name).contains("global bindings only"), "{name}");
    }
}

// M6.3: the README is the landing page; every command and config in it must be real.

fn section(heading: &str) -> &'static str {
    let start = README
        .find(&format!("\n## {heading}"))
        .unwrap_or_else(|| panic!("README has no `## {heading}` section"));
    let rest = &README[start + 1..];
    let end = rest[3..].find("\n## ").map_or(rest.len(), |i| i + 3);
    &rest[..end]
}

#[test]
fn readme_is_the_launch_text_with_all_required_sections() {
    assert!(!README.contains("pre-alpha"), "stale status banner");
    assert!(!README.contains("rewritten in milestone"));
    for heading in [
        "Install",
        "Quick start",
        "Configuration",
        "Support matrix",
        "Comparison",
        "FAQ",
        "Failsafes",
        "License",
    ] {
        section(heading);
    }
    assert!(
        README.contains("<!-- demo-gif -->"),
        "demo GIF slot is missing"
    );
}

#[test]
fn every_toml_block_in_the_readme_passes_check_config() {
    let mut n = 0;
    for (i, block) in README.split("```toml\n").skip(1).enumerate() {
        let body = block.split("```").next().expect("closing fence");
        let path =
            std::env::temp_dir().join(format!("strokebind-readme-{}-{i}.toml", std::process::id()));
        std::fs::write(&path, body).expect("write temp config");
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_scrawl"))
            .arg("check-config")
            .arg(&path)
            .output()
            .expect("run strokebind");
        let _ = std::fs::remove_file(&path);
        assert!(
            out.status.success(),
            "README toml block {i} is invalid: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        n += 1;
    }
    assert!(n >= 1, "README needs at least one toml example");
}

#[test]
fn every_strokebind_subcommand_and_flag_in_the_readme_exists() {
    let help = |args: &[&str]| {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_scrawl"))
            .args(args)
            .output()
            .expect("run strokebind");
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    let top = help(&["--help"]);

    // Commands from inline code spans and from ```sh fences (trailing `# comment` stripped).
    let mut commands: Vec<&str> = Vec::new();
    let mut in_sh = false;
    for line in README.lines() {
        if line.starts_with("```") {
            in_sh = line.trim() == "```sh";
            continue;
        }
        if in_sh {
            let cmd = line.split(" #").next().unwrap_or_default().trim();
            if cmd.starts_with("strokebind ") {
                commands.push(cmd);
            }
        } else {
            commands.extend(line.split('`').skip(1).step_by(2));
        }
    }

    let mut checked = 0;
    let mut checked_in_fence = 0;
    for cmd in commands {
        let mut words = cmd
            .strip_prefix("strokebind ")
            .into_iter()
            .flat_map(str::split_whitespace);
        let Some(sub) = words.next() else { continue };
        if sub.starts_with('-') {
            continue;
        }
        assert!(top.contains(sub), "README uses unknown subcommand `{sub}`");
        let sub_help = help(&[sub, "--help"]);
        for flag in words.filter(|w| w.starts_with("--")) {
            assert!(sub_help.contains(flag), "`strokebind {sub}` has no {flag}");
        }
        checked += 1;
        if README.contains(&format!("\n{cmd}")) {
            checked_in_fence += 1;
        }
    }
    assert!(checked >= 4, "only {checked} README commands were checked");
    assert!(
        checked_in_fence >= 4,
        "the Quick start fence was not scanned ({checked_in_fence} commands)"
    );
}

#[test]
fn comparison_table_lists_strokebind_and_the_researched_tools() {
    let c = section("Comparison");
    for name in [
        "strokebind",
        "Easystroke",
        "wstroke",
        "Input Actions",
        "mouse-actions",
    ] {
        assert!(c.contains(name), "comparison misses {name}");
    }
}
