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

### Requirement: One-shot session

With `-o` / `--oneshot`, a session started in grid or hint mode SHALL exit when
that mode hands over, instead of switching to normal mode: the work of the
hand-over SHALL still happen first, and the exit SHALL remove the overlay and
return the keyboard exactly as any other exit does. `--oneshot` SHALL be refused
when combined with normal mode, which never hands over.

#### Scenario: A one-shot hint selection

- **WHEN** the user runs `windhover --hint --oneshot` and types the characters of
  a label
- **THEN** the pointer is warped to that target, the click is injected first if
  `click_after_warp` is on, and the session then exits with status 0
- **AND** no overlay is left mapped

#### Scenario: A one-shot grid warp

- **WHEN** the user runs `windhover --grid --oneshot` and warps to the region
- **THEN** the pointer is warped, and the session then exits with status 0

#### Scenario: One-shot with normal mode

- **WHEN** the user runs `windhover --normal --oneshot`
- **THEN** no session is started, and one message naming the reason is written to
  standard error

#### Scenario: Without the flag

- **WHEN** the user runs `windhover --hint` and types the characters of a label
- **THEN** the session warps, switches to normal mode, and stays alive, and one
  Escape exits it

### Requirement: Losing the keyboard ends the session

A session SHALL treat the loss of the keyboard as the end of what keys were
doing: the repeat of a held key SHALL stop, and a drag in progress SHALL release
its button rather than leaving a window dragging with it held down. A session
whose keyboard does not return within a short grace period SHALL end itself,
removing its overlay and writing one message naming the reason, because no key
can reach a session without the keyboard and nothing else would dismiss it.

#### Scenario: A key is held when the keyboard goes

- **WHEN** the seat takes the keyboard away while a direction key is held
- **THEN** the pointer stops moving, without waiting for a release that can
  no longer arrive

#### Scenario: A drag is in progress when the keyboard goes

- **WHEN** the seat takes the keyboard away during a drag
- **THEN** the left button is released, leaving no window dragging behind

#### Scenario: The keyboard does not come back

- **WHEN** the keyboard stays away past the grace period
- **THEN** the session ends, and one message on standard error says the
  compositor took the keyboard away

#### Scenario: The keyboard comes straight back

- **WHEN** the keyboard is taken and returned within the grace period, as the
  startup handshake does
- **THEN** the session carries on, and the keys that follow still act

### Requirement: A repeat whose release never arrives is bounded

A key the session believes is held SHALL stop repeating once its hold outlasts a
fixed limit, since a compositor can swallow the release without saying so. At
that point the session SHALL release a drag's button and end itself, writing one
message naming the reason, because a compositor that lost the release may also
have stopped delivering keys, leaving no key that could end the session.

#### Scenario: A hold that outlasts the limit

- **WHEN** a direction key stays down past the limit without its release ever
  arriving
- **THEN** the repeat stops, any drag in progress has its button released, and
  the session ends, naming the reason on standard error

#### Scenario: A hold within the limit

- **WHEN** a key is held for less than the limit and then released
- **THEN** the repeat behaves as it always did, and the release stops it

### Requirement: Bindings the layout cannot produce are reported

A session SHALL check its configured bindings against the keymap the compositor
gives it, and SHALL name, on standard error, the ones that keymap cannot produce,
since such a binding silently does nothing. The check SHALL run whenever a keymap
is installed, so a layout switch is covered too, and SHALL stay silent when every
binding can be produced.

#### Scenario: A layout without a key for a binding

- **WHEN** a session is configured with a binding whose keysym the compositor's
  keymap has no key for
- **THEN** one line on standard error names that binding and the option it came
  from, and the session carries on

#### Scenario: The key that ends the session

- **WHEN** the keymap cannot produce the key bound to `quit`
- **THEN** that binding is among those named

#### Scenario: A layout that can produce everything

- **WHEN** every configured binding can be produced by the keymap
- **THEN** nothing is written to standard error

### Requirement: A traced session reports what it saw

With `-t` / `--trace`, a session SHALL print to standard error what it observed:
changes of keyboard focus, each key with the keysym it resolved to, each keymap it
installed, each mode change, and a line for every turn of its event loop that
produced an effect or had no time left to wait. Without the flag it SHALL write
nothing beyond its warnings.

#### Scenario: Tracing a session

- **WHEN** the session runs with `--trace` and the user presses a key
- **THEN** standard error shows the focus, the key and its keysym, and the loop
  turn that acted on it

#### Scenario: Not tracing

- **WHEN** the session runs without the flag
- **THEN** standard error stays empty

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
