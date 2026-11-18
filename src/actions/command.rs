//! Run-command action: argv is executed directly (no shell; write `["sh", "-c", "..."]` to opt in) with a
//! sanitized environment, in `$HOME`, stdin/stdout detached. Children are reaped on a background thread.

use std::process::{Command, Stdio};

use crate::error::Error;

/// `PATH` given to commands when the daemon has none.
pub const DEFAULT_PATH: &str = "/usr/local/bin:/usr/bin:/bin";

/// Variables passed through unchanged: session identity, locale and what desktop tools need to reach the
/// session (Wayland/X11, D-Bus, compositor IPC). Everything else (e.g. `LD_PRELOAD`, agent sockets) is dropped.
const KEEP: &[&str] = &[
    "HOME",
    "USER",
    "LOGNAME",
    "SHELL",
    "PATH",
    "LANG",
    "LANGUAGE",
    "TZ",
    "DISPLAY",
    "XAUTHORITY",
    "WAYLAND_DISPLAY",
    "DBUS_SESSION_BUS_ADDRESS",
    "DESKTOP_SESSION",
    "SWAYSOCK",
    "I3SOCK",
    "HYPRLAND_INSTANCE_SIGNATURE",
];

fn keep(name: &str) -> bool {
    KEEP.contains(&name) || name.starts_with("LC_") || name.starts_with("XDG_")
}

/// Filter `vars` to the allowlist; adds [`DEFAULT_PATH`] when `PATH` is missing.
pub fn sanitized_env(vars: impl IntoIterator<Item = (String, String)>) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = vars.into_iter().filter(|(k, _)| keep(k)).collect();
    if !out.iter().any(|(k, _)| k == "PATH") {
        out.push(("PATH".to_owned(), DEFAULT_PATH.to_owned()));
    }
    out
}

/// Build the command for `argv` with the sanitized form of `env`, working dir `$HOME` (or `/`).
pub fn build_command(argv: &[String], env: Vec<(String, String)>) -> Result<Command, Error> {
    let (program, args) = argv.split_first().ok_or(Error::EmptyCommand)?;
    if program.is_empty() {
        return Err(Error::EmptyCommand);
    }
    let env = sanitized_env(env);
    let dir = env
        .iter()
        .find(|(k, _)| k == "HOME")
        .map_or_else(|| "/".to_owned(), |(_, v)| v.clone());
    let mut cmd = Command::new(program);
    cmd.args(args)
        .env_clear()
        .envs(env)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    Ok(cmd)
}

/// Start `argv` without waiting for it; a background thread reaps it so no zombies remain.
pub fn spawn_detached(argv: &[String], env: Vec<(String, String)>) -> Result<(), Error> {
    let mut child = build_command(argv, env)?
        .spawn()
        .map_err(|source| Error::Spawn {
            program: argv.first().cloned().unwrap_or_default(),
            source,
        })?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// The daemon's own environment (non-UTF-8 entries skipped), to pass to [`spawn_detached`].
pub fn current_env() -> Vec<(String, String)> {
    std::env::vars_os()
        .filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?)))
        .collect()
}
