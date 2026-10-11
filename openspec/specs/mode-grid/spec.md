# mode-grid Specification

## Purpose
Defines what grid mode does: recursively partition the screen, shrink the active
region key by key until the pointer is close to the target, then position the
pointer at the centre of the region and hand over to normal mode for fine
adjustment. It is for the case where the target's rough position is known but it
is far away.

## Requirements

### Requirement: Initial partition

Entering grid mode SHALL set the active region to the whole target output and lay
out visible cells with their keys, at the configured partition (2x2 by default).

#### Scenario: Entering by default

- **WHEN** the user presses `g` in normal mode to enter grid mode
- **THEN** the overlay shows 2x2 dividing lines and the four cell key hints (`q`
  top left, `w` top right, `a` bottom left, `s` bottom right)

#### Scenario: A custom partition

- **WHEN** the configuration sets a 3x3 partition and provides the matching key
  table
- **THEN** the initial layout shows nine cells and uses the configured keys

#### Scenario: A partition with no matching key table

- **WHEN** the configured partition cannot be given enough keys from the default
  table or the configuration
- **THEN** the session refuses that configuration at startup and reports an error
  that can be diagnosed, rather than running with a partial key set

### Requirement: Region shrinking

Pressing a cell key SHALL shrink the active region to that cell and position the
pointer at the centre of the new region. That SHALL be repeatable until the region
is smaller than the minimum size.

#### Scenario: One level of shrinking

- **WHEN** the user presses `w` under the initial 2x2 partition
- **THEN** the active region becomes the top-right quarter of the previous region,
  and the pointer moves to the new region's centre

#### Scenario: Several levels of shrinking

- **WHEN** the user presses `w` and then `q`
- **THEN** the active region shrinks level by level into the top-left quarter of
  the top-right quadrant, with the pointer following each new centre

#### Scenario: Reaching the minimum size

- **WHEN** the active region is narrower or shorter than the minimum size
  threshold (twice the normal-mode step by default)
- **THEN** grid mode switches to normal mode automatically, leaving the pointer
  where it is for the user to adjust

### Requirement: Region panning

Once the active region is near its minimum size, grid mode SHALL allow the
movement keys to shift the whole region by its own size, positioning the pointer
at the centre of the new region.

#### Scenario: Panning left

- **WHEN** the user presses `h` in a shrunk state
- **THEN** the active region shifts one region-width to the left and the pointer
  moves to the new centre

#### Scenario: Panning into the screen edge

- **WHEN** panning would take the active region outside the output
- **THEN** the region is clamped inside the output and the pointer stays at its
  centre

### Requirement: Region history

Grid mode SHALL provide a key that restores the active region to what it was
before the last shrink. With nothing to undo it SHALL keep the current state.

#### Scenario: One step back

- **WHEN** the user shrinks once and then presses `BackSpace`
- **THEN** the active region returns to the region before the shrink, and the
  pointer moves to that region's centre

#### Scenario: Nothing to undo

- **WHEN** the user presses `BackSpace` without having shrunk
- **THEN** the active region and the pointer position are unchanged and the
  session keeps running

### Requirement: Region actions and exit

Grid mode SHALL provide two finishing actions — position only, and position and
click — as well as a key that exits the session. The finishing actions SHALL hand
the session back to normal mode rather than ending the process (exiting aside).

#### Scenario: Position only

- **WHEN** the user presses `Return` in grid mode
- **THEN** the pointer stays at the current region's centre and the session
  switches to normal mode

#### Scenario: Position and click

- **WHEN** the user presses `Space` in grid mode
- **THEN** the overlay is removed before injection, the left button is clicked once
  at the pointer position, and the session then switches to normal mode

#### Scenario: Exiting

- **WHEN** the user presses `Escape` in grid mode
- **THEN** the session exits normally without injecting any mouse action

### Requirement: Region visibility

Grid mode SHALL draw the active region's border, the cell dividing lines and the
cell key hints in the same visual language as the rest of the interface, so that
the current extent and the available cells are obvious at a glance.

#### Scenario: Showing the current region

- **WHEN** grid mode is active
- **THEN** the current active region's border is drawn as a high-contrast stroke,
  and content outside the region is not covered by an opaque block

#### Scenario: Showing the cell keys

- **WHEN** grid mode has partitioned the region
- **THEN** each cell shows its key hint at its centre, and the hint is readable
  against any background brightness
