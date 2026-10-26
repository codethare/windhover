# Spec Delta

## MODIFIED Requirements

### Requirement: Behaviour after a match

On a unique match the session SHALL position the pointer at the target's centre,
remove the overlay so the target window is visible, and by default switch to
normal mode so the user can adjust. With clicking after a warp enabled, the click
SHALL end the session, as every click does.

#### Scenario: Position and switch by default

- **WHEN** the user types a full label and matches exactly one target
- **THEN** the pointer is at that target's centre, the overlay content switches to
  normal mode, and the keyboard grab is not interrupted

#### Scenario: Clicking after a match

- **WHEN** the configuration enables clicking after a warp
- **THEN** after positioning, one left click is injected at the target's centre
  with the overlay removed, and the session then exits with status 0

#### Scenario: Positioning accuracy

- **WHEN** the target is a lattice target
- **THEN** the pointer lands within one logical pixel of the target's centre
