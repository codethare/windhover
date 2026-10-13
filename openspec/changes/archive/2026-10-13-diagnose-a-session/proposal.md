# Proposal

## Why

The bugs that have cost the most time in this project were all things the client
could not see: a keycode offset that made every binding resolve to the wrong key,
a wheel event the compositor dropped, a pointer grab that swallowed clicks, a
keyboard taken away without a `leave`. Each was found by writing throwaway
instrumentation, reading it, and deleting it again — twice in one session — and
one of them (the keycode offset) was invisible to the project's whole test suite
because the test helper made the same mistake as the code.

Two gaps, then. A session cannot say when its own bindings are dead, and it
cannot say what it saw when something else goes wrong.

## What Changes

- A session checks its configured bindings against the keyboard layout it is
  actually given, and names the ones the layout cannot produce. The one that
  matters is `quit`: a layout that cannot produce the key that ends the session
  leaves a session nothing can leave.
- `-t` / `--trace` prints what the session sees: keyboard focus changes, every
  key with its resolved keysym, keymap loads, mode changes, and a line per turn
  that produced an effect or ran with no time to wait. A session without the flag
  writes exactly what it wrote before.
- The README says how to use the trace in a bug report.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `pointer-session`: two requirements, one for reporting bindings the layout
  cannot produce, one for the trace.

## Impact

- Code: `src/trace.rs` (a flag and a guarded line on standard error),
  `src/wl/seat.rs` (the keymap's reachable keysyms, and when it is loaded),
  `src/config.rs` (which bindings are missing from that set),
  `src/session.rs` and `src/main.rs` (the flag and the report).
- The binding check can only be as good as the keymap: it says nothing about the
  hardware, so a key on a keycode no physical key has still counts as reachable.
  That is deliberate — it catches the class of bug that a wrong keycode mapping
  produces, which is every binding moving at once.
- A warning nobody can act on would be noise, so the check must stay silent on a
  layout that can produce everything: the existing "sessions stayed quiet" check
  fails if it does not.
