use std::path::{Path, PathBuf};
use std::time::Duration;

use clap::Parser;
use strokebind::cli::{Cli, Command, RecordArgs, RunArgs};
use strokebind::config::Config;
use strokebind::device;
use strokebind::device::forward::forward;
use strokebind::device::grab::{install_failsafes, Grabbed};
use strokebind::device::source::{button_code, EvdevSource};
use strokebind::device::virtual_mouse::{MouseCaps, VirtualMouse};
use strokebind::error::Error;
use strokebind::passthrough::Passthrough;
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
        Command::Run(args) => run(&args),
        other => Err(Error::NotImplemented(other.name()).into()),
    }
}

/// Load the config file if one was given.
fn load_config(path: Option<&Path>) -> anyhow::Result<Option<Config>> {
    let Some(path) = path else { return Ok(None) };
    let src = std::fs::read_to_string(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(Some(Config::parse(&src)?))
}

/// The given device, or the first mouse found.
fn pick_device(device: Option<&Path>) -> anyhow::Result<PathBuf> {
    match device {
        Some(p) => Ok(p.to_path_buf()),
        None => Ok(device::list_mice()?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("no mouse devices found"))?
            .path),
    }
}

/// Read a mouse without grabbing it and print the stroke after each trigger release.
fn record(args: &RecordArgs) -> anyhow::Result<()> {
    let params = load_config(args.config.as_deref())?
        .map_or_else(LoopParams::default, |c| LoopParams::from_config(&c));
    let path = pick_device(args.device.as_deref())?;
    eprintln!(
        "recording from {} (no grab); Ctrl-C to quit",
        path.display()
    );
    run_events(EvdevSource::open(&path)?, &params, |stroke| {
        println!("{}", stroke.as_deref().unwrap_or("(no stroke)"));
    })?;
    Ok(())
}

/// Grab the mouse, forward it through a virtual mouse and print each recognized stroke.
fn run(args: &RunArgs) -> anyhow::Result<()> {
    install_failsafes()?;
    if let Some(secs) = args.timeout {
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(secs));
            eprintln!("--timeout {secs}s reached, exiting");
            // Process exit closes the grabbed fd, which releases the grab.
            std::process::exit(0);
        });
    }
    let cfg = load_config(args.config.as_deref())?;
    let params = cfg
        .as_ref()
        .map_or_else(LoopParams::default, LoopParams::from_config);
    let escape = cfg.map(|c| c.escape).unwrap_or_default();
    let engine = Passthrough::new(&params)
        .with_escape(escape.buttons().iter().map(|&b| button_code(b)).collect());
    let path = pick_device(args.device.as_deref())?;
    let mut grab = Grabbed::grab(&path)?;
    let mut out = VirtualMouse::create(&MouseCaps::from_device(grab.device_mut()))?;
    eprintln!("grabbed {}; hold {} to exit", path.display(), escape);
    forward(&mut grab, &mut out, engine, |stroke| println!("{stroke}"))?;
    eprintln!("escape chord held, exiting");
    Ok(())
}
