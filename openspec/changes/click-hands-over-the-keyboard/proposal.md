# Proposal

## Why

A session holds the keyboard exclusively (`keyboard_interactivity=exclusive` on
its layer surface), and a compositor with an exclusive layer surface mapped keeps
the keyboard there: it hands the keyboard to whatever a click focuses and takes it
back immediately. The click itself arrives — the button lands on the window
underneath, which is the whole point of the empty input region — but the focus it
gave is gone before a key can use it.

The reported case is a panel whose text field follows `wl_keyboard`: `aside`
enables its text input on `wl_keyboard::enter` and disables it on `leave`, so
after `m` the field was focused and unfocused inside the same click and typing
went to the session instead. The same is true of any window: no application can
be left focused while a session is up.

A layer surface cannot take the keyboard back once it has given it up — there is no
request for it — so a click has to be the last thing a session does, and the grab
has to go before the button is pressed: while an exclusive layer surface is
mapped, a compositor keeps the keyboard with it, and a click that asks for the
focus of a panel is either refused (river) or granted and revoked when the surface
goes away (sway). Dropping the grab, clicking, and exiting is what leaves the
application holding the focus the click gave it.

## What Changes

- A click ends the session: the layer surface stops asking for the keyboard, the
  button is injected, then the session exits as it does for `Escape`, so the
  keyboard goes to what the click just focused.
- The three button keys, grid mode's warp-and-click, and hint mode's
  `click_after_warp` all reach the session through the same effect, so all of them
  end it. A drag is untouched: it holds a button rather than clicking, and ends
  with the toggle.
- Everything a session does before the click still happens first: the warp, the
  hand-over to normal mode, and the release of a held drag.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `pointer-session`: a click carries the session's exit with it, after the press
  and release have been injected.
- `mode-normal`: the three mouse button keys end the session; ending a drag with a
  click ends it too.
- `mode-grid`: warp-and-click still hands over to normal mode, and the session
  then exits.
- `mode-hint`: with `click_after_warp`, the click ends the session instead of
  leaving it in normal mode.

## Impact

- `src/session.rs` (`finish`: the exit is appended after the click, so every mode
  inherits it)
- `examples/cursor_probe.rs` (reports the keyboard focus it is given, which is how
  the hand-over is checked)
- `scripts/headless-check.sh` (the button checks need one session per button, and
  the click-through check now asserts the focus that is left behind)
- `README.md`
