# Spec Delta

## MODIFIED Requirements

### Requirement: Button and wheel injection

A session SHALL be able to inject a press and a release at the current pointer
position, covering the left, middle and right buttons, and SHALL be able to inject
a wheel event carrying a discrete step count. The overlay SHALL NOT take pointer
events after the session has learned where the pointer started, so that an
injected event lands on the window underneath without the overlay being moved out
of the way first. A click SHALL be the last thing a session does: the button is
injected and the session then exits, because the keyboard is held exclusively and
a compositor returns it to the layer surface the moment the click gives it to the
application. A wheel event SHALL NOT end the session.

#### Scenario: A single click

- **WHEN** a mode asks for a left-button click
- **THEN** the target application receives one press and one release, at the
  current pointer position, and the session then exits with status 0

#### Scenario: The overlay does not swallow the click

- **WHEN** the overlay covers the target position before the button is pressed
- **THEN** the event reaches the window underneath, because the overlay does not
  take pointer events

#### Scenario: A click into a field that follows the keyboard

- **WHEN** the click lands on a field that enables itself on keyboard focus
- **THEN** the field is still focused after the session has exited, so the next
  key reaches it rather than the session

#### Scenario: The wheel

- **WHEN** a mode asks to scroll up by one step
- **THEN** the target application receives one discrete wheel event, does not read
  it as continuous touchpad scrolling, and the session keeps running
