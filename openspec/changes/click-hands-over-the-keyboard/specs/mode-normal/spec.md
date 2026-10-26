# Spec Delta

## MODIFIED Requirements

### Requirement: Mouse button actions

Normal mode SHALL provide three keys that inject a left, middle and right click at
the current pointer position. A click SHALL end the session, after the button has
been injected, so the application it landed on keeps the focus the click gave it.

#### Scenario: Left click

- **WHEN** the user presses `m`
- **THEN** one left click occurs at the current pointer position, the overlay does
  not intercept that click, and the session exits with status 0

#### Scenario: Middle and right click

- **WHEN** the user presses `,` or `.`
- **THEN** one middle or one right click occurs at the current pointer position,
  and the session exits with status 0

### Requirement: Dragging

Normal mode SHALL provide a drag toggle: while it is on, the left button stays
held and the pointer can be moved by any means. Pressing the toggle again, or
injecting any mouse button, SHALL end the drag and release the left button. A drag
does not end the session; the click that ends one does.

#### Scenario: Starting and ending a drag

- **WHEN** the user presses `v`, moves the pointer, then presses `v` again
- **THEN** the left button is held throughout, and released on the second `v`

#### Scenario: Moving during a drag

- **WHEN** a drag is in progress and the user moves using the direction keys, an
  edge jump, or by switching to grid mode
- **THEN** the target application keeps receiving the pointer position during the
  drag

#### Scenario: Ending a drag with a click

- **WHEN** a drag is in progress and the user presses `m`
- **THEN** the left button is released first and a complete left click is injected
  afterwards, with no leaked button state, and the session then exits with status
  0

#### Scenario: Released when the session exits

- **WHEN** the user exits the session while a drag is in progress
- **THEN** the left button is released, leaving no stuck press behind
