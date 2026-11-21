//! X11 overlay backend: a full-screen override-redirect window, click-through (empty XFixes input
//! region) and Shape-masked to the trail pixels, so it works without a compositor. The window's
//! background color is the trail color; only the bounding shape changes while drawing.

use x11rb::connection::{Connection, RequestConnection};
use x11rb::protocol::shape::{self, ConnectionExt as _, SK, SO};
use x11rb::protocol::xfixes::{self, ConnectionExt as _};
use x11rb::protocol::xproto::{
    ClipOrdering, ConfigureWindowAux, ConnectionExt as _, CreateWindowAux, Rectangle, StackMode,
    Window, WindowClass,
};
use x11rb::rust_connection::RustConnection;
use x11rb::COPY_DEPTH_FROM_PARENT;

use super::trail::{render, Trail, TrailStyle};
use super::Overlay;
use crate::error::Error;

/// A screen rectangle in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    /// Left edge.
    pub x: i16,
    /// Top edge.
    pub y: i16,
    /// Width.
    pub width: u16,
    /// Height.
    pub height: u16,
}

impl From<Rect> for Rectangle {
    fn from(r: Rect) -> Self {
        Rectangle {
            x: r.x,
            y: r.y,
            width: r.width,
            height: r.height,
        }
    }
}

fn protocol_rects(rects: &[Rect]) -> Vec<Rectangle> {
    rects.iter().copied().map(Rectangle::from).collect()
}

/// Alpha at or above which a rendered pixel is part of the shape (anti-aliased edges are cut).
const MASK_ALPHA: u8 = 128;

/// The screen area the trail can touch: its points padded by half the line width plus one pixel,
/// clipped to `screen` (width, height). `None` when the trail is empty or entirely off screen.
pub fn trail_bbox(trail: &Trail, width_px: f32, screen: (u16, u16)) -> Option<Rect> {
    let first = trail.points().first()?;
    let (mut x0, mut y0, mut x1, mut y1) = (first.0, first.1, first.0, first.1);
    for &(x, y) in trail.points() {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    let pad = (width_px / 2.0).ceil() + 1.0;
    // f32 -> i64 casts saturate; NaN becomes 0.
    let left = ((x0 - pad).floor() as i64).max(0);
    let top = ((y0 - pad).floor() as i64).max(0);
    let right = ((x1 + pad).ceil() as i64 + 1).min(i64::from(screen.0));
    let bottom = ((y1 + pad).ceil() as i64 + 1).min(i64::from(screen.1));
    if right <= left || bottom <= top {
        return None;
    }
    Some(Rect {
        x: i16::try_from(left).ok()?,
        y: i16::try_from(top).ok()?,
        width: u16::try_from(right - left).ok()?,
        height: u16::try_from(bottom - top).ok()?,
    })
}

/// The shape of a rendered RGBA buffer (`width * height * 4` bytes): per row, the runs of
/// pixels at or above `MASK_ALPHA`; a run identical to one in the row above extends that
/// rectangle. All rectangles are moved by `origin`. A buffer of the wrong size gives none.
pub fn mask_rects(buf: &[u8], width: u16, height: u16, origin: (i16, i16)) -> Vec<Rect> {
    let (w, h) = (usize::from(width), usize::from(height));
    if buf.len() != w * h * 4 {
        return Vec::new();
    }
    let mut rects: Vec<Rect> = Vec::new();
    // Indices into `rects` that end at the previous row.
    let mut open: Vec<usize> = Vec::new();
    for (row, pixels) in buf.chunks_exact(w * 4).enumerate() {
        let y = origin
            .1
            .saturating_add(i16::try_from(row).unwrap_or(i16::MAX));
        let mut next_open = Vec::new();
        let mut x = 0;
        while x < w {
            if pixels[x * 4 + 3] < MASK_ALPHA {
                x += 1;
                continue;
            }
            let start = x;
            while x < w && pixels[x * 4 + 3] >= MASK_ALPHA {
                x += 1;
            }
            let rx = origin
                .0
                .saturating_add(i16::try_from(start).unwrap_or(i16::MAX));
            let rw = u16::try_from(x - start).unwrap_or(u16::MAX);
            match open
                .iter()
                .copied()
                .find(|&i| rects[i].x == rx && rects[i].width == rw)
            {
                Some(i) => {
                    rects[i].height += 1;
                    next_open.push(i);
                }
                None => {
                    next_open.push(rects.len());
                    rects.push(Rect {
                        x: rx,
                        y,
                        width: rw,
                        height: 1,
                    });
                }
            }
        }
        open = next_open;
    }
    rects
}

/// The trail color as a 24-bit TrueColor pixel value (alpha ignored: without a compositor the
/// shape, not alpha, decides what is visible).
pub fn rgb_pixel(rgba: [u8; 4]) -> u32 {
    (u32::from(rgba[0]) << 16) | (u32::from(rgba[1]) << 8) | u32::from(rgba[2])
}

/// Attributes of the overlay window: override-redirect (the window manager leaves it alone),
/// background = trail color, no events selected.
pub fn window_attributes(style: &TrailStyle) -> CreateWindowAux {
    CreateWindowAux::new()
        .override_redirect(1)
        .background_pixel(rgb_pixel(style.rgba))
}

/// The input region of the overlay: empty, so every click reaches the window below.
pub fn input_region() -> Vec<Rect> {
    Vec::new()
}

fn x11_err(e: impl std::fmt::Display) -> Error {
    Error::Overlay(format!("x11: {e}"))
}

/// The X11 trail overlay. Hidden (unmapped) until [`Overlay::begin`].
#[derive(Debug)]
pub struct X11Overlay {
    conn: RustConnection,
    root: Window,
    window: Window,
    screen: (u16, u16),
    style: TrailStyle,
    trail: Trail,
    buf: Vec<u8>,
}

impl X11Overlay {
    /// Connects to `$DISPLAY` and creates the (unmapped) overlay window. Fails when the server
    /// lacks the SHAPE or XFIXES extension.
    pub fn new(style: TrailStyle) -> Result<Self, Error> {
        let (conn, n) = x11rb::connect(None).map_err(x11_err)?;
        for name in [shape::X11_EXTENSION_NAME, xfixes::X11_EXTENSION_NAME] {
            if conn.extension_information(name).map_err(x11_err)?.is_none() {
                return Err(Error::Overlay(format!(
                    "x11: server lacks the {name} extension"
                )));
            }
        }
        conn.xfixes_query_version(5, 0)
            .map_err(x11_err)?
            .reply()
            .map_err(x11_err)?;
        let screen = conn
            .setup()
            .roots
            .get(n)
            .ok_or_else(|| Error::Overlay(format!("x11: no screen {n}")))?;
        let (root, size) = (
            screen.root,
            (screen.width_in_pixels, screen.height_in_pixels),
        );
        let window = conn.generate_id().map_err(x11_err)?;
        conn.create_window(
            COPY_DEPTH_FROM_PARENT,
            window,
            root,
            0,
            0,
            size.0,
            size.1,
            0,
            WindowClass::INPUT_OUTPUT,
            x11rb::COPY_FROM_PARENT,
            &window_attributes(&style),
        )
        .map_err(x11_err)?;
        let region = conn.generate_id().map_err(x11_err)?;
        conn.xfixes_create_region(region, &protocol_rects(&input_region()))
            .map_err(x11_err)?;
        conn.xfixes_set_window_shape_region(window, SK::INPUT, 0, 0, region)
            .map_err(x11_err)?;
        conn.xfixes_destroy_region(region).map_err(x11_err)?;
        let overlay = Self {
            conn,
            root,
            window,
            screen: size,
            style,
            trail: Trail::new((0.0, 0.0)),
            buf: Vec::new(),
        };
        overlay.set_shape(&[])?;
        overlay.conn.flush().map_err(x11_err)?;
        Ok(overlay)
    }

    fn set_shape(&self, rects: &[Rect]) -> Result<(), Error> {
        self.conn
            .shape_rectangles(
                SO::SET,
                SK::BOUNDING,
                ClipOrdering::UNSORTED,
                self.window,
                0,
                0,
                &protocol_rects(rects),
            )
            .map_err(x11_err)?;
        Ok(())
    }

    fn redraw(&mut self) -> Result<(), Error> {
        let rects = match trail_bbox(&self.trail, self.style.width_px, self.screen) {
            None => Vec::new(),
            Some(b) => {
                self.buf.clear();
                self.buf
                    .resize(usize::from(b.width) * usize::from(b.height) * 4, 0);
                let local = self.trail.translated(-f32::from(b.x), -f32::from(b.y));
                render(
                    &local,
                    &self.style,
                    &mut self.buf,
                    u32::from(b.width),
                    u32::from(b.height),
                )?;
                mask_rects(&self.buf, b.width, b.height, (b.x, b.y))
            }
        };
        self.set_shape(&rects)?;
        self.conn.flush().map_err(x11_err)
    }
}

impl Overlay for X11Overlay {
    fn begin(&mut self) -> Result<(), Error> {
        let p = self
            .conn
            .query_pointer(self.root)
            .map_err(x11_err)?
            .reply()
            .map_err(x11_err)?;
        self.trail = Trail::new((f32::from(p.root_x), f32::from(p.root_y)));
        self.set_shape(&[])?;
        self.conn.map_window(self.window).map_err(x11_err)?;
        self.conn
            .configure_window(
                self.window,
                &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
            )
            .map_err(x11_err)?;
        self.conn.flush().map_err(x11_err)
    }

    fn motion(&mut self, dx: i32, dy: i32) -> Result<(), Error> {
        self.trail.push_delta(dx, dy);
        self.redraw()
    }

    fn end(&mut self) -> Result<(), Error> {
        self.trail.clear();
        self.conn.unmap_window(self.window).map_err(x11_err)?;
        self.conn.flush().map_err(x11_err)
    }
}

impl Drop for X11Overlay {
    fn drop(&mut self) {
        // Best effort: closing the connection destroys the window anyway.
        let _ = self.conn.destroy_window(self.window);
        let _ = self.conn.flush();
    }
}
