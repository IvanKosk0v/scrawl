//! uinput virtual mouse that mirrors the capabilities of a real one. Creating it never grabs anything.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use evdev::uinput::VirtualDevice;
use evdev::{
    AttributeSet, Device, InputEvent, InputId, KeyCode, MiscCode, PropType, RelativeAxisCode,
};

use super::source::button_code;
use crate::config::{Escape, MouseButton};
use crate::error::Error;

/// Path of the uinput control node.
const UINPUT_PATH: &str = "/dev/uinput";

/// Prefix of every device name scrawl creates, used to recognize (and skip) our own devices.
pub const NAME_PREFIX: &str = "scrawl: ";

/// Longest name evdev accepts: its builder asserts `len + 1 < UINPUT_MAX_NAME_SIZE (80)`.
pub const MAX_NAME_LEN: usize = 78;

/// Capabilities of a source mouse that the virtual mouse copies. Absolute axes are not mirrored.
#[derive(Debug, Clone)]
pub struct MouseCaps {
    /// Kernel name of the source device.
    pub name: String,
    /// Bus, vendor, product and version, so hwdb and libinput quirks match the real mouse.
    pub id: InputId,
    /// Supported buttons.
    pub keys: AttributeSet<KeyCode>,
    /// Supported relative axes (motion, wheels).
    pub rel: AttributeSet<RelativeAxisCode>,
    /// Supported misc events (e.g. `MSC_SCAN`).
    pub msc: AttributeSet<MiscCode>,
    /// Input properties.
    pub props: AttributeSet<PropType>,
}

impl MouseCaps {
    /// Read the capabilities of an open device.
    pub fn from_device(dev: &Device) -> Self {
        MouseCaps {
            name: dev.name().unwrap_or("unknown").to_owned(),
            id: dev.input_id(),
            keys: dev
                .supported_keys()
                .map(|k| k.iter().collect())
                .unwrap_or_default(),
            rel: dev
                .supported_relative_axes()
                .map(|r| r.iter().collect())
                .unwrap_or_default(),
            msc: dev
                .misc_properties()
                .map(|m| m.iter().collect())
                .unwrap_or_default(),
            props: dev.properties().iter().collect(),
        }
    }
}

impl MouseCaps {
    /// Fail unless every button of `escape` exists on this mouse; otherwise the failsafe could never fire.
    pub fn check_escape(&self, escape: &Escape) -> Result<(), Error> {
        let missing: Vec<&str> = escape
            .buttons()
            .iter()
            .filter(|&&b| !self.keys.contains(button_code(b)))
            .map(MouseButton::as_str)
            .collect();
        if missing.is_empty() {
            return Ok(());
        }
        Err(Error::EscapeUnavailable {
            chord: escape.to_string(),
            device: self.name.clone(),
            missing: missing.join(", "),
        })
    }
}

/// Name for the virtual twin of `source`: prefixed, cut on a char boundary to [`MAX_NAME_LEN`] bytes.
pub fn virtual_name(source: &str) -> String {
    let mut name = format!("{NAME_PREFIX}{source}");
    let mut end = name.len().min(MAX_NAME_LEN);
    while !name.is_char_boundary(end) {
        end -= 1;
    }
    name.truncate(end);
    name
}

/// True if `name` belongs to a device scrawl created.
pub fn is_virtual_name(name: &str) -> bool {
    name.starts_with(NAME_PREFIX)
}

/// A uinput mouse; the kernel removes it when this value is dropped (its fd closes).
#[derive(Debug)]
pub struct VirtualMouse {
    dev: VirtualDevice,
}

/// Map a uinput I/O error, turning permission problems into [`Error::UinputDenied`].
pub(crate) fn uinput_err(source: std::io::Error) -> Error {
    let path = PathBuf::from(UINPUT_PATH);
    if source.kind() == ErrorKind::PermissionDenied {
        Error::UinputDenied(path)
    } else {
        Error::Io { path, source }
    }
}

impl VirtualMouse {
    /// Create a virtual mouse with the capabilities in `caps`.
    pub fn create(caps: &MouseCaps) -> Result<Self, Error> {
        let name = virtual_name(&caps.name);
        let dev = VirtualDevice::builder()
            .map_err(uinput_err)?
            .name(name.as_str())
            .input_id(caps.id.clone())
            .with_keys(&caps.keys)
            .and_then(|b| b.with_relative_axes(&caps.rel))
            .and_then(|b| b.with_msc(&caps.msc))
            .and_then(|b| b.with_properties(&caps.props))
            .and_then(|b| b.build())
            .map_err(uinput_err)?;
        Ok(VirtualMouse { dev })
    }

    /// Write `events` followed by a `SYN_REPORT`.
    pub fn emit(&mut self, events: &[InputEvent]) -> Result<(), Error> {
        self.dev.emit(events).map_err(uinput_err)
    }

    /// The `/dev/input/event*` node of this virtual mouse.
    pub fn dev_node(&mut self) -> Result<PathBuf, Error> {
        first_node(&mut self.dev)
    }
}

/// The first `/dev/input/event*` node of a uinput device.
pub(crate) fn first_node(dev: &mut VirtualDevice) -> Result<PathBuf, Error> {
    let not_found = || Error::Io {
        path: Path::new(UINPUT_PATH).to_path_buf(),
        source: std::io::Error::new(ErrorKind::NotFound, "virtual device has no event node"),
    };
    dev.enumerate_dev_nodes_blocking()
        .map_err(uinput_err)?
        .next()
        .ok_or_else(not_found)?
        .map_err(uinput_err)
}
