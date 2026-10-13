# Tasks

## 1. Hand the pointer over

- [x] 1.1 `src/wl/overlay.rs`: replace the take/restore pair with one call that gives the pointer to whatever is under the overlay, and say in the comment why the overlay is still the thing that draws the ring and the labels.
- [x] 1.2 `src/wl/mod.rs`: hand over once the starting position is known, which is what the existing per-output handshake in `place_overlay` already waits for, and also on the path where no output ever reported the pointer. Verify: the hover check fails when the call is a no-op, and the compiler points out the unused region.
- [x] 1.3 Delete what the mechanism needed: the scoped suspension, the release/capture pair, the drag's use of them, and the remembered last-requested position. Verify: `cargo build` and the whole suite, since four of this session's bugs lived in that machinery.

## 2. Inject directly

- [x] 2.1 A click, a wheel event and a held button are injected without moving the input region first. Verify: the click, button, drag and wheel checks on all three compositors — the same checks that caught the bugs in the old mechanism.

## 3. Cover hover

- [x] 3.1 New check: a real window sees the pointer move when the session moves it. This is the reported bug, and it fails against the old behaviour.
- [x] 3.2 New check: after a drag ends the window still sees the pointer move, which is the regression the old design would reintroduce. Its assertion is relative, not a jump to an edge, because a compositor that tiles the window into a column puts the output's edges outside it.
- [x] 3.3 The scroll check moved its probe onto before scrolling, since it used to be given a position by the suspension it no longer gets.
