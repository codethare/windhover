//! Normal mode: short-distance movement, multiplier prefix, edge jumps, buttons, drag.

use crate::keys::Action;
use crate::mode::{Effect, Mode};
use crate::wl::pointer::{BTN_LEFT, BTN_MIDDLE, BTN_RIGHT};

/// A numeric prefix longer than this is clamped rather than allowed to grow
/// without bound.
pub const MAX_MULTIPLIER: u32 = 999;

/// What normal mode needs from the session to decide an effect.
#[derive(Debug, Clone, Copy)]
pub struct Context {
    pub cursor: (f64, f64),
    pub screen: (u32, u32),
    pub step: f64,
}

#[derive(Debug, Default, Clone)]
pub struct Normal {
    multiplier: u32,
    dragging: bool,
}

impl Normal {
    pub fn multiplier(&self) -> Option<u32> {
        (self.multiplier > 0).then_some(self.multiplier)
    }

    pub fn dragging(&self) -> bool {
        self.dragging
    }

    /// A digit is a prefix, not an action, so it arrives here rather than
    /// through [`Self::on_action`]. `0` extends a prefix but cannot start one.
    pub fn on_digit(&mut self, digit: u32) {
        if self.multiplier == 0 && digit == 0 {
            return;
        }
        let extended = self.multiplier.saturating_mul(10).saturating_add(digit);
        self.multiplier = extended.min(MAX_MULTIPLIER);
    }

    pub fn on_action(&mut self, action: Action, context: &Context) -> Vec<Effect> {
        let multiplier = self.multiplier.max(1) as f64;
        // Every action consumes the prefix, whether or not it used it: a
        // dangling `5` must not silently apply to the key after next.
        self.multiplier = 0;

        match action {
            Action::MoveLeft => vec![Effect::Move {
                dx: -context.step * multiplier,
                dy: 0.0,
            }],
            Action::MoveRight => vec![Effect::Move {
                dx: context.step * multiplier,
                dy: 0.0,
            }],
            Action::MoveUp => vec![Effect::Move {
                dx: 0.0,
                dy: -context.step * multiplier,
            }],
            Action::MoveDown => vec![Effect::Move {
                dx: 0.0,
                dy: context.step * multiplier,
            }],
            Action::EdgeLeft => vec![Effect::Warp {
                x: 0.0,
                y: context.cursor.1,
            }],
            Action::EdgeMiddle => vec![Effect::Warp {
                x: context.screen.0 as f64 / 2.0,
                y: context.cursor.1,
            }],
            Action::EdgeRight => vec![Effect::Warp {
                x: context.screen.0 as f64,
                y: context.cursor.1,
            }],
            Action::ScrollUp => vec![Effect::Scroll {
                steps: -(multiplier as i32),
            }],
            Action::ScrollDown => vec![Effect::Scroll {
                steps: multiplier as i32,
            }],
            Action::ClickLeft | Action::ClickMiddle | Action::ClickRight => {
                let button = action_button(action);
                if self.dragging {
                    // Ending a drag with a button press has to release the left
                    // button first, or the press is swallowed as part of it.
                    self.dragging = false;
                    return vec![Effect::Drag(false), Effect::Click { button }];
                }
                vec![Effect::Click { button }]
            }
            Action::ToggleDrag => {
                self.dragging = !self.dragging;
                vec![Effect::Drag(self.dragging)]
            }
            Action::ToGrid => vec![Effect::Switch(Mode::Grid)],
            Action::ToHint => vec![Effect::Switch(Mode::Hint)],
            Action::Quit => vec![Effect::Exit],
            _ => vec![],
        }
    }

    /// Called when the session leaves normal mode, so a held drag cannot leak
    /// past the mode switch.
    pub fn release_drag(&mut self) -> Vec<Effect> {
        if std::mem::take(&mut self.dragging) {
            vec![Effect::Drag(false)]
        } else {
            vec![]
        }
    }
}

fn action_button(action: Action) -> u32 {
    match action {
        Action::ClickMiddle => BTN_MIDDLE,
        Action::ClickRight => BTN_RIGHT,
        _ => BTN_LEFT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> Context {
        Context {
            cursor: (100.0, 200.0),
            screen: (1280, 720),
            step: 20.0,
        }
    }

    fn normal() -> Normal {
        Normal::default()
    }

    #[test]
    fn a_step_moves_one_step_in_the_pressed_direction() {
        let mut mode = normal();
        let context = context();
        assert_eq!(
            mode.on_action(Action::MoveLeft, &context),
            vec![Effect::Move { dx: -20.0, dy: 0.0 }]
        );
        assert_eq!(
            mode.on_action(Action::MoveDown, &context),
            vec![Effect::Move { dx: 0.0, dy: 20.0 }]
        );
    }

    #[test]
    fn a_prefix_multiplies_the_next_move() {
        let mut mode = normal();
        let context = context();
        mode.on_digit(1);
        mode.on_digit(0);
        assert_eq!(mode.multiplier(), Some(10));
        assert_eq!(
            mode.on_action(Action::MoveDown, &context),
            vec![Effect::Move { dx: 0.0, dy: 200.0 }]
        );
    }

    #[test]
    fn the_prefix_is_consumed_by_the_move_that_used_it() {
        let mut mode = normal();
        let context = context();
        mode.on_digit(5);
        mode.on_action(Action::MoveDown, &context);
        assert_eq!(mode.multiplier(), None);
        assert_eq!(
            mode.on_action(Action::MoveDown, &context),
            vec![Effect::Move { dx: 0.0, dy: 20.0 }]
        );
    }

    #[test]
    fn a_prefix_is_dropped_by_an_unrelated_key() {
        let mut mode = normal();
        let context = context();
        mode.on_digit(5);
        mode.on_action(Action::ToggleDrag, &context);
        assert_eq!(mode.multiplier(), None);
        assert_eq!(
            mode.on_action(Action::MoveDown, &context),
            vec![Effect::Move { dx: 0.0, dy: 20.0 }]
        );
    }

    #[test]
    fn a_long_prefix_is_clamped() {
        let mut mode = normal();
        for _ in 0..8 {
            mode.on_digit(9);
        }
        assert_eq!(mode.multiplier(), Some(MAX_MULTIPLIER));
    }

    #[test]
    fn a_zero_cannot_start_a_prefix() {
        let mut mode = normal();
        mode.on_digit(0);
        assert_eq!(mode.multiplier(), None);
        mode.on_digit(0);
        mode.on_digit(7);
        assert_eq!(mode.multiplier(), Some(7));
    }

    #[test]
    fn a_prefix_multiplies_scrolling_too() {
        let mut mode = normal();
        let context = context();
        mode.on_digit(3);
        assert_eq!(
            mode.on_action(Action::ScrollUp, &context),
            vec![Effect::Scroll { steps: -3 }]
        );
    }

    #[test]
    fn edge_jumps_keep_the_vertical_position() {
        let mut mode = normal();
        let context = context();
        assert_eq!(
            mode.on_action(Action::EdgeLeft, &context),
            vec![Effect::Warp { x: 0.0, y: 200.0 }]
        );
        assert_eq!(
            mode.on_action(Action::EdgeMiddle, &context),
            vec![Effect::Warp { x: 640.0, y: 200.0 }]
        );
        assert_eq!(
            mode.on_action(Action::EdgeRight, &context),
            vec![Effect::Warp { x: 1280.0, y: 200.0 }]
        );
    }

    #[test]
    fn buttons_map_to_their_linux_codes() {
        let mut mode = normal();
        let context = context();
        assert_eq!(
            mode.on_action(Action::ClickLeft, &context),
            vec![Effect::Click { button: BTN_LEFT }]
        );
        assert_eq!(
            mode.on_action(Action::ClickMiddle, &context),
            vec![Effect::Click { button: BTN_MIDDLE }]
        );
        assert_eq!(
            mode.on_action(Action::ClickRight, &context),
            vec![Effect::Click { button: BTN_RIGHT }]
        );
    }

    #[test]
    fn drag_toggles_and_reports_its_state() {
        let mut mode = normal();
        let context = context();
        assert!(!mode.dragging());
        assert_eq!(
            mode.on_action(Action::ToggleDrag, &context),
            vec![Effect::Drag(true)]
        );
        assert!(mode.dragging());
        assert_eq!(
            mode.on_action(Action::ToggleDrag, &context),
            vec![Effect::Drag(false)]
        );
        assert!(!mode.dragging());
    }

    #[test]
    fn a_click_ends_a_drag_by_releasing_first() {
        let mut mode = normal();
        let context = context();
        mode.on_action(Action::ToggleDrag, &context);
        assert_eq!(
            mode.on_action(Action::ClickLeft, &context),
            vec![Effect::Drag(false), Effect::Click { button: BTN_LEFT }]
        );
        assert!(!mode.dragging());
    }

    #[test]
    fn leaving_normal_mode_releases_a_held_drag() {
        let mut mode = normal();
        let context = context();
        assert_eq!(mode.release_drag(), vec![]);
        mode.on_action(Action::ToggleDrag, &context);
        assert_eq!(mode.release_drag(), vec![Effect::Drag(false)]);
        assert_eq!(mode.release_drag(), vec![]);
    }

    #[test]
    fn mode_switches_and_quit_are_reported() {
        let mut mode = normal();
        let context = context();
        assert_eq!(
            mode.on_action(Action::ToGrid, &context),
            vec![Effect::Switch(Mode::Grid)]
        );
        assert_eq!(
            mode.on_action(Action::ToHint, &context),
            vec![Effect::Switch(Mode::Hint)]
        );
        assert_eq!(mode.on_action(Action::Quit, &context), vec![Effect::Exit]);
    }

    #[test]
    fn grid_actions_do_nothing_in_normal_mode() {
        let mut mode = normal();
        let context = context();
        assert_eq!(mode.on_action(Action::GridWarp, &context), vec![]);
    }
}
