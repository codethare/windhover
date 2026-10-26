# Design

## Context

See `proposal.md`. The pieces this touches:

- `Session::finish` is the one funnel every key effect passes through: the action
  table, the digit/multiplier keys, the grid cell keys, the hint labels, and the
  repeats. `--oneshot` already turns a hand-over into an exit there.
- `Effect::Click` is emitted by three places: normal mode's three button keys,
  grid mode's warp-and-click, and hint mode's `click_after_warp`.
- The driver injects a click with `Wayland::release_keyboard_and_click`: the
  overlay stops asking for the keyboard, and then a press and a release go out on
  the virtual pointer. The overlay keeps its empty input region, so the pointer was
  already the application's.
- The overlay's layer surface asks for an exclusive keyboard and cannot ask for it
  back once it has given it up.

## Goals / Non-Goals

**Goals:**

- A click leaves the application holding the keyboard.
- One rule, in the one place every mode reaches, so no mode can inject a click and
  stay up.

**Non-Goals:**

- Handing the keyboard back without ending the session. `zwlr_layer_shell_v1` has
  no request to re-take exclusive keyboard focus, so a session that gave it up
  could never take input again.
- A separate "click and hand over" key next to a click that stays. Two behaviours
  for one button is a distinction the user has to remember for a case — repeated
  clicks in one session — that a keyboard cursor rarely needs.
- Anything about what the application does with the click. Text focus, menus and
  drag handles are the application's own reaction to a button it received.

## Decisions

### The exit is appended in `finish`, not chosen at the call site

Every mode's click already passes through `finish`, so one rule there covers
`m`/`,`/`.` and grid's `Space` and hint's `click_after_warp` without three copies.
It also fixes the order once: the click is injected first, the exit second, so the
application has the button before the session ends.

`finish` also maps `Switch(Normal)` to `Exit` for `--oneshot`; the click's exit is
appended only when the effects do not already carry one, so a one-shot session
exits once.

### The grab goes before the button, the surface stays

A compositor keeps the keyboard with an exclusive layer surface while it is
mapped, so a click that arrives while the grab is held cannot leave a focus
behind: river drops the focus request outright, sway grants it at the press and
loses it when the surface goes away, and only niri remembers the on-demand surface
long enough to re-evaluate after it. `Wayland::release_keyboard_and_click`
therefore sets `keyboard_interactivity` to `none`, roundtrips — the roundtrip is
what makes that order real, rather than two requests a compositor may handle as
one batch — and only then presses the button. There is no request to take the grab
back, which is why a click ends the session.

Unmapping the overlay first would work for the focus and does not: `wlr-layer-shell`
clients cannot be sure the compositor re-derives the pointer focus from the next
motion it queues, and on river roughly one click in four then never reached the
surface underneath — a click is too expensive to lose. Dropping only the grab
leaves the pointer where it was.

### A drag is not a click

`v` toggles a held button (`Effect::Drag`), which is how a text selection or a
window drag is made with the direction keys, and the toggle is what ends it. A
drag does not focus a field for typing, so nothing about it needs the keyboard
handed over, and a session that ended on it could not finish the drag.

### Ending a drag with a click still releases first

`m` while a drag is in progress emits `Drag(false)` then `Click`. Both are in the
same batch, the release is handled before the click, and the exit is appended
after both, so no button is left held when the overlay goes away.

### The hand-over to normal mode still happens

Grid's `Space` and hint's warp still switch the session to normal mode before the
exit is appended. The mode change is what the pointer and the overlay would be in
if the session continued, and with `--oneshot` it was already the exit — so the
two paths now agree instead of differing by a flag.

## Risks / Trade-offs

- **A click cannot be repeated in one session.** Two clicks on the same window
  cost two hotkey presses. Releasing the keyboard is the only way a click's focus
  survives, and the compositor hotkey is one key.
- **`--oneshot` becomes almost redundant for a click.** Its warp-only hand-over
  still matters; its click case now ends the session either way.
- **A middle or right click also ends the session**, though only a left click
  focuses a field. Uniformity is worth more than the two clicks saved: a rule that
  says "a click ends the session" is one line in the README, and a rule that says
  which button does is a rule the user has to remember.
