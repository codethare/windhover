# pointer-session Specification

## Purpose
Defines the boundary of one keyboard-driven pointer session: how the keyboard and
the whole screen are taken over on a Wayland compositor, how the pointer is moved
to an arbitrary coordinate and how buttons and the wheel are injected, and how the
session exits without leaving anything behind. This is the runtime floor the three
modes stand on.

## Requirements

### Requirement: A session is a short-lived process started in one mode

`windhover` SHALL be launched by a compositor hotkey as a short-lived process and
SHALL start in exactly one mode (normal, grid or hint), defaulting to normal when
no mode argument is given. The process SHALL exit once the session ends and SHALL
keep no process, file or IPC state between invocations.

#### Scenario: Started in normal mode

- **WHEN** the user runs `windhover --normal` and the compositor supports the
  protocols it needs
- **THEN** the process establishes a fullscreen overlay, takes the keyboard
  exclusively, and blocks waiting for keys
- **AND** the process exits with status 0 when the session ends

#### Scenario: No mode given

- **WHEN** the user runs `windhover` with no mode argument
- **THEN** the behaviour is identical to `windhover --normal`

#### Scenario: The compositor lacks a required protocol

- **WHEN** the compositor provides no overlay layer or no virtual pointer
- **THEN** the process does not establish a partially working session, writes one
  message naming the missing capability to standard error, and exits non-zero

#### Scenario: An argument that starts no session

- **WHEN** the user runs `windhover --help` or `windhover --version`
- **THEN** the process connects to no compositor and creates no overlay, and exits
  with status 0 after printing

### Requirement: Overlay presentation

A session SHALL present a fullscreen overlay layer, rendered at the target
output's scale, and SHALL not reserve any workspace space (it must not change
other windows' usable area).

#### Scenario: The output under the pointer

- **WHEN** a session starts with more than one output present
- **THEN** the overlay SHALL be created on the output the pointer is on at
  startup, and SHALL cover only that output

#### Scenario: Fractional scaling

- **WHEN** the target output's scale is not an integer (1.5, for instance)
- **THEN** the overlay's buffer is allocated at the scaled physical pixel size and
  what is drawn is sharp rather than blurry
- **AND** every coordinate reported outwards, and every pointer position and hit
  test, SHALL use logical coordinates, so the scale conversion is invisible to the
  modes

#### Scenario: Workspace layout is untouched

- **WHEN** other windows exist while the overlay is presented
- **THEN** their position and size do not change because the overlay appeared

### Requirement: Exclusive keyboard and key resolution

A session SHALL hold the keyboard exclusively for the lifetime of its overlay, so
that keys do not reach other applications. Keys SHALL be resolved to bindable
symbols through the current xkb layout rather than by hard-coded keycode, and
Shift and other modifiers SHALL be distinguishable, as SHALL key auto-repeat.

#### Scenario: Keys are not passed through

- **WHEN** a session is active and the user presses an ordinary character key
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

### Requirement: Absolute pointer positioning

A session SHALL be able to position the pointer directly at any logical
coordinate inside the target output, without accumulating relative movement, so
that the result does not depend on where the pointer started or on the pointer
acceleration settings.

#### Scenario: Positioning at a given coordinate

- **WHEN** a mode asks for the pointer at coordinate (x, y)
- **THEN** the pointer appears at that logical coordinate, within one logical pixel

#### Scenario: Positioning does not depend on where the pointer started

- **WHEN** a session starts with the pointer anywhere and asks for the same point
- **THEN** the pointer lands in the same place as in any other session

### Requirement: Button and wheel injection

A session SHALL be able to inject a press and a release at the current pointer
position, covering the left, middle and right buttons, and SHALL be able to inject
a wheel event carrying a discrete step count.

#### Scenario: A single click

- **WHEN** a mode asks for a left-button click
- **THEN** the target application receives one press and one release, at the
  current pointer position

#### Scenario: The overlay does not swallow the click

- **WHEN** the overlay covers the target position before the button is pressed
- **THEN** the session SHALL remove the overlay before injecting the button event,
  so the event reaches the window underneath

#### Scenario: The wheel

- **WHEN** a mode asks to scroll up by one step
- **THEN** the target application receives one discrete wheel event, and does not
  read it as continuous touchpad scrolling

### Requirement: Exit and cleanup

A session SHALL remove its overlay and return the keyboard when it exits, and
SHALL write no persistent state. Abnormal termination (a signal, for instance)
SHALL likewise leave nothing visible behind.

#### Scenario: The user exits

- **WHEN** the user presses the quit key
- **THEN** the overlay disappears, the keyboard returns to the compositor's
  focused window, and the process exits with status 0

#### Scenario: Killed by a signal

- **WHEN** the session receives SIGTERM while running
- **THEN** the process removes the overlay and the virtual pointer before exiting,
  and no overlay is left on screen

#### Scenario: No persistent state is written

- **WHEN** a session performs any mode action
- **THEN** apart from a configuration file being written explicitly, the session
  creates and modifies no persistent user data
