# Proposal

## Why

Hint mode's cell count came from the spacing alone, and its labels were always
the same length: two characters, because the lattice was sized to the
two-character budget. Two things follow from that, and neither is what stochos
does.

The count was a function of the screen: 390 cells on a 16:9 output, 400 on a
square one, and a `hint_cell` dense enough would have needed three characters and
been refused outright. stochos caps the grid at the alphabet instead — no axis
gets more cells than there are characters to name it — so the labels are bounded
by the alphabet and never grow.

The labels were also all the same length, when the budget that sizes them is
rarely exact. stochos spends the spare room on one-character labels for the
targets it ranks highest, and its rule for how many is exact: each short label
costs `k^(len-1) - 1` of the longer ones.

The type size had the same shape of problem in the other direction: a label was
14 logical pixels on every output, so it read at 19 minutes of arc on a 1440p
panel and at 13 on a 4K one.

## What Changes

- The lattice is capped per axis at the alphabet's length. Twenty characters mean
  at most a 20x20 grid, whatever the spacing asks for, so a cell gets wider
  instead of a label getting longer.
- Labels are assigned by the stochos rule: the shortest length that names the
  count, then `(k^len - count) / (k^(len-1) - 1)` of the first targets named in
  one character, and the rest skipping those characters so that no label is a
  prefix of another.
- The label type size follows the output height, `badge_font_size * height / 720`,
  so a label keeps its angular size from a laptop panel to a 4K monitor.
- The three-character labels, and the refusal for a lattice too dense to label,
  are gone: with the cap the count can never exceed the two-character budget on
  either axis, so both were unreachable.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `mode-hint`: the lattice is capped by the alphabet, spare budget becomes
  one-character labels for the first targets, and label type size follows the
  output.
- `configuration`: `hint_cell` no longer decides the count on its own — it is
  capped per axis by the alphabet — and it can no longer be too dense.

## Impact

- `src/hint/labels.rs`, `src/hint/lattice.rs`, `src/session.rs`
- `src/render/mod.rs`
- `README.md`, `docs/configuration.md`
- `scripts/headless-check.sh` (the hint checks pin an alphabet, and the density
  refusal check is gone)
