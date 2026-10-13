# Proposal

## Why

A session holds the keyboard exclusively for as long as its overlay is up, and
everything about it assumes it keeps holding it: a held direction key is stopped
by the key's release, a drag is ended by the key that toggles it, and the session
itself is left by `Escape`. When the compositor takes the keyboard away mid-flight
— a button press on a window is enough to move focus — none of those arrive. The
observed result is a pointer that keeps travelling in the direction of a key that
is no longer held, a window left dragging with the button down, and a session
that cannot be dismissed at all, so it has to be killed from another terminal.

## What Changes

- A lost keyboard stops what depends on keys: the repeat of a held key, and a
  drag, which releases its button instead of leaving a window dragging forever.
- A keyboard that does not come back within half a second ends the session, which
  removes the overlay and writes one line to standard error saying why. Nothing
  can be pressed into a session without the keyboard, so ending is the only
  outcome that does not need a signal from outside to clean up.
- The grace period exists because the focus flaps for a few milliseconds during
  the startup handshake, which must not be mistaken for a loss.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `pointer-session`: a new requirement for what a lost keyboard does, next to the
  exclusive-keyboard requirement it follows from.

## Impact

- Code: `src/wl/seat.rs` reports focus changes, `src/session.rs` acts on them and
  holds the deadline, `src/wl/mod.rs` gains one method for holding or releasing
  the drag button so both paths share it.
- A second session started while one is running now takes the keyboard from the
  first, which ends it. On sway and river this is what the compositor does; on
  niri it keeps the keyboard with the session that has it, so nothing changes
  there. Either way the sessions no longer pile up.
- No change to a session that keeps its keyboard, which is every session that
  nothing else contests.
