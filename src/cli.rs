//! Command-line interface definition.

use clap::{Parser, Subcommand};

/// Mouse-stroke gestures for Linux.
#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Cli {
    /// Subcommand to run.
    #[command(subcommand)]
    pub command: Command,
}

/// Available subcommands.
#[derive(Debug, Clone, Copy, Subcommand)]
pub enum Command {
    /// Print the recognized stroke after each trigger release.
    Record,
    /// Run the daemon.
    Run,
    /// Validate the configuration file.
    CheckConfig,
    /// List available mouse devices.
    ListDevices,
}

impl Command {
    /// Name used in "not implemented" messages.
    pub fn name(self) -> &'static str {
        match self {
            Command::Record => "record",
            Command::Run => "run",
            Command::CheckConfig => "check-config",
            Command::ListDevices => "list-devices",
        }
    }
}
