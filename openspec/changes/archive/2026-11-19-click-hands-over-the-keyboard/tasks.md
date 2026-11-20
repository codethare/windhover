# Tasks

## 1. The rule

- [x] 1.1 Append `Effect::Exit` after a click in `Session::finish`, and only when
      the effects do not already end the session, so `--oneshot` exits once
- [x] 1.2 Session tests: `m` is `Click` then `Exit`; grid's `Space` is warp, click,
      hand-over, exit; a one-shot click carries one exit

## 2. The check that it works

- [x] 2.1 Report the keyboard focus a probe is given (`examples/cursor_probe.rs`),
      so "the click left the window focused" is assertable
- [x] 2.2 The toplevel click check asserts the session ended and the window's last
      keyboard event is `enter`
- [x] 2.3 One session per button in the three-button check, each asserting the
      button that arrived, a clean exit, and no stderr

## 3. Documentation and the gate

- [x] 3.1 `README.md`: a click ends the session, and why; the hotkey note names a
      click next to `Escape`
- [x] 3.2 Run the gate — `cargo test`, `cargo deny check advisories bans`,
      `sh scripts/headless-check.sh` on sway, niri and river, with
      `TAILRACE` pointing at a built window manager

## 4. The grab goes before the button

- [x] 4.1 `Overlay::release_keyboard` sets `keyboard_interactivity` to `none` and
      commits; `Wayland::release_keyboard_and_click` roundtrips and only then
      injects the press and release, so the grab is gone before the button
- [x] 4.2 A panel check: the probe asks for on-demand interactivity, is clicked,
      and its last keyboard event has to be `enter` — skipped on sway, where the
      hover hands the keyboard over before a click exists. It fails on river
      without 4.1 (`last focus: none`)
- [x] 4.3 Unmapping the overlay instead of dropping the grab was implemented and
      rejected: on river roughly one click in four never reached the surface
      underneath, because the pointer focus is re-derived from a queued motion
- [x] 4.4 `sh scripts/headless-check.sh` on sway, niri and river: all checks passed
      (river four times, for the click that was being lost)

## 5. What the review changed

- [x] 5.1 `Effect::Click` ends the session in the driver, so "a click is the last
      thing" no longer rests on `finish` appending an exit that a future effect
      source could bypass; the append stays for the effect list the tests assert
- [x] 5.2 The reason beside `finish`'s append and beside the order test says what is
      true now (the grab is gone, not "the compositor takes it back")
- [x] 5.3 The grid and hint deltas say "the keyboard grab is given up", not "the
      overlay is removed", which the rejected alternative did
- [x] 5.4 No repaint after the click was tried and **rejected**: without that
      commit river loses the press when the session tears its surface down in the
      same flush (2 runs of 2 → 3 and 4 lost buttons; with the repaint, 10 runs
      clean). The repaint stays, with the measurement beside it
- [x] 5.5 The panel check asserts the session's exit status and empty stderr, and
      kills a leftover session on every failure branch
- [x] 5.6 The exclusive-keyboard requirement in `openspec/specs/pointer-session` is
      overridden by a delta: the grab is held for the overlay's life, except for the
      turn a click ends it (without that delta, archiving contradicts the code)
- [x] 5.7 Wording: "no request to ask for it back" became "nothing obliges a
      compositor to give it back"; the sway skip notes upstream's separate defect
      without borrowing it as the reason
