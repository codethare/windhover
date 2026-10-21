# Proposal

## Why

Hint mode labels every target in four characters. A four-character label buys a
56 px lattice, but the last two characters cost keystrokes to buy precision the
landing hand-over already provides: after a label matches, the session switches
to normal mode anyway, where one nudge closes the remaining gap.

A wider alphabet makes the same target count nameable in fewer keys. Twenty
characters name 400 cells in two keystrokes; nine characters needed four to name
1125. Two keystrokes on a coarse grid, then a nudge, is the shape stochos uses
and the one this change adopts.

## What Changes

- **BREAKING** The default alphabet becomes twenty characters,
  `asdfjkl;ghqwertyuiop`, so a label is two characters over a 400-cell budget
  rather than four over a 6000-cell one.
- `hint_charset` accepts any printable non-whitespace character, not only
  letters and digits: `;` is in the default alphabet and is currently rejected.
- The lattice becomes a partition with a cell budget derived from the alphabet
  instead of a fixed spacing, so the label length is two characters on any
  output rather than following the output's size.
- Hint mode draws a translucent grid of cells with a label centred in each,
  instead of a pill at each target point. Typed characters dim inside the label
  as they do now.
- `hint_cell` keeps its meaning as an override: setting it below the two-key
  budget raises the labels to three characters, and a value that would need more
  than three is refused at load.
- The three-character behaviour stays as the fallback if two keys read worse in
  practice; it needs no separate code path, only a smaller `hint_cell`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `mode-hint`: the target source becomes a budgeted partition, labels are two
  characters by default, and the visual is a translucent cell grid rather than a
  badge per target.
- `configuration`: `hint_charset` accepts punctuation, and `hint_cell`'s default
  is derived from the alphabet and the output instead of being a constant.

## Impact

- `src/hint/lattice.rs`, `src/hint/labels.rs`, `src/mode/hint.rs`
- `src/render/mod.rs`, `src/render/theme.rs`
- `src/config.rs`, `src/session.rs`
- `README.md`, `docs/configuration.md`
- `scripts/headless-check.sh` (label keys and the first cell's expected centre)
