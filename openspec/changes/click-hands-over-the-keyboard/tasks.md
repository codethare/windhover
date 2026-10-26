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
