//! Hyprland active-window adapter: `j/activewindow` over the Hyprland command socket.

use std::ffi::OsString;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;

use crate::window::{AppId, WindowAdapter};

const IO_TIMEOUT: Duration = Duration::from_millis(40);
/// Largest reply accepted; a bigger one is treated as garbage.
const MAX_REPLY: u64 = 1 << 20;

/// The command socket for an instance: `$XDG_RUNTIME_DIR/hypr/<sig>/.socket.sock` when that
/// directory is known and the file exists, else the older `/tmp/hypr/<sig>/.socket.sock`.
/// `None` when the instance signature is unset or empty.
pub fn socket_path(
    env: impl Fn(&str) -> Option<OsString>,
    exists: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let sig = env("HYPRLAND_INSTANCE_SIGNATURE").filter(|s| !s.is_empty())?;
    let rel = Path::new("hypr").join(sig).join(".socket.sock");
    if let Some(dir) = env("XDG_RUNTIME_DIR").filter(|d| !d.is_empty()) {
        let path = Path::new(&dir).join(&rel);
        if exists(&path) {
            return Some(path);
        }
    }
    Some(Path::new("/tmp").join(rel))
}

#[derive(Debug, Deserialize)]
struct Window {
    #[serde(default)]
    class: Option<String>,
}

/// The focused window's `class` from a `j/activewindow` reply. `None` for `{}` (no focus), the
/// plain-text `Invalid` older versions send, an empty class, or any invalid JSON.
pub fn parse_active_window(json: &[u8]) -> Option<AppId> {
    let window: Window = serde_json::from_slice(json).ok()?;
    AppId::new(window.class.as_deref()?)
}

type LookupResult = Result<Option<AppId>, Box<dyn std::error::Error>>;

fn lookup(socket: &Path) -> LookupResult {
    let mut stream = UnixStream::connect(socket)?;
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    stream.write_all(b"j/activewindow")?;
    let mut body = Vec::new();
    stream.take(MAX_REPLY).read_to_end(&mut body)?;
    Ok(parse_active_window(&body))
}

/// Asks Hyprland for its active window on every call.
#[derive(Debug, Clone)]
pub struct HyprlandAdapter {
    socket: PathBuf,
}

impl HyprlandAdapter {
    /// An adapter talking to the command socket at `socket`.
    pub fn new(socket: impl Into<PathBuf>) -> Self {
        Self {
            socket: socket.into(),
        }
    }
}

impl WindowAdapter for HyprlandAdapter {
    fn active_window(&self) -> Option<AppId> {
        lookup(&self.socket).unwrap_or_else(|e| {
            eprintln!("hyprland window lookup failed: {e}");
            None
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> Option<AppId> {
        AppId::new(s)
    }

    // Trimmed from a recorded `hyprctl -j activewindow`.
    const ACTIVE: &str = r#"{"address":"0x5581a6b0c1d0","mapped":true,"hidden":false,
      "at":[10,40],"size":[1900,1000],"workspace":{"id":1,"name":"1"},"floating":false,
      "pid":4242,"class":"org.mozilla.firefox","title":"Hyprland - Mozilla Firefox",
      "initialClass":"firefox","initialTitle":"Mozilla Firefox","xwayland":false}"#;

    #[test]
    fn class_of_the_active_window() {
        assert_eq!(
            parse_active_window(ACTIVE.as_bytes()),
            id("org.mozilla.firefox")
        );
    }

    #[test]
    fn no_focus_or_garbage_is_unknown() {
        assert_eq!(parse_active_window(b"{}"), None);
        assert_eq!(parse_active_window(b"Invalid"), None);
        assert_eq!(parse_active_window(b""), None);
        assert_eq!(parse_active_window(br#"{"class":""}"#), None);
        assert_eq!(parse_active_window(br#"{"class":null}"#), None);
    }

    fn env(k: &str) -> Option<OsString> {
        match k {
            "HYPRLAND_INSTANCE_SIGNATURE" => Some("abc_123".into()),
            "XDG_RUNTIME_DIR" => Some("/run/user/1000".into()),
            _ => None,
        }
    }

    #[test]
    fn socket_prefers_runtime_dir_when_present() {
        assert_eq!(
            socket_path(env, |_| true),
            Some(PathBuf::from("/run/user/1000/hypr/abc_123/.socket.sock"))
        );
    }

    #[test]
    fn socket_falls_back_to_tmp() {
        let expected = Some(PathBuf::from("/tmp/hypr/abc_123/.socket.sock"));
        assert_eq!(socket_path(env, |_| false), expected);
        let no_runtime = |k: &str| (k != "XDG_RUNTIME_DIR").then(|| env(k)).flatten();
        assert_eq!(socket_path(no_runtime, |_| true), expected);
    }

    #[test]
    fn socket_needs_a_signature() {
        assert_eq!(socket_path(|_| None, |_| true), None);
        let empty = |k: &str| (k == "HYPRLAND_INSTANCE_SIGNATURE").then(OsString::new);
        assert_eq!(socket_path(empty, |_| true), None);
    }

    #[test]
    fn talks_to_a_fake_socket() {
        let dir = std::env::temp_dir().join(format!("scrawl-hypr-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sock");
        let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut req = [0u8; 14];
            s.read_exact(&mut req).unwrap();
            assert_eq!(&req, b"j/activewindow");
            s.write_all(ACTIVE.as_bytes()).unwrap();
        });
        assert_eq!(
            HyprlandAdapter::new(&path).active_window(),
            id("org.mozilla.firefox")
        );
        server.join().unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn missing_socket_is_unknown() {
        assert_eq!(
            HyprlandAdapter::new("/nonexistent/hypr.sock").active_window(),
            None
        );
    }
}
