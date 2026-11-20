# Spec Delta

## MODIFIED Requirements

### Requirement: Dragging

Normal mode SHALL provide a drag toggle: while it is on, the left button stays
held and the pointer can be moved by the direction keys and by the edge jumps.
Pressing the toggle again, or injecting any mouse button, SHALL end the drag and
release the left button. Changing mode SHALL end the drag as well, releasing the
button before the session answers the change, and so SHALL the session ending,
however it ends.

#### Scenario: Starting and ending a drag

- **WHEN** the user presses `v`, moves the pointer, then presses `v` again
- **THEN** the left button is held throughout, and released on the second `v`

#### Scenario: Moving during a drag

- **WHEN** a drag is in progress and the user moves using the direction keys or an
  edge jump
- **THEN** the target application keeps receiving the pointer position during the
  drag

#### Scenario: A mode change ends the drag

- **WHEN** a drag is in progress and the user switches to grid mode or hint mode
- **THEN** the left button is released before the mode changes, and the session
  injects no press while it is held

#### Scenario: Ending a drag with a click

- **WHEN** a drag is in progress and the user presses `m`
- **THEN** the left button is released first and a complete left click is injected
  afterwards, with no leaked button state

#### Scenario: Released when the session exits

- **WHEN** the user exits the session while a drag is in progress, by `Escape`, by
  a signal, by the compositor closing the overlay, or by the keyboard going away
- **THEN** the left button is released, leaving no stuck press behind
