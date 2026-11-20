# Spec Delta

## MODIFIED Requirements

### Requirement: Button and wheel injection

A session SHALL be able to inject a press and a release at the current pointer
position, covering the left, middle and right buttons, and SHALL be able to inject
a wheel event carrying a discrete step count. The overlay SHALL NOT take pointer
events after the session has learned where the pointer started, so that an
injected event lands on the window underneath without the overlay being moved out
of the way first. A click SHALL be the last thing a session does: the layer
surface stops asking for the keyboard, the button is injected, and the session
then exits. The grab goes first because a compositor keeps the keyboard with an
exclusive layer surface: a click that asks for focus while it is held is refused,
or loses what it was given when the surface goes away. A wheel event SHALL NOT end
the session.

#### Scenario: A single click

- **WHEN** a mode asks for a left-button click
- **THEN** the target application receives one press and one release, at the
  current pointer position, and the session then exits with status 0

#### Scenario: The overlay does not swallow the click

- **WHEN** the overlay covers the target position and a mode asks for a click
- **THEN** the event reaches the window underneath, because the overlay does not
  take pointer events, and the keyboard grab is given up before the press so that
  the focus the click asks for can be granted

#### Scenario: A click into a field that follows the keyboard

- **WHEN** the click lands on a field that enables itself on keyboard focus, in an
  ordinary window or in a layer surface that asks for on-demand interactivity
- **THEN** the field is still focused after the session has exited, so the next
  key reaches it rather than the session

#### Scenario: The wheel

- **WHEN** a mode asks to scroll up by one step
- **THEN** the target application receives one discrete wheel event, does not read
  it as continuous touchpad scrolling, and the session keeps running

### Requirement: Exclusive keyboard and key resolution

A session SHALL hold the keyboard exclusively for the lifetime of its overlay, so
that keys do not reach other applications, **except for the turn a click ends it**:
before the button is pressed the layer surface SHALL stop asking for the keyboard,
because a compositor keeps an exclusive layer surface's keyboard and would refuse
or revoke the focus the click asks for. The overlay itself SHALL stay mapped until
the session exits, so the pointer keeps landing where it did. Keys SHALL be
resolved to bindable symbols through the current xkb layout rather than by
hard-coded keycode, and Shift and other modifiers SHALL be distinguishable, as
SHALL key auto-repeat.

#### Scenario: The grab is given up for a click

- **WHEN** a mode injects a click
- **THEN** the layer surface stops asking for the keyboard before the press, the
  overlay stays mapped, and the session exits without taking the grab back

#### Scenario: Keys are not passed through

- **WHEN** a session is active, no click has been injected, and the user presses an
  ordinary character key
- **THEN** that key has no effect on any other application

#### Scenario: Bindings independent of layout

- **WHEN** the user's xkb layout maps a physical key to a different symbol than
  the default
- **THEN** the binding takes effect by the resolved symbol, not by physical
  position

#### Scenario: Holding a movement key

- **WHEN** the user holds a movement key past the repeat delay
- **THEN** the pointer keeps moving at a fixed rate until the key is released

#### Scenario: A key that cannot be resolved

- **WHEN** a key arrives that maps to no symbol, or a non-keyboard event arrives
- **THEN** the session ignores it and keeps running, without exiting and without
  reporting an error
