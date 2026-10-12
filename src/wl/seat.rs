//! Seat, keyboard, and xkb key resolution.

use std::collections::HashSet;
use std::io::{Read, Seek};
use std::os::fd::OwnedFd;

use wayland_client::{
    protocol::{wl_keyboard, wl_pointer, wl_seat},
    Dispatch, WEnum,
};
use xkbcommon::xkb;

use super::State;

/// Wayland reports the raw evdev code, and xkbcommon indexes keys by the XKB
/// code, which sits eight above it. Resolving an evdev code directly reads the
/// wrong key, and every code below 9 reads no key at all.
const XKB_KEYCODE_OFFSET: u32 = 8;

/// One keyboard transition, already resolved through the active xkb layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    pub keysym: xkb::Keysym,
    pub pressed: bool,
}

#[derive(Default)]
pub struct Keyboard {
    context: Option<xkb::Context>,
    // `xkb::State` does not own its keymap; both are kept here.
    keymap: Option<xkb::Keymap>,
    state: Option<xkb::State>,
    /// Keys held down right now. Seeded from `wl_keyboard::Enter`, which is how
    /// the modifiers the user is still holding from the hotkey become visible
    /// without being mistaken for input.
    down: HashSet<u32>,
    events: Vec<KeyEvent>,
}

impl Keyboard {
    pub fn drain(&mut self) -> Vec<KeyEvent> {
        std::mem::take(&mut self.events)
    }

    fn load_keymap(
        &mut self,
        format: WEnum<wl_keyboard::KeymapFormat>,
        fd: OwnedFd,
        size: u32,
    ) {
        if format != WEnum::Value(wl_keyboard::KeymapFormat::XkbV1) {
            return;
        }
        let mut file = std::fs::File::from(fd);
        // The descriptor can arrive with its offset already past the data, so
        // read from the start explicitly instead of from wherever it was left.
        let _ = file.seek(std::io::SeekFrom::Start(0));
        let mut buffer = vec![0u8; size as usize];
        if file.read_exact(&mut buffer).is_err() {
            buffer.clear();
            if file.read_to_end(&mut buffer).is_err() {
                return;
            }
        }
        // The compositor's buffer is NUL-terminated but xkbcommon is handed an
        // explicit length, and the terminator makes it fail to parse.
        let end = buffer
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(buffer.len());
        let text = String::from_utf8_lossy(&buffer[..end]).into_owned();
        let context = self
            .context
            .get_or_insert_with(|| xkb::Context::new(xkb::CONTEXT_NO_FLAGS));
        let Some(keymap) = xkb::Keymap::new_from_string(
            context,
            text,
            xkb::KEYMAP_FORMAT_TEXT_V1,
            xkb::KEYMAP_COMPILE_NO_FLAGS,
        ) else {
            eprintln!("windhover: compositor sent an xkb keymap that could not be parsed");
            return;
        };
        self.state = Some(xkb::State::new(&keymap));
        self.keymap = Some(keymap);
    }

    fn handle(&mut self, event: wl_keyboard::Event) {
        match event {
            wl_keyboard::Event::Keymap { format, fd, size } => {
                self.load_keymap(format, fd, size)
            }
            wl_keyboard::Event::Enter { keys, .. } => {
                self.down.clear();
                self.down.extend(keys.iter().map(|key| u32::from(*key)));
            }
            wl_keyboard::Event::Leave { .. } => self.down.clear(),
            wl_keyboard::Event::Key { key, state, .. } => {
                let key = u32::from(key);
                let pressed = match state {
                    WEnum::Value(wl_keyboard::KeyState::Pressed) => true,
                    WEnum::Value(wl_keyboard::KeyState::Released) => false,
                    _ => return,
                };
                if pressed && !self.down.insert(key) {
                    // Already held when we took focus: not a new press.
                    return;
                }
                if !pressed {
                    self.down.remove(&key);
                }
                if let Some(xkb_state) = &self.state {
                    let keysym =
                        xkb_state.key_get_one_sym(xkb::Keycode::new(key + XKB_KEYCODE_OFFSET));
                    self.events.push(KeyEvent { keysym, pressed });
                }
            }
            wl_keyboard::Event::Modifiers {
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
                ..
            } => {
                if let Some(xkb_state) = &mut self.state {
                    // `group` is the effective layout; the Wayland protocol has
                    // no separate depressed or latched layout.
                    xkb_state.update_mask(mods_depressed, mods_latched, mods_locked, 0, 0, group);
                }
            }
            wl_keyboard::Event::RepeatInfo { .. } => {}
            _ => {}
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn event(
        state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &wayland_client::Connection,
        qh: &wayland_client::QueueHandle<Self>,
    ) {
        let wl_seat::Event::Capabilities { capabilities } = event else {
            return;
        };
        let WEnum::Value(capabilities) = capabilities else {
            return;
        };
        if capabilities.contains(wl_seat::Capability::Keyboard) && state.keyboard.is_none() {
            state.keyboard = Some(seat.get_keyboard(qh, ()));
            state.keyboard_ready = true;
        }
        if capabilities.contains(wl_seat::Capability::Pointer) && state.pointer.is_none() {
            state.pointer = Some(seat.get_pointer(qh, ()));
            state.pointer_ready = true;
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for State {
    fn event(
        state: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &wayland_client::QueueHandle<Self>,
    ) {
        state.keys.handle(event);
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for State {
    fn event(
        state: &mut Self,
        _: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            // Surface-local coordinates, which for our fullscreen overlay are
            // already the logical coordinates the modes work in.
            wl_pointer::Event::Enter { surface_x, surface_y, .. }
            | wl_pointer::Event::Motion { surface_x, surface_y, .. } => {
                state.pointer_pos = Some((surface_x, surface_y));
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::parse_key;

    /// Keys, spelled out instead of included so the test needs no system
    /// xkeyboard-config data. `<ESC>` is xkb keycode 9, `<AD11>` is 35 and
    /// `<AC06>` is 43, which are evdev 1, 27 and 35.
    const KEYMAP: &str = concat!(
        "xkb_keymap {\n",
        "  xkb_keycodes \"t\" { minimum = 8; maximum = 255; <ESC> = 9; <AD11> = 35; \
         <AC06> = 43; };\n",
        "  xkb_types \"t\" { type \"ONE_LEVEL\" { modifiers = None; map[None] = Level1; \
         level_name[Level1] = \"Any\"; }; };\n",
        "  xkb_compat \"t\" { };\n",
        "  xkb_symbols \"t\" { key <ESC> { [ Escape ] }; key <AD11> { [ bracketright ] }; \
         key <AC06> { [ h ] }; };\n",
        "};\n"
    );

    fn keyboard() -> Keyboard {
        let fd = rustix::fs::memfd_create("keymap", rustix::fs::MemfdFlags::CLOEXEC).unwrap();
        rustix::io::write(&fd, KEYMAP.as_bytes()).unwrap();
        let mut keyboard = Keyboard::default();
        keyboard.load_keymap(
            WEnum::Value(wl_keyboard::KeymapFormat::XkbV1),
            fd,
            KEYMAP.len() as u32,
        );
        keyboard
    }

    /// `evdev` is what the compositor sends; the offset into the xkb keycode is
    /// the client's to apply. Without it a key resolves to whichever key sits
    /// eight codes away, and anything below 9 resolves to nothing at all, which
    /// is where Escape lives.
    fn press(keyboard: &mut Keyboard, evdev: u32) -> Vec<KeyEvent> {
        keyboard.handle(wl_keyboard::Event::Key {
            serial: 0,
            time: 0,
            key: evdev,
            state: WEnum::Value(wl_keyboard::KeyState::Pressed),
        });
        keyboard.drain()
    }

    #[test]
    fn key_events_carry_the_keysym_the_evdev_code_names() {
        let mut keyboard = keyboard();
        assert_eq!(
            press(&mut keyboard, 35),
            vec![KeyEvent {
                keysym: parse_key("h").unwrap(),
                pressed: true,
            }]
        );
        assert_eq!(
            press(&mut keyboard, 1),
            vec![KeyEvent {
                keysym: parse_key("Escape").unwrap(),
                pressed: true,
            }]
        );
    }
}
