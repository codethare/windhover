# Compositors

`windhover` needs two protocols and nothing else:

| protocol | used for |
|---|---|
| `zwlr_layer_shell_v1` | the fullscreen overlay surface, and exclusive keyboard focus |
| `zwlr_virtual_pointer_manager_v1` | absolute pointer positioning, buttons, the wheel |

`wp_fractional_scale_v1` and `wp_viewporter` are used when present and skipped
when not; `zxdg_output_v1` is used for output geometry when present.

Nothing else is required: no daemon, no root, no `/dev/uinput`, no
compositor-specific IPC. That is deliberate — the first three implementations
this project learned from each pay for one of those in a different way.

## How it was tested

`scripts/headless-check.sh <compositor>` runs the whole suite against a real
compositor on the headless wlroots backend. It drives windhover through a virtual
keyboard and measures the cursor with a second client, so the assertions are on
where the pointer actually landed rather than on "it did not crash":

```sh
sh scripts/headless-check.sh sway
sh scripts/headless-check.sh river
sh scripts/headless-check.sh niri
sh scripts/dual-output-check.sh     # sway only: one output per check cannot show it
```

The counts below are the assertions that run on each compositor: a check whose
premise that compositor cannot exhibit is skipped and counted out. CI runs the
sway column only, so the on-demand panel check — which sway skips — is a manual
gate on river or niri.

| compositor | result | notes |
|---|---|---|
| **sway** 1.12 | 47/48 | headless backend, `WLR_RENDERER=pixman`; the on-demand panel check is skipped (the hover is the hand-over there) |
| **river** 0.5.0-dev + tailrace | 48/48 | river is only the compositor; the window management policy comes from the window manager given to `-c` |
| **niri** 26.04 | 46/48 | no headless backend of its own, so it is run nested inside the headless sway; it picks the winit backend and the output is a window there. Two checks are skipped: a seat always has a keyboard, and it never takes the keyboard from a session |

## Several outputs

The virtual pointer addresses the compositor's layout, not one output:
`motion_absolute` is normalised over the box the outputs occupy, and on all three
compositors that box starts at the leftmost output's corner rather than at
(0, 0). A session learns which output the overlay is on from `wl_surface::enter`,
and moves the overlay-local point to that output's place in the box; without
`zxdg_output_v1` no output reports a position, and an overlay on an output away
from the box's corner cannot be aimed at — which the session says once on stderr.

`scripts/dual-output-check.sh` is the check for it: headless sway with both
outputs away from the origin, where a missing translation puts the pointer in the
empty space before them. `scripts/headless-check.sh` cannot see this, because one
output at the origin needs no translation.

## Things that differ per compositor

These came out of the runs above; they are worth knowing before filing a bug.

### niri

- **A seat always has a keyboard.** niri advertises the capability whether or
  not any keyboard is attached, so windhover never takes the "no keyboard" exit on
  niri. The check for that path is skipped there.
- **`motion_absolute` normalised to 1.0 is off the surface.** Asking for `x ==
  output_width` puts the pointer past the right edge, so the session addresses
  `width - 1` instead. Without that, "jump to the right edge" moved the pointer
  off the output and the compositor stopped reporting its position at all.
- Output geometry is reported for the nested window, so the logical size is
  whatever the host window happens to be — 1276x691 in the runs above rather
  than 1280x720. Nothing in windhover assumes a resolution, but a configuration
  with `step:` in absolute pixels and a `hint_cell:` that is too small on a
  small output will feel different.

### sway

- wlroots does **not** re-run pointer focus for a freshly mapped layer surface.
  A client that maps a surface under a stationary pointer gets no `enter` until
  something moves. windhover works around this for its own overlay placement by
  committing an empty frame and waiting; the test probe has to be started before
  the keys that move the cursor.
- Shift pressed with the virtual keyboard does not update the seat's modifier
  state, so `Mod4+Mod1+x`-style hotkeys that need a shifted symbol rely on the
  compositor's own key handling, not on the injected one. This is an artefact of
  driving the session through a virtual keyboard in the tests, not something a
  real user hits.
- An **on-demand** layer surface takes the keyboard as soon as the pointer enters
  it, where niri waits for a click. So on sway, moving onto a panel that asks for
  on-demand keyboard focus ends the session at that moment, before any click is
  injected: the session lost the keyboard, which is its own exit condition. The
  hand-over is the hover there, not the click, so the panel check in the suite is
  skipped on sway — and a click aimed at such a panel never reaches the widget
  under it, only the focus moves. Separately, sway 1.12 drops that focus again
  when another layer surface goes away, which upstream fixed after 1.12
  (`layer_shell: fix focus stealing for ON_DEMAND surfaces`); that is a second
  defect, and not why the check is skipped.

### river

- One implementation each of `zwlr_layer_shell_v1` and
  `zwlr_virtual_pointer_manager_v1`, both behaving as wlroots does; exclusive
  keyboard focus on the overlay layer works, so river needs no special casing.
- river has no IPC socket, so the check just waits for its Wayland socket.

## Requirements from a real session

The headless runs above cover the protocol behaviour. Two things can only be
checked with a screen, a real keyboard and windows on it:

- whether a click reaches the application under the overlay and leaves it with
  the keyboard — the suite verifies the button arrives at a surface *below* the
  overlay and that what it landed on is still focused afterwards: an ordinary
  window, and a layer surface that asks for on-demand keyboard focus (skipped on
  sway, where the hover hands the keyboard over before a click exists). That is
  the same mechanism, but a real application's opinion is worth having;
- how the overlay looks over real content. `scripts/capture.sh` renders one
  frame per mode to PNG against a headless compositor, which is enough to catch
  layout and colour mistakes but not to judge them.
