use clap::Parser;
use strokebind::cli::{Cli, Command, RecordArgs};
use strokebind::config::Config;
use strokebind::device;
use strokebind::device::source::EvdevSource;
use strokebind::error::Error;
use strokebind::stroke_loop::{run_events, LoopParams};

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
        Command::Record(args) => record(&args),
        other => Err(Error::NotImplemented(other.name()).into()),
    }
}

/// Read a mouse without grabbing it and print the stroke after each trigger release.
fn record(args: &RecordArgs) -> anyhow::Result<()> {
    let params = match &args.config {
        Some(path) => {
            let src = std::fs::read_to_string(path).map_err(|source| Error::Io {
                path: path.clone(),
                source,
            })?;
            LoopParams::from_config(&Config::parse(&src)?)
        }
        None => LoopParams::default(),
    };
    let path = match &args.device {
        Some(p) => p.clone(),
        None => {
            device::list_mice()?
                .into_iter()
                .next()
                .ok_or_else(|| anyhow::anyhow!("no mouse devices found"))?
                .path
        }
    };
    eprintln!(
        "recording from {} (no grab); Ctrl-C to quit",
        path.display()
    );
    run_events(EvdevSource::open(&path)?, &params, |stroke| {
        println!("{}", stroke.as_deref().unwrap_or("(no stroke)"));
    })?;
    Ok(())
}
