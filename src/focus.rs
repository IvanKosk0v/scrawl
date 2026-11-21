//! Ties the focused app to each stroke: the window adapter is asked once per trigger press, so the
//! app under the cursor when the stroke starts wins even if focus moves while drawing.
//!
//! Pure logic over the engine and a [`WindowAdapter`]; the adapter does any I/O.

use evdev::InputEvent;

use crate::overlay::feed::TrailFeed;
use crate::overlay::thread::OverlayHandle;
use crate::passthrough::{Output, Passthrough};
use crate::window::{AppId, WindowAdapter};

/// The passthrough engine plus the app that had focus at the latest trigger press.
pub struct Focused {
    engine: Passthrough,
    adapter: Box<dyn WindowAdapter>,
    app: Option<AppId>,
    overlay: Option<(OverlayHandle, TrailFeed)>,
}

impl std::fmt::Debug for Focused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Focused")
            .field("engine", &self.engine)
            .field("app", &self.app)
            .field("overlay", &self.overlay.is_some())
            .finish_non_exhaustive()
    }
}

impl Focused {
    /// Wrap `engine`; `adapter` is queried at every trigger press.
    pub fn new(engine: Passthrough, adapter: Box<dyn WindowAdapter>) -> Self {
        Focused {
            engine,
            adapter,
            app: None,
            overlay: None,
        }
    }

    /// Also feed the trail overlay: `Begin` at each trigger press, the motion while it is held,
    /// `End` at release. Sending never waits for rendering.
    pub fn with_overlay(mut self, handle: OverlayHandle) -> Self {
        self.overlay = Some((handle, TrailFeed::default()));
        self
    }

    /// Feed one raw event to the engine; samples the focused app when a trigger press starts.
    pub fn push(&mut self, raw: InputEvent) -> Vec<Output> {
        let was_pending = self.engine.is_pending();
        let out = self.engine.push(raw);
        let pending = self.engine.is_pending();
        if !was_pending && pending {
            self.app = self.adapter.active_window();
        }
        if let Some((handle, feed)) = self.overlay.as_mut() {
            if let Some(ev) = feed.push(&raw, was_pending, pending) {
                handle.send(ev);
            }
        }
        out
    }

    /// The app focused at the latest trigger press (`None`: unknown, use global bindings).
    pub fn press_app(&self) -> Option<&AppId> {
        self.app.as_ref()
    }
}
