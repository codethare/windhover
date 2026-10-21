# Proposal

## Why

Hint mode now lands the pointer on a coarse 400-cell lattice whose worst case is
a third of the way into a cell. The hand-over to normal mode is what closes that
gap, but normal mode has no slow speed: its smallest movement is one step, about
20 logical pixels on a 1440p output, so fine positioning costs several careful
presses and overshoots in between.

mouseless solves this with live speed multipliers bound to keys, moving the same
step at `0.1x`, `1x` or `4x` on demand. Normal mode has only the held-key
acceleration (up to four times) and a doubled step with `Ctrl`, both of which
only go faster.

## What Changes

- Three speed presets — fine (`0.1x`), normal (`1x`) and coarse (`4x`) — applied
  on top of the step, bound to `[`, `\` and `]` by default.
- The preset is session state, not a mode: it survives a hand-over from grid or
  hint mode, and every movement, edge jump, scroll and drag of a later press uses
  it.
- The overlay shows the preset beside the cursor whenever it is not `1x`, so a
  session cannot be stuck slow or fast without saying so.
- The held-key acceleration and the `Ctrl` factor still multiply on top of the
  preset: the preset sets the floor, holding raises it from there.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `mode-normal`: a new speed-preset concern beside the existing movement,
  multiplier and edge-jump behaviour.
- `configuration`: the three preset bindings are configurable key options.

## Impact

- `src/keys.rs`, `src/config.rs`, `src/mode/mod.rs`, `src/mode/normal.rs`,
  `src/session.rs`, `src/render/mod.rs`
- `README.md`, `docs/configuration.md`
- `scripts/headless-check.sh`
