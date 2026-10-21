# Spec Delta

## ADDED Requirements

### Requirement: Speed presets

Normal mode SHALL provide three speed presets that scale the step every movement
and scroll uses: fine, normal and coarse. The preset SHALL be session state
rather than a mode, so it survives a hand-over from grid or hint mode, and SHALL
multiply together with the held-key acceleration and the fast modifier rather
than replacing them. The overlay SHALL show the preset whenever it is not the
normal one, so a session cannot be left slow or fast without saying so.

#### Scenario: A fine preset shortens every move

- **WHEN** the user selects the fine preset and presses a direction key
- **THEN** the pointer moves a fraction of the step in that direction instead of
  a whole one

#### Scenario: A coarse preset lengthens every move

- **WHEN** the user selects the coarse preset and presses a direction key
- **THEN** the pointer moves several steps in that direction

#### Scenario: The preset survives a hand-over

- **WHEN** the user selects the fine preset, jumps through grid or hint mode, and
  then presses a direction key
- **THEN** the pointer still moves by the fine fraction of the step

#### Scenario: Returning to the normal preset

- **WHEN** the user selects the normal preset
- **THEN** a direction key moves exactly one step again, and the overlay stops
  showing a preset

#### Scenario: The preset is visible

- **WHEN** a preset other than the normal one is active
- **THEN** the status shown beside the cursor names it, and nothing is shown for
  the normal preset

#### Scenario: Holding a key from a preset

- **WHEN** the user holds a direction key while a preset is active
- **THEN** the movement still accelerates with the hold, starting from the
  preset's speed rather than from the step
