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
}
