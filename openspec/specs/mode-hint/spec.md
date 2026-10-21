# mode-hint Specification

## Purpose
Defines what hint mode does: lay a set of labelled prompts over the screen's
targets so that typing a label's characters moves the pointer straight onto that
target. The targets are a regular lattice across the output. It is for the case
where the target is known but its coordinates are not.

## Requirements

### Requirement: The lattice target source

By default hint mode SHALL generate a regular lattice across the target output as
its target source: the output is partitioned into a grid of cells, and each cell
carries one target at its centre and one label from the configured alphabet. The
partition SHALL cover the whole output, and the label set SHALL be prefix-free
(no label is a prefix of another), so that reaching a unique match while typing
is enough to position immediately. The label length SHALL be the shortest the
alphabet can name the chosen number of cells in: two characters for the default
twenty-character alphabet.

#### Scenario: Generating lattice hints

- **WHEN** the user presses `f` in normal mode to enter hint mode
- **THEN** the overlay draws a translucent cell over each target region with a
  label centred in it, the labels are taken from the configured alphabet, every
  label is two characters long under the default alphabet, and no two labels
  where one is a prefix of the other are produced

#### Scenario: The cells tile the output

- **WHEN** the lattice is generated for an output
- **THEN** the cells cover the output without gaps or overlap, and each cell's
  label sits at that cell's centre

#### Scenario: More targets than the alphabet can label

- **WHEN** the configured lattice spacing asks for more cells than the alphabet
  can name in two characters
- **THEN** the session raises the label length to three characters while keeping
  the labels prefix-free and never producing duplicate labels

#### Scenario: More targets than the alphabet can name at all

- **WHEN** the configured lattice spacing asks for more cells than the alphabet
  can name in three characters
- **THEN** the session refuses to start and says which option is too dense,
  rather than dropping targets or repeating labels

#### Scenario: Density is configurable

- **WHEN** the configuration sets the hint lattice spacing
- **THEN** the number of cells follows that spacing, and the labels stay
  prefix-free at whatever length the cell count needs

### Requirement: Prefix filtering

Typed characters SHALL filter the labels by prefix. Input that matches no label
SHALL be ignored without changing state, and labels that no longer match SHALL
disappear from the overlay or be visibly de-emphasised.

#### Scenario: Narrowing the candidates

- **WHEN** the user types one character
- **THEN** only the labels starting with that character stay highlighted and
  readable, and the rest are removed or de-emphasised

#### Scenario: Input that matches nothing

- **WHEN** the user types a character matching none of the remaining labels
- **THEN** that input is ignored and the typed sequence and the candidate set are
  unchanged

#### Scenario: Converging one character at a time

- **WHEN** the user keeps typing characters until only one candidate is left
- **THEN** the session immediately positions the pointer at that target's centre

### Requirement: Behaviour after a match

On a unique match the session SHALL position the pointer at the target's centre,
remove the overlay so the target window is visible, and by default switch to
normal mode so the user can adjust.

#### Scenario: Position and switch by default

- **WHEN** the user types a full label and matches exactly one target
- **THEN** the pointer is at that target's centre, the overlay content switches to
  normal mode, and the keyboard grab is not interrupted

#### Scenario: Clicking after a match

- **WHEN** the configuration enables clicking after a warp
- **THEN** after positioning, one left click is injected at the target's centre
  with the overlay removed, and the session then switches to normal mode

#### Scenario: Positioning accuracy

- **WHEN** the target is a lattice target
- **THEN** the pointer lands within one logical pixel of the target's centre

### Requirement: Label editing and exit

Hint mode SHALL provide an editing key that removes typed characters, and a key
that exits the session immediately.

#### Scenario: Removing typed characters

- **WHEN** the user presses `BackSpace` after typing some characters
- **THEN** the last character is removed and the candidate set returns to what it
  was before that character

#### Scenario: The editing key with nothing typed

- **WHEN** the user presses `BackSpace` with nothing typed
- **THEN** the session returns to normal mode without exiting

#### Scenario: Exiting

- **WHEN** the user presses `Escape` in hint mode
- **THEN** the session exits normally without injecting any mouse action

### Requirement: Consistent hint cells

Hint mode's cells SHALL be tied to their target and SHALL not make the visual
content at the target's centre completely unreadable: the cell SHALL be a
translucent fill with a hairline edge rather than an opaque panel, so what is
underneath stays visible. The cell and its label SHALL use the same font, corner
radii and colour rules as grid mode's key hints.

#### Scenario: The badge does not cover the target centre

- **WHEN** a cell is drawn for a target
- **THEN** the area it covers does not render the visual content at the target's
  centre completely unreadable, because the fill is translucent and what is under
  it stays visible

#### Scenario: Legible on any background

- **WHEN** a cell is drawn over a light or a dark background
- **THEN** the label text and the cell's hairline edge keep enough contrast
  against it, with a stroke or a backing panel if that is what it takes
