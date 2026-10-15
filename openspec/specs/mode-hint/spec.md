# mode-hint Specification

## Purpose
Defines what hint mode does: lay a set of labelled prompts over the screen's
targets so that typing a label's characters moves the pointer straight onto that
target. The targets are a regular lattice across the output. It is for the case
where the target is known but its coordinates are not.

## Requirements

### Requirement: The lattice target source

By default hint mode SHALL generate a regular lattice across the target output as
its target source, each target carrying a letter label. The label set SHALL be
prefix-free (no label is a prefix of another), so that reaching a unique match
while typing is enough to position immediately.

#### Scenario: Generating lattice hints

- **WHEN** the user presses `f` in normal mode to enter hint mode
- **THEN** the overlay draws a compact label badge at each target position, with
  labels taken from the configured alphabet, and no two labels where one is a
  prefix of the other

#### Scenario: More targets than the alphabet can label

- **WHEN** the required number of targets exceeds what the current alphabet can
  cover at a fixed label length
- **THEN** the session increases the label length or reduces the lattice density
  while keeping the labels prefix-free, and never produces duplicate labels

#### Scenario: Density is configurable

- **WHEN** the configuration changes the hint lattice spacing
- **THEN** the number of generated targets follows, and the labels stay
  prefix-free

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

### Requirement: Consistent hint badges

Hint mode's label badges SHALL be bound to their target and SHALL not make the
visual content at the target's centre completely unreadable. The badge style SHALL
use the same font, corner radii and colour rules as grid mode's key hints.

#### Scenario: The badge does not cover the target centre

- **WHEN** a label badge is drawn for a target
- **THEN** the area it covers does not render the visual content at the target's
  centre completely unreadable

#### Scenario: Legible on any background

- **WHEN** a badge is drawn over a light or a dark background
- **THEN** the label text keeps enough contrast against it, with a stroke or a
  backing panel if that is what it takes
