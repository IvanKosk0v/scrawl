//! Library error type.

use std::path::PathBuf;

use thiserror::Error;

/// Errors returned by the strokebind library.
#[derive(Debug, Error)]
pub enum Error {
    /// The requested feature is not available yet.
    #[error("unsupported: {0}")]
    Unsupported(&'static str),
    /// The subcommand is a stub that is not implemented yet.
    #[error("{0}: not implemented")]
    NotImplemented(&'static str),
    /// A device node exists but this user may not read it.
    #[error(
        "permission denied opening {}: add your user to the `input` group and log in again, \
         or install the strokebind udev rule (see README)",
        .0.display()
    )]
    PermissionDenied(PathBuf),
    /// The uinput node exists but this user may not write it.
    #[error(
        "permission denied opening {}: add your user to the `input` group and log in again, \
         or install the strokebind udev rule for uinput (see README)",
        .0.display()
    )]
    UinputDenied(PathBuf),
    /// The escape chord uses buttons the grabbed mouse does not have, so the failsafe could never fire.
    #[error(
        "escape chord `{chord}` cannot work on {device}: it has no {missing} button; \
         set `escape` in the config to buttons this mouse has, e.g. `escape = \"left+right\"`"
    )]
    EscapeUnavailable {
        /// The configured chord, e.g. `left+right+middle`.
        chord: String,
        /// Device name.
        device: String,
        /// Missing buttons joined by `, `.
        missing: String,
    },
    /// A run-command action has no program.
    #[error("run action needs a program, e.g. run = [\"notify-send\", \"hello\"]")]
    EmptyCommand,
    /// A run-command action could not be started.
    #[error("cannot run `{program}`: {source}")]
    Spawn {
        /// The program (argv[0]).
        program: String,
        /// Underlying error.
        source: std::io::Error,
    },
    /// Installing the SIGINT/SIGTERM failsafe failed.
    #[error("cannot install signal handlers: {0}")]
    Signals(std::io::Error),
    /// An I/O operation on a path failed.
    #[error("{}: {source}", path.display())]
    Io {
        /// Path the operation was on.
        path: PathBuf,
        /// Underlying error.
        source: std::io::Error,
    },
    /// The config file is invalid.
    #[error("{}", format_config(*line, message))]
    Config {
        /// 1-based line of the problem, when known.
        line: Option<usize>,
        /// What is wrong.
        message: String,
    },
}

fn format_config(line: Option<usize>, message: &str) -> String {
    match line {
        Some(l) => format!("config line {l}: {message}"),
        None => format!("config: {message}"),
    }
}
