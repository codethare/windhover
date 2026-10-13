# Tasks

## 1. Report the focus change

- [x] 1.1 `src/wl/seat.rs` tracks whether the seat has given this session the keyboard and reports each change once. The startup handshake flaps it for a few milliseconds, so the report is edge-triggered rather than a polled state. Verify: the keyboard unit tests still pass, and the timestamps of the flap were measured to be within the first 15 ms.

## 2. Act on it

- [x] 2.1 `src/session.rs` stops the repeat of every held key and releases a drag when the keyboard goes, and remembers when it went. Verify: the unit test drives a drag and a held key, loses the keyboard, and asserts the button is released and no repeat is left scheduled.
- [x] 2.2 The session ends once the keyboard has been gone for the grace period, with one line on standard error. Verify: the unit test covers the deadline, including a keyboard that comes back.
- [x] 2.3 One method holds or releases the drag button, used by both the drag effect and the loss path, so the two cannot drift apart. Verify: `cargo test` and the drag end-to-end check.

## 3. Cover it

- [x] 3.1 End-to-end: a second session takes the keyboard on sway and river, and the first ends itself saying why. niri keeps the keyboard with the session that holds it, so the check reports a skip there rather than pretending to cover it. Verify: `scripts/headless-check.sh` on all three compositors.
- [x] 3.2 Check that nothing else regressed: the drag, the clicks, the wheel and the mode round trips all still pass, since the repeat and the drag both changed. Verify: the full check run on sway, river and niri.
