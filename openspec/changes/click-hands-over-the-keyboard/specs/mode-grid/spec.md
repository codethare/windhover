# Spec Delta

## MODIFIED Requirements

### Requirement: Region actions and exit

Grid mode SHALL provide two finishing actions — position only, and position and
click — as well as a key that exits the session. The position-only action SHALL
hand the session back to normal mode rather than ending the process. The
position-and-click action SHALL hand over the same way, and the click SHALL then
end the session, as every click does.

#### Scenario: Position only

- **WHEN** the user presses `Return` in grid mode
- **THEN** the pointer stays at the current region's centre and the session
  switches to normal mode

#### Scenario: Position and click

- **WHEN** the user presses `Space` in grid mode
- **THEN** the keyboard grab is given up before the press, the left button is
  clicked once at the pointer position (the overlay itself stays, so the pointer
  lands where it already was), the session switches to normal mode, and it then
  exits with status 0

#### Scenario: Exiting

- **WHEN** the user presses `Escape` in grid mode
- **THEN** the session exits normally without injecting any mouse action
