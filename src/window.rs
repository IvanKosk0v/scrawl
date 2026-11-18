//! Active-window adapters for per-app profiles.
//!
//! An adapter only reads which application has focus. Any failure yields `None`,
//! which means "use the global bindings", so strokes always keep working.

use std::cell::RefCell;
use std::fmt;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, TryRecvError};
use std::sync::Arc;
use std::time::Duration;

/// Identifier of the focused application (e.g. Wayland `app_id` or X11 `WM_CLASS`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AppId(String);

impl AppId {
    /// Builds an id from a raw string, trimming whitespace; blank input gives `None`.
    pub fn new(raw: &str) -> Option<AppId> {
        let id = raw.trim();
        (!id.is_empty()).then(|| AppId(id.to_string()))
    }

    /// The id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AppId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Source of the currently focused application.
pub trait WindowAdapter {
    /// The focused app, or `None` when unknown (no focus, IPC unavailable, parse error).
    /// Implementations query fresh state on every call and report failures themselves.
    fn active_window(&self) -> Option<AppId>;
}

/// Adapter for desktops without support: always `None`, so only global bindings apply.
#[derive(Debug, Clone, Copy, Default)]
pub struct GlobalOnly;

impl WindowAdapter for GlobalOnly {
    fn active_window(&self) -> Option<AppId> {
        None
    }
}

/// Longest a window lookup may delay a trigger press; past it the stroke uses global bindings.
pub const ADAPTER_TIMEOUT: Duration = Duration::from_millis(50);

/// Wraps an adapter so each lookup takes at most [`ADAPTER_TIMEOUT`]. The lookup runs on its own
/// thread; one that has not answered in time yields `None`, and while it is still outstanding every
/// later lookup returns `None` at once (no threads pile up behind a hung compositor). Its late
/// answer is dropped, never used for a later press.
pub struct Bounded {
    inner: Arc<dyn WindowAdapter + Send + Sync>,
    /// A lookup that timed out and has not answered yet.
    outstanding: RefCell<Option<Receiver<Option<AppId>>>>,
}

impl Bounded {
    /// Bound `adapter`'s lookups by [`ADAPTER_TIMEOUT`].
    pub fn new(adapter: impl WindowAdapter + Send + Sync + 'static) -> Self {
        Bounded {
            inner: Arc::new(adapter),
            outstanding: RefCell::new(None),
        }
    }
}

impl fmt::Debug for Bounded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Bounded")
            .field("outstanding", &self.outstanding.borrow().is_some())
            .finish_non_exhaustive()
    }
}

impl WindowAdapter for Bounded {
    fn active_window(&self) -> Option<AppId> {
        let mut outstanding = self.outstanding.borrow_mut();
        if let Some(rx) = outstanding.as_ref() {
            match rx.try_recv() {
                Err(TryRecvError::Empty) => return None,
                // Answered late (dropped) or the thread is gone: free to ask again.
                Ok(_) | Err(TryRecvError::Disconnected) => *outstanding = None,
            }
        }
        let (tx, rx) = mpsc::channel();
        let inner = Arc::clone(&self.inner);
        let spawned = std::thread::Builder::new()
            .name("window-lookup".into())
            .spawn(move || {
                let _ = tx.send(inner.active_window());
            });
        if let Err(e) = spawned {
            eprintln!("window lookup failed, using global bindings: {e}");
            return None;
        }
        match rx.recv_timeout(ADAPTER_TIMEOUT) {
            Ok(app) => app,
            Err(RecvTimeoutError::Timeout) => {
                eprintln!(
                    "window lookup took over {} ms, using global bindings",
                    ADAPTER_TIMEOUT.as_millis()
                );
                *outstanding = Some(rx);
                None
            }
            Err(RecvTimeoutError::Disconnected) => None,
        }
    }
}

/// Test support: a lookup that blocks forever, standing in for a hung compositor.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NeverAnswers;

impl WindowAdapter for NeverAnswers {
    fn active_window(&self) -> Option<AppId> {
        loop {
            std::thread::park();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::VecDeque;

    use super::*;

    /// Mock that returns a scripted answer per call (then `None`).
    struct MockAdapter(RefCell<VecDeque<Option<AppId>>>);

    impl MockAdapter {
        fn new(script: Vec<Option<&str>>) -> Self {
            let ids = script
                .into_iter()
                .map(|s| s.map(|s| AppId::new(s).expect("valid id")))
                .collect();
            MockAdapter(RefCell::new(ids))
        }
    }

    impl WindowAdapter for MockAdapter {
        fn active_window(&self) -> Option<AppId> {
            self.0.borrow_mut().pop_front().flatten()
        }
    }

    #[test]
    fn app_id_trims_and_rejects_blank() {
        assert_eq!(AppId::new(""), None);
        assert_eq!(AppId::new("  \t\n"), None);
        let id = AppId::new(" firefox\n").expect("non-blank");
        assert_eq!(id.as_str(), "firefox");
        assert_eq!(id.to_string(), "firefox");
    }

    #[test]
    fn global_only_is_none() {
        assert_eq!(GlobalOnly.active_window(), None);
    }

    #[test]
    fn mock_adapter_reports_focused_app() {
        let mock = MockAdapter::new(vec![Some("org.kde.dolphin")]);
        let adapter: &dyn WindowAdapter = &mock;
        assert_eq!(
            adapter.active_window().as_ref().map(AppId::as_str),
            Some("org.kde.dolphin")
        );
    }

    #[test]
    fn mock_adapter_focus_change_is_seen() {
        let mock = MockAdapter::new(vec![Some("firefox"), None, Some("kitty")]);
        let adapters: [&dyn WindowAdapter; 2] = [&mock, &GlobalOnly];
        let seen: Vec<Option<String>> = (0..3)
            .map(|_| adapters[0].active_window().map(|a| a.to_string()))
            .collect();
        assert_eq!(
            seen,
            vec![Some("firefox".to_string()), None, Some("kitty".to_string())]
        );
        assert_eq!(adapters[1].active_window(), None);
    }
}
