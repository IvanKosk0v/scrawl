use std::path::{Path, PathBuf};
use std::time::Duration;

use clap::Parser;
use strokebind::actions::{command, feedback, select, Actions};
use strokebind::backend::{window_backend, WindowBackend};
use strokebind::check::check;
use strokebind::cli::{CheckArgs, Cli, Command, RecordArgs, RunArgs};
use strokebind::config::{Config, Resolved, DEFAULT_CLICK_SLOP_PX};
use strokebind::device;
use strokebind::device::forward::forward;
use strokebind::device::grab::{install_failsafes, Grabbed};
use strokebind::device::source::{button_code, EvdevSource};
use strokebind::device::virtual_mouse::{MouseCaps, VirtualMouse};
use strokebind::error::Error;
use strokebind::focus::Focused;
use strokebind::hyprland::{socket_path, HyprlandAdapter};
use strokebind::passthrough::{Mode, Passthrough};
use strokebind::reload::ConfigWatcher;
use strokebind::stroke_loop::{run_events, LoopParams};
use strokebind::sway::SwayAdapter;
use strokebind::window::{Bounded, GlobalOnly, NeverAnswers};
use strokebind::x11::X11Adapter;

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
        Command::CheckConfig(args) => check_config(&args),
    }
}

/// Validate a config file: the result on stdout (exit 0) or the error on stderr (exit 1).
fn check_config(args: &CheckArgs) -> anyhow::Result<()> {
    let label = args.file.display().to_string();
    let report = match std::fs::read_to_string(&args.file) {
        Ok(src) => check(&label, &src),
        Err(source) => Err(format!(
            "error: {}\n",
            Error::Io {
                path: args.file.clone(),
                source,
            }
        )),
    };
    match report {
        Ok(text) => {
            print!("{text}");
            Ok(())
        }
        Err(text) => {
            eprint!("{text}");
            // Nothing is grabbed or open here; exit without anyhow's `Error:` wrapper.
            std::process::exit(1);
        }
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
    let path = pick_device(args.device.as_deref())?;
    let mut grab = Grabbed::grab(&path)?;
    let caps = MouseCaps::from_device(grab.device_mut());
    let mut current = cfg.as_ref().map(|c| c.resolve(&caps.name));
    let params = current.as_ref().map_or_else(LoopParams::default, |r| {
        LoopParams::new(r.trigger, r.min_stroke_px)
    });
    let escape = current
        .as_ref()
        .map(|r| r.escape.clone())
        .unwrap_or_default();
    let mode = match current.as_ref() {
        Some(r) if r.freeze_pointer => Mode::Frozen,
        Some(r) => Mode::Live {
            click_slop_px: r.click_slop_px,
        },
        None => Mode::Live {
            click_slop_px: DEFAULT_CLICK_SLOP_PX,
        },
    };
    let engine = Passthrough::new(&params)
        .with_mode(mode)
        .with_escape(escape.buttons().iter().map(|&b| button_code(b)).collect());
    // Refuse to run without a working escape; returning drops `grab`, releasing the mouse.
    caps.check_escape(&escape)?;
    let mut out = VirtualMouse::create(&caps)?;
    // Hot reload needs a config file; a failure to watch only disables it.
    let mut watcher = args
        .config
        .as_deref()
        .and_then(|p| match ConfigWatcher::new(p) {
            Ok(w) => Some(w),
            Err(e) => {
                eprintln!("hot reload disabled: {e}");
                None
            }
        });
    let mut actions = Actions::new();
    // Create the keyboard now so the compositor has opened it before the first key action.
    if let Some(r) = current.as_ref() {
        actions.prepare(&r.bindings)?;
    }
    eprintln!("grabbed {}; hold {} to exit", path.display(), escape);
    let engine = Focused::new(engine, Box::new(window_adapter()));
    forward(&mut grab, &mut out, engine, |stroke, app| {
        println!("{stroke}");
        match watcher.as_mut().and_then(ConfigWatcher::poll) {
            Some(Ok(new)) => {
                let new = new.resolve(&caps.name);
                if let Some(old) = current.as_ref() {
                    let restart = |r: &Resolved| {
                        (
                            r.trigger,
                            r.min_stroke_px,
                            r.escape.clone(),
                            r.freeze_pointer,
                            r.click_slop_px,
                        )
                    };
                    if restart(old) != restart(&new) {
                        eprintln!(
                            "trigger, min_stroke_px, escape, freeze_pointer and click_slop_px \
                             change only after a restart"
                        );
                    }
                }
                if let Err(e) = actions.prepare(&new.bindings) {
                    eprintln!("cannot create the virtual keyboard: {e}");
                }
                current = Some(new);
                eprintln!("config reloaded");
            }
            Some(Err(e)) => eprintln!("config reload failed, keeping the previous one: {e}"),
            None => {}
        }
        let bindings = current.as_ref().map(|r| r.bindings_for(app.as_ref()));
        let action = bindings.as_deref().and_then(|b| select(&stroke, b));
        if let Some(action) = action {
            if let Err(e) = actions.perform(action) {
                eprintln!("action for {stroke} failed: {e}");
            }
        }
        // After the action, so feedback never delays it; a failure (e.g. no notify-send) is only reported.
        let mode = current.as_ref().map(|r| r.feedback).unwrap_or_default();
        if let Err(e) = feedback::give(mode, &stroke, action.is_some(), |argv| {
            command::spawn_detached(argv, command::current_env())
        }) {
            eprintln!("feedback failed: {e}");
        }
    })?;
    eprintln!("escape chord held, exiting");
    Ok(())
}

/// The bounded window adapter for `run`, picked from the session environment; backends without an
/// adapter (KDE, GNOME) use the global bindings. `STROKEBIND_TEST_ADAPTER=hang` is a test hook:
/// lookups never answer.
fn window_adapter() -> Bounded {
    if std::env::var_os("STROKEBIND_TEST_ADAPTER").is_some_and(|v| v == "hang") {
        return Bounded::new(NeverAnswers);
    }
    match window_backend(|k| std::env::var_os(k)) {
        WindowBackend::X11 => Bounded::new(X11Adapter),
        WindowBackend::Sway => match std::env::var_os("SWAYSOCK") {
            Some(socket) => Bounded::new(SwayAdapter::new(socket)),
            None => Bounded::new(GlobalOnly),
        },
        WindowBackend::Hyprland => match socket_path(|k| std::env::var_os(k), |p| p.exists()) {
            Some(socket) => Bounded::new(HyprlandAdapter::new(socket)),
            None => Bounded::new(GlobalOnly),
        },
        WindowBackend::Unsupported => Bounded::new(GlobalOnly),
    }
}
