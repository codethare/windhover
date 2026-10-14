# Proposal

## Why

The derived step is `output_height / 36`, around 20 logical pixels on a 1080p
output. One press then overshoots small targets: the thing being aimed at is
usually a word, a button or a menu row, and 20 pixels is most of one. Half of it
keeps a press inside the target the user is looking at, and a hold still crosses
the output because the step grows to four times while a key is down.

## What Changes

- The derived step becomes `output_height / 72`, with the floor halved with it
  from 8 to 4 logical pixels, so an output too small to derive a useful value
  keeps moving by the same fraction of itself.
- Nothing else moves. An explicit `step:` in a configuration still wins, the
  multiplier, the growth while a key is held and the fast modifier are untouched,
  so a press is half as far and a hold is half as fast at every point of its ramp.
- The documented default and the sample configuration follow, and the end-to-end
  checks express their expectations in terms of the step, so they follow too.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `configuration`: the option list names the derived step.

## Impact

- Code: one expression in `src/config.rs`, and the test expectations that read a
  position after a press.
- Anyone who wants the old distance back sets `step:` once, which the option
  exists for.
- A press at the edge of an output moves the pointer onto the last pixel rather
  than near it, unchanged: the clamp is what handles that, not the step.
