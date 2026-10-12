//! Mode state machine and the session that drives it.
//!
//! [`Session`] is pure: it owns the cursor, the three mode states, and the
//! binding tables, and turns key presses into [`Effect`]s. [`run`] is the only
//! part that talks to Wayland.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::Result;
use xkbcommon::xkb;

use crate::config::Config;
use crate::hint::lattice::Lattice;
use crate::keys::{Action, Keysym};
use crate::mode::grid::Grid;
use crate::mode::hint::Hint;
use crate::mode::normal::{Context, Normal};
use crate::mode::Effect;
pub use crate::mode::Mode;
use crate::render::{self, Canvas, Fonts, Frame, GridFrame, HintFrame};
use crate::wl::pointer::BTN_LEFT;
use crate::wl::Wayland;

/// How long a movement key has to be held before it starts repeating, and the
/// gap between repeats after that. Repeated here rather than taken from the
/// compositor's `repeat_info`, which only arrives when focus moves and would
/// also make grid and hint keys repeat.
const REPEAT_DELAY: Duration = Duration::from_millis(300);
const REPEAT_INTERVAL: Duration = Duration::from_millis(40);
/// Nothing above this matters: repeats are bounded so a stuck key cannot spin.
const REPEAT_CEILING: Duration = Duration::from_millis(500);

/// Set from a signal handler, acted on by the session loop. Doing the teardown
/// in the loop rather than in the handler keeps the handler async-signal-safe
/// and makes a `kill` leave exactly as little behind as Escape does.
static INTERRUPTED: AtomicBool = AtomicBool::new(false);

extern "C" fn note_signal(_signal: libc::c_int) {
    INTERRUPTED.store(true, Ordering::Relaxed);
}

fn watch_for_signals() {
    // SAFETY: the handler only stores to an atomic, which is async-signal-safe.
    unsafe {
        let handler = note_signal as extern "C" fn(libc::c_int) as libc::sighandler_t;
        libc::signal(libc::SIGTERM, handler);
        libc::signal(libc::SIGINT, handler);
        libc::signal(libc::SIGHUP, handler);
    }
}

fn interrupted() -> bool {
    INTERRUPTED.load(Ordering::Relaxed)
}

pub struct Session {
    mode: Mode,
    cursor: (f64, f64),
    screen: (u32, u32),
    config: Config,
    normal: Normal,
    grid: Grid,
    hint: Hint,
    cell_labels: Vec<String>,
    /// Keys held down that repeat, with the time their next repeat is due.
    held: Vec<(Keysym, Instant)>,
    /// Whether anything the overlay draws has changed. Not every input produces
    /// an effect — typing a digit or a hint label character changes what is on
    /// screen and nothing else — so the driver cannot infer this from effects.
    redraw: bool,
}

impl Session {
    pub fn new(screen: (u32, u32), config: Config) -> Self {
        let mut session = Self {
            mode: Mode::Normal,
            cursor: (screen.0 as f64 / 2.0, screen.1 as f64 / 2.0),
            screen,
            config,
            normal: Normal::default(),
            grid: Grid::new(screen, 2, 2, 40.0),
            hint: Hint::default(),
            cell_labels: Vec::new(),
            held: Vec::new(),
            redraw: true,
        };
        session.reset_grid();
        session
    }

    pub fn cursor(&self) -> (f64, f64) {
        self.cursor
    }

    #[cfg(test)]
    pub fn mode(&self) -> Mode {
        self.mode
    }

    #[cfg(test)]
    pub fn hint(&self) -> &Hint {
        &self.hint
    }

    #[cfg(test)]
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// Adopt the position the compositor reports. Optimistic local updates drift
    /// when the compositor clamps a warp, so the truth is taken back whenever it
    /// is offered.
    pub fn observe_cursor(&mut self, position: (f64, f64)) {
        if self.cursor != position {
            self.cursor = position;
            self.redraw = true;
        }
    }

    /// Whether the overlay needs repainting, clearing the flag.
    pub fn take_redraw(&mut self) -> bool {
        std::mem::take(&mut self.redraw)
    }

    fn step(&self) -> f64 {
        self.config.step_for(self.screen)
    }

    fn context(&self) -> Context {
        Context {
            cursor: self.cursor,
            screen: self.screen,
            step: self.step(),
        }
    }

    pub fn on_press(&mut self, keysym: Keysym, now: Instant) -> Vec<Effect> {
        if let Some(action) = self.config.bindings.table(self.mode).get(&keysym).copied() {
            return self.act(action, now);
        }

        // Things that are not named actions still produce effects: a digit is a
        // multiplier prefix, a grid cell key selects a sub-region, and a hint
        // label character narrows the candidates. They all go through `apply`
        // like everything else, or the session's own state would not follow.
        let effects = match self.mode {
            Mode::Normal => {
                if let Some(digit) = ascii_digit(keysym) {
                    self.normal.on_digit(digit);
                    // The multiplier is shown on the overlay, so a digit the
                    // session accepted is a visual change on its own.
                    self.redraw = true;
                }
                Vec::new()
            }
            Mode::Grid => {
                // The lookup ends before `self.grid` is borrowed mutably.
                let index = self
                    .config
                    .bindings
                    .grid_cells
                    .iter()
                    .position(|key| *key == keysym);
                match index {
                    Some(index) => {
                        let context = self.context();
                        self.grid.on_cell(index, &context)
                    }
                    None => Vec::new(),
                }
            }
            Mode::Hint => match printable(keysym) {
                Some(ch) => self.hint.on_char(ch, self.config.click_after_warp),
                None => Vec::new(),
            },
        };
        self.apply(&effects);
        effects
    }

    pub fn on_release(&mut self, keysym: Keysym) {
        self.held.retain(|(held, _)| *held != keysym);
    }

    /// Effects that have come due since the last turn: held movement keys.
    pub fn tick(&mut self, now: Instant) -> Vec<Effect> {
        let mut due = Vec::new();
        for (keysym, next) in &mut self.held {
            if *next <= now {
                // Cap the schedule rather than catching up: after a stall the
                // cursor should move once, not a hundred times.
                let late = now.saturating_duration_since(*next).min(REPEAT_CEILING);
                *next = now + REPEAT_INTERVAL.saturating_sub(late);
                due.push(*keysym);
            }
        }
        let mut effects = Vec::new();
        for keysym in due {
            // Routed through `act` so the repeat reschedules itself and the
            // cursor update happens in one place.
            if let Some(action) = self.config.bindings.normal.get(&keysym).copied() {
                effects.extend(self.act(action, now));
            }
        }
        effects
    }

    /// How long until the next repeat is due, so the event loop can sleep that
    /// long instead of waking at a fixed rate.
    pub fn time_to_next_repeat(&self, now: Instant) -> Option<Duration> {
        self.held
            .iter()
            .map(|(_, next)| next.saturating_duration_since(now))
            .min()
    }

    fn act(&mut self, action: Action, now: Instant) -> Vec<Effect> {
        if action == Action::Quit {
            self.held.clear();
            return vec![Effect::Exit];
        }

        let context = self.context();
        let effects = match self.mode {
            Mode::Normal => self.normal.on_action(action, &context),
            Mode::Grid => self.grid.on_action(action, &context),
            Mode::Hint => self.hint.on_action(action),
        };

        if self.mode == Mode::Normal && is_repeatable(action) {
            let keysym = self
                .config
                .bindings
                .normal
                .iter()
                .find(|(_, bound)| **bound == action)
                .map(|(keysym, _)| *keysym);
            if let Some(keysym) = keysym {
                self.held.retain(|(held, _)| *held != keysym);
                self.held.push((keysym, now + REPEAT_DELAY));
            }
        }

        self.apply(&effects);
        effects
    }

    /// Effects the session owns are folded in here; the driver handles the
    /// rest, which is everything that touches the compositor.
    fn apply(&mut self, effects: &[Effect]) {
        self.redraw = true;
        for effect in effects {
            match effect {
                Effect::Move { dx, dy } => {
                    self.warp_cursor(self.cursor.0 + dx, self.cursor.1 + dy);
                }
                Effect::Warp { x, y } => self.warp_cursor(*x, *y),
                Effect::Switch(mode) => self.switch(*mode),
                Effect::Exit => self.held.clear(),
                _ => {}
            }
        }
    }

    fn warp_cursor(&mut self, x: f64, y: f64) {
        self.cursor = (
            x.clamp(0.0, self.screen.0 as f64),
            y.clamp(0.0, self.screen.1 as f64),
        );
    }

    /// Enter a mode directly, as a command-line flag does.
    pub fn switch_to(&mut self, mode: Mode) {
        self.switch(mode);
    }

    fn switch(&mut self, mode: Mode) {
        if mode == self.mode {
            return;
        }
        let mut effects = Vec::new();
        if self.mode == Mode::Normal {
            effects = self.normal.release_drag();
        }
        self.apply(&effects);
        self.held.clear();
        self.redraw = true;
        self.mode = mode;
        match mode {
            Mode::Grid => self.reset_grid(),
            Mode::Hint => self.reset_hint(),
            Mode::Normal => {}
        }
    }

    fn reset_grid(&mut self) {
        self.grid = Grid::new(
            self.screen,
            self.config.grid_columns,
            self.config.grid_rows,
            self.config.grid_min_span,
        );
        self.cell_labels = self
            .config
            .grid_cells()
            .map(|keys| {
                keys.iter()
                    .map(|keysym| crate::keys::key_name(*keysym))
                    .collect()
            })
            .unwrap_or_default();
    }

    fn reset_hint(&mut self) {
        let lattice = Lattice {
            cell: self.config.hint_cell,
        };
        let points = lattice.targets(self.screen);
        self.hint = Hint::new(points, &self.config.hint_charset);
    }

    pub fn frame(&self, scale: f32) -> Frame<'_> {
        let mut frame = Frame::normal(scale, self.cursor);
        frame.multiplier = (self.mode == Mode::Normal)
            .then(|| self.normal.multiplier())
            .flatten();
        frame.dragging = self.normal.dragging();
        if self.mode == Mode::Grid {
            frame.grid = Some(GridFrame {
                grid: &self.grid,
                cell_labels: &self.cell_labels,
            });
        }
        if self.mode == Mode::Hint {
            frame.hint = Some(HintFrame { hint: &self.hint });
        }
        frame
    }
}

/// Whether a key should keep firing while it is held.
fn is_repeatable(action: Action) -> bool {
    matches!(
        action,
        Action::MoveLeft
            | Action::MoveRight
            | Action::MoveUp
            | Action::MoveDown
            | Action::ScrollUp
            | Action::ScrollDown
    )
}

fn ascii_digit(keysym: Keysym) -> Option<u32> {
    let value = xkb::keysym_to_utf32(keysym);
    (0x30..=0x39).contains(&value).then(|| value - 0x30)
}

/// The character a key produces, when it produces one that could be part of a
/// label.
fn printable(keysym: Keysym) -> Option<char> {
    let text = xkb::keysym_to_utf8(keysym);
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(ch), None) if ch.is_ascii_graphic() => Some(ch),
        _ => None,
    }
}

pub fn run(start: Mode, config_path: Option<&std::path::Path>) -> Result<()> {
    // Read the configuration before connecting: a bad `--config` path should
    // fail immediately rather than after the overlay has taken the keyboard.
    let (config, warnings) = Config::load(config_path)?;
    for warning in warnings {
        eprintln!("{warning}");
    }

    watch_for_signals();
    let mut wl = Wayland::connect()?;
    wl.place_overlay()?;

    let screen = wl.state.logical_size();
    if config.grid_cells().is_none() {
        anyhow::bail!(
            "the grid partition is {}x{}, which needs {} cell keys but only {} are bound",
            config.grid_columns,
            config.grid_rows,
            config.grid_columns * config.grid_rows,
            config.bindings.grid_cells.len()
        );
    }

    let mut session = Session::new(screen, config);
    session.observe_cursor(wl.state.cursor());
    if start != Mode::Normal {
        // The mode flags hand the session over to the requested mode without
        // going through normal mode, so the mode state has to be primed.
        session.switch_to(start);
    }
    let (mut fonts, warning) = Fonts::load(&session.config.theme.font_family);
    if let Some(warning) = warning {
        eprintln!("{warning}");
    }

    redraw(&mut wl, &session, &mut fonts)?;

    let mut exit = false;
    while !exit && !wl.state.closed && !interrupted() {
        let now = Instant::now();
        let timeout = session
            .time_to_next_repeat(now)
            .unwrap_or(Duration::from_millis(200))
            .min(Duration::from_millis(200));
        wl.dispatch(Some(timeout))?;

        if let Some(position) = wl.state.take_pointer_pos() {
            session.observe_cursor(position);
        }

        let mut effects = Vec::new();
        for key in wl.take_keys() {
            if key.pressed {
                effects.extend(session.on_press(key.keysym, now));
            } else {
                session.on_release(key.keysym);
            }
        }
        effects.extend(session.tick(now));

        // One pointer move per batch, before anything that depends on where the
        // cursor ended up: a click has to land after the warp that preceded it.
        if effects
            .iter()
            .any(|effect| matches!(effect, Effect::Move { .. } | Effect::Warp { .. }))
        {
            let (x, y) = session.cursor();
            wl.move_pointer(x, y)?;
        }
        for effect in &effects {
            match *effect {
                Effect::Move { .. } | Effect::Warp { .. } => {}
                Effect::Click { button } => {
                    // The overlay has to be out of the way for the click to
                    // reach the window underneath.
                    wl.without_overlay(|wl| wl.click(button))?;
                }
                Effect::Scroll { steps } => wl.without_overlay(|wl| wl.scroll(steps))?,
                Effect::Drag(pressed) => wl.button(BTN_LEFT, pressed)?,
                Effect::Switch(_) => {}
                Effect::Exit => exit = true,
            }
        }

        if session.take_redraw() {
            redraw(&mut wl, &session, &mut fonts)?;
        }
    }

    wl.finish();
    Ok(())
}

fn redraw(wl: &mut Wayland, session: &Session, fonts: &mut Option<Fonts>) -> Result<()> {
    let scale = wl.state.pixel_scale();
    let frame = session.frame(scale);
    let theme = session.config.theme.clone();
    wl.present_with(|pixels, width, height| {
        let Some(mut canvas) = Canvas::new(pixels, width, height) else {
            return;
        };
        render::draw(&mut canvas, &theme, fonts, &frame);
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::parse_key;

    fn session() -> Session {
        Session::new((1280, 720), Config::default())
    }

    fn press(session: &mut Session, name: &str) -> Vec<Effect> {
        let keysym = parse_key(name).unwrap();
        session.on_press(keysym, Instant::now())
    }

    #[test]
    fn sessions_start_in_normal_mode_in_the_middle() {
        let session = session();
        assert_eq!(session.mode(), Mode::Normal);
        assert_eq!(session.cursor(), (640.0, 360.0));
    }

    #[test]
    fn a_move_updates_the_cursor_by_one_step() {
        let mut session = session();
        press(&mut session, "l");
        assert_eq!(session.cursor(), (660.0, 360.0));
        press(&mut session, "k");
        assert_eq!(session.cursor(), (660.0, 340.0));
    }

    #[test]
    fn the_cursor_stops_at_the_edges_instead_of_wrapping() {
        let mut session = session();
        for _ in 0..200 {
            press(&mut session, "l");
        }
        assert_eq!(session.cursor(), (1280.0, 360.0));
        for _ in 0..200 {
            press(&mut session, "j");
        }
        assert_eq!(session.cursor(), (1280.0, 720.0));
        for _ in 0..400 {
            press(&mut session, "h");
        }
        assert_eq!(session.cursor(), (0.0, 720.0));
        for _ in 0..400 {
            press(&mut session, "k");
        }
        assert_eq!(session.cursor(), (0.0, 0.0));
    }

    #[test]
    fn edge_jumps_land_exactly_on_the_edges() {
        let mut session = session();
        press(&mut session, "l");
        press(&mut session, "H");
        assert_eq!(session.cursor(), (0.0, 360.0));
        press(&mut session, "M");
        assert_eq!(session.cursor(), (640.0, 360.0));
        press(&mut session, "L");
        assert_eq!(session.cursor(), (1280.0, 360.0));
    }

    #[test]
    fn digits_prefix_a_single_move() {
        let mut session = session();
        press(&mut session, "1");
        press(&mut session, "0");
        assert_eq!(session.frame(1.0).multiplier, Some(10));
        press(&mut session, "l");
        assert_eq!(session.cursor(), (840.0, 360.0));
        assert_eq!(session.frame(1.0).multiplier, None);
        press(&mut session, "l");
        assert_eq!(session.cursor(), (860.0, 360.0));
    }

    #[test]
    fn switching_to_grid_resets_it_to_the_whole_output() {
        let mut session = session();
        press(&mut session, "g");
        assert_eq!(session.mode(), Mode::Grid);
        assert_eq!(session.grid().region().w, 1280.0);
        press(&mut session, "w");
        assert!(session.grid().region().w < 1280.0);
        press(&mut session, "Escape");
    }

    #[test]
    fn grid_cell_keys_move_the_cursor_to_the_cell_centre() {
        let mut session = session();
        press(&mut session, "g");
        press(&mut session, "s");
        assert_eq!(session.cursor(), (960.0, 540.0));
    }

    #[test]
    fn warping_out_of_grid_returns_to_normal_mode() {
        let mut session = session();
        press(&mut session, "g");
        press(&mut session, "w");
        let effects = press(&mut session, "Return");
        assert_eq!(session.mode(), Mode::Normal);
        assert!(effects.contains(&Effect::Switch(Mode::Normal)));
    }

    #[test]
    fn hint_labels_filter_and_warp_on_a_unique_match() {
        let mut session = session();
        press(&mut session, "x");
        assert_eq!(session.mode(), Mode::Hint);
        assert!(session.hint().target_count() > 3);
        let label: Vec<char> = {
            let visible = session.hint().visible();
            visible[0].0.label.chars().collect()
        };
        for (index, ch) in label.iter().enumerate() {
            let effects = press(&mut session, &ch.to_string());
            if index + 1 == label.len() {
                assert!(effects.contains(&Effect::Switch(Mode::Normal)));
            }
        }
        assert_eq!(session.mode(), Mode::Normal);
    }

    #[test]
    fn an_unmatched_hint_key_leaves_the_mode_alone() {
        let mut session = session();
        press(&mut session, "x");
        // `z` is not in the label charset.
        press(&mut session, "z");
        assert_eq!(session.mode(), Mode::Hint);
        assert!(session.hint().typed().is_empty());
    }

    #[test]
    fn backspace_with_nothing_typed_leaves_hint_mode() {
        let mut session = session();
        press(&mut session, "x");
        press(&mut session, "BackSpace");
        assert_eq!(session.mode(), Mode::Normal);
    }

    #[test]
    fn a_held_movement_key_repeats_after_the_delay() {
        let mut session = session();
        let start = Instant::now();
        let keysym = parse_key("l").unwrap();
        session.on_press(keysym, start);
        assert_eq!(session.cursor(), (660.0, 360.0));

        assert!(session.tick(start).is_empty(), "repeated before the delay");
        let later = start + REPEAT_DELAY + Duration::from_millis(1);
        assert_eq!(
            session.tick(later),
            vec![Effect::Move { dx: 20.0, dy: 0.0 }]
        );
        assert_eq!(session.cursor(), (680.0, 360.0));

        session.on_release(keysym);
        assert!(
            session.tick(later + Duration::from_secs(1)).is_empty(),
            "repeated after the key was released"
        );
    }

    #[test]
    fn grid_and_hint_keys_do_not_repeat() {
        let mut session = session();
        let start = Instant::now();
        press(&mut session, "g");
        let keysym = parse_key("w").unwrap();
        session.on_press(keysym, start);
        let later = start + REPEAT_DELAY + Duration::from_millis(1);
        assert!(session.tick(later).is_empty());
    }

    #[test]
    fn the_next_repeat_is_scheduled_only_while_a_key_is_held() {
        let mut session = session();
        let start = Instant::now();
        assert_eq!(session.time_to_next_repeat(start), None);
        session.on_press(parse_key("j").unwrap(), start);
        assert_eq!(session.time_to_next_repeat(start), Some(REPEAT_DELAY));
        session.on_release(parse_key("j").unwrap());
        assert_eq!(session.time_to_next_repeat(start), None);
    }

    #[test]
    fn every_visible_change_asks_for_a_redraw() {
        let mut session = session();
        assert!(session.take_redraw(), "the first frame has to be drawn");
        assert!(!session.take_redraw());

        // A digit only shows up in the status pill, and produces no effect.
        assert!(press(&mut session, "5").is_empty());
        assert!(session.take_redraw(), "the multiplier is not shown");
        assert!(!session.take_redraw());

        // A hint label character only narrows the candidates.
        press(&mut session, "x");
        assert!(session.take_redraw(), "entering hint mode drew nothing");
        assert!(press(&mut session, "a").is_empty());
        assert!(session.take_redraw(), "the filtered labels are not shown");

        // And a plain move, which does produce an effect.
        press(&mut session, "Escape");
        session.take_redraw();
        press(&mut session, "l");
        assert!(session.take_redraw());
    }

    #[test]
    fn a_repeated_cursor_report_does_not_ask_for_a_redraw() {
        let mut session = session();
        session.observe_cursor((10.0, 20.0));
        session.take_redraw();
        session.observe_cursor((10.0, 20.0));
        assert!(!session.take_redraw());
        session.observe_cursor((11.0, 20.0));
        assert!(session.take_redraw());
    }

    #[test]
    fn observed_positions_override_the_local_guess() {
        let mut session = session();
        session.observe_cursor((10.0, 20.0));
        assert_eq!(session.cursor(), (10.0, 20.0));
        press(&mut session, "l");
        assert_eq!(session.cursor(), (30.0, 20.0));
    }

    #[test]
    fn quit_is_reported_from_every_mode() {
        for start in [Mode::Normal, Mode::Grid, Mode::Hint] {
            let mut session = session();
            session.switch_to(start);
            assert_eq!(press(&mut session, "Escape"), vec![Effect::Exit]);
        }
    }
}
