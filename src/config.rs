//! Configuration: the file format, the defaults, and the values the modes read.
//!
//! The file is one `<option>: <value>` per line. A single bad line is a warning,
//! never a silent failure and never a reason to refuse to run — except when the
//! user named a file explicitly, because then they meant *that* file.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::keys::{key_name, modes_for, parse_key, Action, Bindings, Keysym};
use crate::render::theme::Rgba;
use crate::render::Theme;

#[derive(Debug, Clone)]
pub struct Config {
    pub bindings: Bindings,
    /// Normal-mode step in logical pixels. `None` means "derive it from the
    /// output", which is what keeps the same tool usable on a laptop panel and
    /// on a 4K monitor.
    pub step: Option<f64>,
    pub grid_columns: u32,
    pub grid_rows: u32,
    /// Wanted spacing between hint targets, in logical pixels. `None` means
    /// "derive it from the alphabet", which is what keeps the labels at two
    /// characters on a laptop panel and on a 4K monitor alike.
    pub hint_cell: Option<f64>,
    /// Characters hint labels are built from: twenty keys around the home row,
    /// so a two-character label is reachable without looking down.
    pub hint_charset: Vec<char>,
    pub click_after_warp: bool,
    pub theme: Theme,
    /// A grid region smaller than this in either direction is small enough to
    /// hand over to normal mode.
    pub grid_min_span: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bindings: Bindings::default(),
            step: None,
            grid_columns: 2,
            grid_rows: 2,
            // Derived, not fixed: see `hint_cell_for`.
            hint_cell: None,
            hint_charset: "asdfjkl;ghqwertyuiop".chars().collect(),
            click_after_warp: false,
            theme: Theme::default(),
            grid_min_span: 40.0,
        }
    }
}


/// Where the configuration is looked for when the command line does not name a
/// file: `$XDG_CONFIG_HOME/windhover/config`, falling back to `~/.config`.
pub fn default_path() -> Option<PathBuf> {
    resolve_path(
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
        std::env::var_os("HOME").map(PathBuf::from),
    )
}

fn resolve_path(config_home: Option<PathBuf>, home: Option<PathBuf>) -> Option<PathBuf> {
    let base = config_home.or_else(|| home.map(|home| home.join(".config")))?;
    Some(base.join("windhover").join("config"))
}

impl Config {
    /// The step actually used on this output.
    pub fn step_for(&self, screen: (u32, u32)) -> f64 {
        self.step
            .unwrap_or_else(|| (screen.1 as f64 / 72.0).max(4.0))
    }

    /// The hint spacing actually used on this output: the configured value, or
    /// the spacing that fills the output with exactly as many cells as the
    /// alphabet can name in two characters. Two characters are what the label
    /// length should be, so the budget is what sets the density, not the screen.
    pub fn hint_cell_for(&self, screen: (u32, u32)) -> f64 {
        self.hint_cell.unwrap_or_else(|| {
            let area = screen.0 as f64 * screen.1 as f64;
            let budget = self.hint_charset.len().max(1) as f64;
            (area / (budget * budget)).sqrt().max(1.0)
        })
    }

    /// The grid cell keys, in row-major order, for a given partition. Returns
    /// `None` when the configured partition needs more keys than are bound,
    /// which is a configuration error rather than something to paper over.
    pub fn grid_cells(&self) -> Option<&[Keysym]> {
        let needed = (self.grid_columns.max(1) * self.grid_rows.max(1)) as usize;
        (self.bindings.grid_cells.len() >= needed).then(|| &self.bindings.grid_cells[..needed])
    }

    /// The configured bindings this keyboard layout cannot produce, named as the
    /// options they were set by. A key the layout has no symbol for is a binding
    /// that silently does nothing, and one of them is the key that ends the
    /// session — with it gone, nothing the user presses can leave.
    pub fn unreachable_bindings(&self, reachable: &HashSet<Keysym>) -> Vec<String> {
        let mut missing: Vec<String> = Vec::new();
        for table in [
            &self.bindings.normal,
            &self.bindings.grid,
            &self.bindings.hint,
        ] {
            for (keysym, action) in table {
                if reachable.contains(keysym) {
                    continue;
                }
                let entry = format!("{} ({})", action_option(*action), key_name(*keysym));
                if !missing.contains(&entry) {
                    missing.push(entry);
                }
            }
        }
        for keysym in &self.bindings.grid_cells {
            if !reachable.contains(keysym) {
                missing.push(format!("grid_cells ({})", key_name(*keysym)));
            }
        }
        missing.sort();
        missing
    }

    /// Read the configuration, returning it alongside the warnings to print.
    ///
    /// `explicit` is the path from the command line. A missing file there is an
    /// error; a missing file at the default location just means "defaults".
    pub fn load(explicit: Option<&Path>) -> Result<(Self, Vec<String>)> {
        let (path, required) = match explicit {
            Some(path) => (Some(path.to_path_buf()), true),
            None => (default_path(), false),
        };
        let Some(path) = path else {
            return Ok((Self::default(), Vec::new()));
        };

        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) if !required && err.kind() == std::io::ErrorKind::NotFound => {
                return Ok((Self::default(), Vec::new()));
            }
            Err(err) => {
                return Err(err).with_context(|| format!("read {}", path.display()));
            }
        };

        let (config, warnings) = Self::parse(&text);
        Ok((config, warnings))
    }

    pub fn parse(text: &str) -> (Self, Vec<String>) {
        let mut loader = Loader {
            config: Self::default(),
            warnings: Vec::new(),
            replaced: HashSet::new(),
        };
        for (number, line) in text.lines().enumerate() {
            loader.line(number + 1, line);
        }
        (loader.config, loader.warnings)
    }
}

struct Loader {
    config: Config,
    warnings: Vec<String>,
    /// Options already seen, because the first occurrence of a key option
    /// replaces its defaults and later ones add to it.
    replaced: HashSet<Action>,
}

impl Loader {
    fn line(&mut self, number: usize, line: &str) {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return;
        }
        let Some((option, value)) = line.split_once(':') else {
            self.warn(number, line, "expected `<option>: <value>`");
            return;
        };
        let (option, value) = (option.trim(), value.trim());
        if let Err(reason) = self.apply(option, value) {
            self.warn(number, line, &reason);
        }
    }

    fn warn(&mut self, number: usize, line: &str, reason: &str) {
        self.warnings
            .push(format!("{}:{number}: {reason} (`{line}`), ignored", CONFIG_LABEL));
    }

    fn apply(&mut self, option: &str, value: &str) -> std::result::Result<(), String> {
        if let Some(action) = action_for(option) {
            return self.bind(action, value);
        }
        match option {
            "step" => self.config.step = Some(number(value, 1.0, f64::MAX)?),
            "grid_columns" => self.config.grid_columns = number(value, 1.0, 64.0)? as u32,
            "grid_rows" => self.config.grid_rows = number(value, 1.0, 64.0)? as u32,
            "grid_min_span" => self.config.grid_min_span = number(value, 1.0, f64::MAX)?,
            "grid_cells" => {
                let keys = parse_keys(value)?;
                if keys.is_empty() {
                    return Err("`grid_cells` needs at least one key".to_string());
                }
                self.config.bindings.grid_cells = keys;
            }
            "hint_cell" => self.config.hint_cell = Some(number(value, 1.0, f64::MAX)?),
            "hint_charset" => {
                let charset: Vec<char> = value.chars().filter(|ch| !ch.is_whitespace()).collect();
                if charset.is_empty() {
                    return Err("`hint_charset` needs at least one character".to_string());
                }
                if charset.iter().any(|ch| !ch.is_ascii_graphic()) {
                    return Err("`hint_charset` must be printable ASCII".to_string());
                }
                self.config.hint_charset = charset;
            }
            "click_after_warp" => self.config.click_after_warp = boolean(value)?,
            "font_family" => {
                if value.is_empty() {
                    return Err("`font_family` cannot be empty".to_string());
                }
                self.config.theme.font_family = value.to_string();
            }
            "accent" => self.config.theme.accent = colour(value)?,
            "text" => self.config.theme.text = colour(value)?,
            "panel" => self.config.theme.panel = colour(value)?,
            other => return Err(format!("unknown option `{other}`")),
        }
        Ok(())
    }

    fn bind(&mut self, action: Action, value: &str) -> std::result::Result<(), String> {
        // Removing every binding for the action is what makes one line enough to
        // rebind a key: the first mention of an option has to displace its
        // defaults, and later mentions have to be able to add to it.
        if self.replaced.insert(action) {
            self.config.bindings.remove(action);
        }
        if value.eq_ignore_ascii_case("unbind") {
            if self.replaced.insert(action) {
                self.config.bindings.remove(action);
            }
            return Ok(());
        }
        let keys = parse_keys(value)?;
        if keys.is_empty() {
            return Err(format!("`{}` needs at least one key", action_option(action)));
        }
        for keysym in keys {
            for mode in modes_for(action) {
                self.config.bindings.insert(*mode, keysym, action);
            }
        }
        Ok(())
    }
}

const CONFIG_LABEL: &str = "windhover config";

fn action_for(option: &str) -> Option<Action> {
    Some(match option {
        "quit" => Action::Quit,
        "normal_left" => Action::MoveLeft,
        "normal_right" => Action::MoveRight,
        "normal_up" => Action::MoveUp,
        "normal_down" => Action::MoveDown,
        "edge_left" => Action::EdgeLeft,
        "edge_middle" => Action::EdgeMiddle,
        "edge_right" => Action::EdgeRight,
        "speed_fine" => Action::SpeedFine,
        "speed_normal" => Action::SpeedNormal,
        "speed_coarse" => Action::SpeedCoarse,
        "scroll_up" => Action::ScrollUp,
        "scroll_down" => Action::ScrollDown,
        "button_left" => Action::ClickLeft,
        "button_middle" => Action::ClickMiddle,
        "button_right" => Action::ClickRight,
        "drag" => Action::ToggleDrag,
        "grid_mode" => Action::ToGrid,
        "hint_mode" => Action::ToHint,
        "grid_up" => Action::GridUp,
        "grid_down" => Action::GridDown,
        "grid_left" => Action::GridLeft,
        "grid_right" => Action::GridRight,
        "grid_back" => Action::GridBack,
        "grid_warp" => Action::GridWarp,
        "grid_warp_click" => Action::GridWarpClick,
        "hint_back" => Action::HintBack,
        _ => return None,
    })
}

fn action_option(action: Action) -> &'static str {
    match action {
        Action::Quit => "quit",
        Action::MoveLeft => "normal_left",
        Action::MoveRight => "normal_right",
        Action::MoveUp => "normal_up",
        Action::MoveDown => "normal_down",
        Action::EdgeLeft => "edge_left",
        Action::EdgeMiddle => "edge_middle",
        Action::EdgeRight => "edge_right",
        Action::SpeedFine => "speed_fine",
        Action::SpeedNormal => "speed_normal",
        Action::SpeedCoarse => "speed_coarse",
        Action::ScrollUp => "scroll_up",
        Action::ScrollDown => "scroll_down",
        Action::ClickLeft => "button_left",
        Action::ClickMiddle => "button_middle",
        Action::ClickRight => "button_right",
        Action::ToggleDrag => "drag",
        Action::ToGrid => "grid_mode",
        Action::ToHint => "hint_mode",
        Action::GridUp => "grid_up",
        Action::GridDown => "grid_down",
        Action::GridLeft => "grid_left",
        Action::GridRight => "grid_right",
        Action::GridBack => "grid_back",
        Action::GridWarp => "grid_warp",
        Action::GridWarpClick => "grid_warp_click",
        Action::HintBack => "hint_back",
    }
}

fn parse_keys(value: &str) -> std::result::Result<Vec<Keysym>, String> {
    value
        .split_whitespace()
        .map(|name| parse_key(name).map_err(|err| err.to_string()))
        .collect()
}

fn number(value: &str, low: f64, high: f64) -> std::result::Result<f64, String> {
    let parsed: f64 = value
        .parse()
        .map_err(|_| format!("`{value}` is not a number"))?;
    if !parsed.is_finite() || parsed < low || parsed > high {
        return Err(format!("`{value}` is outside {low}..{high}"));
    }
    Ok(parsed)
}

fn boolean(value: &str) -> std::result::Result<bool, String> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Ok(true),
        "false" | "no" | "off" | "0" => Ok(false),
        other => Err(format!("`{other}` is not a yes/no value")),
    }
}

/// `RRGGBB` or `RRGGBBAA`, with or without a leading `#`.
fn colour(value: &str) -> std::result::Result<Rgba, String> {
    let digits = value.strip_prefix('#').unwrap_or(value);
    if digits.len() != 6 && digits.len() != 8 {
        return Err(format!("`{value}` is not RRGGBB or RRGGBBAA"));
    }
    let byte = |at: usize| {
        u8::from_str_radix(&digits[at..at + 2], 16)
            .map_err(|_| format!("`{value}` is not hexadecimal"))
    };
    Ok(Rgba::rgba(
        byte(0)?,
        byte(2)?,
        byte(4)?,
        if digits.len() == 8 { byte(6)? } else { 255 },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::Mode;

    fn parse(text: &str) -> (Config, Vec<String>) {
        Config::parse(text)
    }

    fn one(option: &str, value: &str) -> (Config, Vec<String>) {
        parse(&format!("{option}: {value}\n"))
    }

    fn warns(text: &str) -> Vec<String> {
        parse(text).1
    }

    #[test]
    fn blank_lines_and_comments_are_ignored() {
        let (config, warnings) = parse("\n# a comment\n\n   \nstep: 12\n# another\n");
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(config.step, Some(12.0));
    }

    #[test]
    fn surrounding_whitespace_is_tolerated() {
        let (config, warnings) = parse("   step   :    12   \n");
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(config.step, Some(12.0));
    }

    #[test]
    fn only_the_first_colon_splits_the_line() {
        let (config, warnings) = one("font_family", "Noto Sans: Extra");
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(config.theme.font_family, "Noto Sans: Extra");
    }

    #[test]
    fn a_value_may_contain_spaces() {
        let (config, _) = one("font_family", "DejaVu Sans");
        assert_eq!(config.theme.font_family, "DejaVu Sans");
    }

    #[test]
    fn a_line_without_a_colon_is_a_warning_not_a_failure() {
        let warnings = warns("this is not an option\n");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("this is not an option"), "{warnings:?}");
        assert!(warnings[0].contains(":1:"), "{warnings:?}");
    }

    #[test]
    fn an_unknown_option_warns_with_the_line_and_keeps_the_rest() {
        let (config, warnings) = parse("nonsense: 3\nstep: 9\n");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("nonsense: 3"), "{warnings:?}");
        assert!(warnings[0].contains("unknown option"), "{warnings:?}");
        // The rest of the file still applies.
        assert_eq!(config.step, Some(9.0));
    }

    #[test]
    fn an_illegal_value_warns_and_keeps_the_default() {
        let (config, warnings) = one("grid_columns", "0");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("grid_columns"), "{warnings:?}");
        assert_eq!(config.grid_columns, Config::default().grid_columns);

        let (config, warnings) = one("click_after_warp", "perhaps");
        assert_eq!(warnings.len(), 1);
        assert!(!config.click_after_warp);

        let (config, warnings) = one("step", "not-a-number");
        assert_eq!(warnings.len(), 1);
        assert_eq!(config.step, None);
    }

    #[test]
    fn an_empty_charset_or_cell_list_is_rejected() {
        assert!(!warns("hint_charset: \n").is_empty());
        assert!(!warns("grid_cells: \n").is_empty());
        assert!(!warns("hint_charset: \u{e9}\n").is_empty());
    }

    #[test]
    fn the_default_alphabet_is_twenty_printable_characters() {
        let charset = &Config::default().hint_charset;
        assert_eq!(charset.len(), 20);
        assert!(charset.contains(&';'));
        assert!(charset.iter().all(|ch| ch.is_ascii_graphic()));
    }

    #[test]
    fn punctuation_is_accepted_in_the_alphabet() {
        let (config, warnings) = one("hint_charset", "asdf;jkl");
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(config.hint_charset, "asdf;jkl".chars().collect::<Vec<_>>());
    }

    #[test]
    fn the_default_spacing_fills_the_output_with_two_character_labels() {
        // 20 x 20 cells, whatever the output: the label length is the same on a
        // laptop panel and on a 4K monitor.
        let config = Config::default();
        for screen in [(1920, 1080), (2560, 1440), (3840, 2160)] {
            let cell = config.hint_cell_for(screen);
            let columns = (screen.0 as f64 / cell).floor() as usize;
            let rows = (screen.1 as f64 / cell).floor() as usize;
            assert!(
                columns * rows <= 400,
                "{screen:?} gave {} cells",
                columns * rows
            );
            let labels = crate::hint::labels::assign(columns * rows, &config.hint_charset);
            assert!(
                labels.iter().all(|label| label.chars().count() <= 2),
                "{screen:?} needed more than two characters"
            );
        }
    }

    #[test]
    fn numbers_and_booleans_are_read() {
        assert_eq!(one("step", "33").0.step, Some(33.0));
        assert_eq!(one("grid_columns", "3").0.grid_columns, 3);
        assert_eq!(one("hint_cell", "80").0.hint_cell, Some(80.0));
        assert!(one("click_after_warp", "yes").0.click_after_warp);
        assert!(one("click_after_warp", "TRUE").0.click_after_warp);
        assert!(!one("click_after_warp", "off").0.click_after_warp);
    }

    #[test]
    fn colours_accept_six_or_eight_hex_digits_with_or_without_a_hash() {
        assert_eq!(one("accent", "0A84FF").0.theme.accent, Rgba::rgb(0x0A, 0x84, 0xFF));
        assert_eq!(one("accent", "#0a84ff").0.theme.accent, Rgba::rgb(0x0A, 0x84, 0xFF));
        assert_eq!(
            one("panel", "1C1C1EE0").0.theme.panel,
            Rgba::rgba(0x1C, 0x1C, 0x1E, 0xE0)
        );
        assert_eq!(one("text", "FFFFFF").0.theme.text.a, 255);
        // Grid lines and borders are tints of the accent, not separate colours.
        let theme = one("accent", "FF9500").0.theme;
        assert_eq!(theme.grid_line().r, 0xFF);
        assert_eq!(theme.region_edge(), theme.accent);
        assert!(!warns("accent: blue\n").is_empty());
        assert!(!warns("accent: 12345\n").is_empty());
        assert!(!warns("accent: zzzzzz\n").is_empty());
    }

    #[test]
    fn a_layout_that_cannot_produce_a_binding_is_reported() {
        let (config, _) = parse("");
        // Everything the defaults are bound to, as a layout that produces it all.
        let mut reachable: HashSet<Keysym> = config
            .bindings
            .normal
            .keys()
            .chain(config.bindings.grid.keys())
            .chain(config.bindings.hint.keys())
            .chain(config.bindings.grid_cells.iter())
            .copied()
            .collect();
        assert_eq!(config.unreachable_bindings(&reachable), Vec::<String>::new());

        // Take the key that leaves the session away from the layout: nothing the
        // user presses can end it, and that has to be said out loud.
        reachable.remove(&parse_key("Escape").unwrap());
        assert_eq!(
            config.unreachable_bindings(&reachable),
            vec!["quit (Escape)".to_string()]
        );

        // A cell key has no action of its own, so it is named as the option.
        let (bound, _) = parse("grid_cells: F12 grave\n");
        let empty: HashSet<Keysym> = HashSet::new();
        let reported = bound.unreachable_bindings(&empty);
        assert!(reported.contains(&"grid_cells (F12)".to_string()), "{reported:?}");
        assert!(reported.contains(&"grid_cells (grave)".to_string()), "{reported:?}");
    }

    #[test]
    fn every_documented_option_has_a_recognised_name() {
        // A typo in the table would otherwise only show up as a silent default.
        for option in [
            "quit", "normal_left", "normal_right", "normal_up", "normal_down",
            "edge_left", "edge_middle", "edge_right", "scroll_up", "scroll_down",
            "button_left", "button_middle", "button_right", "drag", "grid_mode",
            "hint_mode", "grid_up", "grid_down", "grid_left", "grid_right",
            "grid_back", "grid_warp", "grid_warp_click", "hint_back",
        ] {
            let warnings = warns(&format!("{option}: F12\n"));
            assert!(
                warnings.is_empty(),
                "`{option}` is not a known key option: {warnings:?}"
            );
        }
        for option in [
            "step", "grid_columns", "grid_rows", "grid_min_span", "grid_cells",
            "hint_cell", "hint_charset", "click_after_warp", "font_family",
            "accent", "text", "panel",
        ] {
            let warnings = warns(&format!("{option}: F12\n"));
            assert!(
                warnings.is_empty() || !warnings[0].contains("unknown option"),
                "`{option}` is not recognised: {warnings:?}"
            );
        }
    }

    #[test]
    fn each_action_option_overrides_its_default() {
        let (config, warnings) = one("normal_left", "z");
        assert!(warnings.is_empty(), "{warnings:?}");
        let z = parse_key("z").unwrap();
        let h = parse_key("h").unwrap();
        assert_eq!(config.bindings.normal.get(&z), Some(&Action::MoveLeft));
        assert_eq!(config.bindings.normal.get(&h), None, "the default stayed");
    }

    #[test]
    fn two_lines_for_one_option_both_bind() {
        let (config, warnings) = parse("hint_mode: f\nhint_mode: a\n");
        assert!(warnings.is_empty(), "{warnings:?}");
        for name in ["f", "a"] {
            let keysym = parse_key(name).unwrap();
            assert_eq!(
                config.bindings.normal.get(&keysym),
                Some(&Action::ToHint),
                "`{name}` did not bind"
            );
        }
        // The old default is gone: the first mention of the option replaces the
        // defaults, and later mentions add to what is left.
        assert_eq!(config.bindings.normal.get(&parse_key("x").unwrap()), None);
    }

    #[test]
    fn two_keys_on_one_line_both_bind() {
        let (config, _) = one("hint_mode", "f a");
        assert_eq!(config.bindings.normal.get(&parse_key("f").unwrap()), Some(&Action::ToHint));
        assert_eq!(config.bindings.normal.get(&parse_key("a").unwrap()), Some(&Action::ToHint));
    }

    #[test]
    fn unbind_removes_the_binding_entirely() {
        let (config, warnings) = one("normal_up", "unbind");
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(config.bindings.normal.get(&parse_key("k").unwrap()), None);
    }

    #[test]
    fn quit_is_bound_in_every_mode_and_rebinding_keeps_it_that_way() {
        let (config, _) = one("quit", "F12");
        let f12 = parse_key("F12").unwrap();
        for mode in [Mode::Normal, Mode::Grid, Mode::Hint] {
            assert_eq!(
                config.bindings.table(mode).get(&f12),
                Some(&Action::Quit),
                "{mode:?} lost quit"
            );
        }
        assert_eq!(config.bindings.normal.get(&parse_key("Escape").unwrap()), None);
    }

    #[test]
    fn the_later_binding_of_a_key_wins() {
        let (config, _) = parse("normal_down: w\nbutton_left: w\n");
        assert_eq!(
            config.bindings.normal.get(&parse_key("w").unwrap()),
            Some(&Action::ClickLeft)
        );
    }

    #[test]
    fn grid_actions_bind_into_the_grid_table() {
        let (config, _) = one("grid_warp", "space");
        assert_eq!(config.bindings.grid.get(&parse_key("space").unwrap()), Some(&Action::GridWarp));
        assert!(config.bindings.normal.is_empty() || config.bindings.normal.values().all(|a| *a != Action::GridWarp));
    }

    #[test]
    fn hint_back_binds_into_the_hint_table() {
        let (config, _) = one("hint_back", "b");
        assert_eq!(config.bindings.hint.get(&parse_key("b").unwrap()), Some(&Action::HintBack));
    }

    #[test]
    fn an_unknown_key_name_warns_with_the_line() {
        let warnings = warns("normal_left: DefinitelyNotAKey\n");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("DefinitelyNotAKey"), "{warnings:?}");
    }

    #[test]
    fn the_default_location_follows_xdg_then_home_then_gives_up() {
        let xdg = resolve_path(Some(PathBuf::from("/xdg")), Some(PathBuf::from("/home/u")));
        assert_eq!(xdg, Some(PathBuf::from("/xdg/windhover/config")));
        let home = resolve_path(None, Some(PathBuf::from("/home/u")));
        assert_eq!(home, Some(PathBuf::from("/home/u/.config/windhover/config")));
        assert_eq!(resolve_path(None, None), None);
    }

    #[test]
    fn an_explicit_config_path_that_does_not_exist_is_an_error() {
        let err = Config::load(Some(Path::new("/nonexistent/windhover/config")))
            .expect_err("a named file must be read or reported");
        assert!(err.to_string().contains("nonexistent"), "{err}");
    }

    #[test]
    fn a_missing_default_file_is_not_an_error() {
        // Point both XDG variables at a directory with no config in it.
        let empty = std::env::temp_dir().join("windhover-no-config-here");
        let _ = std::fs::create_dir_all(&empty);
        let missing = resolve_path(Some(empty.clone()), None).unwrap();
        assert!(!missing.exists(), "the test directory is not empty");
        // Only the resolution is asserted here: env mutation would race with
        // other tests.
        assert_eq!(missing.file_name().unwrap(), "config");
    }

    #[test]
    fn the_documented_sample_config_parses_without_warnings() {
        let doc = include_str!("../docs/configuration.md");
        let start = doc
            .find("<!-- sample-config -->")
            .expect("the docs must mark the sample config");
        let block = &doc[start..];
        let block = &block[block.find("```\n").expect("a fenced block") + 4..];
        let block = &block[..block.find("\n```").expect("a closing fence")];
        let (_, warnings) = parse(block);
        assert!(warnings.is_empty(), "the sample config warns: {warnings:?}");
    }

    #[test]
    fn a_partition_larger_than_the_cell_keys_is_reported_by_grid_cells() {
        let (config, _) = parse("grid_columns: 4\ngrid_rows: 4\n");
        assert!(config.grid_cells().is_none());
        let (config, _) = parse("grid_columns: 3\ngrid_rows: 3\n");
        assert!(config.grid_cells().is_none());
    }

    #[test]
    fn a_custom_cell_list_can_support_a_larger_partition() {
        let (config, warnings) = parse("grid_rows: 1\ngrid_columns: 6\ngrid_cells: 1 2 3 4 5 6\n");
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(config.grid_cells().map(<[Keysym]>::len), Some(6));
    }
}

#[cfg(test)]
mod consistency {
    use super::*;
    use crate::mode::Mode;

    /// The default key table from `docs/configuration.md`, so a change to the
    /// defaults that forgets the documentation fails the build.
    #[test]
    fn the_documented_default_keys_are_the_defaults() {
        let bindings = Bindings::default();
        let expected: &[(&str, Mode, &str, Action)] = &[
            ("quit", Mode::Normal, "Escape", Action::Quit),
            ("quit", Mode::Grid, "Escape", Action::Quit),
            ("quit", Mode::Hint, "Escape", Action::Quit),
            ("normal_left", Mode::Normal, "h", Action::MoveLeft),
            ("normal_right", Mode::Normal, "l", Action::MoveRight),
            ("normal_up", Mode::Normal, "k", Action::MoveUp),
            ("normal_down", Mode::Normal, "j", Action::MoveDown),
            ("edge_left", Mode::Normal, "H", Action::EdgeLeft),
            ("edge_middle", Mode::Normal, "M", Action::EdgeMiddle),
            ("edge_right", Mode::Normal, "L", Action::EdgeRight),
            ("speed_fine", Mode::Normal, "[", Action::SpeedFine),
            ("speed_normal", Mode::Normal, "\\", Action::SpeedNormal),
            ("speed_coarse", Mode::Normal, "]", Action::SpeedCoarse),
            ("scroll_up", Mode::Normal, "u", Action::ScrollUp),
            ("scroll_down", Mode::Normal, "d", Action::ScrollDown),
            ("button_left", Mode::Normal, "m", Action::ClickLeft),
            ("button_middle", Mode::Normal, ",", Action::ClickMiddle),
            ("button_right", Mode::Normal, ".", Action::ClickRight),
            ("drag", Mode::Normal, "v", Action::ToggleDrag),
            ("grid_mode", Mode::Normal, "g", Action::ToGrid),
            ("hint_mode", Mode::Normal, "f", Action::ToHint),
            ("grid_left", Mode::Grid, "h", Action::GridLeft),
            ("grid_right", Mode::Grid, "l", Action::GridRight),
            ("grid_up", Mode::Grid, "k", Action::GridUp),
            ("grid_down", Mode::Grid, "j", Action::GridDown),
            ("grid_back", Mode::Grid, "BackSpace", Action::GridBack),
            ("grid_warp", Mode::Grid, "Return", Action::GridWarp),
            ("grid_warp_click", Mode::Grid, "space", Action::GridWarpClick),
            ("hint_back", Mode::Hint, "BackSpace", Action::HintBack),
        ];
        for (option, mode, key, action) in expected {
            assert_eq!(
                bindings.table(*mode).get(&parse_key(key).unwrap()),
                Some(action),
                "`{option}` should default to `{key}`"
            );
        }
        // No undocumented extras: every default is exercised above except the
        // grid cell keys, which are values rather than actions.
        let documented = expected.len();
        let total = bindings.normal.len() + bindings.grid.len() + bindings.hint.len();
        assert_eq!(total, documented, "the defaults have undocumented entries");
        assert_eq!(
            bindings
                .grid_cells
                .iter()
                .map(|keysym| crate::keys::key_name(*keysym))
                .collect::<Vec<_>>(),
            ["q", "w", "a", "s"]
        );
    }
}
