# Spec Delta

## MODIFIED Requirements

### Requirement: Configurable options

The configuration SHALL cover at least the following options, with the value in
parentheses as the default: the quit key (`Escape`), the four normal-mode
direction keys (`h` `j` `k` `l`), the normal-mode step (derived from the output
size, about 10 logical pixels), the three edge-jump keys (`H` `M` `L`), the three
mouse-button keys (`m` `,` `.`), the drag toggle (`v`), the keys that enter grid
and hint mode (`g` `x`), the grid partition (`2x2`), the grid cell key table, the
grid minimum size, the grid undo key (`BackSpace`), the grid warp key (`Return`),
the grid warp-and-click key (`Space`), the hint lattice spacing, the hint label
alphabet, whether to click after a warp (no), the hint editing key (`BackSpace`),
the two scroll keys (`u` up, `d` down), the font family, and the colours used by
grid and hint mode.

#### Scenario: Changing the movement step

- **WHEN** the configuration sets the normal-mode step to a larger value
- **THEN** each directional movement covers the correspondingly larger distance

#### Scenario: Changing the hint density

- **WHEN** the configuration sets the hint lattice spacing to a smaller value
- **THEN** more targets are generated, and the labels stay prefix-free

#### Scenario: Changing the colours

- **WHEN** the configuration provides a new accent colour
- **THEN** grid mode and hint mode use the same colour rules with that accent for
  labels, borders and grid lines alike
