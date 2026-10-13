# Spec Delta

## MODIFIED Requirements

### Requirement: Button and wheel injection

A session SHALL be able to inject a press and a release at the current pointer
position, covering the left, middle and right buttons, and SHALL be able to inject
a wheel event carrying a discrete step count. The overlay SHALL NOT take pointer
events after the session has learned where the pointer started, so that an
injected event lands on the window underneath without the overlay being moved out
of the way first.

#### Scenario: A single click

- **WHEN** a mode asks for a left-button click
- **THEN** the target application receives one press and one release, at the
  current pointer position

#### Scenario: The overlay does not swallow the click

- **WHEN** the overlay covers the target position before the button is pressed
- **THEN** the event reaches the window underneath, because the overlay does not
  take pointer events

#### Scenario: The wheel

- **WHEN** a mode asks to scroll up by one step
- **THEN** the target application receives one discrete wheel event, and does not
  read it as continuous touchpad scrolling

## ADDED Requirements

### Requirement: The application sees the pointer

A session SHALL let the application under the pointer receive the pointer's
motion, so that interface that reacts to hovering works while a session is up.
Only the keyboard SHALL be held exclusively; the pointer SHALL belong to the
application, and the session SHALL track its own position from its own moves
rather than from the pointer it handed over.

#### Scenario: Hovering over interface that reacts

- **WHEN** the user moves the pointer over an application element that shows
  something on hover — a video's controls, for instance
- **THEN** the application receives the motion and reacts, as it would for a
  physical mouse

#### Scenario: Moving after a drag

- **WHEN** a drag has been ended and the user moves the pointer again
- **THEN** the application keeps receiving the motion: the pointer is not taken
  back by the overlay when the drag ends
