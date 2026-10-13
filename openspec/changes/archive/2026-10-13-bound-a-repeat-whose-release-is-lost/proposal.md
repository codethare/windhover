# Proposal

## Why

A repeat is stopped by its key's release, and everything about the session
assumes that release arrives. A compositor can swallow it and say nothing: river
does exactly that when a press on a selection turns into a drag-and-drop, and it
also stops delivering keys without sending a `leave`, so the session never learns
that anything happened. What is left is a pointer that travels in the direction
of a key nobody is holding until it pins at the edge, a window that keeps
dragging with the button down, a frame drawn as fast as the loop can manage, and
no key at all able to end the session: the only way out is a signal from another
terminal.

Nothing available to the client tells a lost release apart from a user who is
genuinely holding a key down, so the damage is bounded rather than the cause
detected.

## What Changes

- A hold that outlasts a fixed limit is treated as a lost release: the repeat
  stops, a drag's button is released, and the session ends with one line on
  standard error saying so. The limit is five seconds, which is far past what any
  movement needs: the pointer crosses the output in well under a second at the
  accelerated rate.
- A hold within the limit is untouched, and its release still stops it as before.
- A lost release that a compositor *does* announce, by taking the keyboard away,
  is still handled by the previous change and ends the session sooner.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `pointer-session`: a new requirement bounding a repeat whose release never
  arrives, next to the key resolution the repeat belongs to.

## Impact

- Code: `src/session.rs` holds the limit and the effect it produces; nothing else
  moves.
- The cost of the bound is that a genuine hold past five seconds ends the
  session. For a movement key that costs nothing, because the pointer cannot
  travel further once it reaches the edge; for a wheel key it costs one more
  press after about four screens of scrolling.
- The end-to-end check drives a `down:` with no `up:` through the virtual
  keyboard, so this is covered on every compositor rather than only on the one
  that swallows releases.
