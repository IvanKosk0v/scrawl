//! M10.2: a v0.1.0 config left in the old config dir is named, with the `mv` fix, instead of
//! being used silently.
#![allow(clippy::expect_used)] // test helpers outside #[test] fns

use std::path::Path;
use std::process::Command;

use scrawl::migrate::old_config;

fn write(path: &Path) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, "").expect("write");
}

#[test]
fn old_config_found_when_new_one_is_missing() {
    let home = tempfile::tempdir().expect("tempdir");
    let old = home.path().join("strokebind/config.toml");
    write(&old);
    let new = home.path().join("scrawl/config.toml");
    let found = old_config(&new, Some(home.path())).expect("hint");
    assert_eq!(found.old, old);
    assert_eq!(
        found.mv,
        format!(
            "mv '{}' '{}'",
            home.path().join("strokebind").display(),
            home.path().join("scrawl").display()
        )
    );
}

#[test]
fn existing_empty_new_dir_gets_a_file_move_not_a_nested_dir() {
    let home = tempfile::tempdir().expect("tempdir");
    write(&home.path().join("strokebind/config.toml"));
    std::fs::create_dir_all(home.path().join("scrawl")).expect("mkdir");
    let new = home.path().join("scrawl/config.toml");
    let found = old_config(&new, Some(home.path())).expect("hint");
    assert_eq!(
        found.mv,
        format!(
            "mv '{}' '{}/'",
            home.path().join("strokebind/config.toml").display(),
            home.path().join("scrawl").display()
        )
    );
}

#[test]
fn mv_command_quotes_paths_with_spaces() {
    let home = tempfile::tempdir().expect("tempdir");
    let cfg = home.path().join("my config");
    write(&cfg.join("strokebind/config.toml"));
    let found = old_config(&cfg.join("scrawl/config.toml"), Some(&cfg)).expect("hint");
    assert!(
        found
            .mv
            .contains(&format!("'{}'", cfg.join("strokebind").display())),
        "{}",
        found.mv
    );
}

#[test]
fn no_hint_without_an_old_config() {
    let home = tempfile::tempdir().expect("tempdir");
    let new = home.path().join("scrawl/config.toml");
    assert!(old_config(&new, Some(home.path())).is_none());
}

#[test]
fn no_hint_when_the_new_config_exists() {
    let home = tempfile::tempdir().expect("tempdir");
    write(&home.path().join("strokebind/config.toml"));
    let new = home.path().join("scrawl/config.toml");
    write(&new);
    assert!(old_config(&new, Some(home.path())).is_none());
}

#[test]
fn no_hint_for_a_config_outside_the_default_dir() {
    let home = tempfile::tempdir().expect("tempdir");
    write(&home.path().join("strokebind/config.toml"));
    let elsewhere = home.path().join("other/config.toml");
    assert!(old_config(&elsewhere, Some(home.path())).is_none());
    assert!(old_config(&home.path().join("scrawl/config.toml"), None).is_none());
}

#[test]
fn missing_config_error_names_the_old_path_and_the_mv_command() {
    let home = tempfile::tempdir().expect("tempdir");
    let old = home.path().join("strokebind/config.toml");
    write(&old);
    let new = home.path().join("scrawl/config.toml");
    for sub in [&["check-config"][..], &["record", "--config"][..]] {
        let out = Command::new(env!("CARGO_BIN_EXE_scrawl"))
            .args(sub)
            .arg(&new)
            .env("XDG_CONFIG_HOME", home.path())
            .output()
            .expect("run scrawl");
        assert!(!out.status.success(), "{sub:?} must fail, not fall back");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains(&old.display().to_string()), "{sub:?}: {err}");
        assert!(
            err.contains(&format!(
                "mv '{}'",
                home.path().join("strokebind").display()
            )),
            "{sub:?}: {err}"
        );
    }
}
