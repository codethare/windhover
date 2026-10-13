//! Optional diagnostics for a session.
//!
//! The failures that matter here are the ones the client never sees: a key the
//! layout cannot produce, a release that never arrives, a keyboard taken away
//! without a `leave`. Everything the session can observe about those is printed
//! under `--trace`, so a report of one can be read instead of guessed at.

use std::sync::atomic::{AtomicBool, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(false);

pub fn enable() {
    ENABLED.store(true, Ordering::Relaxed);
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// One line on standard error, marked so it cannot be mistaken for one of the
/// warnings a session prints on its own. The formatting is left to `format_args`
/// so that a session without `--trace` pays nothing for it.
pub fn log(message: std::fmt::Arguments<'_>) {
    if enabled() {
        eprintln!("TRACE {message}");
    }
}
