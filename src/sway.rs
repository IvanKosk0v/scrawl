//! Sway active-window adapter: `GET_TREE` over the i3 IPC socket (`SWAYSOCK`).

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

use serde::Deserialize;

use crate::window::{AppId, WindowAdapter};

const MAGIC: &[u8; 6] = b"i3-ipc";
const HEADER_LEN: usize = 14;
const GET_TREE: u32 = 4;
/// Largest reply accepted; a bigger one is treated as garbage.
const MAX_REPLY: usize = 16 << 20;
const IO_TIMEOUT: Duration = Duration::from_millis(40);

/// An IPC request: magic, payload length, message type (native endian), payload.
pub fn encode_message(kind: u32, payload: &[u8]) -> Vec<u8> {
    let len = u32::try_from(payload.len()).unwrap_or(u32::MAX);
    let mut out = Vec::with_capacity(HEADER_LEN + payload.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&len.to_ne_bytes());
    out.extend_from_slice(&kind.to_ne_bytes());
    out.extend_from_slice(payload);
    out
}

/// The payload length announced by a reply header, or `None` when the magic is wrong.
pub fn parse_header(header: &[u8; HEADER_LEN]) -> Option<usize> {
    if &header[..6] != MAGIC {
        return None;
    }
    let len = u32::from_ne_bytes([header[6], header[7], header[8], header[9]]);
    Some(len as usize)
}

#[derive(Debug, Deserialize)]
struct Node {
    #[serde(default)]
    focused: bool,
    #[serde(default)]
    app_id: Option<String>,
    #[serde(default)]
    window_properties: Option<Props>,
    #[serde(default)]
    nodes: Vec<Node>,
    #[serde(default)]
    floating_nodes: Vec<Node>,
}

#[derive(Debug, Deserialize)]
struct Props {
    #[serde(default)]
    class: Option<String>,
}

impl Node {
    fn focused_app(&self) -> Option<Option<AppId>> {
        if self.focused {
            let native = self.app_id.as_deref().and_then(AppId::new);
            let xwayland = || {
                let class = self.window_properties.as_ref()?.class.as_deref()?;
                AppId::new(class)
            };
            return Some(native.or_else(xwayland));
        }
        self.nodes
            .iter()
            .chain(&self.floating_nodes)
            .find_map(Node::focused_app)
    }
}

/// The focused window's `app_id` (XWayland: `window_properties.class`) from a `GET_TREE` reply.
/// `None` when nothing is focused, the focused node is not a window, or the JSON is invalid.
pub fn parse_focused_app(json: &[u8]) -> Option<AppId> {
    let tree: Node = serde_json::from_slice(json).ok()?;
    tree.focused_app().flatten()
}

type LookupResult = Result<Option<AppId>, Box<dyn std::error::Error>>;

fn lookup(socket: &std::path::Path) -> LookupResult {
    let mut stream = UnixStream::connect(socket)?;
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    stream.write_all(&encode_message(GET_TREE, b""))?;
    let mut header = [0u8; HEADER_LEN];
    stream.read_exact(&mut header)?;
    let len = parse_header(&header).ok_or("bad IPC reply header")?;
    if len > MAX_REPLY {
        return Err("IPC reply too large".into());
    }
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body)?;
    Ok(parse_focused_app(&body))
}

/// Asks Sway for its window tree on every call.
#[derive(Debug, Clone)]
pub struct SwayAdapter {
    socket: std::path::PathBuf,
}

impl SwayAdapter {
    /// An adapter talking to the IPC socket at `socket` (the value of `SWAYSOCK`).
    pub fn new(socket: impl Into<std::path::PathBuf>) -> Self {
        Self {
            socket: socket.into(),
        }
    }
}

impl WindowAdapter for SwayAdapter {
    fn active_window(&self) -> Option<AppId> {
        lookup(&self.socket).unwrap_or_else(|e| {
            eprintln!("sway window lookup failed: {e}");
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

    // Trimmed from a recorded `swaymsg -t get_tree`: root > output > workspace > con > windows.
    const TREE: &str = r#"{"id":1,"type":"root","focused":false,"nodes":[
      {"id":2,"type":"output","name":"eDP-1","focused":false,"nodes":[
        {"id":4,"type":"workspace","name":"1","focused":false,"nodes":[
          {"id":5,"type":"con","focused":false,"app_id":"foot","pid":10,"window_properties":null,"nodes":[],"floating_nodes":[]},
          {"id":6,"type":"con","focused":true,"app_id":"org.mozilla.firefox","pid":11,"nodes":[],"floating_nodes":[]}
        ],"floating_nodes":[]}
      ],"floating_nodes":[]}
    ],"floating_nodes":[]}"#;

    #[test]
    fn finds_the_focused_native_window() {
        assert_eq!(
            parse_focused_app(TREE.as_bytes()),
            id("org.mozilla.firefox")
        );
    }

    #[test]
    fn xwayland_window_uses_class() {
        let json = r#"{"focused":false,"nodes":[{"focused":true,"app_id":null,
            "window_properties":{"class":"Steam","instance":"steam"},"nodes":[]}]}"#;
        assert_eq!(parse_focused_app(json.as_bytes()), id("Steam"));
    }

    #[test]
    fn focused_floating_window_is_found() {
        let json = r#"{"focused":false,"nodes":[{"focused":false,"nodes":[],
            "floating_nodes":[{"focused":true,"app_id":"pavucontrol"}]}]}"#;
        assert_eq!(parse_focused_app(json.as_bytes()), id("pavucontrol"));
    }

    #[test]
    fn focused_empty_workspace_is_unknown() {
        let json = r#"{"focused":false,"nodes":[{"focused":true,"type":"workspace","nodes":[]}]}"#;
        assert_eq!(parse_focused_app(json.as_bytes()), None);
    }

    #[test]
    fn nothing_focused_or_invalid_json_is_unknown() {
        assert_eq!(parse_focused_app(br#"{"focused":false,"nodes":[]}"#), None);
        assert_eq!(parse_focused_app(b"not json"), None);
        assert_eq!(parse_focused_app(b""), None);
    }

    #[test]
    fn request_has_magic_length_and_type() {
        let msg = encode_message(GET_TREE, b"");
        assert_eq!(&msg[..6], b"i3-ipc");
        assert_eq!(&msg[6..10], &0u32.to_ne_bytes());
        assert_eq!(&msg[10..14], &4u32.to_ne_bytes());
        assert_eq!(msg.len(), 14);
        assert_eq!(&encode_message(0, b"ab")[6..10], &2u32.to_ne_bytes());
    }

    #[test]
    fn header_parsing_checks_magic() {
        let mut h = [0u8; 14];
        h[..6].copy_from_slice(b"i3-ipc");
        h[6..10].copy_from_slice(&1234u32.to_ne_bytes());
        assert_eq!(parse_header(&h), Some(1234));
        h[0] = b'x';
        assert_eq!(parse_header(&h), None);
    }

    #[test]
    fn talks_to_a_fake_socket() {
        let dir = std::env::temp_dir().join(format!("scrawl-sway-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sock");
        let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut req = [0u8; 14];
            s.read_exact(&mut req).unwrap();
            assert_eq!(req.to_vec(), encode_message(GET_TREE, b""));
            s.write_all(&encode_message(GET_TREE, TREE.as_bytes()))
                .unwrap();
        });
        assert_eq!(
            SwayAdapter::new(&path).active_window(),
            id("org.mozilla.firefox")
        );
        server.join().unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn missing_socket_is_unknown() {
        assert_eq!(
            SwayAdapter::new("/nonexistent/sway.sock").active_window(),
            None
        );
    }
}
