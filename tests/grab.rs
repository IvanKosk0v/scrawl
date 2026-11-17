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
