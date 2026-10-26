//! Headless test helper: print where the pointer is.
//!
//! Wayland only reveals the cursor position to a client that owns the surface
//! under it, so the probe puts a fullscreen transparent layer surface on the
//! overlay layer and reports the last pointer position it is told about.
//!
//! It stays alive for `--min-lifetime` (default 1500ms) regardless: sway does
//! not re-run pointer focus for a freshly mapped surface, so the keys that move
//! the cursor arrive after the probe is up, and it must still be there when
//! they do. Exit is otherwise driven by `--settle` (default 400ms) of no
//! movement, so a check costs about a second and a half rather than the full
//! timeout.
//! Keyboard interactivity stays `None` so the probe never steals the keyboard
//! from the session being measured, and every keyboard focus it is given is
//! reported: a click has to leave the surface it landed on focused.
//!
//! Test infrastructure only — it is not part of the shipped binary.
//!
//! With `--layer bottom` the probe sits below the overlay layer, so a click only
//! reaches it if the session really did take its overlay down first. With
//! `--fill RRGGBB` it paints the output that colour, which is how the overlay is
//! looked at over a light background without needing a light application.
//!
//! With `--band HEIGHT` it is a strip at the bottom of the output instead of the
//! whole of it, and `--keyboard on-demand` is what a panel like `aside` asks for:
//! the compositor focuses it on hover or on a click, never exclusively. The two
//! together are the reproduction of "a click into a panel's text field is not
//! typable", which no other shape of probe shows.
//!
//! With `--toplevel` the probe is an ordinary fullscreen xdg-shell window rather
//! than a layer surface. Layer surfaces live in the overlay stack, so a click
//! landing on one says nothing about whether it reaches an application window;
//! the toplevel is what a real desktop puts under the pointer.
//!
//! ```sh
//! cargo run --example cursor_probe -- --timeout 2000
//! # prints "<x> <y> <width> <height>" in logical pixels
//! # then a "<button> <x> <y>" line for every click received
//! # and `keyboard enter` / `keyboard leave` in the order they arrived
//! # exits 1 if no pointer position was reported
//! ```

use std::os::fd::AsFd;
use std::time::{Duration, Instant};

use rustix::event::{poll, PollFd, PollFlags, Timespec};
use rustix::fs::{memfd_create, ftruncate, MemfdFlags};
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_keyboard, wl_pointer, wl_registry, wl_seat, wl_shm, wl_shm_pool,
    wl_surface,
};
use wayland_client::{delegate_noop, Connection, Dispatch, QueueHandle};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

#[derive(Default)]
struct State {
    configured: Option<(u32, u32)>,
    position: Option<(f64, f64)>,
    /// Every button that arrived, in order: a check can then tell which of a
    /// left, middle and right click made it through and which did not.
    clicks: Vec<(u32, f64, f64)>,
    /// The last scroll the probe was sent: the axis number and its value, so a
    /// check can tell a wheel event that reached it from one that did not.
    axis: Option<(u32, f64)>,
    /// When the position last changed, so the probe can report as soon as the
    /// session stops moving rather than always waiting out the timeout.
    motion_at: Option<Instant>,
    /// Size asked for by `xdg_toplevel::Configure`, in logical pixels. Only used
    /// in `--toplevel` mode.
    toplevel_size: Option<(u32, u32)>,
    /// Every `wl_keyboard::Enter` and `Leave`, in order. A panel that never
    /// enters never gets the keyboard, whatever a click delivered to it.
    keyboard_focus: Vec<&'static str>,
    closed: bool,
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for State {
    fn event(
        state: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Enter { .. } => state.keyboard_focus.push("enter"),
            wl_keyboard::Event::Leave { .. } => state.keyboard_focus.push("leave"),
            _ => {}
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for State {
    fn event(
        state: &mut Self,
        _: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Enter { surface_x, surface_y, .. }
            | wl_pointer::Event::Motion { surface_x, surface_y, .. } => {
                // The last position wins: the probe is started before the keys
                // that move the cursor.
                state.position = Some((surface_x, surface_y));
                state.motion_at = Some(Instant::now());
            }
            wl_pointer::Event::Button {
                button,
                state: pressed,
                ..
            } => {
                if let wayland_client::WEnum::Value(wayland_client::protocol::wl_pointer::ButtonState::Pressed) = pressed
                {
                    state.clicks.push((
                        button,
                        state.position.map(|p| p.0).unwrap_or_default(),
                        state.position.map(|p| p.1).unwrap_or_default(),
                    ));
                }
            }
            wl_pointer::Event::Axis { axis, value, .. } => {
                if let wayland_client::WEnum::Value(axis) = axis {
                    state.axis.get_or_insert((axis as u32, value));
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn event(
        _state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities { capabilities } = event {
            if let wayland_client::WEnum::Value(capabilities) = capabilities {
                if capabilities.contains(wl_seat::Capability::Pointer) {
                    seat.get_pointer(qh, ());
                }
                if capabilities.contains(wl_seat::Capability::Keyboard) {
                    // Kept alive by the seat's own refcount while the probe runs;
                    // only the focus changes are interesting here.
                    let _ = seat.get_keyboard(qh, ());
                }
            }
        }
    }
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for State {    fn event(
        state: &mut Self,
        surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure { serial, width, height } => {
                surface.ack_configure(serial);
                state.configured = Some((width, height));
            }
            zwlr_layer_surface_v1::Event::Closed => state.closed = true,
            _ => {}
        }
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for State {
    fn event(
        _state: &mut Self,
        base: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            base.pong(serial);
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, ()> for State {
    fn event(
        state: &mut Self,
        surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            surface.ack_configure(serial);
            // The size comes from the toplevel configure that precedes this.
            state.configured = state.toplevel_size;
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for State {
    fn event(
        state: &mut Self,
        _: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            xdg_toplevel::Event::Configure { width, height, .. } => {
                // Sway sends 0 for "you choose" on a tiling window; a buffer
                // cannot be zero-sized, so fall back to something usable.
                state.toplevel_size = Some((
                    width.max(1) as u32,
                    height.max(1) as u32,
                ));
            }
            xdg_toplevel::Event::Close => state.closed = true,
            _ => {}
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _state: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

delegate_noop!(State: ignore wl_compositor::WlCompositor);
delegate_noop!(State: ignore wl_shm::WlShm);
delegate_noop!(State: ignore wl_shm_pool::WlShmPool);
delegate_noop!(State: ignore wl_surface::WlSurface);
delegate_noop!(State: ignore wl_buffer::WlBuffer);
delegate_noop!(State: ignore zwlr_layer_shell_v1::ZwlrLayerShellV1);
fn main() {
    let mut timeout = Duration::from_millis(5000);
    let mut layer = zwlr_layer_shell_v1::Layer::Overlay;
    let mut min_lifetime = Duration::from_millis(1500);
    let mut settle = Duration::from_millis(400);
    let mut fill: Option<(u8, u8, u8)> = None;
    let mut toplevel = false;
    // `on-demand` is what a panel like aside asks for: the compositor focuses it
    // on hover or on a click, and never exclusively.
    let mut keyboard = zwlr_layer_surface_v1::KeyboardInteractivity::None;
    // A bottom band rather than the whole output, so the pointer can start
    // outside it and be moved onto it, the way it is moved onto a panel.
    let mut band: Option<u32> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || {
            args.next()
                .and_then(|value| value.parse().ok())
                .expect("option needs a millisecond value")
        };
        match arg.as_str() {
            "--timeout" => timeout = Duration::from_millis(value()),
            "--min-lifetime" => min_lifetime = Duration::from_millis(value()),
            "--settle" => settle = Duration::from_millis(value()),
            "--fill" => {
                let digits = args.next().expect("--fill needs RRGGBB");
                let byte = |at: usize| {
                    u8::from_str_radix(&digits[at..at + 2], 16).expect("--fill needs hexadecimal")
                };
                fill = Some((byte(0), byte(2), byte(4)));
            }
            "--layer" => {
                layer = match args.next().as_deref() {
                    Some("bottom") => zwlr_layer_shell_v1::Layer::Bottom,
                    Some("top") => zwlr_layer_shell_v1::Layer::Top,
                    Some("overlay") => zwlr_layer_shell_v1::Layer::Overlay,
                    other => panic!("unknown layer `{other:?}`"),
                }
            }
            "--toplevel" => toplevel = true,
            "--keyboard" => {
                keyboard = match args.next().as_deref() {
                    Some("on-demand") => zwlr_layer_surface_v1::KeyboardInteractivity::OnDemand,
                    Some("none") => zwlr_layer_surface_v1::KeyboardInteractivity::None,
                    other => panic!("unknown keyboard interactivity `{other:?}`"),
                }
            }
            "--band" => band = Some(value() as u32),
            other => panic!("unknown option `{other}`"),
        }
    }

    let connection = Connection::connect_to_env().expect("connect to Wayland");
    let (globals, mut queue) = registry_queue_init::<State>(&connection).expect("globals");
    let qh = queue.handle();
    let compositor: wl_compositor::WlCompositor = globals.bind(&qh, 4..=6, ()).expect("compositor");
    let shm: wl_shm::WlShm = globals.bind(&qh, 1..=1, ()).expect("wl_shm");
    let layer_shell: zwlr_layer_shell_v1::ZwlrLayerShellV1 =
        globals.bind(&qh, 1..=4, ()).expect("layer shell");
    // Held for the lifetime of the probe: the pointer proxy comes from it.
    let _seat: wl_seat::WlSeat = globals.bind(&qh, 1..=7, ()).expect("seat");

    let mut state = State::default();
    let surface = compositor.create_surface(&qh, ());
    // A regular window rather than a layer surface, so a check can tell whether
    // a click reaches an ordinary application rather than only another overlay.
    if toplevel {
        let xdg: xdg_wm_base::XdgWmBase = globals.bind(&qh, 1..=6, ()).expect("xdg_wm_base");
        let xdg_surface = xdg.get_xdg_surface(&surface, &qh, ());
        let tl = xdg_surface.get_toplevel(&qh, ());
        tl.set_title("cursor-probe".to_string());
        tl.set_app_id("cursor-probe".to_string());
        // Fullscreen so the window covers the output whatever the compositor's
        // tiling policy is: a tiled window can be a narrow column, and the
        // pointer is only over it by accident.
        tl.set_fullscreen(None);
        surface.commit();
        for _ in 0..16 {
            queue.roundtrip(&mut state).expect("roundtrip");
            if state.configured.is_some() {
                break;
            }
        }
        let (width, height) = state.configured.expect("toplevel was never configured");
        // Kept alive: dropping the xdg proxies would unmap the window.
        std::mem::forget((xdg_surface, tl));
        attach_probe_buffer(&shm, &surface, &qh, width, height, fill);
        run_probe(connection, queue, state, shm, surface, qh, fill, timeout, min_lifetime, settle);
        return;
    }
    let layer_surface = layer_shell.get_layer_surface(
        &surface,
        None,
        layer,
        "cursor-probe".to_string(),
        &qh,
        (),
    );
    match band {
        Some(height) => {
            layer_surface.set_size(0, height);
            layer_surface.set_anchor(
                zwlr_layer_surface_v1::Anchor::Bottom
                    | zwlr_layer_surface_v1::Anchor::Left
                    | zwlr_layer_surface_v1::Anchor::Right,
            );
        }
        None => {
            layer_surface.set_size(0, 0);
            layer_surface.set_anchor(
                zwlr_layer_surface_v1::Anchor::Top
                    | zwlr_layer_surface_v1::Anchor::Bottom
                    | zwlr_layer_surface_v1::Anchor::Left
                    | zwlr_layer_surface_v1::Anchor::Right,
            );
        }
    }
    layer_surface.set_exclusive_zone(-1);
    layer_surface.set_keyboard_interactivity(keyboard);
    surface.commit();

    for _ in 0..16 {
        queue.roundtrip(&mut state).expect("roundtrip");
        if state.configured.is_some() {
            break;
        }
    }
    let (width, height) = state.configured.expect("layer surface was never configured");
    attach_probe_buffer(&shm, &surface, &qh, width, height, fill);
    run_probe(connection, queue, state, shm, surface, qh, fill, timeout, min_lifetime, settle);
}

/// A fully transparent buffer at the configured surface size. It cannot be
/// seen, but it has to be the full size: a `wl_surface` is only as big as its
/// buffer, and the input region follows, so a 1x1 buffer would give a 1x1
/// probe that the pointer almost never lands on.
fn attach_probe_buffer(
    shm: &wl_shm::WlShm,
    surface: &wl_surface::WlSurface,
    qh: &QueueHandle<State>,
    width: u32,
    height: u32,
    fill: Option<(u8, u8, u8)>,
) {
    let stride = width as i32 * 4;
    let len = stride as usize * height as usize;
    let fd = memfd_create("cursor-probe", MemfdFlags::CLOEXEC).expect("memfd");
    ftruncate(&fd, len as u64).expect("resize");
    if let Some((r, g, b)) = fill {
        // Argb8888 is a native-endian 0xAARRGGBB word: BGRA in memory.
        let pixel = [b, g, r, 0xFF];
        let mut bytes = Vec::with_capacity(len);
        while bytes.len() < len {
            bytes.extend_from_slice(&pixel);
        }
        use std::io::Write;
        std::fs::File::from(fd.try_clone().expect("clone"))
            .write_all(&bytes)
            .expect("paint");
    }
    let pool = shm.create_pool(fd.as_fd(), len as i32, qh, ());
    let buffer = pool.create_buffer(0, width as i32, height as i32, stride, wl_shm::Format::Argb8888, qh, ());
    pool.destroy();
    surface.attach(Some(&buffer), 0, 0);
    surface.damage(0, 0, width as i32, height as i32);
    surface.commit();
}

/// Wait out the probe's lifetime and print what it saw.
#[allow(clippy::too_many_arguments)]
fn run_probe(
    connection: Connection,
    mut queue: wayland_client::EventQueue<State>,
    mut state: State,
    shm: wl_shm::WlShm,
    surface: wl_surface::WlSurface,
    qh: QueueHandle<State>,
    fill: Option<(u8, u8, u8)>,
    timeout: Duration,
    min_lifetime: Duration,
    settle: Duration,
) {
    // A toplevel is told `0x0` ("you choose") before the compositor tiles it, so
    // the size has to be followed: a buffer left at the first configure would
    // make the probe a 1x1 surface that the pointer never lands on.
    let mut attached = state.configured;
    let started = Instant::now();
    let deadline = started + timeout;
    while !state.closed && Instant::now() < deadline {
        let old_enough = started.elapsed() >= min_lifetime;
        if old_enough && state.motion_at.is_some_and(|at| at.elapsed() > settle) {
            break;
        }
        if state.configured != attached {
            if let Some((w, h)) = state.configured {
                attach_probe_buffer(&shm, &surface, &qh, w, h, fill);
                attached = state.configured;
            }
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        connection.flush().expect("flush");
        if let Some(guard) = connection.prepare_read() {
            let fd = guard.connection_fd();
            let mut fds = [PollFd::new(&fd, PollFlags::IN)];
            let timespec = Timespec {
                tv_sec: 0,
                tv_nsec: remaining.as_nanos().min(100_000_000) as i64,
            };
            match poll(&mut fds, Some(&timespec)) {
                Ok(ready) if ready > 0 => {
                    guard.read().ok();
                }
                Ok(_) => continue,
                Err(rustix::io::Errno::INTR) => continue,
                Err(err) => panic!("poll: {err}"),
            }
        }
        queue.dispatch_pending(&mut state).expect("dispatch");
    }

    // The size the probe ended up with, not the first one it was offered: a
    // toplevel is told `0x0` and then the size the compositor actually wants, so
    // reporting the first would name a surface it no longer has.
    let (width, height) = state.configured.expect("surface was never configured");
    match state.position {
        // The surface size is the output's logical size once the surface is
        // configured, which is what lets a caller express expectations
        // relative to the output instead of hard-coding one resolution.
        Some((x, y)) => {
            println!("{x} {y} {width} {height}");
            for (button, at_x, at_y) in &state.clicks {
                println!("click {button} {at_x} {at_y}");
            }
            if let Some((axis, value)) = state.axis {
                println!("axis {axis} {value}");
            }
            for change in &state.keyboard_focus {
                println!("keyboard {change}");
            }
        }
        None => {
            eprintln!("cursor-probe: no pointer position was reported");
            std::process::exit(1);
        }
    }
}
