//! Actions run when a stroke is recognized.

pub mod command;
pub mod feedback;
pub mod key;

use crate::config::{Action, Binding};
use crate::device::virtual_keyboard::VirtualKeyboard;
use crate::error::Error;
use crate::recognizer::matcher::{match_stroke, MatchResult};

/// The action bound to exactly `stroke`; `None` when nothing is bound or the binding is ambiguous.
pub fn select<'a>(stroke: &str, bindings: &'a [Binding]) -> Option<&'a Action> {
    match match_stroke(stroke, bindings, 0) {
        MatchResult::Found(b) => Some(&b.action),
        MatchResult::Ambiguous | MatchResult::NoMatch => None,
    }
}

/// True when some binding emits a key combo, so a virtual keyboard is required.
pub fn needs_keyboard(bindings: &[Binding]) -> bool {
    bindings.iter().any(|b| matches!(b.action, Action::Key(_)))
}

/// Performs actions. Call [`Actions::prepare`] at startup and after each reload: a freshly created
/// uinput keyboard is not yet opened by the compositor, so keys sent right away are lost. A key
/// action without a prepared keyboard still creates one on the spot.
#[derive(Debug, Default)]
pub struct Actions {
    keyboard: Option<VirtualKeyboard>,
}

impl Actions {
    /// New runner; no device is created yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create the virtual keyboard now if `bindings` need one and it does not exist yet.
    pub fn prepare(&mut self, bindings: &[Binding]) -> Result<(), Error> {
        if self.keyboard.is_none() && needs_keyboard(bindings) {
            self.keyboard = Some(VirtualKeyboard::create()?);
        }
        Ok(())
    }

    /// Emit the key combo or start the command.
    pub fn perform(&mut self, action: &Action) -> Result<(), Error> {
        match action {
            Action::Key(combo) => {
                let combo = combo.parse::<key::KeyCombo>().map_err(Error::KeyCombo)?;
                if self.keyboard.is_none() {
                    self.keyboard = Some(VirtualKeyboard::create()?);
                }
                match self.keyboard.as_mut() {
                    Some(kb) => kb.press(&combo),
                    None => Ok(()),
                }
            }
            Action::Run(argv) => command::spawn_detached(argv, command::current_env()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn bindings() -> Vec<Binding> {
        Config::parse(
            "trigger = \"right\"\n\
             [[binding]]\nstroke = \"L\"\naction = { key = \"alt+left\" }\n\
             [[binding]]\nstroke = \"R\"\naction = { run = [\"true\"] }\n",
        )
        .map(|c| c.binding)
        .unwrap_or_default()
    }

    #[test]
    fn keyboard_is_needed_only_for_key_bindings() {
        let b = bindings();
        assert!(needs_keyboard(&b));
        assert!(!needs_keyboard(&b[1..]), "run-only bindings");
        assert!(!needs_keyboard(&[]));
    }

    #[test]
    fn select_finds_exact_stroke_only() {
        let b = bindings();
        assert_eq!(select("L", &b), Some(&Action::Key("alt+left".into())));
        assert_eq!(select("R", &b), Some(&Action::Run(vec!["true".into()])));
        assert_eq!(select("U", &b), None);
        assert_eq!(select("L-U", &b), None);
    }
}
