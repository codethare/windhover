# Proposal

## Why

The README tells the user to bind the mode flags to compositor hotkeys, and the
pointer-session spec calls a session a short-lived process launched that way. The
process does not actually end when the mode it was started in is finished: a
`--hint` session warps to the label and then stays alive in normal mode, holding
the keyboard exclusively and keeping its fullscreen overlay (with its cursor
ring) mapped. Bound to a hotkey, every activation therefore stacks another
session: each one is invisible except for its ring, so the screen accumulates
rings, and Escape has to be pressed once per leftover session before the last
one exits.

warpd, which this project follows, answers this with `--oneshot`: exit as soon as
the mode is complete instead of dropping into normal mode.

## What Changes

- Add `-o` / `--oneshot`: the session exits when the mode it started in hands
  over, rather than switching to normal mode. The hand-over's own work — the
  warp, and the click when `click_after_warp` is on — still happens first.
- Refuse `--normal --oneshot`: normal mode never hands over, so the combination
  would silently do nothing.
- Nothing else changes. Without the flag a session still returns to normal mode
  and stays alive, and a single session still exits on one Escape.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `pointer-session`: a new requirement for the one-shot lifetime and the
  argument that is refused with it.

## Impact

- Code: `src/main.rs` parses and validates the flag, `src/session.rs` turns the
  hand-over into the end of the session in one place, which covers grid and hint
  because both hand over through the same effect.
- README and the hotkey snippets: a `--hint --oneshot` example, since a hotkey
  per activation is the workflow the flag exists for.
- No change to any mode's behaviour, and no change for a session started
  without the flag.
