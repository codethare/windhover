# Proposal

## Why

A held direction key moves at a fixed rate, so crossing a large screen takes as
long as it takes, and the only way to cover more ground is to type a numeric
prefix before every movement. warpd, which this mode follows, ramps the speed
while a key is down instead, and that is what makes one key usable both for a
nudge and for a traverse.

## What Changes

- The step of a held direction key grows the longer the key stays down: by one
  factor for every 200 ms, up to four times the configured step. Releasing the
  key and pressing it again starts from the base step.
- Holding Control with a direction key moves twice the step in one press, and
  accelerates the same way when held.
- The numeric multiplier is unchanged and still multiplies whatever the
  movement comes to.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `mode-normal`: the "Directional movement" requirement gains the growth while a
  key is held, the limit it levels off at, and the fast modifier.

## Impact

- Code: `src/wl/seat.rs` reports whether the fast modifier was down for a key,
  `src/session.rs` keeps the time each key went down and scales the step from it,
  and `src/mode/normal.rs` receives the scaled step through its context.
- Configuration: unchanged. The growth interval and its limit are constants, and
  the fast modifier is Control rather than a configurable key.
- Scroll is deliberately unchanged: only movement's step grows, so the wheel keeps
  the fixed repeat rate the spec already describes.
- README and `docs/configuration.md`: the grown movement and Control's meaning.
- The growth is time-dependent, so the end-to-end checks cover the modifier and
  the unit tests cover the ramp against fabricated clocks.
