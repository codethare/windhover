//! Wayland connection, registry, and event loop.

pub mod output;
pub mod overlay;
pub mod pointer;
pub mod seat;

use std::ops::RangeInclusive;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use rustix::event::{poll, PollFd, PollFlags, Timespec};
use wayland_client::globals::{registry_queue_init, BindError, GlobalList, GlobalListContents};
use wayland_client::protocol::{
    wl_compositor, wl_keyboard, wl_output, wl_pointer, wl_region, wl_registry, wl_seat, wl_shm,
    wl_shm_pool, wl_surface,
};
use wayland_client::{
    delegate_noop, Connection, Dispatch, EventQueue, Proxy, QueueHandle,
};
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1;
use wayland_protocols::wp::viewporter::client::{wp_viewport, wp_viewporter};
use wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_manager_v1;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1;
use wayland_protocols_wlr::virtual_pointer::v1::client::{
    zwlr_virtual_pointer_manager_v1, zwlr_virtual_pointer_v1,
};

use output::OutputEntry;
use overlay::{BufferRing, Overlay};
use pointer::VirtualPointer;
use seat::{KeyEvent, Keyboard};

/// The seat capability we were promised; keyboard focus and pointer position
/// only start arriving after `wl_seat` reports its capabilities.
pub struct State {
    compositor: wl_compositor::WlCompositor,
    shm: wl_shm::WlShm,
    layer_shell: zwlr_layer_shell_v1::ZwlrLayerShellV1,
    viewporter: Option<wp_viewporter::WpViewporter>,
    fractional_manager: Option<wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1>,
    /// Held for the session: dropping it would drop the per-output proxies.
    _xdg_output_manager: Option<zxdg_output_manager_v1::ZxdgOutputManagerV1>,
    vp_manager: zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1,
    seat: wl_seat::WlSeat,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    pointer: Option<wl_pointer::WlPointer>,

    pub outputs: Vec<OutputEntry>,
    pub overlay: Option<Overlay>,
    pub ring: Option<BufferRing>,
    pub output_index: Option<usize>,
    pub keys: Keyboard,
    /// Where the compositor last reported the pointer. Only filled during the
    /// handshake at startup: the overlay stops taking pointer events once the
    /// starting position is known, so the session tracks its own moves from there.
    pub pointer_pos: Option<(f64, f64)>,
    pub closed: bool,
    pub keyboard_ready: bool,
    pub pointer_ready: bool,
    vp: Option<VirtualPointer>,
}

impl State {
    pub fn output(&self) -> Option<&OutputEntry> {
        self.output_index.and_then(|i| self.outputs.get(i))
    }

    /// Logical size of the overlay, which is also the coordinate space the
    /// virtual pointer is addressed in.
    ///
    /// The layer surface configure is preferred over the output record: when the
    /// compositor places the overlay for us we never learn which output it chose,
    /// and an output-relative lookup would come back empty.
    pub fn logical_size(&self) -> (u32, u32) {
        if let Some(overlay) = &self.overlay {
            let (w, h) = overlay.logical_size();
            if w > 0 && h > 0 {
                return (w, h);
            }
        }
        self.output()
            .map(|entry| entry.info.logical_size())
            .unwrap_or((0, 0))
    }

    /// Buffer pixels per logical pixel, so the renderer can draw in logical
    /// units and still fill the physical buffer exactly.
    pub fn pixel_scale(&self) -> f32 {
        let (logical_w, logical_h) = self.logical_size();
        let Some((buffer_w, buffer_h)) = self.ring.as_ref().map(overlay::BufferRing::size) else {
            return 1.0;
        };
        if logical_w == 0 || logical_h == 0 {
            return 1.0;
        }
        let horizontal = buffer_w as f32 / logical_w as f32;
        let vertical = buffer_h as f32 / logical_h as f32;
        if horizontal > 0.0 && vertical > 0.0 {
            horizontal.min(vertical)
        } else {
            1.0
        }
    }

    /// The last position the compositor reported, consumed once.
    ///
    /// `wl_pointer` reports where the pointer *was*; re-applying that snapshot
    /// on every turn would undo the session's own moves, because we move the
    /// pointer without the compositor having told us anything new yet.
    pub fn take_pointer_pos(&mut self) -> Option<(f64, f64)> {
        self.pointer_pos.take()
    }

    /// Where the pointer is, in logical coordinates. Falls back to the centre of
    /// the output when the compositor never told us (for example on a headless
    /// output, where there is no pointer to report).
    pub fn cursor(&self) -> (f64, f64) {
        let (w, h) = self.logical_size();
        self.pointer_pos
            .unwrap_or((w as f64 / 2.0, h as f64 / 2.0))
    }
}

pub struct Wayland {
    conn: Connection,
    queue: EventQueue<State>,
    qh: QueueHandle<State>,
    pub state: State,
}

impl Wayland {
    pub fn connect() -> Result<Self> {
        let conn = Connection::connect_to_env()
            .context("connect to the Wayland display (is WAYLAND_DISPLAY set?)")?;
        let (globals, mut queue) = registry_queue_init::<State>(&conn)
            .map_err(|e| anyhow!("read the compositor's global list: {e}"))?;
        let qh = queue.handle();

        let compositor = bind::<wl_compositor::WlCompositor, _>(
            &globals,
            &qh,
            4..=6,
            (),
            "compositor overlay support",
        )?;
        let shm = bind::<wl_shm::WlShm, _>(&globals, &qh, 1..=1, (), "shared memory buffers")?;
        let layer_shell = bind::<zwlr_layer_shell_v1::ZwlrLayerShellV1, _>(
            &globals,
            &qh,
            1..=4,
            (),
            "fullscreen overlay layer",
        )?;
        let vp_manager = bind::<zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1, _>(
            &globals,
            &qh,
            1..=2,
            (),
            "virtual pointer",
        )?;
        // Optional: without it the overlay is placed on the compositor's active
        // output and rendered at the integer scale.
        let viewporter = globals
            .bind::<wp_viewporter::WpViewporter, State, _>(&qh, 1..=1, ())
            .ok();
        let fractional_manager = globals
            .bind::<wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1, State, _>(
                &qh,
                1..=1,
                (),
            )
            .ok();
        let xdg_output_manager = globals
            .bind::<zxdg_output_manager_v1::ZxdgOutputManagerV1, State, _>(&qh, 1..=3, ())
            .ok();

        let mut outputs = Vec::new();
        for global in globals.contents().clone_list() {
            if global.interface != "wl_output" {
                continue;
            }
            let index = outputs.len();
            let output: wl_output::WlOutput =
                globals
                    .registry()
                    .bind(global.name, global.version.min(4), &qh, index);
            let xdg = xdg_output_manager.as_ref().map(|manager| {
                manager.get_xdg_output(&output, &qh, index)
            });
            outputs.push(OutputEntry {
                output: Some(output),
                _xdg: xdg,
                ..Default::default()
            });
        }

        let (seat_name, seat_version) = globals
            .contents()
            .with_list(|list| {
                list.iter()
                    .find(|global| global.interface == "wl_seat")
                    .map(|global| (global.name, global.version))
            })
            .ok_or_else(|| anyhow!("compositor does not provide a seat (no wl_seat global)"))?;
        let seat: wl_seat::WlSeat =
            globals
                .registry()
                .bind(seat_name, seat_version.min(7), &qh, ());

        let mut state = State {
            compositor,
            shm,
            layer_shell,
            viewporter,
            fractional_manager,
            _xdg_output_manager: xdg_output_manager,
            vp_manager,
            seat,
            keyboard: None,
            pointer: None,
            outputs,
            overlay: None,
            ring: None,
            output_index: None,
            keys: Keyboard::default(),
            pointer_pos: None,
            closed: false,
            keyboard_ready: false,
            pointer_ready: false,
            vp: None,
        };

        drop(globals);

        // Wait for seat capabilities, otherwise the keyboard object may not
        // exist yet when the overlay takes focus.
        for _ in 0..16 {
            queue.roundtrip(&mut state).context("initial roundtrip")?;
            if state.keyboard_ready {
                break;
            }
        }
        if !state.keyboard_ready {
            return Err(anyhow!(
                "compositor does not provide a keyboard on its seat; \
                 windhover has no way to receive input"
            ));
        }

        Ok(Self {
            conn,
            queue,
            qh,
            state,
        })
    }

    pub fn roundtrip(&mut self) -> Result<()> {
        self.queue
            .roundtrip(&mut self.state)
            .map(|_| ())
            .map_err(|err| anyhow!("wayland roundtrip: {err}"))
    }

    /// Dispatch everything that has arrived, waiting at most `timeout` for more.
    pub fn dispatch(&mut self, timeout: Option<Duration>) -> Result<()> {
        self.conn.flush().context("flush wayland requests")?;
        let mut waited = false;
        if let Some(guard) = self.conn.prepare_read() {
            let fd = guard.connection_fd();
            let mut fds = [PollFd::new(&fd, PollFlags::IN)];
            let timespec = timeout.map(|d| Timespec {
                tv_sec: d.as_secs() as i64,
                tv_nsec: d.subsec_nanos() as i64,
            });
            let ready = match poll(&mut fds, timespec.as_ref()) {
                Ok(ready) => ready,
                // A signal interrupted the wait; the caller re-checks its own
                // state and comes back.
                Err(rustix::io::Errno::INTR) => 0,
                Err(err) => return Err(err).context("poll wayland socket"),
            };
            waited = ready > 0 && fds[0].revents().contains(PollFlags::IN);
            if waited {
                match guard.read() {
                    Ok(_) => {}
                    Err(wayland_client::backend::WaylandError::Io(err))
                        if err.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(err) => return Err(err).context("read wayland events"),
                }
            }
        }
        let _ = waited;
        self.queue
            .dispatch_pending(&mut self.state)
            .context("dispatch wayland events")?;
        Ok(())
    }

    /// Choose the output under the pointer and put the overlay there.
    ///
    /// Wayland does not expose the cursor position until one of our own
    /// surfaces has the pointer, so placement and position discovery are the
    /// same operation: put the overlay somewhere, and if the pointer enters it,
    /// that output is the right one. The compositor's own choice is tried
    /// first, which is the answer in the common single-output case.
    pub fn place_overlay(&mut self) -> Result<()> {
        let candidates: Vec<Option<usize>> = std::iter::once(None)
            .chain((0..self.state.outputs.len()).map(Some))
            .collect();

        for candidate in candidates {
            self.state.pointer_pos = None;
            self.create_overlay(candidate)?;
            self.establish_virtual_pointer()?;
            if self.state.pointer_pos.is_some() {
                // The pointer position is known, which is the last thing the
                // overlay was needed for. From here the applications have it.
                self.hand_over_pointer()?;
                return Ok(());
            }
        }

        // No output ever saw the pointer (headless, or a compositor that does
        // not send enter before the first frame). Keep the compositor's choice.
        self.state.pointer_pos = None;
        self.create_overlay(None)?;
        self.establish_virtual_pointer()?;
        self.hand_over_pointer()?;
        Ok(())
    }

    /// Let applications see the pointer, for the rest of the session.
    fn hand_over_pointer(&mut self) -> Result<()> {
        if let Some(overlay) = self.state.overlay.as_ref() {
            overlay.hand_over_pointer();
        }
        self.roundtrip()
    }

    fn create_overlay(&mut self, output_index: Option<usize>) -> Result<()> {
        if let Some(previous) = self.state.overlay.take() {
            previous.destroy();
            // Let the compositor send the matching `wl_pointer::leave`, so a
            // stale enter cannot be mistaken for the new output's answer.
            self.roundtrip()?;
            self.state.pointer_pos = None;
        }

        // Cloned up front: `wl_output` handles are refcounted, and holding a
        // borrow of `self.state` across the roundtrips below will not compile.
        let output = output_index
            .and_then(|i| self.state.outputs.get(i))
            .and_then(|entry| entry.output.clone());
        let fallback_scale = output_index
            .and_then(|i| self.state.outputs.get(i))
            .map(|entry| entry.info.scale)
            .unwrap_or(1);
        let overlay = Overlay::create(
            &self.state.compositor,
            &self.state.layer_shell,
            self.state.viewporter.as_ref(),
            self.state.fractional_manager.as_ref(),
            output.as_ref(),
            &self.qh,
        );
        self.state.overlay = Some(overlay);

        for _ in 0..16 {
            self.roundtrip()?;
            if self.state.overlay.as_ref().is_some_and(Overlay::is_configured) {
                break;
            }
        }
        self.state.output_index = output_index;
        if self.state.overlay.as_ref().is_none_or(|o| !o.is_configured()) {
            return Err(anyhow!(
                "the compositor never sent a layer surface configure event"
            ));
        }

        // One more roundtrip so `wp_fractional_scale_v1` has landed before the
        // buffer size is computed from it.
        self.roundtrip()?;

        let geometry = self
            .state
            .overlay
            .as_mut()
            .expect("overlay was just created")
            .geometry(fallback_scale);
        let ring = BufferRing::new(
            &self.state.shm,
            &self.qh,
            geometry.buffer.0,
            geometry.buffer.1,
        )?;
        if let Some(overlay) = &self.state.overlay {
            overlay.apply(&geometry);
        }
        if let Some(previous) = self.state.ring.replace(ring) {
            previous.destroy();
        }

        // A transparent first frame maps the surface, and the pointer enter
        // that follows is how we learn which output the pointer is on.
        self.present_with(|_, _, _| {})?;
        for _ in 0..4 {
            self.roundtrip()?;
            if self.state.pointer_pos.is_some() {
                break;
            }
        }
        Ok(())
    }

    fn establish_virtual_pointer(&mut self) -> Result<()> {
        if self.state.vp.is_some() {
            return Ok(());
        }
        let output = self
            .state
            .output()
            .and_then(|entry| entry.output.as_ref());
        let pointer = VirtualPointer::create(
            &self.state.vp_manager,
            Some(&self.state.seat),
            output,
            &self.qh,
        );
        self.state.vp = Some(pointer);
        Ok(())
    }

    /// Render a frame into a free shared-memory buffer and commit it.
    pub fn present_with<F>(&mut self, draw: F) -> Result<()>
    where
        F: FnOnce(&mut [u8], u32, u32),
    {
        let mut index = None;
        for _ in 0..8 {
            index = self.state.ring.as_ref().and_then(BufferRing::free);
            if index.is_some() {
                break;
            }
            // Both buffers are still held by the compositor; give it a chance to
            // release one.
            self.dispatch(Some(Duration::from_millis(20)))?;
        }
        let index = index.ok_or_else(|| anyhow!("compositor never released a shared memory buffer"))?;
        let ring = self.state.ring.as_mut().expect("ring exists");
        let (width, height) = ring.size();
        draw(ring.pixels_mut(index), width, height);
        let surface = self
            .state
            .overlay
            .as_ref()
            .ok_or_else(|| anyhow!("no overlay to present on"))?
            .surface()
            .clone();
        ring.present(index, &surface)?;
        self.conn.flush().context("flush frame")?;
        Ok(())
    }

    pub fn take_keys(&mut self) -> Vec<KeyEvent> {
        self.state.keys.drain()
    }

    pub fn move_pointer(&mut self, x: f64, y: f64) -> Result<()> {
        let (w, h) = self.state.logical_size();
        if let Some(pointer) = &self.state.vp {
            pointer.move_absolute(x, y, w.max(1), h.max(1));
        }
        self.conn.flush().context("flush pointer motion")?;
        Ok(())
    }

    pub fn button(&mut self, button: u32, pressed: bool) -> Result<()> {
        if let Some(pointer) = &self.state.vp {
            pointer.button(button, pressed);
        }
        self.conn.flush().context("flush button event")?;
        Ok(())
    }

    pub fn scroll(&mut self, steps: i32) -> Result<()> {
        if let Some(pointer) = &self.state.vp {
            pointer.scroll(steps);
        }
        self.conn.flush().context("flush scroll event")?;
        Ok(())
    }

    pub fn click(&mut self, button: u32) -> Result<()> {
        self.button(button, true)?;
        self.button(button, false)
    }

    /// Stop the overlay taking pointer events, so that a click, a wheel event or
    /// a drag reaches the window underneath. The surface is left mapped:
    /// destroying it would drop the keyboard grab with it, and the point of the
    /// overlay staying up is that the session keeps receiving keys.
    pub fn finish(mut self) {
        if let Some(overlay) = self.state.overlay.take() {
            overlay.destroy();
        }
        if let Some(ring) = self.state.ring.take() {
            let _ = self.roundtrip();
            ring.destroy();
        }
        if let Some(pointer) = self.state.vp.take() {
            pointer.destroy();
        }
        let _ = self.conn.flush();
    }
}

fn bind<I, U>(
    globals: &GlobalList,
    qh: &QueueHandle<State>,
    version: RangeInclusive<u32>,
    udata: U,
    what: &str,
) -> Result<I>
where
    I: Proxy + 'static,
    State: Dispatch<I, U> + 'static,
    U: Send + Sync + 'static,
{
    globals
        .bind::<I, State, U>(qh, version, udata)
        .map_err(|err| bind_error(I::interface().name, what, err))
}

fn bind_error(interface: &str, what: &str, err: BindError) -> anyhow::Error {
    match err {
        BindError::NotPresent => {
            anyhow!("compositor does not support {what}: no {interface} global")
        }
        BindError::UnsupportedVersion => {
            anyhow!("compositor's {interface} is older than the version {what} requires")
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _state: &mut Self,
        _registry: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // Device hotplug mid-session is out of scope: the design assumes the
        // output layout is stable for the lifetime of one invocation.
    }
}

delegate_noop!(State: ignore wl_compositor::WlCompositor);
delegate_noop!(State: ignore wl_shm::WlShm);
delegate_noop!(State: ignore wl_shm_pool::WlShmPool);
delegate_noop!(State: ignore wl_region::WlRegion);
delegate_noop!(State: ignore wl_surface::WlSurface);
delegate_noop!(State: ignore zwlr_layer_shell_v1::ZwlrLayerShellV1);
delegate_noop!(State: ignore zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1);
delegate_noop!(State: ignore zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1);
delegate_noop!(State: ignore wp_viewporter::WpViewporter);
delegate_noop!(State: ignore wp_viewport::WpViewport);
delegate_noop!(State: ignore wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1);
delegate_noop!(State: ignore zxdg_output_manager_v1::ZxdgOutputManagerV1);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_global_names_both_the_capability_and_the_interface() {
        let err = bind_error(
            "zwlr_virtual_pointer_manager_v1",
            "virtual pointer",
            BindError::NotPresent,
        )
        .to_string();
        assert!(err.contains("virtual pointer"), "{err}");
        assert!(err.contains("zwlr_virtual_pointer_manager_v1"), "{err}");
    }

    #[test]
    fn unsupported_version_names_the_interface() {
        let err = bind_error(
            "zwlr_layer_shell_v1",
            "fullscreen overlay layer",
            BindError::UnsupportedVersion,
        )
        .to_string();
        assert!(err.contains("zwlr_layer_shell_v1"), "{err}");
        assert!(err.contains("fullscreen overlay layer"), "{err}");
    }
}
