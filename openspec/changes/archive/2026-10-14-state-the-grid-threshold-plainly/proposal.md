# Proposal

## Why

Halving the derived movement step made a description in the grid specification
false. The scenario about reaching the minimum region size says the threshold is
"twice the normal-mode step by default", which was true by coincidence: the
threshold is a fixed 40 logical pixels and the step was 20. The step is 10 now,
and the threshold has not moved, because nothing derives it from the step —
`src/mode/grid.rs` never reads one.

The threshold itself is fine: it is configurable, its default is a sensible
region size, and the requirement that grid mode hand over when the region gets
small still holds. Only the description of where the default comes from is
wrong, and it is the kind of sentence that sends the next reader looking for a
relationship that does not exist.

## What Changes

- The scenario states the default for what it is: 40 logical pixels, rather than
  a multiple of something else.
- Nothing else. No code changes, no behaviour changes: the threshold was already
  independent of the step, which the audit for this confirmed.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `mode-grid`: the scenario that describes the minimum size threshold.

## Impact

- Documentation only. The derivation of the step, the threshold, and grid mode
  itself are untouched.
- Recorded here rather than silently edited because the specification is what
  someone reads to learn how the thing behaves, and a wrong sentence in it is the
  same kind of defect as a wrong line of code.
