//! Headless test helper: inject key events through `zwp_virtual_keyboard_v1`.
//!
//! A headless compositor has no input devices, so its seat advertises no
//! keyboard and nothing can drive `windhover` for real. This helper creates a
//! virtual keyboard and replays a short command list.
//!
//! Test infrastructure only — it is not part of the shipped binary.
//!
//! ```sh
//! cargo run --example virtual_keyboard -- --linger 300 "tap:x" "sleep:100" "tap:Escape"
//! ```
//!
//! Commands: `tap:<key>`, `down:<key>`, `up:<key>`, `mods:<alt,logo,...>`,
//! `sleep:<ms>`. Key names are xkb keysym names.
//!
//! `--gap <ms>` (default 30) separates consecutive events in time. Events are
//! stamped from a monotonically increasing clock and a press/release pair is
//! split by half the gap, because events that share a timestamp in one
//! compositor batch are not reliably delivered.
//!
//! With `--stdin` the commands are read from standard input instead, one or more
//! per line, until end of file. That mode keeps a single virtual keyboard alive
//! for a whole check, which matters because the seat loses its keyboard
//! capability whenever the last one disconnects.

use std::io::Write;
use std::os::fd::AsFd;
use std::time::{Duration, Instant};

use rustix::fs::{memfd_create, MemfdFlags};
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1, zwp_virtual_keyboard_v1,
};
use xkbcommon::xkb;

/// evdev codes for the keys the checks use. `wl_keyboard` carries the raw
/// evdev code; the `+ 8` into the XKB keycode is the receiving client's job.
fn keycode(name: &str) -> Option<u32> {
    let code = match name {
        "Escape" => 1,
        "1" => 2,
        "2" => 3,
        "3" => 4,
        "4" => 5,
        "5" => 6,
        "6" => 7,
        "7" => 8,
        "8" => 9,
        "9" => 10,
        "0" => 11,
        "minus" => 12,
        "equal" => 13,
        "BackSpace" => 14,
        "Return" => 28,
        "space" => 57,
        "Tab" => 15,
        "q" => 16,
        "w" => 17,
        "e" => 18,
        "r" => 19,
        "t" => 20,
        "y" => 21,
        "u" => 22,
        "i" => 23,
        "o" => 24,
        "p" => 25,
        "a" => 30,
        "s" => 31,
        "d" => 32,
        "f" => 33,
        "g" => 34,
        "h" => 35,
        "j" => 36,
        "k" => 37,
        "l" => 38,
        "z" => 44,
        "x" => 45,
        "c" => 46,
        "v" => 47,
        "b" => 48,
        "n" => 49,
        "m" => 50,
        "comma" => 51,
        "period" => 52,
        "slash" => 53,
        "Control_L" => 29,
        "Shift_L" => 42,
        "Alt_L" => 56,
        "Super_L" => 125,
        _ => return None,
    };
    Some(code)
}

fn modifier_mask(name: &str) -> Option<u32> {
    Some(match name {
        "shift" => 1,
        "ctrl" => 4,
        "alt" => 8,
        "logo" | "super" => 64,
        _ => return None,
    })
}

#[derive(Default)]
struct State;

impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &wl_seat::WlSeat,
        _event: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

wayland_client::delegate_noop!(State: ignore zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1);
wayland_client::delegate_noop!(State: ignore zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1);

fn main() {
    let started = Instant::now();
    let mut args = std::env::args().skip(1);
    let mut commands = Vec::new();
    let mut linger = 200u64;
    let mut gap = Duration::from_millis(30);
    let mut stdin = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--linger" => {
                linger = args
                    .next()
                    .and_then(|value| value.parse().ok())
                    .expect("--linger needs a millisecond value")
            }
            "--gap" => {
                gap = Duration::from_millis(
                    args.next()
                        .and_then(|value| value.parse().ok())
                        .expect("--gap needs a millisecond value"),
                )
            }
            "--stdin" => stdin = true,
            _ => commands.push(arg),
        }
    }

    let connection = Connection::connect_to_env().expect("connect to Wayland");
    let (globals, queue) = registry_queue_init::<State>(&connection).expect("globals");
    let qh = queue.handle();
    let manager: zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1 = globals
        .bind(&qh, 1..=1, ())
        .expect("compositor has no zwp_virtual_keyboard_manager_v1");
    let seat: wl_seat::WlSeat = globals.bind(&qh, 1..=7, ()).expect("no wl_seat");

    let keyboard = manager.create_virtual_keyboard(&seat, &qh, ());

    // The protocol requires a keymap before any key event; the compositor's own
    // layout is irrelevant once a virtual keyboard owns the seat.
    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    let keymap = xkb::Keymap::new_from_names(
        &context,
        "",
        "",
        "us",
        "",
        None,
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    )
    .expect("build a us keymap");
    let text = keymap.get_as_string(xkb::KEYMAP_FORMAT_TEXT_V1);
    let fd = memfd_create("keymap", MemfdFlags::CLOEXEC).expect("memfd");
    rustix::fs::ftruncate(&fd, text.len() as u64).expect("resize keymap");
    let mut file = std::fs::File::from(fd);
    file.write_all(text.as_bytes()).expect("write keymap");
    keyboard.keymap(1, file.as_fd(), text.len() as u32);
    connection.flush().expect("flush keymap");

    for command in commands {
        run(&keyboard, &connection, &command, gap, started);
        std::thread::sleep(gap);
    }

    if stdin {
        use std::io::BufRead;
        let input = std::io::stdin();
        for line in input.lock().lines() {
            let Ok(line) = line else { break };
            for command in line.split_whitespace() {
                run(&keyboard, &connection, command, gap, started);
                std::thread::sleep(gap);
            }
        }
        connection.flush().ok();
        return;
    }

    // Stay connected: the virtual keyboard disappears with the client, and with
    // it the seat's keyboard capability.
    std::thread::sleep(Duration::from_millis(linger));
}

fn run(
    keyboard: &zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
    connection: &Connection,
    command: &str,
    gap: Duration,
    started: Instant,
) {
    let now = || started.elapsed().as_millis() as u32;
    let (verb, value) = command.split_once(':').unwrap_or((command, ""));
    match verb {
        "tap" | "down" | "up" => {
            let code = keycode(value).unwrap_or_else(|| panic!("unknown key `{value}`"));
            if verb != "up" {
                keyboard.key(now(), code, 1); // pressed
            }
            if verb == "tap" {
                connection.flush().expect("flush press");
                std::thread::sleep(gap / 2);
            }
            if verb != "down" {
                keyboard.key(now(), code, 0); // released
            }
        }
        "mods" => {
            let mask = value
                .split(',')
                .filter(|part| !part.is_empty())
                .map(|part| modifier_mask(part).unwrap_or_else(|| panic!("unknown modifier")))
                .fold(0, |acc, bit| acc | bit);
            keyboard.modifiers(mask, 0, 0, 0);
        }
        "sleep" => std::thread::sleep(Duration::from_millis(value.parse().unwrap_or(0))),
        other => panic!("unknown command `{other}`"),
    }
    connection.flush().expect("flush key event");
}
