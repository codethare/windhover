//! Seat, keyboard, and xkb key resolution.

use std::collections::HashSet;
use std::io::{Read, Seek};
use std::os::fd::OwnedFd;

use wayland_client::{
    Dispatch, WEnum,
    protocol::{wl_keyboard, wl_pointer, wl_seat},
};
use xkbcommon::xkb;

use super::State;
use crate::trace;

/// Wayland reports the raw evdev code, and xkbcommon indexes keys by the XKB
/// code, which sits eight above it. Resolving an evdev code directly reads the
/// wrong key, and every code below 9 reads no key at all.
const XKB_KEYCODE_OFFSET: u32 = 8;

/// Resolve a key the way a shortcut wants it: Lock is ignored so Caps Lock does
/// not change which binding is reached, while Shift and the other modifiers
/// still select the level they normally do. The state is restored before it is
/// used for anything else, so repeat detection and Control still see the real
/// modifier state.
fn shortcut_keysym(
    state: &mut xkb::State,
    lock_mask: xkb::ModMask,
    key: xkb::Keycode,
) -> xkb::Keysym {
    if lock_mask == 0 {
        return state.key_get_one_sym(key);
    }
    let depressed = state.serialize_mods(xkb::STATE_MODS_DEPRESSED);
    let latched = state.serialize_mods(xkb::STATE_MODS_LATCHED);
    let locked = state.serialize_mods(xkb::STATE_MODS_LOCKED);
    if depressed & lock_mask == 0 && latched & lock_mask == 0 && locked & lock_mask == 0 {
        return state.key_get_one_sym(key);
    }
    let depressed_layout = state.serialize_layout(xkb::STATE_LAYOUT_DEPRESSED);
    let latched_layout = state.serialize_layout(xkb::STATE_LAYOUT_LATCHED);
    let locked_layout = state.serialize_layout(xkb::STATE_LAYOUT_LOCKED);
    state.update_mask(
        depressed & !lock_mask,
        latched & !lock_mask,
        locked & !lock_mask,
        depressed_layout,
        latched_layout,
        locked_layout,
    );
    let keysym = state.key_get_one_sym(key);
    state.update_mask(
        depressed,
        latched,
        locked,
        depressed_layout,
        latched_layout,
        locked_layout,
    );
    keysym
}

/// The modifier that scales a movement. The keysym cannot show it: a modifier
/// does not change which symbol a letter resolves to.
pub const FAST_MODIFIER: &str = "Control";

/// One keyboard transition, already resolved through the active xkb layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    pub keysym: xkb::Keysym,
    pub pressed: bool,
    /// Whether the fast modifier was down when the key was pressed.
    pub fast: bool,
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
    /// Whether the seat has given this session the keyboard. A button press on a
    /// window is enough to take it away, and without it no key can reach this
    /// session: not the release of a held key, not the one that ends a drag, and
    /// not the one that quits.
    focused: bool,
    /// Set when that changes, and consumed by the session.
    focus_change: Option<bool>,
    /// Set when a keymap is installed, and consumed by the session: that is the
    /// point at which the configured bindings can be checked against the layout.
    keymap_loaded: bool,
    /// The Lock modifier's bit, or 0 when this keymap has none. Shortcuts are
    /// resolved with it cleared: Caps Lock must not turn `h` into `H`.
    lock_mask: xkb::ModMask,
    /// Why the last keymap could not be installed, for the startup check. A
    /// keyboard with no keymap receives events that cannot be resolved, so the
    /// session would run with no key able to reach it.
    keymap_error: Option<String>,
    events: Vec<KeyEvent>,
}

impl Keyboard {
    pub fn drain(&mut self) -> Vec<KeyEvent> {
        std::mem::take(&mut self.events)
    }

    /// Whether keys can be resolved at all.
    pub fn keymap_ready(&self) -> bool {
        self.state.is_some()
    }

    /// The reason the keyboard has no usable keymap, if one was reported.
    pub fn keymap_error(&self) -> Option<&str> {
        self.keymap_error.as_deref()
    }

    /// Whether the session has the keyboard, reported once per change.
    pub fn take_focus_change(&mut self) -> Option<bool> {
        self.focus_change.take()
    }

    /// Whether a keymap has been installed since this was last asked.
    pub fn take_keymap_loaded(&mut self) -> bool {
        std::mem::take(&mut self.keymap_loaded)
    }

    /// Every keysym this keymap can produce, over all of its layouts and levels.
    /// A binding that is not in here cannot be pressed whatever the user does,
    /// which is worth saying out loud: one of the bindings is the one that ends
    /// the session.
    pub fn reachable_keysyms(&self) -> Vec<xkb::Keysym> {
        let Some(keymap) = &self.keymap else {
            return Vec::new();
        };
        let mut seen = HashSet::new();
        let mut reachable = Vec::new();
        for raw in keymap.min_keycode().raw()..=keymap.max_keycode().raw() {
            let key = xkb::Keycode::new(raw);
            for layout in 0..keymap.num_layouts_for_key(key) {
                for level in 0..keymap.num_levels_for_key(key, layout) {
                    for sym in keymap.key_get_syms_by_level(key, layout, level) {
                        if *sym != xkb::Keysym::NoSymbol && seen.insert(*sym) {
                            reachable.push(*sym);
                        }
                    }
                }
            }
        }
        reachable
    }

    fn load_keymap(&mut self, format: WEnum<wl_keyboard::KeymapFormat>, fd: OwnedFd, size: u32) {
        if format != WEnum::Value(wl_keyboard::KeymapFormat::XkbV1) {
            self.keymap_error =
                Some("the compositor sent a keymap in a format other than XkbV1".to_string());
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
                self.keymap_error = Some("the compositor's keymap could not be read".to_string());
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
            self.keymap_error =
                Some("the compositor sent an xkb keymap that could not be parsed".to_string());
            eprintln!("windhover: compositor sent an xkb keymap that could not be parsed");
            return;
        };
        trace::log(format_args!(
            "keymap loaded, keycodes {}..={}",
            keymap.min_keycode().raw(),
            keymap.max_keycode().raw()
        ));
        let lock = keymap.mod_get_index("Lock");
        self.lock_mask = if lock == xkb::MOD_INVALID || lock >= 32 {
            0
        } else {
            1 << lock
        };
        self.keymap_error = None;
        self.state = Some(xkb::State::new(&keymap));
        self.keymap = Some(keymap);
        self.keymap_loaded = true;
    }

    fn handle(&mut self, event: wl_keyboard::Event) {
        match event {
            wl_keyboard::Event::Keymap { format, fd, size } => self.load_keymap(format, fd, size),
            wl_keyboard::Event::Enter { keys, .. } => {
                trace::log(format_args!(
                    "focus enter, {} keys already down",
                    keys.len()
                ));
                if !self.focused {
                    self.focused = true;
                    self.focus_change = Some(true);
                }
                self.down.clear();
                self.down.extend(keys.iter().map(|key| u32::from(*key)));
            }
            wl_keyboard::Event::Leave { .. } => {
                trace::log(format_args!("focus leave"));
                if self.focused {
                    self.focused = false;
                    self.focus_change = Some(false);
                }
                self.down.clear();
            }
            wl_keyboard::Event::Key { key, state, .. } => {
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
                if let Some(xkb_state) = &mut self.state {
                    let keysym = shortcut_keysym(
                        xkb_state,
                        self.lock_mask,
                        xkb::Keycode::new(key + XKB_KEYCODE_OFFSET),
                    );
                    let fast =
                        xkb_state.mod_name_is_active(FAST_MODIFIER, xkb::STATE_MODS_EFFECTIVE);
                    trace::log(format_args!(
                        "key {:?} {} fast={fast}",
                        keysym,
                        if pressed { "down" } else { "up" }
                    ));
                    self.events.push(KeyEvent {
                        keysym,
                        pressed,
                        fast,
                    });
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
            wl_pointer::Event::Enter {
                surface_x,
                surface_y,
                ..
            }
            | wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } => {
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
         level_name[Level1] = \"Any\"; }; \
         type \"ALPHABETIC\" { modifiers = Shift+Lock; map[Shift] = Level2; \
         map[Lock] = Level2; map[Shift+Lock] = Level1; level_name[Level1] = \"Base\"; \
         level_name[Level2] = \"Caps\"; }; };\n",
        "  xkb_compat \"t\" { };\n",
        "  xkb_symbols \"t\" { key <ESC> { [ Escape ] }; key <AD11> { [ bracketright ] }; \
         key <AC06> { type[Group1] = \"ALPHABETIC\", [ h, H ] }; };\n",
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
    fn the_reachable_keysyms_are_the_ones_the_keymap_defines() {
        let keyboard = keyboard();
        let reachable = keyboard.reachable_keysyms();
        assert!(
            reachable.contains(&parse_key("Escape").unwrap()),
            "{reachable:?}"
        );
        assert!(
            reachable.contains(&parse_key("h").unwrap()),
            "{reachable:?}"
        );
        // Defined as a keycode, but this keymap gives it no symbol.
        assert!(
            !reachable.contains(&parse_key("j").unwrap()),
            "a key the keymap has no symbol for was reported as reachable"
        );
    }

    #[test]
    fn a_keymap_is_reported_once_when_it_arrives() {
        let mut keyboard = keyboard();
        assert!(keyboard.take_keymap_loaded());
        assert!(!keyboard.take_keymap_loaded());
    }

    fn feed_keymap(keyboard: &mut Keyboard, format: wl_keyboard::KeymapFormat, text: &[u8]) {
        let fd = rustix::fs::memfd_create("keymap", rustix::fs::MemfdFlags::CLOEXEC).unwrap();
        rustix::io::write(&fd, text).unwrap();
        keyboard.load_keymap(WEnum::Value(format), fd, text.len() as u32);
    }

    #[test]
    fn a_keymap_that_cannot_be_parsed_is_reported() {
        let mut keyboard = Keyboard::default();
        feed_keymap(
            &mut keyboard,
            wl_keyboard::KeymapFormat::XkbV1,
            b"this is not an xkb keymap",
        );
        assert!(!keyboard.keymap_ready());
        assert!(keyboard.keymap_error().is_some());
    }

    #[test]
    fn a_non_xkb_keymap_is_reported() {
        let mut keyboard = Keyboard::default();
        feed_keymap(&mut keyboard, wl_keyboard::KeymapFormat::NoKeymap, b"");
        assert!(!keyboard.keymap_ready());
        assert!(
            keyboard
                .keymap_error()
                .is_some_and(|reason| reason.contains("XkbV1")),
            "{:?}",
            keyboard.keymap_error()
        );
    }

    #[test]
    fn key_events_carry_the_keysym_the_evdev_code_names() {
        let mut keyboard = keyboard();
        assert_eq!(
            press(&mut keyboard, 35),
            vec![KeyEvent {
                keysym: parse_key("h").unwrap(),
                pressed: true,
                fast: false,
            }]
        );
        assert_eq!(
            press(&mut keyboard, 1),
            vec![KeyEvent {
                keysym: parse_key("Escape").unwrap(),
                pressed: true,
                fast: false,
            }]
        );
    }

    fn set_modifiers(keyboard: &mut Keyboard, depressed: u32, locked: u32) {
        keyboard.handle(wl_keyboard::Event::Modifiers {
            serial: 0,
            mods_depressed: depressed,
            mods_latched: 0,
            mods_locked: locked,
            group: 0,
        });
    }

    fn release(keyboard: &mut Keyboard, evdev: u32) {
        keyboard.handle(wl_keyboard::Event::Key {
            serial: 0,
            time: 0,
            key: evdev,
            state: WEnum::Value(wl_keyboard::KeyState::Released),
        });
        keyboard.drain();
    }

    #[test]
    fn caps_lock_does_not_change_which_shortcut_a_key_reaches() {
        let mut keyboard = keyboard();
        let lock = keyboard.lock_mask;
        assert_ne!(lock, 0, "the test keymap has a Lock modifier");

        set_modifiers(&mut keyboard, 0, lock);
        assert_eq!(
            press(&mut keyboard, 35)[0].keysym,
            parse_key("h").unwrap(),
            "Caps Lock turned h into H"
        );
        release(&mut keyboard, 35);

        let shift_index = keyboard
            .keymap
            .as_ref()
            .expect("keymap")
            .mod_get_index("Shift");
        assert!(shift_index < 32, "the test keymap has a Shift modifier");
        set_modifiers(&mut keyboard, 1 << shift_index, lock);
        assert_eq!(
            press(&mut keyboard, 35)[0].keysym,
            parse_key("H").unwrap(),
            "Shift with Caps Lock must still select the shifted level"
        );
    }
}
