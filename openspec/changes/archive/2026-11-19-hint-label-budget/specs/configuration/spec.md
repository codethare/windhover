# Spec Delta

## MODIFIED Requirements

### Requirement: Configurable options

The configuration SHALL cover at least the following options, with the value in
parentheses as the default: the quit key (`Escape`), the four normal-mode
direction keys (`h` `j` `k` `l`), the normal-mode step (derived from the output
size, about 10 logical pixels), the three edge-jump keys (`H` `M` `L`), the three
mouse-button keys (`m` `,` `.`), the drag toggle (`v`), the keys that enter grid
and hint mode (`g` `f`), the grid partition (`2x2`), the grid cell key table, the
grid minimum size, the grid undo key (`BackSpace`), the grid warp key (`Return`),
the grid warp-and-click key (`Space`), the hint lattice spacing (derived so the
labels are two characters long, and capped per axis at the alphabet's length), the hint label alphabet (twenty characters,
letters and `;`), whether to click after a warp (no), the hint editing key
(`BackSpace`), the two scroll keys (`u` up, `d` down), the font family, and the
colours used by grid and hint mode.

#### Scenario: Changing the movement step

- **WHEN** the configuration sets the normal-mode step to a larger value
- **THEN** each directional movement covers the correspondingly larger distance

#### Scenario: Changing the hint density

- **WHEN** the configuration sets the hint lattice spacing to a smaller value
- **THEN** more targets are generated up to the alphabet's cap per axis, and the
  labels stay prefix-free at one or two characters

#### Scenario: A label alphabet containing punctuation

- **WHEN** the configuration gives a hint label alphabet containing a printable
  non-whitespace character that is not a letter or a digit, such as `;`
- **THEN** that character is accepted and used in labels

#### Scenario: Changing the colours

- **WHEN** the configuration provides a new accent colour
- **THEN** grid mode and hint mode use the same colour rules with that accent for
  labels, borders and grid lines alike
