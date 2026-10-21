//! Key symbol names to actions.

use std::collections::HashMap;

use anyhow::{anyhow, Result};
use xkbcommon::xkb;

pub type Keysym = xkb::Keysym;

/// A named action a key can be bound to. Each mode keeps its own table, so an
/// action never has to carry the mode it belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    EdgeLeft,
    EdgeMiddle,
    EdgeRight,
    SpeedFine,
    SpeedNormal,
    SpeedCoarse,
    ScrollUp,
    ScrollDown,
    ClickLeft,
    ClickMiddle,
    ClickRight,
    ToggleDrag,
    ToGrid,
    ToHint,
    GridUp,
    GridDown,
    GridLeft,
    GridRight,
    GridBack,
    GridWarp,
    GridWarpClick,
    HintBack,
    Quit,
}

/// Which mode tables an action is bound in.
pub fn modes_for(action: Action) -> &'static [crate::mode::Mode] {
    use crate::mode::Mode;
    match action {
        // Quit is bound in every mode so Escape always works.
        Action::Quit => &[Mode::Normal, Mode::Grid, Mode::Hint],
        Action::GridUp
        | Action::GridDown
        | Action::GridLeft
        | Action::GridRight
        | Action::GridBack
        | Action::GridWarp
        | Action::GridWarpClick => &[Mode::Grid],
        Action::HintBack => &[Mode::Hint],
        _ => &[Mode::Normal],
    }
}

/// Turn a key name into a key symbol. Accepts canonical names (`Escape`,
/// `BackSpace`, `space`) as well as punctuation the canonical table has no entry
/// for (`xkb_keysym_from_name` rejects a bare `,`).
pub fn parse_key(name: &str) -> Result<Keysym> {
    let mut chars = name.chars();
    if let (Some(ch), None) = (chars.next(), chars.next()) {
        let keysym = xkb::utf32_to_keysym(ch as u32);
        if keysym != Keysym::NoSymbol {
            return Ok(keysym);
        }
    }
    let keysym = xkb::keysym_from_name(name, xkb::KEYSYM_NO_FLAGS);
    if keysym == Keysym::NoSymbol {
        return Err(anyhow!("`{name}` is not a key name"));
    }
    Ok(keysym)
}

pub fn key_name(keysym: Keysym) -> String {
    xkb::keysym_get_name(keysym)
}

#[derive(Debug, Clone)]
pub struct Bindings {
    pub normal: HashMap<Keysym, Action>,
    pub grid: HashMap<Keysym, Action>,
    pub hint: HashMap<Keysym, Action>,
    /// Grid cell selection keys, row major. These are not [`Action`]s: which one
    /// was pressed chooses a sub-region rather than a fixed behaviour.
    pub grid_cells: Vec<Keysym>,
}

impl Bindings {
    /// Every binding for `action`, across the modes that use it. Configuring an
    /// option replaces its defaults the first time it appears, so removing by
    /// action is how that happens.
    pub fn remove(&mut self, action: Action) {
        for table in [&mut self.normal, &mut self.grid, &mut self.hint] {
            table.retain(|_, bound| *bound != action);
        }
    }

    pub fn insert(&mut self, mode: crate::mode::Mode, keysym: Keysym, action: Action) {
        self.table_mut(mode).insert(keysym, action);
    }

    pub fn table_mut(&mut self, mode: crate::mode::Mode) -> &mut HashMap<Keysym, Action> {
        match mode {
            crate::mode::Mode::Normal => &mut self.normal,
            crate::mode::Mode::Grid => &mut self.grid,
            crate::mode::Mode::Hint => &mut self.hint,
        }
    }

    pub fn table(&self, mode: crate::mode::Mode) -> &HashMap<Keysym, Action> {
        match mode {
            crate::mode::Mode::Normal => &self.normal,
            crate::mode::Mode::Grid => &self.grid,
            crate::mode::Mode::Hint => &self.hint,
        }
    }
}

fn table(entries: &[(&str, Action)]) -> HashMap<Keysym, Action> {
    entries
        .iter()
        .map(|(name, action)| {
            (
                parse_key(name).expect("built-in binding names are valid"),
                *action,
            )
        })
        .collect()
}

impl Default for Bindings {
    /// The defaults follow warpd's normal-mode keys, except the scroll keys:
    /// warpd binds `r` and `e`, and these are `u` and `d`, the pair `less` and
    /// `vi` use for up and down.
    fn default() -> Self {
        Self {
            normal: table(&[
                ("h", Action::MoveLeft),
                ("j", Action::MoveDown),
                ("k", Action::MoveUp),
                ("l", Action::MoveRight),
                ("H", Action::EdgeLeft),
                ("M", Action::EdgeMiddle),
                ("L", Action::EdgeRight),
                ("bracketleft", Action::SpeedFine),
                ("backslash", Action::SpeedNormal),
                ("bracketright", Action::SpeedCoarse),
                ("u", Action::ScrollUp),
                ("d", Action::ScrollDown),
                ("m", Action::ClickLeft),
                ("comma", Action::ClickMiddle),
                ("period", Action::ClickRight),
                ("v", Action::ToggleDrag),
                ("g", Action::ToGrid),
                ("f", Action::ToHint),
                ("Escape", Action::Quit),
            ]),
            grid: table(&[
                ("h", Action::GridLeft),
                ("j", Action::GridDown),
                ("k", Action::GridUp),
                ("l", Action::GridRight),
                ("BackSpace", Action::GridBack),
                ("Return", Action::GridWarp),
                ("space", Action::GridWarpClick),
                ("Escape", Action::Quit),
            ]),
            hint: table(&[
                ("BackSpace", Action::HintBack),
                ("Escape", Action::Quit),
            ]),
            grid_cells: ["q", "w", "a", "s"]
                .iter()
                .map(|name| parse_key(name).expect("built-in cell keys are valid"))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_characters_and_canonical_names_both_parse() {
        assert_eq!(parse_key("h").unwrap(), parse_key("h").unwrap());
        assert_ne!(parse_key("h").unwrap(), parse_key("H").unwrap());
        assert_eq!(key_name(parse_key("Escape").unwrap()), "Escape");
        assert_eq!(key_name(parse_key("BackSpace").unwrap()), "BackSpace");
        assert_eq!(key_name(parse_key("space").unwrap()), "space");
    }

    #[test]
    fn shift_still_selects_a_different_key() {
        assert_eq!(parse_key("h").unwrap(), parse_key("h").unwrap());
        assert_ne!(parse_key("k").unwrap(), parse_key("K").unwrap());
    }

    #[test]
    fn punctuation_names_parse() {
        assert_eq!(key_name(parse_key(",").unwrap()), "comma");
        assert_eq!(key_name(parse_key("comma").unwrap()), "comma");
        assert_eq!(key_name(parse_key(".").unwrap()), "period");
        assert_eq!(key_name(parse_key("period").unwrap()), "period");
    }

    #[test]
    fn nonsense_names_are_rejected_with_the_name_in_the_message() {
        let err = parse_key("DefinitelyNotAKey").unwrap_err().to_string();
        assert!(err.contains("DefinitelyNotAKey"), "{err}");
    }

    #[test]
    fn every_default_binding_resolves_to_an_action() {
        let bindings = Bindings::default();
        assert_eq!(
            bindings.normal.get(&parse_key("h").unwrap()),
            Some(&Action::MoveLeft)
        );
        assert_eq!(
            bindings.normal.get(&parse_key("Escape").unwrap()),
            Some(&Action::Quit)
        );
        assert_eq!(
            bindings.grid.get(&parse_key("space").unwrap()),
            Some(&Action::GridWarpClick)
        );
        assert_eq!(
            bindings.hint.get(&parse_key("BackSpace").unwrap()),
            Some(&Action::HintBack)
        );
        assert_eq!(bindings.grid_cells.len(), 4);
    }

    #[test]
    fn shift_distinguishes_the_edge_jumps_from_the_moves() {
        let bindings = Bindings::default();
        assert_eq!(
            bindings.normal.get(&parse_key("H").unwrap()),
            Some(&Action::EdgeLeft)
        );
        assert_eq!(
            bindings.normal.get(&parse_key("h").unwrap()),
            Some(&Action::MoveLeft)
        );
    }
}
