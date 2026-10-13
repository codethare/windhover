# Tasks

## 1. The binding

- [x] 1.1 Point `scroll_up` at `u` and `scroll_down` at `d` in the default table in `src/keys.rs`, and record in the comment above it that this is where the defaults part company with warpd's. Verify: `cargo test`.
- [x] 1.2 Move the table of expected defaults in `src/config.rs` with it, so the documented defaults stay tied to the code. Verify: `cargo test`.
- [x] 1.3 Nothing else about scrolling changes: the step count per event, the multiplier and the repeat while held are untouched. Verify: the wheel unit test still expects three steps for a `3` prefix.

## 2. Records

- [x] 2.1 The sample configuration and the options table in `docs/configuration.md`. The sample block is parsed by a test, so it has to move with the code. Verify: `cargo test` parses the sample without warnings.
- [x] 2.2 The end-to-end scroll check drives `d` instead of `e`. Verify: `scripts/headless-check.sh` passes on sway, river and niri, with the wheel arriving as one downward step.
