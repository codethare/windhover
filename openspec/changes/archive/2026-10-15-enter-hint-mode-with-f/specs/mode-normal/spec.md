# Spec Delta

## MODIFIED Requirements

### Requirement: Mode switching and exit

Normal mode SHALL provide keys to switch to grid mode, to switch to hint mode, and
to exit the session. Returning from grid or hint mode SHALL land in normal mode
without re-initialising the session: the overlay and the pointer state stay
continuous.

#### Scenario: Entering grid mode

- **WHEN** the user presses `g`
- **THEN** the session switches to grid mode and the overlay content follows

#### Scenario: Entering hint mode

- **WHEN** the user presses `f`
- **THEN** the session switches to hint mode and the overlay content follows

#### Scenario: Returning from another mode

- **WHEN** the user returns from grid or hint mode
- **THEN** the session is in normal mode, the pointer stays where it was returned
  at, and the keyboard grab is not interrupted

#### Scenario: Exiting

- **WHEN** the user presses `Escape`
- **THEN** the session exits normally
