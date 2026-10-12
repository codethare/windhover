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
```

| compositor | result | notes |
|---|---|---|
| **sway** 1.12 | 26/26 | headless backend, `WLR_RENDERER=pixman` |
| **river** 0.5.0-dev + tailrace | 26/26 | river is only the compositor; the window management policy comes from the window manager given to `-c` |
| **niri** 26.04 | 26/26 | no headless backend of its own, so it is run nested inside the headless sway; it picks the winit backend and the output is a window there |

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

### river

- One implementation each of `zwlr_layer_shell_v1` and
  `zwlr_virtual_pointer_manager_v1`, both behaving as wlroots does; exclusive
  keyboard focus on the overlay layer works, so river needs no special casing.
- river has no IPC socket, so the check just waits for its Wayland socket.

## Requirements from a real session

The headless runs above cover the protocol behaviour. Two things can only be
checked with a screen, a real keyboard and windows on it:

- whether a click reaches the application under the overlay — the suite verifies
  the button arrives at a surface *below* the overlay on the bottom layer, which
  is the same mechanism, but a real application's opinion is worth having;
- how the overlay looks over real content. `scripts/capture.sh` renders one
  frame per mode to PNG against a headless compositor, which is enough to catch
  layout and colour mistakes but not to judge them.
