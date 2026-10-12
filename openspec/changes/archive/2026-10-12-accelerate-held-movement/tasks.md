# Tasks

## 1. Grow the step while a key is held

- [x] 1.1 Report the fast modifier with each key event in `src/wl/seat.rs`: a `Control` modifier does not change the keysym a letter resolves to, so the session cannot see it any other way. Verify: the keymap unit test still resolves keysyms, and the end-to-end check below proves the modifier arrives.
- [x] 1.2 Keep the time each repeatable key went down in `src/session.rs` and scale the step from it, one factor per 200 ms up to four. Verify: `cargo test` covers the growth, the limit, and that a press after a release starts from the base step.
- [x] 1.3 Fix the repeat cadence that `act` was overwriting: it rescheduled every repeat with `REPEAT_DELAY`, so holding a key moved once per 300 ms rather than once per 40 ms, and the entry's press time was reset on every repeat. Scheduling now lives in one place, which the growth needs to accumulate at all. Verify: `a held_movement_key_repeats_after_the_delay` observes a second repeat, which the old cadence would have hidden.
- [x] 1.4 Leave `src/mode/normal.rs` alone: the scaled step arrives through `Context::step`, which is what it already moves by. Verify: `cargo test` passes with no change to its tests.

## 2. Cover it

- [x] 2.1 Unit tests for the growth: it never shrinks, it grows over a hold, it stops at the limit however long the hold runs, the fast modifier doubles every repeat, and a fresh press starts from the base step. Verify: `cargo test`.
- [x] 2.2 An end-to-end check for the fast modifier, which is the only part the modifier events can be observed through: `mods:ctrl tap:l` moves two steps. Verify: `scripts/headless-check.sh` passes on sway, river and niri.

## 3. Records

- [x] 3.1 README key table and the `step` row in `docs/configuration.md`. Verify: the documented growth matches the constants in `src/session.rs`.
- [x] 3.2 Confirm scroll and the numeric multiplier are untouched: only movement's step is scaled, so the wheel keeps the fixed rate its requirement describes. Verify: the wheel and multiplier checks still pass.
