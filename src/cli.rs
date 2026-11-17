//! Command-line interface definition.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// Mouse-stroke gestures for Linux.
#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Cli {
    /// Subcommand to run.
    #[command(subcommand)]
    pub command: Command,
}

/// Options of `record`.
#[derive(Debug, Clone, Args)]
pub struct RecordArgs {
    /// Evdev node to read (default: the first mouse found).
    #[arg(long)]
    pub device: Option<PathBuf>,
    /// Config file supplying `trigger` and `min_stroke_px` (default: right button, 40 px).
    #[arg(long)]
    pub config: Option<PathBuf>,
}

/// Available subcommands.
#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Print the recognized stroke after each trigger release.
    Record(RecordArgs),
    /// Run the daemon.
    Run,
    /// Validate the configuration file.
    CheckConfig,
    /// List available mouse devices.
    ListDevices,
}

impl Command {
    /// Name used in "not implemented" messages.
    pub fn name(&self) -> &'static str {
        match self {
            Command::Record(_) => "record",
            Command::Run => "run",
            Command::CheckConfig => "check-config",
            Command::ListDevices => "list-devices",
        }
    }
}
