//! Hot reload: watch the config file and re-parse it when it changes.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};

use notify::{recommended_watcher, RecommendedWatcher, RecursiveMode, Watcher};

use crate::config::Config;
use crate::error::Error;

/// Watches a config file. Editors often replace the file by renaming, so the parent directory is watched.
#[derive(Debug)]
pub struct ConfigWatcher {
    path: PathBuf,
    events: Receiver<notify::Result<notify::Event>>,
    // Kept alive: dropping it stops the watch.
    _watcher: RecommendedWatcher,
}

impl ConfigWatcher {
    /// Start watching `path`; fails if its directory cannot be watched.
    pub fn new(path: &Path) -> Result<Self, Error> {
        let io = |e: notify::Error| Error::Io {
            path: path.to_path_buf(),
            source: std::io::Error::other(e),
        };
        let dir = match path.parent() {
            Some(d) if !d.as_os_str().is_empty() => d,
            _ => Path::new("."),
        };
        let (tx, events) = channel();
        let mut watcher = recommended_watcher(move |ev| {
            // The receiver is gone only when the watcher itself is being dropped.
            let _ = tx.send(ev);
        })
        .map_err(io)?;
        watcher
            .watch(dir, RecursiveMode::NonRecursive)
            .map_err(io)?;
        Ok(ConfigWatcher {
            path: path.to_path_buf(),
            events,
            _watcher: watcher,
        })
    }

    /// Non-blocking. If the file changed since the last call, re-read it: `Some(Ok(config))` for a valid
    /// new config, `Some(Err(_))` if it is unreadable or invalid (the caller keeps its current config), and
    /// `None` if nothing changed.
    pub fn poll(&mut self) -> Option<Result<Config, Error>> {
        let mut changed = false;
        while let Ok(ev) = self.events.try_recv() {
            if let Ok(ev) = ev {
                changed |= ev
                    .paths
                    .iter()
                    .any(|p| p.file_name() == self.path.file_name());
            }
        }
        changed.then(|| self.load())
    }

    fn load(&self) -> Result<Config, Error> {
        let src = std::fs::read_to_string(&self.path).map_err(|source| Error::Io {
            path: self.path.clone(),
            source,
        })?;
        Config::parse(&src)
    }
}
