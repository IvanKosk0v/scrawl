//! TOML configuration types and validation.

use std::str::FromStr;

use serde::{Deserialize, Deserializer};

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

/// What to do when a stroke is recognized.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    /// Emit a key combo, e.g. `alt+left`.
    Key(String),
    /// Run a command with arguments (no shell).
    Run(Vec<String>),
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
    /// Stroke bindings.
    #[serde(default)]
    pub binding: Vec<Binding>,
}

fn default_min_stroke_px() -> u32 {
    40
}

impl Config {
    /// Parse and validate a TOML config; errors carry the 1-based line when known.
    pub fn parse(src: &str) -> Result<Self, Error> {
        toml::from_str(src).map_err(|e| Error::Config {
            line: e
                .span()
                .map(|s| src[..s.start].bytes().filter(|&b| b == b'\n').count() + 1),
            message: e.message().to_owned(),
        })
    }
}
