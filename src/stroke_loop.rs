//! Event loop: trigger button + relative motion to recognized strokes.

use crate::config::{Config, Trigger, DEFAULT_MIN_STROKE_PX};
use crate::device::InputEvent;
use crate::error::Error;
use crate::recognizer::recognize;
use crate::recognizer::segment::Point;

/// Parameters of the loop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoopParams {
    /// Button held while drawing.
    pub trigger: Trigger,
    /// Steps shorter than this are jitter (pixels).
    pub min_step_len: f64,
    /// Direction runs shorter than this are noise (pixels).
    pub min_run_len: f64,
}

impl Default for LoopParams {
    /// Right button with the default `min_stroke_px`.
    fn default() -> Self {
        Self::new(Trigger::Right, DEFAULT_MIN_STROKE_PX)
    }
}

impl LoopParams {
    /// Runs must reach `min_stroke_px`, steps a quarter of it.
    pub fn new(trigger: Trigger, min_stroke_px: u32) -> Self {
        let min_run_len = f64::from(min_stroke_px);
        LoopParams {
            trigger,
            min_step_len: min_run_len / 4.0,
            min_run_len,
        }
    }

    /// Derive parameters from the config.
    pub fn from_config(cfg: &Config) -> Self {
        Self::new(cfg.trigger, cfg.min_stroke_px)
    }
}

/// Consume events and call `on_release` with the recognized stroke (`None` = no stroke) each time
/// the trigger button is released. Motion outside a press and other buttons are ignored; a stroke
/// still open when the stream ends is discarded. The first source error stops the loop.
pub fn run_events(
    events: impl IntoIterator<Item = Result<InputEvent, Error>>,
    params: &LoopParams,
    mut on_release: impl FnMut(Option<String>),
) -> Result<(), Error> {
    let mut path: Option<Vec<Point>> = None;
    for event in events {
        match event? {
            InputEvent::Button { button, pressed } if button == params.trigger => {
                if pressed {
                    path.get_or_insert_with(|| vec![Point { x: 0, y: 0 }]);
                } else if let Some(points) = path.take() {
                    on_release(recognize(&points, params.min_step_len, params.min_run_len));
                }
            }
            InputEvent::Motion { dx, dy } => {
                if let Some(points) = path.as_mut() {
                    let last = points[points.len() - 1];
                    points.push(Point {
                        x: last.x.saturating_add(dx),
                        y: last.y.saturating_add(dy),
                    });
                }
            }
            InputEvent::Button { .. } => {}
        }
    }
    Ok(())
}
