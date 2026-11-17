//! Library error type.

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
