//! M4.2: run-command action. argv is executed directly (no shell) with a sanitized environment.

use std::ffi::OsStr;
use std::path::Path;
use std::time::{Duration, Instant};

use scrawl::actions::command::{build_command, sanitized_env, spawn_detached, DEFAULT_PATH};
use scrawl::error::Error;

fn argv(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

fn env(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

fn lookup<'a>(vars: &'a [(String, String)], key: &str) -> Option<&'a str> {
    vars.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

fn wait_for(path: &Path) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if path.exists() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

#[test]
fn env_keeps_session_vars_and_drops_the_rest() {
    let vars = sanitized_env(env(&[
        ("HOME", "/home/u"),
        ("PATH", "/usr/bin"),
        ("WAYLAND_DISPLAY", "wayland-1"),
        ("DBUS_SESSION_BUS_ADDRESS", "unix:path=/run/user/1000/bus"),
        ("LC_TIME", "C"),
        ("LD_PRELOAD", "/tmp/evil.so"),
        ("LD_LIBRARY_PATH", "/tmp"),
        ("SSH_AUTH_SOCK", "/tmp/agent"),
        ("MY_SECRET_TOKEN", "x"),
        ("RUST_LOG", "trace"),
    ]));
    for kept in [
        "HOME",
        "PATH",
        "WAYLAND_DISPLAY",
        "DBUS_SESSION_BUS_ADDRESS",
        "LC_TIME",
    ] {
        assert!(lookup(&vars, kept).is_some(), "{kept} should be kept");
    }
    for dropped in [
        "LD_PRELOAD",
        "LD_LIBRARY_PATH",
        "SSH_AUTH_SOCK",
        "MY_SECRET_TOKEN",
        "RUST_LOG",
    ] {
        assert!(
            lookup(&vars, dropped).is_none(),
            "{dropped} should be dropped"
        );
    }
}

#[test]
fn env_gets_a_default_path_when_missing() {
    let vars = sanitized_env(env(&[("HOME", "/home/u")]));
    assert_eq!(lookup(&vars, "PATH"), Some(DEFAULT_PATH));
}

#[test]
fn command_runs_argv_directly_without_a_shell() {
    let cmd = build_command(
        &argv(&["notify-send", "hello; rm -rf ~", "$(id)"]),
        env(&[("HOME", "/home/u"), ("LD_PRELOAD", "x")]),
    )
    .expect("valid argv");
    assert_eq!(cmd.get_program(), OsStr::new("notify-send"));
    let args: Vec<_> = cmd.get_args().collect();
    assert_eq!(args, [OsStr::new("hello; rm -rf ~"), OsStr::new("$(id)")]);
    assert_eq!(cmd.get_current_dir(), Some(Path::new("/home/u")));
    let envs: Vec<_> = cmd.get_envs().map(|(k, _)| k.to_owned()).collect();
    assert!(!envs.iter().any(|k| k == "LD_PRELOAD"), "{envs:?}");
}

#[test]
fn command_without_home_runs_in_root() {
    let cmd = build_command(&argv(&["true"]), Vec::new()).expect("valid argv");
    assert_eq!(cmd.get_current_dir(), Some(Path::new("/")));
}

#[test]
fn empty_argv_is_rejected() {
    for bad in [argv(&[]), argv(&[""])] {
        assert!(matches!(
            build_command(&bad, Vec::new()),
            Err(Error::EmptyCommand)
        ));
    }
}

#[test]
fn missing_program_reports_its_name() {
    let err = spawn_detached(&argv(&["scrawl-no-such-program-xyz"]), Vec::new())
        .expect_err("program does not exist");
    let msg = err.to_string();
    assert!(matches!(err, Error::Spawn { .. }), "{err:?}");
    assert!(msg.contains("scrawl-no-such-program-xyz"), "{msg}");
}

#[test]
fn spawned_command_sees_only_the_sanitized_env_and_no_shell() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = dir.path().to_str().expect("utf-8 tempdir").to_owned();
    let fake_env = env(&[
        ("HOME", home.as_str()),
        ("PATH", "/usr/bin:/bin"),
        ("LD_PRELOAD", "/nonexistent/evil.so"),
        ("MY_SECRET_TOKEN", "leak"),
    ]);
    // `env` writes the child's environment; the shell metacharacters must reach `touch` literally.
    spawn_detached(
        &argv(&["sh", "-c", "env > env.txt", "ignored"]),
        fake_env.clone(),
    )
    .expect("spawn env");
    spawn_detached(&argv(&["touch", "a; touch b"]), fake_env).expect("spawn touch");

    let env_file = dir.path().join("env.txt");
    assert!(
        wait_for(&dir.path().join("a; touch b")),
        "literal file name created in HOME"
    );
    assert!(
        wait_for(&env_file),
        "env.txt written in HOME (the working dir)"
    );
    // The file appears before `env` finishes writing; wait for the PATH line.
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut seen = String::new();
    while Instant::now() < deadline && !seen.contains("PATH=") {
        seen = std::fs::read_to_string(&env_file).unwrap_or_default();
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(seen.contains("PATH=/usr/bin:/bin"), "{seen}");
    assert!(!seen.contains("LD_PRELOAD"), "{seen}");
    assert!(!seen.contains("MY_SECRET_TOKEN"), "{seen}");
    assert!(!dir.path().join("b").exists(), "no shell interpreted `;`");
}
