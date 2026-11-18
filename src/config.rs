//! TOML configuration types and validation.

use std::str::FromStr;

use serde::{Deserialize, Deserializer};

use crate::actions::key::KeyCombo;
use crate::error::Error;

const DIRECTIONS: [&str; 8] = ["U", "D", "L", "R", "UL", "UR", "DL", "DR"];

/// Mouse button that starts a stroke while held.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Trigger {
    /// Right button.
    Right,
    /// Middle button.
    Middle,
    /// Side button.
    Side,
    /// Extra button.
    Extra,
}

/// Any mouse button that can be part of the escape chord.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    /// Left button.
    Left,
    /// Right button.
    Right,
    /// Middle button.
    Middle,
    /// Side button.
    Side,
    /// Extra button.
    Extra,
}

impl MouseButton {
    /// Name as written in the config.
    pub fn as_str(&self) -> &'static str {
        match self {
            MouseButton::Left => "left",
            MouseButton::Right => "right",
            MouseButton::Middle => "middle",
            MouseButton::Side => "side",
            MouseButton::Extra => "extra",
        }
    }
}

impl FromStr for MouseButton {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "left" => Ok(MouseButton::Left),
            "right" => Ok(MouseButton::Right),
            "middle" => Ok(MouseButton::Middle),
            "side" => Ok(MouseButton::Side),
            "extra" => Ok(MouseButton::Extra),
            _ => Err(format!(
                "unknown mouse button `{s}` (expected left, right, middle, side, extra)"
            )),
        }
    }
}

/// Default escape chord.
pub const DEFAULT_ESCAPE: &str = "left+right+middle";

/// Failsafe chord: holding all these mouse buttons at once makes `run` exit.
/// Written like `left+right+middle`; at least two distinct buttons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Escape(Vec<MouseButton>);

impl Escape {
    /// The buttons of the chord, in config order.
    pub fn buttons(&self) -> &[MouseButton] {
        &self.0
    }
}

impl std::fmt::Display for Escape {
    /// Written as in the config, e.g. `left+right+middle`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<&str> = self.0.iter().map(MouseButton::as_str).collect();
        f.write_str(&names.join("+"))
    }
}

impl Default for Escape {
    fn default() -> Self {
        Escape(vec![
            MouseButton::Left,
            MouseButton::Right,
            MouseButton::Middle,
        ])
    }
}

impl FromStr for Escape {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut buttons: Vec<MouseButton> = Vec::new();
        for name in s.split('+').filter(|n| !n.is_empty()) {
            let b = name.parse()?;
            if !buttons.contains(&b) {
                buttons.push(b);
            }
        }
        if buttons.len() < 2 {
            return Err(format!(
                "escape `{s}` needs at least two distinct buttons, e.g. `{DEFAULT_ESCAPE}`"
            ));
        }
        Ok(Escape(buttons))
    }
}

impl<'de> Deserialize<'de> for Escape {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// A validated stroke string: one or more 8-direction tokens joined by `-`, e.g. `R`, `DR`, `L-U`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stroke(String);

impl Stroke {
    /// The stroke as written in the config.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Stroke {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.split('-').all(|t| DIRECTIONS.contains(&t)) {
            Ok(Stroke(s.to_owned()))
        } else {
            Err(format!(
                "unknown stroke `{s}` (expected directions U, D, L, R, UL, UR, DL, DR joined by `-`)"
            ))
        }
    }
}

impl<'de> Deserialize<'de> for Stroke {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// What to do when a stroke is recognized. Validated when the config is loaded.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "RawAction")]
pub enum Action {
    /// Emit a key combo, e.g. `alt+left` (always a valid [`KeyCombo`]).
    Key(String),
    /// Run a command with arguments (no shell); the program is never empty.
    Run(Vec<String>),
}

/// Action as written in the file, before validation.
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum RawAction {
    Key(String),
    Run(Vec<String>),
}

impl TryFrom<RawAction> for Action {
    type Error = String;

    fn try_from(raw: RawAction) -> Result<Self, String> {
        match raw {
            RawAction::Key(combo) => {
                combo.parse::<KeyCombo>()?;
                Ok(Action::Key(combo))
            }
            RawAction::Run(argv) if argv.first().is_none_or(String::is_empty) => {
                Err(Error::EmptyCommand.to_string())
            }
            RawAction::Run(argv) => Ok(Action::Run(argv)),
        }
    }
}

/// What the user is shown after a recognized stroke.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Feedback {
    /// Nothing (default).
    #[default]
    Off,
    /// A desktop notification via `notify-send` naming the stroke, or saying nothing is bound.
    Notify,
}

/// One stroke-to-action binding.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    /// Stroke that triggers the action.
    pub stroke: Stroke,
    /// Action to perform.
    pub action: Action,
}

/// Parsed user configuration.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Button held while drawing a stroke.
    pub trigger: Trigger,
    /// Minimum movement in pixels before a press counts as a stroke.
    #[serde(default = "default_min_stroke_px")]
    pub min_stroke_px: u32,
    /// Failsafe chord that exits `run` (default `left+right+middle`).
    #[serde(default)]
    pub escape: Escape,
    /// Feedback after each recognized stroke (default off).
    #[serde(default)]
    pub feedback: Feedback,
    /// Buffer all motion until the trigger is released instead of moving the pointer live (default false).
    #[serde(default)]
    pub freeze_pointer: bool,
    /// Live mode: a release within this many pixels of the press point is a click (default 6; must be
    /// less than `min_stroke_px`).
    #[serde(default = "default_click_slop_px")]
    pub click_slop_px: u32,
    /// Stroke bindings.
    #[serde(default)]
    pub binding: Vec<Binding>,
    /// Per-device overrides; the first whose `device` matches wins.
    #[serde(default)]
    pub profile: Vec<Profile>,
}

/// Overrides for one kind of mouse, selected by device name.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// Case-insensitive substring of the device name this profile applies to (never blank: a blank
    /// one would match every mouse).
    #[serde(deserialize_with = "non_blank")]
    pub device: String,
    /// Trigger button for this device (default: the global one).
    pub trigger: Option<Trigger>,
    /// Minimum stroke length for this device (default: the global one).
    pub min_stroke_px: Option<u32>,
    /// Escape chord for this device (default: the global one).
    pub escape: Option<Escape>,
    /// Bindings that take precedence over the global ones for the same stroke.
    #[serde(default)]
    pub binding: Vec<Binding>,
}

/// The settings that apply to one device: global values with its profile applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// Button held while drawing a stroke.
    pub trigger: Trigger,
    /// Minimum movement in pixels before a press counts as a stroke.
    pub min_stroke_px: u32,
    /// Failsafe chord.
    pub escape: Escape,
    /// Feedback after each recognized stroke (always the global setting).
    pub feedback: Feedback,
    /// Buffer motion until release (always the global setting).
    pub freeze_pointer: bool,
    /// Click distance in live mode (always the global setting).
    pub click_slop_px: u32,
    /// Profile bindings first, then the global ones for strokes the profile does not bind.
    pub bindings: Vec<Binding>,
}

/// Default for `min_stroke_px`.
pub const DEFAULT_MIN_STROKE_PX: u32 = 40;

fn default_min_stroke_px() -> u32 {
    DEFAULT_MIN_STROKE_PX
}

/// Default for `click_slop_px`.
pub const DEFAULT_CLICK_SLOP_PX: u32 = 6;

fn default_click_slop_px() -> u32 {
    DEFAULT_CLICK_SLOP_PX
}

impl Config {
    /// Parse and validate a TOML config; errors carry the 1-based line when known. A missing
    /// top-level field has no line (toml blames the root table, which starts at offset 0).
    pub fn parse(src: &str) -> Result<Self, Error> {
        let cfg: Config = toml::from_str(src).map_err(|e| {
            let message = e.message().to_owned();
            let root_missing = message.starts_with("missing field");
            Error::Config {
                line: e
                    .span()
                    .filter(|s| !(root_missing && s.start == 0))
                    .map(|s| src[..s.start].bytes().filter(|&b| b == b'\n').count() + 1),
                message,
            }
        })?;
        cfg.check_slop(src)?;
        Ok(cfg)
    }

    /// Live mode only: `click_slop_px` must stay below every effective `min_stroke_px`, or clicks
    /// would swallow strokes. The error points at the profile's own `min_stroke_px` when a profile
    /// is at fault, else at the top-level `click_slop_px` (or `min_stroke_px` when slop is default).
    fn check_slop(&self, src: &str) -> Result<(), Error> {
        if self.freeze_pointer {
            return Ok(());
        }
        let slop = self.click_slop_px;
        let error = |min: u32, place: String, line: Option<usize>| Error::Config {
            line,
            message: format!(
                "click_slop_px ({slop}) must be less than min_stroke_px ({min}){place}"
            ),
        };
        if slop >= self.min_stroke_px {
            let line = key_line(src, Section::Top, "click_slop_px")
                .or_else(|| key_line(src, Section::Top, "min_stroke_px"));
            return Err(error(self.min_stroke_px, String::new(), line));
        }
        for (i, p) in self.profile.iter().enumerate() {
            if let Some(min) = p.min_stroke_px.filter(|&m| slop >= m) {
                let place = format!(" in profile \"{}\"", p.device);
                let line = key_line(src, Section::Profile(i), "min_stroke_px");
                return Err(error(min, place, line));
            }
        }
        Ok(())
    }

    /// The settings for the device called `device_name`: the first profile whose `device` is a
    /// case-insensitive substring of the name is applied over the global settings.
    pub fn resolve(&self, device_name: &str) -> Resolved {
        let name = device_name.to_lowercase();
        let profile = self
            .profile
            .iter()
            .find(|p| name.contains(&p.device.to_lowercase()));
        let mut bindings = profile.map_or_else(Vec::new, |p| p.binding.clone());
        for b in &self.binding {
            if !bindings.iter().any(|x| x.stroke == b.stroke) {
                bindings.push(b.clone());
            }
        }
        Resolved {
            trigger: profile.and_then(|p| p.trigger).unwrap_or(self.trigger),
            min_stroke_px: profile
                .and_then(|p| p.min_stroke_px)
                .unwrap_or(self.min_stroke_px),
            escape: profile
                .and_then(|p| p.escape.clone())
                .unwrap_or_else(|| self.escape.clone()),
            feedback: self.feedback,
            freeze_pointer: self.freeze_pointer,
            click_slop_px: self.click_slop_px,
            bindings,
        }
    }
}

/// Where [`key_line`] looks for a key.
#[derive(Clone, Copy)]
enum Section {
    /// Top-level keys, before the first table header.
    Top,
    /// Keys of the n-th (0-based) `[[profile]]` table, up to its next header.
    Profile(usize),
}

/// 1-based line of the first `key = ...` assignment in `section`; a best-effort text search for
/// errors found after parsing (`None` if the layout is unusual).
fn key_line(src: &str, section: Section, key: &str) -> Option<usize> {
    let lines: Vec<&str> = src.lines().map(str::trim_start).collect();
    let start = match section {
        Section::Top => 0,
        Section::Profile(n) => {
            lines
                .iter()
                .enumerate()
                .filter(|(_, l)| l.starts_with("[[profile]]"))
                .nth(n)?
                .0
                + 1
        }
    };
    lines[start..]
        .iter()
        .take_while(|l| !l.starts_with('['))
        .position(|l| {
            l.strip_prefix(key)
                .is_some_and(|rest| rest.trim_start().starts_with('='))
        })
        .map(|i| start + i + 1)
}

/// A string that is not empty or only whitespace (profile `device`).
fn non_blank<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let s = String::deserialize(d)?;
    if s.trim().is_empty() {
        return Err(serde::de::Error::custom(
            "profile device must name part of a mouse name (see `strokebind list-devices`); \
             an empty one would match every mouse",
        ));
    }
    Ok(s)
}
