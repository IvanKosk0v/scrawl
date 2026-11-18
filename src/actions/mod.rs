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

/// Performs actions. The virtual keyboard is created on the first key action.
#[derive(Debug, Default)]
pub struct Actions {
    keyboard: Option<VirtualKeyboard>,
}

impl Actions {
    /// New runner; no device is created yet.
    pub fn new() -> Self {
        Self::default()
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
    fn select_finds_exact_stroke_only() {
        let b = bindings();
        assert_eq!(select("L", &b), Some(&Action::Key("alt+left".into())));
        assert_eq!(select("R", &b), Some(&Action::Run(vec!["true".into()])));
        assert_eq!(select("U", &b), None);
        assert_eq!(select("L-U", &b), None);
    }
}
