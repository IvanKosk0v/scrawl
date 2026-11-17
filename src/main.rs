use clap::Parser;
use strokebind::cli::{Cli, Command};
use strokebind::device;
use strokebind::error::Error;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::ListDevices => {
            let mice = device::list_mice()?;
            if mice.is_empty() {
                eprintln!("no mouse devices found");
            }
            for m in mice {
                println!("{}\t{}", m.name, m.path.display());
            }
            Ok(())
        }
        other => Err(Error::NotImplemented(other.name()).into()),
    }
}
