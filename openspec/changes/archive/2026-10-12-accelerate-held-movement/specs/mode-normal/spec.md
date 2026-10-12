# Spec Delta

## MODIFIED Requirements

### Requirement: Directional movement

Normal mode SHALL provide four direction keys (`h` left, `j` down, `k` up, `l`
right by default) that move the pointer by the current step, which defaults to a
derived pixel value and can be changed in the configuration. Holding a key SHALL
trigger auto-repeat, and the distance covered per repeat SHALL grow the longer
the key stays down, up to a bounded multiple of the step, so that a hold
accelerates and then levels off rather than growing without end. Movement with
the fast modifier held SHALL move twice the step, and SHALL accelerate while held
the same way. Releasing a key SHALL reset it to the base step.

#### Scenario: A single key press

- **WHEN** the user presses `l` once
- **THEN** the pointer moves one step to the right, without leaving the screen

#### Scenario: Reaching the screen edge

- **WHEN** the pointer is already at the rightmost column and the user keeps
  pressing `l`
- **THEN** the pointer stays at that edge, without wrapping and without going out
  of bounds

#### Scenario: Holding a key

- **WHEN** the user holds `j`
- **THEN** the pointer keeps moving down until the key is released, covering more
  ground per repeat the longer it is held

#### Scenario: The hold levels off

- **WHEN** the user holds a direction key for longer than the growth takes to
  reach its limit
- **THEN** the distance per repeat stops growing and stays at the limit until the
  key is released

#### Scenario: Pressing again after a hold

- **WHEN** the user holds a direction key, releases it, and presses it again
- **THEN** the movement starts again from the base step

#### Scenario: Moving with the fast modifier

- **WHEN** the user presses `l` with the fast modifier held
- **THEN** the pointer moves two steps to the right

#### Scenario: Holding with the fast modifier

- **WHEN** the user holds `l` with the fast modifier held
- **THEN** every repeat moves twice what the same repeat would move without it,
  and the repeats accelerate as they do without it
