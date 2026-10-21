# mode-normal Specification

## Purpose
Defines what normal mode does: move the pointer short distances continuously,
move it further with a multiplier, jump to the screen edges, inject mouse actions
at the current position, and switch to the other modes. It is where grid mode and
hint mode hand over, and the mode a session starts in.

## Requirements

### Requirement: Directional movement

Normal mode SHALL provide four direction keys (`h` left, `j` down, `k` up, `l`
right by default) that move the pointer by the current step, which defaults to a
derived pixel value and can be changed in the configuration. Holding a key SHALL
trigger auto-repeat, and the distance covered per repeat SHALL grow the longer
the key stays down, up to a bounded multiple of the step, so that a hold
accelerates and then levels off rather than growing without end. Movement with
the fast modifier held SHALL move twice the step, and SHALL accelerate while held
the same way. Releasing a key SHALL reset it to the base step.

#### Scenario: A single key press

- **WHEN** the user presses `l` once
- **THEN** the pointer moves one step to the right, without leaving the screen

#### Scenario: Reaching the screen edge

- **WHEN** the pointer is already at the rightmost column and the user keeps
  pressing `l`
- **THEN** the pointer stays at that edge, without wrapping and without going out
  of bounds

#### Scenario: Holding a key

- **WHEN** the user holds `j`
- **THEN** the pointer keeps moving down until the key is released, covering more
  ground per repeat the longer it is held

#### Scenario: The hold levels off

- **WHEN** the user holds a direction key for longer than the growth takes to
  reach its limit
- **THEN** the distance per repeat stops growing and stays at the limit until the
  key is released

#### Scenario: Pressing again after a hold

- **WHEN** the user holds a direction key, releases it, and presses it again
- **THEN** the movement starts again from the base step

#### Scenario: Moving with the fast modifier

- **WHEN** the user presses `l` with the fast modifier held
- **THEN** the pointer moves two steps to the right

#### Scenario: Holding with the fast modifier

- **WHEN** the user holds `l` with the fast modifier held
- **THEN** every repeat moves twice what the same repeat would move without it,
  and the repeats accelerate as they do without it

### Requirement: Numeric multiplier prefix

Normal mode SHALL accept consecutive digit keys as a multiplier prefix, applied to
the next movement action. The multiplier SHALL be cleared once the action runs,
and SHALL be bounded to avoid overflow.

#### Scenario: Moving with a multiplier

- **WHEN** the user presses `1`, then `0`, then `j`
- **THEN** the pointer moves down ten steps

#### Scenario: The multiplier is cleared

- **WHEN** the user presses `5`, performs one movement, then presses `j`
- **THEN** the second movement moves one step

#### Scenario: The multiplier is bounded

- **WHEN** the user types more digits than the prefix length allows
- **THEN** the multiplier is clamped at the bound, with no arithmetic overflow or
  unusual behaviour

#### Scenario: A dangling prefix

- **WHEN** the user types a digit prefix and then presses a key that is not a
  movement
- **THEN** the multiplier is discarded and that key does what it normally does

### Requirement: Screen edge jumps

Normal mode SHALL provide three jump keys that move the pointer directly to the
left edge, the horizontal middle and the right edge of the target screen, leaving
the vertical position unchanged.

#### Scenario: Jumping to the left edge

- **WHEN** the user presses `H`
- **THEN** the pointer's x coordinate becomes the leftmost of the screen and its y
  coordinate is unchanged

#### Scenario: Jumping to the middle

- **WHEN** the user presses `M`
- **THEN** the pointer's x coordinate becomes the horizontal middle of the screen
  and its y coordinate is unchanged

#### Scenario: Jumping to the right edge

- **WHEN** the user presses `L`
- **THEN** the pointer's x coordinate becomes the rightmost of the screen and its y
  coordinate is unchanged

### Requirement: Wheel

Normal mode SHALL provide keys that scroll up and down (`u` and `d` by default),
whose step count SHALL be affected by the numeric multiplier, and which SHALL
repeat while held.

#### Scenario: Scrolling up

- **WHEN** the user presses `u`
- **THEN** one upward discrete wheel event is produced at the current pointer
  position

#### Scenario: Scrolling with a multiplier

- **WHEN** the user presses `3` and then `d`
- **THEN** one downward wheel event of three steps is produced

#### Scenario: Holding a scroll key

- **WHEN** the user holds `d` past the repeat delay
- **THEN** scroll events keep being produced at the same fixed rate as movement
  keys, until the key is released

### Requirement: Mouse button actions

Normal mode SHALL provide three keys that inject a left, middle and right click at
the current pointer position.

#### Scenario: Left click

- **WHEN** the user presses `m`
- **THEN** one left click occurs at the current pointer position, and the overlay
  does not intercept that click

#### Scenario: Middle and right click

- **WHEN** the user presses `,` or `.`
- **THEN** one middle or one right click occurs at the current pointer position

### Requirement: Dragging

Normal mode SHALL provide a drag toggle: while it is on, the left button stays
held and the pointer can be moved by any means. Pressing the toggle again, or
injecting any mouse button, SHALL end the drag and release the left button.

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
  afterwards, with no leaked button state

#### Scenario: Released when the session exits

- **WHEN** the user exits the session while a drag is in progress
- **THEN** the left button is released, leaving no stuck press behind

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

### Requirement: Normal mode reports its own state

The normal-mode overlay SHALL show the multiplier or step currently in effect, so
the size of a movement is visible, and SHALL stay visually minimal when nothing is
being pressed.

#### Scenario: Showing the multiplier

- **WHEN** the user has typed a numeric multiplier but not yet performed an action
- **THEN** the current multiplier is visible on the overlay

#### Scenario: Minimal when idle

- **WHEN** normal mode is idle
- **THEN** the overlay shows only a few elements that do not hinder reading what is
  underneath (a cursor marker and a status hint, for instance), and does not cover
  the bulk of the screen

### Requirement: Speed presets

Normal mode SHALL provide three speed presets that scale the step every movement
and scroll uses: fine, normal and coarse. The preset SHALL be session state
rather than a mode, so it survives a hand-over from grid or hint mode, and SHALL
multiply together with the held-key acceleration and the fast modifier rather
than replacing them. The overlay SHALL show the preset whenever it is not the
normal one, so a session cannot be left slow or fast without saying so.

#### Scenario: A fine preset shortens every move

- **WHEN** the user selects the fine preset and presses a direction key
- **THEN** the pointer moves a fraction of the step in that direction instead of
  a whole one

#### Scenario: A coarse preset lengthens every move

- **WHEN** the user selects the coarse preset and presses a direction key
- **THEN** the pointer moves several steps in that direction

#### Scenario: The preset survives a hand-over

- **WHEN** the user selects the fine preset, jumps through grid or hint mode, and
  then presses a direction key
- **THEN** the pointer still moves by the fine fraction of the step

#### Scenario: Returning to the normal preset

- **WHEN** the user selects the normal preset
- **THEN** a direction key moves exactly one step again, and the overlay stops
  showing a preset

#### Scenario: The preset is visible

- **WHEN** a preset other than the normal one is active
- **THEN** the status shown beside the cursor names it, and nothing is shown for
  the normal preset

#### Scenario: Holding a key from a preset

- **WHEN** the user holds a direction key while a preset is active
- **THEN** the movement still accelerates with the hold, starting from the
  preset's speed rather than from the step
