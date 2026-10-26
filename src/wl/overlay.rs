//! Layer-shell overlay surface and shared-memory buffers.

use std::os::fd::{AsFd, OwnedFd};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result};
use rustix::fs::{memfd_create, ftruncate, MemfdFlags};
use rustix::io::pwrite;
use wayland_client::{
    protocol::{wl_buffer, wl_compositor, wl_output, wl_region, wl_shm, wl_surface},
    Dispatch, QueueHandle,
};
use wayland_protocols::wp::fractional_scale::v1::client::{
    wp_fractional_scale_manager_v1, wp_fractional_scale_v1,
};
use wayland_protocols::wp::viewporter::client::{wp_viewport, wp_viewporter};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

use super::State;

/// Set by `wl_buffer::release`, which is the compositor telling us it is done
/// reading that buffer.
pub struct BufferData(pub Arc<AtomicBool>);

impl Dispatch<wl_buffer::WlBuffer, BufferData> for State {
    fn event(
        _state: &mut Self,
        _proxy: &wl_buffer::WlBuffer,
        event: wl_buffer::Event,
        data: &BufferData,
        _conn: &wayland_client::Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wl_buffer::Event::Release = event {
            data.0.store(true, Ordering::Relaxed);
        }
    }
}

fn make_buffer(shm: &wl_shm::WlShm, qh: &QueueHandle<State>, w: u32, h: u32) -> Result<ShmBuffer> {
    let stride = w as i32 * 4;
    let len = stride as usize * h as usize;
    let fd = memfd_create("windhover", MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING)
        .context("memfd_create")?;
    ftruncate(&fd, len as u64).context("resize shared memory")?;
    let released = Arc::new(AtomicBool::new(true));
    let pool = shm.create_pool(fd.as_fd(), len as i32, qh, ());
    let buffer = pool.create_buffer(
        0,
        w as i32,
        h as i32,
        stride,
        wl_shm::Format::Argb8888,
        qh,
        BufferData(released.clone()),
    );
    pool.destroy();
    Ok(ShmBuffer {
        buffer,
        fd,
        pixels: vec![0; len],
        released,
    })
}

pub struct ShmBuffer {
    buffer: wl_buffer::WlBuffer,
    fd: OwnedFd,
    pixels: Vec<u8>,
    released: Arc<AtomicBool>,
}

/// A small ring of shared-memory buffers. Kept across overlay rebuilds, because
/// `wl_buffer` is not bound to a surface and destroying one the compositor is
/// still reading is a protocol error.
pub struct BufferRing {
    buffers: Vec<ShmBuffer>,
    next: usize,
    width: u32,
    height: u32,
}

impl BufferRing {
    pub fn new(shm: &wl_shm::WlShm, qh: &QueueHandle<State>, w: u32, h: u32) -> Result<Self> {
        let buffers = vec![make_buffer(shm, qh, w, h)?, make_buffer(shm, qh, w, h)?];
        Ok(Self {
            buffers,
            next: 0,
            width: w,
            height: h,
        })
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Any buffer the compositor has released, preferring the next in rotation.
    pub fn free(&self) -> Option<usize> {
        let released = |b: &ShmBuffer| b.released.load(Ordering::Relaxed);
        if self.buffers.get(self.next).is_some_and(released) {
            return Some(self.next);
        }
        self.buffers.iter().position(released)
    }

    pub fn pixels_mut(&mut self, index: usize) -> &mut [u8] {
        &mut self.buffers[index].pixels
    }

    pub fn present(&mut self, index: usize, surface: &wl_surface::WlSurface) -> Result<()> {
        let buffer = &mut self.buffers[index];
        pwrite(&buffer.fd, &buffer.pixels, 0).context("upload pixels to shared memory")?;
        // Written to the backing fd, so the buffer is on its way to the
        // compositor; it stays busy until `wl_buffer::release`.
        buffer.released.store(false, Ordering::Relaxed);
        surface.attach(Some(&buffer.buffer), 0, 0);
        surface.damage_buffer(0, 0, self.width as i32, self.height as i32);
        surface.commit();
        self.next = (index + 1) % self.buffers.len();
        Ok(())
    }

    pub fn destroy(mut self) {
        for buffer in self.buffers.drain(..) {
            buffer.buffer.destroy();
        }
    }
}

/// How a logical surface maps onto a pixel buffer.
pub struct Geometry {
    pub buffer: (u32, u32),
    pub buffer_scale: i32,
    pub destination: Option<(u32, u32)>,
}

fn compute_geometry(
    logical: (u32, u32),
    preferred_scale: Option<u32>,
    fallback_scale: i32,
) -> Geometry {
    let (lw, lh) = (logical.0.max(1), logical.1.max(1));
    if let Some(scale) = preferred_scale {
        let factor = scale as f64 / 120.0;
        return Geometry {
            buffer: (
                (lw as f64 * factor).ceil() as u32,
                (lh as f64 * factor).ceil() as u32,
            ),
            buffer_scale: 1,
            destination: Some((lw, lh)),
        };
    }
    let scale = fallback_scale.max(1);
    Geometry {
        buffer: (lw * scale as u32, lh * scale as u32),
        buffer_scale: scale,
        destination: None,
    }
}

/// The layer surface itself. Recreated when the session needs the overlay out
/// of the way (to click through it) and then back.
pub struct Overlay {
    surface: wl_surface::WlSurface,
    layer_surface: zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
    viewport: Option<wp_viewport::WpViewport>,
    fractional: Option<wp_fractional_scale_v1::WpFractionalScaleV1>,
    /// (width, height) from `zwlr_layer_surface_v1::configure`, in logical units.
    configure: Option<(u32, u32)>,
    /// An empty region, kept so a click or a scroll can be sent past the
    /// overlay without rebuilding the surface around it.
    empty_region: wl_region::WlRegion,
    /// 120ths of a logical pixel, from `wp_fractional_scale_v1`.
    preferred_scale: Option<u32>,
    scale_warned: bool,
}

impl Overlay {
    pub fn create(
        compositor: &wl_compositor::WlCompositor,
        layer_shell: &zwlr_layer_shell_v1::ZwlrLayerShellV1,
        viewporter: Option<&wp_viewporter::WpViewporter>,
        fractional: Option<&wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1>,
        output: Option<&wl_output::WlOutput>,
        qh: &QueueHandle<State>,
    ) -> Self {
        let surface = compositor.create_surface(qh, ());
        let empty_region = compositor.create_region(qh, ());
        let viewport = viewporter.map(|v| v.get_viewport(&surface, qh, ()));
        let fractional = fractional.map(|m| m.get_fractional_scale(&surface, qh, ()));
        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            output,
            zwlr_layer_shell_v1::Layer::Overlay,
            "windhover".to_string(),
            qh,
            (),
        );
        layer_surface.set_size(0, 0);
        layer_surface.set_anchor(
            zwlr_layer_surface_v1::Anchor::Top
                | zwlr_layer_surface_v1::Anchor::Bottom
                | zwlr_layer_surface_v1::Anchor::Left
                | zwlr_layer_surface_v1::Anchor::Right,
        );
        // Reserve nothing: other windows must keep their usable area.
        layer_surface.set_exclusive_zone(-1);
        layer_surface.set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::Exclusive);
        surface.commit();

        Self {
            surface,
            layer_surface,
            viewport,
            fractional,
            configure: None,
            empty_region,
            preferred_scale: None,
            scale_warned: false,
        }
    }

    /// Give the pointer to whatever is under the overlay, for good.
    ///
    /// A layer surface that takes pointer events hides every motion from the
    /// application below it, and hover-dependent UI — a video's controls, a menu,
    /// a tooltip — only reacts to motion it can see: without this the cursor can
    /// click things but never hover them. The overlay stays mapped and painted
    /// either way; what it contributes is the ring and the labels, not the input,
    /// and a click reaches the window underneath because nothing takes it first.
    pub fn hand_over_pointer(&self) {
        self.surface.set_input_region(Some(&self.empty_region));
        self.surface.commit();
    }

    /// Give the keyboard up, so that a click on a panel can take it: while an
    /// exclusive layer surface is mapped a compositor keeps the keyboard with it.
    /// There is no request to ask for it back, and a session that clicks is over.
    pub fn release_keyboard(&self) {
        self.layer_surface
            .set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::None);
        self.surface.commit();
    }

    pub fn is_configured(&self) -> bool {
        self.configure.is_some()
    }

    /// Logical size of the surface as the compositor laid it out.
    pub fn logical_size(&self) -> (u32, u32) {
        self.configure.unwrap_or((0, 0))
    }

    pub fn accept_configure(&mut self, serial: u32, width: u32, height: u32) {
        self.configure = Some((width, height));
        self.layer_surface.ack_configure(serial);
    }

    /// How to map the logical surface onto a pixel buffer.
    ///
    /// With `wp_fractional_scale_v1` the buffer carries `logical * factor`
    /// pixels while the viewport destination keeps the surface logical, which
    /// is what makes 1.25x and 1.5x outputs sharp. Without it the integer
    /// `wl_output` scale is all that is left, and the picture is merely scaled.
    pub fn geometry(&mut self, fallback_scale: i32) -> Geometry {
        if self.preferred_scale.is_none() && !self.scale_warned && fallback_scale > 1 {
            self.scale_warned = true;
            eprintln!(
                "windhover: compositor offers no fractional scale; rendering at integer scale {fallback_scale}"
            );
        }
        compute_geometry(
            self.logical_size(),
            self.preferred_scale,
            fallback_scale,
        )
    }

    pub fn apply(&self, geometry: &Geometry) {
        self.surface.set_buffer_scale(geometry.buffer_scale);
        if let (Some(viewport), Some((w, h))) = (&self.viewport, geometry.destination) {
            viewport.set_destination(w as i32, h as i32);
        }
    }

    pub fn surface(&self) -> &wl_surface::WlSurface {
        &self.surface
    }

    pub fn destroy(self) {
        self.layer_surface.destroy();
        if let Some(viewport) = self.viewport {
            viewport.destroy();
        }
        if let Some(fractional) = self.fractional {
            fractional.destroy();
        }
        self.surface.destroy();
    }
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for State {
    fn event(
        state: &mut Self,
        _proxy: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                if let Some(overlay) = &mut state.overlay {
                    overlay.accept_configure(serial, width, height);
                }
            }
            zwlr_layer_surface_v1::Event::Closed => state.closed = true,
            _ => {}
        }
    }
}

impl Dispatch<wp_fractional_scale_v1::WpFractionalScaleV1, ()> for State {
    fn event(
        state: &mut Self,
        _proxy: &wp_fractional_scale_v1::WpFractionalScaleV1,
        event: wp_fractional_scale_v1::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event {
            if let Some(overlay) = &mut state.overlay {
                overlay.preferred_scale = Some(scale);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::compute_geometry;

    #[test]
    fn fractional_scale_sizes_the_buffer_and_keeps_the_surface_logical() {
        // 1.5x: 1280 logical pixels become 1920 buffer pixels, and the viewport
        // destination keeps the surface at 1280 so the picture stays sharp.
        let geometry = compute_geometry((1280, 720), Some(180), 1);
        assert_eq!(geometry.buffer, (1920, 1080));
        assert_eq!(geometry.buffer_scale, 1);
        assert_eq!(geometry.destination, Some((1280, 720)));
    }

    #[test]
    fn fractional_scale_rounds_the_buffer_up() {
        // 1.25x of 1001 is 1251.25; a buffer can only be a whole number of
        // pixels, and rounding down would leave a gap at the edge.
        let geometry = compute_geometry((1001, 601), Some(150), 1);
        assert_eq!(geometry.buffer, (1252, 752));
        assert_eq!(geometry.destination, Some((1001, 601)));
    }

    #[test]
    fn integer_scale_is_used_when_fractional_scale_is_absent() {
        let geometry = compute_geometry((1280, 720), None, 2);
        assert_eq!(geometry.buffer, (2560, 1440));
        assert_eq!(geometry.buffer_scale, 2);
        assert_eq!(geometry.destination, None);
    }

    #[test]
    fn missing_scale_information_still_yields_a_usable_buffer() {
        let geometry = compute_geometry((0, 0), None, 1);
        assert_eq!(geometry.buffer, (1, 1));
    }
}
