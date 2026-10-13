# Proposal

## Why

The scroll keys are `r` and `e`, taken from warpd, which pairs them with nothing
else on the keyboard. `u` and `d` are what the rest of the terminal world uses for
up and down — `less` and `vi` move a half page with exactly those two letters —
and they read as the direction they mean, which `r` and `e` do not. warpd itself
expects the pair to be rebound: its own examples use `C-d` and `C-u`.

## What Changes

- `scroll_up` defaults to `u` and `scroll_down` to `d`, replacing `r` and `e`.
- The default-binding table, the documented defaults, and the end-to-end check
  follow. Nothing else about scrolling changes: the multiplier and the repeat
  while held are untouched.
- This is a deliberate departure from warpd's defaults, which the surrounding
  comment records so the next reader does not take it for a mistake.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `mode-normal`: the wheel scenarios name the two keys.
- `configuration`: the option list names the two defaults.

## Impact

- Code: `src/keys.rs` default bindings, and the table of expected defaults in
  `src/config.rs` that keeps the documented ones honest.
- Documentation: the sample configuration and the options table in
  `docs/configuration.md`. The sample block is parsed by a test, so it has to
  move with the code.
- Anyone who had `r` and `e` in muscle memory, or who had them in a
  configuration file: an explicit `scroll_up` / `scroll_down` line keeps working
  unchanged, and rebinding restores the old keys in one line each.
