//! The three interaction modes.
//!
//! Modes are pure state machines: they decide *what* should happen and hand back
//! an [`Effect`], and the session is the only place that touches Wayland. That
//! split is what makes the modes testable without a compositor.

pub mod grid;
pub mod hint;
pub mod normal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Grid,
    Hint,
}

/// What a mode asks the session to do.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Effect {
    /// Relative move, in logical pixels.
    Move { dx: f64, dy: f64 },
    /// Absolute move, in logical pixels inside the overlay.
    Warp { x: f64, y: f64 },
    Click { button: u32 },
    /// Positive scrolls down, matching the `wl_pointer` sign convention.
    Scroll { steps: i32 },
    /// Hold or release the left button with no matching release, for dragging.
    Drag(bool),
    /// The factor every later movement and scroll is scaled by.
    Speed(f64),
    Switch(Mode),
    Exit,
}
