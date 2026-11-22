//! Command-line interface definition.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// Mouse-stroke gestures for Linux.
#[derive(Debug, Parser)]
#[command(name = "scrawl", version, about)]
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

/// Options of `run`.
#[derive(Debug, Clone, Args)]
pub struct RunArgs {
    /// Evdev node to grab (default: the first mouse found).
    #[arg(long)]
    pub device: Option<PathBuf>,
    /// Config file (default: right-button trigger, 40 px, escape chord left+right+middle).
    #[arg(long)]
    pub config: Option<PathBuf>,
    /// Failsafe: exit (releasing the mouse) after this many seconds.
    #[arg(long, value_name = "SECONDS", value_parser = clap::value_parser!(u64).range(1..))]
    pub timeout: Option<u64>,
}

/// Options of `check-config`.
#[derive(Debug, Clone, Args)]
pub struct CheckArgs {
    /// Config file to validate.
    pub file: PathBuf,
}

/// Available subcommands.
#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Print the recognized stroke after each trigger release.
    Record(RecordArgs),
    /// Grab the mouse, forward it to a virtual mouse and print recognized strokes. Holding the escape chord
    /// (default left+right+middle) exits.
    Run(RunArgs),
    /// Validate a config file; errors name the line and quote it.
    CheckConfig(CheckArgs),
    /// List available mouse devices.
    ListDevices,
}
