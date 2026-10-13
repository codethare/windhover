# Spec Delta

## MODIFIED Requirements

### Requirement: Wheel

Normal mode SHALL provide keys that scroll up and down (`u` and `d` by default),
whose step count SHALL be affected by the numeric multiplier, and which SHALL
repeat while held.

#### Scenario: Scrolling up

- **WHEN** the user presses `u`
- **THEN** one upward discrete wheel event is produced at the current pointer
  position

#### Scenario: Scrolling with a multiplier

- **WHEN** the user presses `3` and then `d`
- **THEN** one downward wheel event of three steps is produced

#### Scenario: Holding a scroll key

- **WHEN** the user holds `d` past the repeat delay
- **THEN** scroll events keep being produced at the same fixed rate as movement
  keys, until the key is released
