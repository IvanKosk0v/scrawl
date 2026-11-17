use clap::Parser;
use strokebind::cli::Cli;
use strokebind::error::Error;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    Err(Error::NotImplemented(cli.command.name()).into())
}
