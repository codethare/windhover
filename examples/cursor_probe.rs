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
//! from the session being measured.
//!
//! Test infrastructure only — it is not part of the shipped binary.
//!
//! With `--layer bottom` the probe sits below the overlay layer, so a click only
//! reaches it if the session really did take its overlay down first. With
//! `--fill RRGGBB` it paints the output that colour, which is how the overlay is
//! looked at over a light background without needing a light application.
//!
//! ```sh
//! cargo run --example cursor_probe -- --timeout 2000
//! # prints "<x> <y> <width> <height>" in logical pixels
//! # and, on a second line, "<button> <x> <y>" for the first click received
//! # exits 1 if no pointer position was reported
//! ```

use std::os::fd::AsFd;
use std::time::{Duration, Instant};

use rustix::event::{poll, PollFd, PollFlags, Timespec};
use rustix::fs::{memfd_create, ftruncate, MemfdFlags};
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_pointer, wl_registry, wl_seat, wl_shm, wl_shm_pool, wl_surface,
};
use wayland_client::{delegate_noop, Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

#[derive(Default)]
struct State {
    configured: Option<(u32, u32)>,
    position: Option<(f64, f64)>,
    click: Option<(u32, f64, f64)>,
    /// The last scroll the probe was sent: the axis number and its value, so a
    /// check can tell a wheel event that reached it from one that did not.
    axis: Option<(u32, f64)>,
    /// When the position last changed, so the probe can report as soon as the
    /// session stops moving rather than always waiting out the timeout.
    motion_at: Option<Instant>,
    closed: bool,
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
                    state.click.get_or_insert((
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
            }
        }
    }
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for State {
    fn event(
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
    let layer_surface = layer_shell.get_layer_surface(
        &surface,
        None,
        layer,
        "cursor-probe".to_string(),
        &qh,
        (),
    );
    layer_surface.set_size(0, 0);
    layer_surface.set_anchor(
        zwlr_layer_surface_v1::Anchor::Top
            | zwlr_layer_surface_v1::Anchor::Bottom
            | zwlr_layer_surface_v1::Anchor::Left
            | zwlr_layer_surface_v1::Anchor::Right,
    );
    layer_surface.set_exclusive_zone(-1);
    layer_surface.set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::None);
    surface.commit();

    for _ in 0..16 {
        queue.roundtrip(&mut state).expect("roundtrip");
        if state.configured.is_some() {
            break;
        }
    }
    let (width, height) = state.configured.expect("layer surface was never configured");

    // A fully transparent buffer at the configured surface size. It cannot be
    // seen, but it has to be the full size: a `wl_surface` is only as big as its
    // buffer, and the input region follows, so a 1x1 buffer would give a 1x1
    // probe that the pointer almost never lands on.
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
        std::fs::File::from(fd.try_clone().expect("clone")).write_all(&bytes).expect("paint");
    }
    let pool = shm.create_pool(fd.as_fd(), len as i32, &qh, ());
    let buffer = pool.create_buffer(
        0,
        width as i32,
        height as i32,
        stride,
        wl_shm::Format::Argb8888,
        &qh,
        (),
    );
    pool.destroy();
    surface.attach(Some(&buffer), 0, 0);
    surface.damage(0, 0, width as i32, height as i32);
    surface.commit();

    let started = Instant::now();
    let deadline = started + timeout;
    while !state.closed && Instant::now() < deadline {
        let old_enough = started.elapsed() >= min_lifetime;
        if old_enough && state.motion_at.is_some_and(|at| at.elapsed() > settle) {
            break;
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

    match state.position {
        // The surface size is the output's logical size once the layer surface
        // is configured, which is what lets a caller express expectations
        // relative to the output instead of hard-coding one resolution.
        Some((x, y)) => {
            println!("{x} {y} {width} {height}");
            if let Some((button, at_x, at_y)) = state.click {
                println!("{button} {at_x} {at_y}");
            }
            if let Some((axis, value)) = state.axis {
                println!("axis {axis} {value}");
            }
        }
        None => {
            eprintln!("cursor-probe: no pointer position was reported");
            std::process::exit(1);
        }
    }
}
