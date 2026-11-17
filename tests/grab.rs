//! M3.2: exclusive grab is released on Drop, panic (any thread), SIGINT and SIGTERM.
//!
//! Every test grabs a uinput *virtual* mouse, never real hardware. Exit paths run in a child process: the test
//! binary re-executes itself and runs `child_grabber`.
#![allow(clippy::expect_used)] // test helpers outside #[test] fns

mod common;

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use common::{fake_caps, open_when_ready};
use strokebind::device::grab::{install_failsafes, Grabbed};
use strokebind::device::virtual_mouse::VirtualMouse;

const NODE_ENV: &str = "SB_CHILD_NODE";
const MODE_ENV: &str = "SB_CHILD_MODE";
const EBUSY: i32 = 16;
const O_CLOEXEC: u32 = 0o2000000;

/// Create a virtual mouse and return it with its (readable) event node.
fn virtual_source(name: &str) -> (VirtualMouse, PathBuf) {
    let mut vm = VirtualMouse::create(&fake_caps(name)).expect("create virtual mouse");
    let node = vm.dev_node().expect("dev node");
    drop(open_when_ready(&node));
    (vm, node)
}

/// True if another fd holds an exclusive grab on `node`.
fn is_grabbed(node: &Path) -> bool {
    let mut dev = strokebind::device::source::open_readonly(node).expect("open probe");
    match dev.grab() {
        Ok(()) => {
            dev.ungrab().expect("ungrab probe");
            false
        }
        Err(e) if e.raw_os_error() == Some(EBUSY) => true,
        Err(e) => panic!("probe grab failed: {e}"),
    }
}

/// Child side: grabs `SB_CHILD_NODE` and then waits or panics per `SB_CHILD_MODE`. No-op when run directly.
#[test]
#[ignore = "needs writable /dev/uinput"]
fn child_grabber() {
    let Ok(node) = std::env::var(NODE_ENV) else {
        return;
    };
    install_failsafes().expect("failsafes");
    let _grab = Grabbed::grab(Path::new(&node)).expect("grab");
    println!("GRABBED");
    if std::env::var(MODE_ENV).as_deref() == Ok("panic") {
        std::thread::spawn(|| panic!("deliberate panic in a worker thread"));
    }
    // Without a working failsafe the grab outlives the parent's 5 s deadline.
    std::thread::sleep(Duration::from_secs(10));
}

fn spawn_child(node: &Path, mode: &str) -> Child {
    let mut child = Command::new(std::env::current_exe().expect("current exe"))
        .args(["--exact", "child_grabber", "--ignored", "--nocapture"])
        .env(NODE_ENV, node)
        .env(MODE_ENV, mode)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn child");
    let stdout = child.stdout.take().expect("child stdout");
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if line.trim() == "GRABBED" {
                let _ = tx.send(());
            }
        }
    });
    if rx.recv_timeout(Duration::from_secs(5)).is_err() {
        let _ = child.kill();
        panic!("child never reported GRABBED");
    }
    child
}

/// Wait up to 5 s for the child to exit; returns its exit code.
fn wait_exit(child: &mut Child) -> Option<i32> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait().expect("try_wait") {
            return status.code();
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let _ = child.kill();
    let _ = child.wait();
    panic!("child still running (and holding the grab) after 5 s");
}

fn signal(child: &Child, sig: &str) {
    let ok = Command::new("kill")
        .args([sig, &child.id().to_string()])
        .status()
        .expect("run kill")
        .success();
    assert!(ok, "kill {sig} failed");
}

#[test]
#[ignore = "needs writable /dev/uinput"]
fn second_grab_is_busy() {
    let (_vm, node) = virtual_source("Grab Busy Test");
    let grab = Grabbed::grab(&node).expect("first grab");
    assert!(is_grabbed(&node));
    assert!(Grabbed::grab(&node).is_err(), "second grab must fail");
    drop(grab);
}

#[test]
#[ignore = "needs writable /dev/uinput"]
fn release_on_drop() {
    let (_vm, node) = virtual_source("Grab Drop Test");
    let grab = Grabbed::grab(&node).expect("grab");
    assert!(is_grabbed(&node));
    drop(grab);
    assert!(!is_grabbed(&node));
}

#[test]
#[ignore = "needs writable /dev/uinput"]
fn release_on_panic_in_other_thread() {
    let (_vm, node) = virtual_source("Grab Panic Test");
    let mut child = spawn_child(&node, "panic");
    assert_eq!(wait_exit(&mut child), Some(101));
    assert!(!is_grabbed(&node));
}

#[test]
#[ignore = "needs writable /dev/uinput"]
fn release_on_sigint() {
    let (_vm, node) = virtual_source("Grab Sigint Test");
    let mut child = spawn_child(&node, "wait");
    assert!(is_grabbed(&node));
    signal(&child, "-INT");
    assert_eq!(wait_exit(&mut child), Some(130));
    assert!(!is_grabbed(&node));
}

#[test]
#[ignore = "needs writable /dev/uinput"]
fn release_on_sigterm() {
    let (_vm, node) = virtual_source("Grab Sigterm Test");
    let mut child = spawn_child(&node, "wait");
    assert!(is_grabbed(&node));
    signal(&child, "-TERM");
    assert_eq!(wait_exit(&mut child), Some(143));
    assert!(!is_grabbed(&node));
}

#[test]
#[ignore = "needs writable /dev/uinput"]
fn grab_fd_is_cloexec() {
    use std::os::fd::AsRawFd;
    let (_vm, node) = virtual_source("Grab Cloexec Test");
    let mut grab = Grabbed::grab(&node).expect("grab");
    let fd = grab.device_mut().as_raw_fd();
    let info = std::fs::read_to_string(format!("/proc/self/fdinfo/{fd}")).expect("fdinfo");
    let flags = info
        .lines()
        .find_map(|l| l.strip_prefix("flags:"))
        .and_then(|v| u32::from_str_radix(v.trim(), 8).ok())
        .expect("flags line");
    assert_ne!(
        flags & O_CLOEXEC,
        0,
        "grabbed fd would leak into child processes"
    );
}

/// M3.3 end to end: grabbed virtual source A -> forward -> mirrored virtual mouse B.
#[test]
#[ignore = "needs writable /dev/uinput"]
fn forward_end_to_end() {
    use evdev::{EventType, InputEvent, KeyCode, RelativeAxisCode};
    use strokebind::config::Trigger;
    use strokebind::device::forward::forward;
    use strokebind::device::virtual_mouse::MouseCaps;
    use strokebind::passthrough::Passthrough;
    use strokebind::stroke_loop::LoopParams;

    let (mut src_vm, node) = virtual_source("Forward Source Test");
    let mut grab = Grabbed::grab(&node).expect("grab");
    let mut caps = MouseCaps::from_device(grab.device_mut());
    caps.name = "Forward Out Test".into();
    let mut out = VirtualMouse::create(&caps).expect("create out");
    let mut reader = open_when_ready(&out.dev_node().expect("out node"));

    let (key_tx, key_rx) = mpsc::channel();
    std::thread::spawn(move || {
        while let Ok(events) = reader.fetch_events() {
            for e in events.filter(|e| e.event_type() == EventType::KEY) {
                let _ = key_tx.send((e.code(), e.value()));
            }
        }
    });
    let (stroke_tx, stroke_rx) = mpsc::channel();
    let fwd = std::thread::spawn(move || {
        let engine = Passthrough::new(&LoopParams::new(Trigger::Right, 40));
        forward(&mut grab, &mut out, engine, |s| {
            let _ = stroke_tx.send(s);
        })
    });

    let right = |v| InputEvent::new(EventType::KEY.0, KeyCode::BTN_RIGHT.0, v);
    let wait = Duration::from_secs(2);
    src_vm.emit(&[right(1)]).expect("emit press");
    src_vm.emit(&[right(0)]).expect("emit release");
    assert_eq!(
        key_rx.recv_timeout(wait).ok(),
        Some((KeyCode::BTN_RIGHT.0, 1))
    );
    assert_eq!(
        key_rx.recv_timeout(wait).ok(),
        Some((KeyCode::BTN_RIGHT.0, 0))
    );

    src_vm.emit(&[right(1)]).expect("emit press");
    for _ in 0..5 {
        let mv = InputEvent::new(EventType::RELATIVE.0, RelativeAxisCode::REL_X.0, 20);
        src_vm.emit(&[mv]).expect("emit move");
    }
    src_vm.emit(&[right(0)]).expect("emit release");
    assert_eq!(stroke_rx.recv_timeout(wait).ok().as_deref(), Some("R"));
    assert!(
        key_rx.recv_timeout(Duration::from_millis(300)).is_err(),
        "stroke buttons must not reach the virtual mouse"
    );

    drop(src_vm);
    let res = fwd.join().expect("forward thread");
    assert!(res.is_err(), "unplugging the source ends the loop");
}

/// Start `strokebind run` on `node`; returns once it reports the grab on stderr.
fn spawn_run(node: &Path, timeout_s: &str) -> Child {
    let mut child = Command::new(env!("CARGO_BIN_EXE_strokebind"))
        .args(["run", "--device"])
        .arg(node)
        .args(["--timeout", timeout_s])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn strokebind run");
    let stderr = child.stderr.take().expect("run stderr");
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if line.contains("grabbed") {
                let _ = tx.send(());
            }
        }
    });
    if rx.recv_timeout(Duration::from_secs(5)).is_err() {
        let _ = child.kill();
        panic!("run never reported the grab");
    }
    child
}

/// M3.4: `run --timeout 1` exits by itself and releases the grab.
#[test]
#[ignore = "needs writable /dev/uinput"]
fn run_timeout_exits_and_releases() {
    let (_vm, node) = virtual_source("Run Timeout Test");
    let mut child = spawn_run(&node, "1");
    assert!(is_grabbed(&node));
    assert_eq!(wait_exit(&mut child), Some(0));
    assert!(!is_grabbed(&node));
}

/// M3.4: holding the default escape chord (left+right+middle) exits `run` and releases the grab.
#[test]
#[ignore = "needs writable /dev/uinput"]
fn run_escape_chord_exits_and_releases() {
    use evdev::{EventType, InputEvent, KeyCode};
    let (mut vm, node) = virtual_source("Run Escape Test");
    // Long timeout only as a backstop; the chord must end the run well before it.
    let mut child = spawn_run(&node, "30");
    for code in [KeyCode::BTN_LEFT, KeyCode::BTN_RIGHT, KeyCode::BTN_MIDDLE] {
        vm.emit(&[InputEvent::new(EventType::KEY.0, code.0, 1)])
            .expect("emit press");
    }
    assert_eq!(wait_exit(&mut child), Some(0));
    assert!(!is_grabbed(&node));
}
