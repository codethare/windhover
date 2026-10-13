# Proposal

## Why

A session kept the pointer to itself: the overlay covered the output and took
pointer events for as long as it was up, handing them over only for the instant a
click or a wheel event was injected. An application therefore never saw the
pointer move. Clicking a video worked, because a click is injected with the
overlay out of the way, but moving the cursor onto the video did nothing: its
controls appear on hover, and hover is a motion event the application never got.
Anything hover-dependent was dead — video controls, tooltips, menus, a table row
that highlights, a drag handle that appears — while the keyboard-driven cursor
looked like it was working, because it could click.

The mechanism was also expensive to own. Handing the pointer over and taking it
back needed a round trip between the two, and getting that order wrong is what
lost clicks on river, lost drags entirely, and made a drag fight the overlay for
the whole of its length.

## What Changes

- The overlay gives the pointer to whatever is under it once the session has
  learned where the pointer started, and never takes it back. The overlay is
  still mapped and painted: what it contributes is the ring and the labels, not
  the input.
- Everything that used to move the input region goes: the release/capture pair,
  the scoped suspension around a click or a wheel event, the drag's own handling
  of it, and the round trip that made the order safe. A click, a wheel event and a
  held button are injected directly, and land where the pointer is because nothing
  is in the way.
- The starting position is still observed — the handshake that tries outputs
  until the compositor reports where the pointer is already does this — because
  otherwise a session would have nothing to move relative to and the first key
  would teleport the pointer.
- The session tracks its own position from there, which the spec already allows:
  positioning must not depend on where the pointer started.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `pointer-session`: the click scenario says the overlay is removed before
  injecting the button event; it now says the overlay does not take pointer
  events at all, and a requirement states what follows for hover.

## Impact

- Code deleted: `Wayland::without_overlay`, `release_pointer` and
  `capture_pointer`, `Overlay::capture_pointer`, the drag's use of them, and the
  state that remembered the last requested position. Net: the interaction paths
  get shorter.
- Behaviour that changes for the user: hover works, and a physical mouse can
  reach the application under the pointer while a session is up, where before its
  clicks and its motion were swallowed by the overlay. The keyboard stays
  exclusive, so no keystroke reaches anything else.
- The session no longer follows the physical mouse: its own model of the pointer
  is authoritative, and a nudge of the physical mouse is corrected by the next
  movement key. That is a consequence of not owning the pointer, and it is what
  the spec's "does not depend on where the pointer started" already asks for.
