//! uinput virtual keyboard that performs key-combo actions. strokebind never reads or grabs real keyboards.

use std::path::PathBuf;

use evdev::uinput::VirtualDevice;

use super::virtual_mouse::{uinput_err, NAME_PREFIX};
use crate::actions::key::{keyboard_keys, KeyCombo};
use crate::error::Error;

/// A uinput keyboard declaring every key a [`KeyCombo`] can name; removed when dropped.
#[derive(Debug)]
pub struct VirtualKeyboard {
    dev: VirtualDevice,
}

impl VirtualKeyboard {
    /// Create the keyboard, named `strokebind: keyboard`.
    pub fn create() -> Result<Self, Error> {
        let name = format!("{NAME_PREFIX}keyboard");
        let dev = VirtualDevice::builder()
            .map_err(uinput_err)?
            .name(name.as_str())
            .with_keys(&keyboard_keys())
            .and_then(|b| b.build())
            .map_err(uinput_err)?;
        Ok(VirtualKeyboard { dev })
    }

    /// Press and release `combo`, one report per key transition.
    pub fn press(&mut self, combo: &KeyCombo) -> Result<(), Error> {
        for report in combo.reports() {
            self.dev.emit(&report).map_err(uinput_err)?;
        }
        Ok(())
    }

    /// The `/dev/input/event*` node of this keyboard.
    pub fn dev_node(&mut self) -> Result<PathBuf, Error> {
        super::virtual_mouse::first_node(&mut self.dev)
    }
}
