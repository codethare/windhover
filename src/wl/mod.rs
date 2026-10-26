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

use crate::trace;

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
    /// The output the overlay was placed on, as the compositor reported it.
    /// `output_index` is only what was asked for; this is the answer, and it is
    /// what the virtual pointer has to be bound to.
    pub overlay_output: Option<usize>,
    pub output_index: Option<usize>,
    pub keys: Keyboard,
    /// Where the compositor last reported the pointer. Only filled during the
    /// handshake at startup: the overlay stops taking pointer events once the
    /// starting position is known, so the session tracks its own moves from there.
    pub pointer_pos: Option<(f64, f64)>,
    pub closed: bool,
    pub keyboard_ready: bool,
    pub pointer_ready: bool,
    /// Set once the layout fallback has been explained, so it is said once.
    space_warned: bool,
    vp: Option<VirtualPointer>,
}

/// Where absolute pointer coordinates are measured from, and how far they reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PointerSpace {
    origin: (i32, i32),
    extent: (u32, u32),
}

impl PointerSpace {
    /// Absolute motion is normalised over the box the outputs occupy, so an
    /// overlay-local point reaches the right output by moving from the overlay's
    /// own corner to its place in that box.
    fn of(overlay: (u32, u32), overlay_origin: (i32, i32), outputs: &[OutputEntry]) -> Self {
        let (corner, extent) = output::layout_box(outputs, overlay);
        Self {
            origin: (
                overlay_origin.0.saturating_sub(corner.0),
                overlay_origin.1.saturating_sub(corner.1),
            ),
            extent,
        }
    }
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

    /// The coordinate space the virtual pointer addresses.
    ///
    /// A pointer bound to an output is given that output's own pixels, which is
    /// what the overlay draws in. An unbound one — a version 1 manager, or an
    /// output the compositor never told us about — is given the compositor's
    /// layout, so an overlay-local point has to be moved to where the overlay
    /// sits before it is normalised against the layout.
    fn pointer_space(&self) -> PointerSpace {
        let origin = self
            .overlay_output
            .or(self.output_index)
            .and_then(|index| self.outputs.get(index))
            .map(|entry| entry.info.logical_origin())
            .unwrap_or((0, 0));
        PointerSpace::of(self.logical_size(), origin, &self.outputs)
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
            overlay_output: None,
            output_index: None,
            keys: Keyboard::default(),
            pointer_pos: None,
            closed: false,
            keyboard_ready: false,
            space_warned: false,
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
            if self.state.pointer_pos.is_some() {
                // The pointer position is known, which is the last thing the
                // overlay was needed for. From here the applications have it.
                self.hand_over_pointer()?;
                // Created after placement, never before: a virtual pointer bound
                // to an output addresses that output's pixels, so the output has
                // to be the one the overlay is really on.
                self.establish_virtual_pointer()?;
                return Ok(());
            }
        }

        // No output ever saw the pointer (headless, or a compositor that does
        // not send enter before the first frame). Keep the compositor's choice.
        self.state.pointer_pos = None;
        self.create_overlay(None)?;
        self.hand_over_pointer()?;
        self.establish_virtual_pointer()?;
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
        // The surface has not been shown yet, so whatever the previous one
        // entered says nothing about this one.
        self.state.overlay_output = None;

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
        // One more roundtrip so a `wl_surface::Enter` that came with the
        // configure has been dispatched: without it a compositor that placed the
        // overlay for us leaves us guessing, and the guess is the origin.
        self.roundtrip()?;
        let index = self.state.overlay_output.or(self.state.output_index);
        let pointer = VirtualPointer::create(
            &self.state.vp_manager,
            Some(&self.state.seat),
            &self.qh,
        );
        let space = self.state.pointer_space();
        let name = index
            .and_then(|i| self.state.outputs.get(i))
            .map(|entry| entry.info.name.as_str())
            .unwrap_or("an unnamed output");
        trace::log(format_args!(
            "virtual pointer in the layout: {name} starts at {},{} and the layout is {}x{}",
            space.origin.0, space.origin.1, space.extent.0, space.extent.1
        ));
        self.warn_untranslatable_pointer();
        self.state.vp = Some(pointer);
        Ok(())
    }

    /// Say so when the layout cannot be translated into: an overlay on an output
    /// away from the origin needs that output's position, and without
    /// `zxdg_output_v1` every output claims (0, 0).
    fn warn_untranslatable_pointer(&mut self) {
        if self.state.space_warned || self.state.outputs.len() < 2 {
            return;
        }
        if self
            .state
            .outputs
            .iter()
            .all(|entry| entry.info.position_known())
        {
            return;
        }
        self.state.space_warned = true;
        eprintln!(
            "windhover: no output reported its position (zxdg_output_v1 is missing), so an \
             overlay on an output away from the layout's origin cannot be aimed at"
        );
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
        let space = self.state.pointer_space();
        if let Some(pointer) = &self.state.vp {
            pointer.move_absolute(
                x + space.origin.0 as f64,
                y + space.origin.1 as f64,
                space.extent.0,
                space.extent.1,
            );
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

    /// Click under the overlay, and give up the session with it.
    ///
    /// The keyboard grab goes before the button is pressed, and the roundtrip is
    /// what makes that order real: while an exclusive layer surface is mapped a
    /// compositor keeps the keyboard with it, so a click that asks for focus on a
    /// panel is refused (river) or taken back when the surface goes away (sway).
    /// Only the grab is dropped — the surface stays, so the pointer keeps landing
    /// where it would have anyway.
    pub fn release_keyboard_and_click(&mut self, button: u32) -> Result<()> {
        if let Some(overlay) = self.state.overlay.as_ref() {
            overlay.release_keyboard();
            self.roundtrip()?;
        }
        self.button(button, true)?;
        self.button(button, false)
    }

    /// Tear the session down: whatever is left of the overlay, its buffers, and
    /// the virtual pointer.
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
/// The overlay's surface is the only `wl_surface` this client creates, so the
/// output it enters is the output the overlay is on — which is the one the
/// virtual pointer has to be bound to.
impl Dispatch<wl_surface::WlSurface, ()> for State {
    fn event(
        state: &mut Self,
        _proxy: &wl_surface::WlSurface,
        event: wl_surface::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &QueueHandle<Self>,
    ) {
        let index = |state: &Self, output: &wl_output::WlOutput| {
            state
                .outputs
                .iter()
                .position(|entry| entry.output.as_ref() == Some(output))
        };
        match event {
            wl_surface::Event::Enter { output } => {
                state.overlay_output = index(state, &output);
            }
            wl_surface::Event::Leave { output }
                if state.overlay_output == index(state, &output) =>
            {
                state.overlay_output = None;
            }
            _ => {}
        }
    }
}

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

    fn two_outputs() -> Vec<OutputEntry> {
        let side = |x: i32, w: u32| OutputEntry {
            info: output::OutputInfo {
                logical_x: x,
                logical_w: w,
                logical_h: 720,
                position_known: true,
                ..output::OutputInfo::default()
            },
            ..OutputEntry::default()
        };
        vec![side(0, 1280), side(1280, 1920)]
    }

    #[test]
    fn a_single_output_at_the_origin_is_the_overlays_own_pixels() {
        let space = PointerSpace::of((1280, 720), (0, 0), &two_outputs()[..1]);
        assert_eq!(
            space,
            PointerSpace {
                origin: (0, 0),
                extent: (1280, 720)
            },
            "one output at (0,0) must not be translated"
        );
    }

    #[test]
    fn a_point_moves_to_where_the_overlay_sits_in_the_layout() {
        // The layout starts at the leftmost output, so the overlay on the second
        // one is 1280 in from that corner.
        let space = PointerSpace::of((1920, 1080), (1280, 0), &two_outputs());
        assert_eq!(
            space,
            PointerSpace {
                origin: (1280, 0),
                extent: (3200, 720)
            }
        );
    }

    #[test]
    fn a_layout_with_no_output_at_the_origin_still_starts_at_its_corner() {
        // Both outputs start away from (0, 0): the overlay on the first one is at
        // the box's corner, so its own x=0 is x=0 in the box.
        let outputs = [
            OutputEntry {
                info: output::OutputInfo {
                    logical_x: 1280,
                    logical_w: 1280,
                    logical_h: 720,
                    position_known: true,
                    ..output::OutputInfo::default()
                },
                ..OutputEntry::default()
            },
            OutputEntry {
                info: output::OutputInfo {
                    logical_x: 2560,
                    logical_w: 1280,
                    logical_h: 720,
                    position_known: true,
                    ..output::OutputInfo::default()
                },
                ..OutputEntry::default()
            },
        ];
        assert_eq!(
            PointerSpace::of((1280, 720), (1280, 0), &outputs),
            PointerSpace {
                origin: (0, 0),
                extent: (2560, 720)
            }
        );
        assert_eq!(
            PointerSpace::of((1280, 720), (2560, 0), &outputs),
            PointerSpace {
                origin: (1280, 0),
                extent: (2560, 720)
            }
        );
    }

    #[test]
    fn an_unknown_output_origin_leaves_the_coordinates_alone() {
        // Nothing reported a position or a size, so (0, 0) and the overlay's own
        // size are all there is.
        let space = PointerSpace::of((1280, 720), (0, 0), &[]);
        assert_eq!(
            space,
            PointerSpace {
                origin: (0, 0),
                extent: (1280, 720)
            }
        );
    }

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
