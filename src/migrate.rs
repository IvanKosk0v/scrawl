//! Migration hint for v0.1.0 users: the config moved from `<config home>/strokebind/` to
//! `<config home>/scrawl/`. The old file is never read; the missing-config error names it instead.

use std::path::{Path, PathBuf};

/// Config dir name used by v0.1.0, before the rename.
const OLD_DIR: &str = "strokebind";
/// Config dir name used now.
const NEW_DIR: &str = "scrawl";

/// A config left behind in the old dir, and the command that moves it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OldConfig {
    /// The old config file that exists.
    pub old: PathBuf,
    /// Shell command moving the old dir to the new one.
    pub mv: String,
}

/// `$XDG_CONFIG_HOME`, else `$HOME/.config` (what systemd's `%E` expands to).
pub fn config_home() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
}

/// If `path` is the default `<home>/scrawl/config.toml`, it does not exist, and
/// `<home>/strokebind/config.toml` does, return the old file and the `mv` fix (paths single-quoted;
/// moves only the file when `<home>/scrawl/` already exists).
pub fn old_config(path: &Path, home: Option<&Path>) -> Option<OldConfig> {
    let home = home?;
    let new_dir = home.join(NEW_DIR);
    if path != new_dir.join("config.toml") || path.exists() {
        return None;
    }
    let old_dir = home.join(OLD_DIR);
    let old = old_dir.join("config.toml");
    if !old.is_file() {
        return None;
    }
    // Moving the dir onto an existing one would nest it; move just the file then.
    let mv = if new_dir.exists() {
        format!("mv '{}' '{}/'", old.display(), new_dir.display())
    } else {
        format!("mv '{}' '{}'", old_dir.display(), new_dir.display())
    };
    Some(OldConfig { old, mv })
}
