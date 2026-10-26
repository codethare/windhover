# Proposal

## Why

A session can hold the left button down with `v`, and every path that ends the
drag has to release it: the second `v`, any mouse button, the keyboard going
away, the session ending. A mode change was not one of them. `Session::switch`
took the drag out of normal mode's own state and applied the release to itself,
where the driver never saw it, so `v` then `g` left the window dragging with the
button still held. `Escape` and the paths out of the loop (a signal, the overlay
closed by the compositor, the keyboard taken away) had the same hole: the session
ended and the button stayed down in the application.

The behaviour the code was written for is what this change makes real: a drag
belongs to normal mode, and leaving normal mode gives the button up. The
`mode-normal` dragging requirement says the opposite in one place — its "Moving
during a drag" scenario keeps the drag alive "by switching to grid mode" — and
that was only ever true because the release was being dropped.

## What Changes

- `Session::apply` hands back the effects it produces, and `Session::finish` puts
  them in front of the batch: a release has to reach the driver before the press
  that follows it.
- `Session::switch` returns the drag release instead of applying it, and
  `switch_to` asserts a fresh session has nothing to release.
- `Action::Quit` releases the drag ahead of `Effect::Exit`.
- `Session::leaving` is what `run` sends on every other way out — a signal, a
  closed overlay, the keyboard going away — before the overlay is torn down.
- The `mode-normal` dragging requirement drops "by switching to grid mode" and
  gains a scenario: a mode change releases the button.

## Cost

A drag no longer survives into grid mode, so "warp while holding the button" is
not a single-session gesture any more: while dragging, the pointer moves with the
direction keys and the edge jumps. In exchange the session can never inject a
press on top of a held button, which is what grid's warp-and-click would do if it
inherited one.

## Impact

- `src/session.rs`: `apply`, `switch`, `switch_to`, `act`, `finish`, `leaving`,
  and the driver's exit path in `run`; four session tests.
- The `mode-normal` spec delta in this change.
- No configuration, no dependency, nothing written to disk.
