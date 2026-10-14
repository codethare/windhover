# Spec Delta

## MODIFIED Requirements

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
  threshold (40 logical pixels by default, configurable, and independent of the
  movement step)
- **THEN** grid mode switches to normal mode automatically, leaving the pointer
  where it is for the user to adjust
