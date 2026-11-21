//! Wayland overlay backend: a wlr-layer-shell surface on the `overlay` layer, anchored to every
//! edge, click-through (empty input region) and drawn with a full-surface transparent shm buffer.
//! Wayland clients never learn the cursor position, so the trail starts at the surface center.
//! With one output the surface is mapped once (first [`Overlay::begin`]) and hidden with a
//! transparent buffer, so compositors with layer animations do not play them per stroke. With
//! several outputs, or after the compositor closed it, each `begin` creates a new surface, which
//! the compositor puts on the focused output ([`recreate_surface`]). Buffers follow the output's
//! integer scale ([`buffer_size`]). Compositors without
//! wlr-layer-shell (GNOME) get [`no_layer_shell_error`]; `feedback = "notify"` covers them.

use smithay_client_toolkit::compositor::{CompositorHandler, CompositorState, Region};
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::reexports::client::backend::WaylandError;
use smithay_client_toolkit::reexports::client::globals::registry_queue_init;
use smithay_client_toolkit::reexports::client::protocol::{wl_output, wl_shm, wl_surface};
use smithay_client_toolkit::reexports::client::{Connection, EventQueue, Proxy, QueueHandle};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::shell::wlr_layer::{
    Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
    LayerSurfaceConfigure,
};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shm::slot::{Buffer, SlotPool};
use smithay_client_toolkit::shm::{Shm, ShmHandler};
use smithay_client_toolkit::{
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_shm,
    registry_handlers,
};

use super::trail::{render, Trail, TrailStyle};
use super::Overlay;
use crate::error::Error;

/// Layer-shell namespace of the overlay surface, for compositor rules (e.g. Hyprland `layerrule`).
pub const LAYER_NAMESPACE: &str = "strokebind-overlay";

/// How the layer surface is set up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerSettings {
    /// Stacking layer.
    pub layer: Layer,
    /// Edges the surface is anchored to.
    pub anchor: Anchor,
    /// `-1`: ignore other surfaces' exclusive zones and reserve none.
    pub exclusive_zone: i32,
    /// Keyboard focus policy.
    pub keyboard: KeyboardInteractivity,
    /// Namespace shown to the compositor.
    pub namespace: &'static str,
}

/// The overlay's layer surface: top-most layer, whole output, never takes keyboard focus.
pub fn layer_settings() -> LayerSettings {
    LayerSettings {
        layer: Layer::Overlay,
        anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
        exclusive_zone: -1,
        keyboard: KeyboardInteractivity::None,
        namespace: LAYER_NAMESPACE,
    }
}

/// Rectangles `(x, y, width, height)` of the input region: none, so every click reaches the
/// surface below.
pub fn input_region_rects() -> Vec<(i32, i32, i32, i32)> {
    Vec::new()
}

/// Where the trail starts on a surface of `size` (width, height): its center.
pub fn center(size: (u32, u32)) -> (f32, f32) {
    (size.0 as f32 / 2.0, size.1 as f32 / 2.0)
}

/// Converts tiny-skia premultiplied RGBA pixels in place to `wl_shm` ARGB8888 (a little-endian
/// `0xAARRGGBB`, i.e. bytes B, G, R, A). A trailing partial pixel is left unchanged.
pub fn rgba_to_argb8888(buf: &mut [u8]) {
    for px in buf.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
}

/// Buffer size in pixels for a surface of `logical` size at integer `scale` (below 1 = 1).
/// `None` when the size, the shm stride (`width * 4`) or the byte size would not fit an `i32`.
pub fn buffer_size(logical: (u32, u32), scale: i32) -> Option<(u32, u32)> {
    let s = u32::try_from(scale.max(1)).ok()?;
    let (w, h) = (logical.0.checked_mul(s)?, logical.1.checked_mul(s)?);
    let max = u64::from(i32::MAX.unsigned_abs());
    let fits = u64::from(w) * 4 <= max && u64::from(w) * u64::from(h) * 4 <= max;
    fits.then_some((w, h))
}

/// Whether [`Overlay::begin`] must replace the layer surface: with several `outputs` the
/// compositor then places the new one on the focused output; a `closed` one (e.g. its output
/// was unplugged) cannot be drawn on.
pub fn recreate_surface(outputs: usize, closed: bool) -> bool {
    closed || outputs > 1
}

/// The outcome of flushing requests on the non-blocking socket: "would block" (the compositor is
/// slow to read) is not an error, the rest is sent with the next flush; anything else is.
pub fn flush_result(r: Result<(), WaylandError>) -> Result<(), Error> {
    match r {
        Err(WaylandError::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(()),
        other => other.map_err(wl_err),
    }
}

/// The error for compositors without wlr-layer-shell.
pub fn no_layer_shell_error() -> Error {
    Error::Overlay(
        "wayland: the compositor has no wlr-layer-shell (e.g. GNOME), so no trail can be drawn; \
         use `feedback = \"notify\"` to see recognized strokes"
            .to_string(),
    )
}

fn wl_err(e: impl std::fmt::Display) -> Error {
    Error::Overlay(format!("wayland: {e}"))
}

/// Protocol state driven by the event queue.
struct State {
    registry: RegistryState,
    outputs: OutputState,
    shm: Shm,
    /// Size from the latest layer-surface configure.
    size: Option<(u32, u32)>,
    closed: bool,
    /// Integer buffer scale of the current surface (1 until the compositor says otherwise).
    scale: i32,
}

/// The Wayland trail overlay.
pub struct WaylandOverlay {
    conn: Connection,
    queue: EventQueue<State>,
    state: State,
    compositor: CompositorState,
    layer_shell: LayerShell,
    /// `None` only between dropping a surface and configuring its replacement.
    layer: Option<LayerSurface>,
    pool: SlotPool,
    /// Last attached buffer, kept alive until the compositor releases it.
    buffer: Option<Buffer>,
    style: TrailStyle,
    trail: Trail,
}

impl std::fmt::Debug for WaylandOverlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WaylandOverlay")
            .field("size", &self.state.size)
            .field("style", &self.style)
            .finish_non_exhaustive()
    }
}

impl WaylandOverlay {
    /// Connects to `$WAYLAND_DISPLAY`, creates the (not yet mapped) layer surface and waits for
    /// its first configure. Fails without wlr-layer-shell ([`no_layer_shell_error`]).
    pub fn new(style: TrailStyle) -> Result<Self, Error> {
        let conn = Connection::connect_to_env().map_err(wl_err)?;
        let (globals, queue) = registry_queue_init::<State>(&conn).map_err(wl_err)?;
        let qh = queue.handle();
        let compositor = CompositorState::bind(&globals, &qh).map_err(wl_err)?;
        let layer_shell = LayerShell::bind(&globals, &qh).map_err(|_| no_layer_shell_error())?;
        let shm = Shm::bind(&globals, &qh).map_err(wl_err)?;
        let state = State {
            registry: RegistryState::new(&globals),
            outputs: OutputState::new(&globals, &qh),
            shm,
            size: None,
            closed: false,
            scale: 1,
        };
        let pool = SlotPool::new(4, &state.shm).map_err(wl_err)?;
        let mut overlay = Self {
            conn,
            queue,
            state,
            compositor,
            layer_shell,
            layer: None,
            pool,
            buffer: None,
            style,
            trail: Trail::new((0.0, 0.0)),
        };
        overlay.create_layer()?;
        Ok(overlay)
    }

    /// Drops the current layer surface (it shows only a transparent buffer between strokes) and
    /// creates a new one without an output, so the compositor picks the focused one; waits for
    /// its first configure.
    fn create_layer(&mut self) -> Result<(), Error> {
        self.layer = None;
        self.buffer = None;
        self.state.size = None;
        self.state.closed = false;
        self.state.scale = 1;
        let qh = self.queue.handle();
        let s = layer_settings();
        let surface = self.compositor.create_surface(&qh);
        let layer =
            self.layer_shell
                .create_layer_surface(&qh, surface, s.layer, Some(s.namespace), None);
        layer.set_anchor(s.anchor);
        layer.set_exclusive_zone(s.exclusive_zone);
        layer.set_keyboard_interactivity(s.keyboard);
        layer.set_size(0, 0);
        let region = Region::new(&self.compositor).map_err(wl_err)?;
        for (x, y, w, h) in input_region_rects() {
            region.add(x, y, w, h);
        }
        layer.set_input_region(Some(region.wl_region()));
        layer.commit();
        self.layer = Some(layer);
        while self.state.size.is_none() {
            if self.state.closed {
                return Err(wl_err("layer surface closed before its first configure"));
            }
            self.queue
                .blocking_dispatch(&mut self.state)
                .map_err(wl_err)?;
        }
        Ok(())
    }

    /// Surface size from the latest configure.
    pub fn size(&self) -> (u32, u32) {
        self.state.size.unwrap_or((1, 1))
    }

    /// Renders the current trail (fewer than two points = fully transparent) into a new buffer at
    /// the surface's scale and commits it. A closed surface is skipped until the next `begin`.
    fn present(&mut self) -> Result<(), Error> {
        self.pump()?;
        let Some(layer) = self.layer.as_ref().filter(|_| !self.state.closed) else {
            return Ok(());
        };
        let scale = self.state.scale.max(1);
        let (w, h) = buffer_size(self.size(), scale).ok_or_else(|| wl_err("surface too large"))?;
        let (wi, hi) = (
            i32::try_from(w).map_err(wl_err)?,
            i32::try_from(h).map_err(wl_err)?,
        );
        let (buffer, canvas) = self
            .pool
            .create_buffer(wi, hi, wi * 4, wl_shm::Format::Argb8888)
            .map_err(wl_err)?;
        let f = scale as f32;
        render(&self.trail.scaled(f), &self.style.scaled(f), canvas, w, h)?;
        rgba_to_argb8888(canvas);
        let surface = layer.wl_surface();
        // set_buffer_scale exists from wl_surface version 3; older ones stay at scale 1.
        if surface.version() >= 3 {
            surface.set_buffer_scale(scale);
        }
        buffer.attach_to(surface).map_err(wl_err)?;
        surface.damage_buffer(0, 0, wi, hi);
        layer.commit();
        self.buffer = Some(buffer);
        self.pump()
    }

    /// Sends queued requests and handles whatever events already arrived, without blocking.
    fn pump(&mut self) -> Result<(), Error> {
        flush_result(self.queue.flush())?;
        if let Some(guard) = self.conn.prepare_read() {
            // Non-blocking socket: "would block" just means nothing arrived yet.
            let _ = guard.read();
        }
        self.queue
            .dispatch_pending(&mut self.state)
            .map_err(wl_err)?;
        Ok(())
    }
}

impl Overlay for WaylandOverlay {
    fn begin(&mut self) -> Result<(), Error> {
        self.pump()?;
        if recreate_surface(self.state.outputs.outputs().count(), self.state.closed) {
            self.create_layer()?;
        }
        self.trail = Trail::new(center(self.size()));
        self.present()
    }

    fn motion(&mut self, dx: i32, dy: i32) -> Result<(), Error> {
        self.trail.push_delta(dx, dy);
        self.present()
    }

    fn end(&mut self) -> Result<(), Error> {
        self.trail.clear();
        self.present()
    }
}

impl CompositorHandler for State {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        new_factor: i32,
    ) {
        self.scale = new_factor.max(1);
    }

    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }

    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {}

    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for State {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl LayerShellHandler for State {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
        self.closed = true;
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _: u32,
    ) {
        let (w, h) = configure.new_size;
        // Anchored to all edges, the compositor sends the output size; never use a 0 dimension.
        self.size = Some((w.max(1), h.max(1)));
    }
}

impl ShmHandler for State {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }

    registry_handlers![OutputState];
}

delegate_compositor!(State);
delegate_output!(State);
delegate_shm!(State);
delegate_layer!(State);
delegate_registry!(State);
