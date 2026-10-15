# Proposal

## Why

Hint mode is entered with `x`. warpd, whose key choices this project follows
wherever it has one, uses `f` for hint mode, and `f` sits under the left hand on
the home row rather than being a stretch for the left little finger. Grid mode is
`g` and stays where it is, so the two entry keys read as a pair that the hand
already covers: `f` for the thing you pick, `g` for the thing you narrow.

## What Changes

- `hint_mode` defaults to `f` instead of `x`. Nothing else about hint mode moves:
  the labels, the filtering, the hand-over to normal mode and the configuration
  option are the same, and an explicit `hint_mode:` still wins.
- One sentence of prose in the hint specification is corrected while its file is
  open: it still offers targets "from the interactive elements the accessibility
  tree reports", which went with the `atspi` feature and has not been true since.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `mode-normal`: the scenario that enters hint mode names the key.
- `mode-hint`: the scenario that enters hint mode names the key, and the purpose
  no longer describes a target source that was removed.
- `configuration`: the option list names the default entry keys.

## Impact

- Code: the default binding table and the table of expected defaults.
- Documentation: the sample configuration, the options table, the README's mode
  table and its note on which key re-enters hint mode.
- A configuration that binds `hint_mode` itself is unaffected; anyone who wants
  `x` back writes `hint_mode: x`.
