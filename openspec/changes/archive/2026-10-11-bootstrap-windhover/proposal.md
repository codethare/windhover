# Proposal

## Why

Wayland gives clients no way to grab the keyboard globally, and none of the
existing keyboard-driven pointer tools fit the target environments at once:
warpd's Wayland port has only a lattice hint mode and looks dated, waynav has
only grid mode and carries keynav's config syntax as legacy, and stochos is
built around Hyprland with a very large dependency graph. Users of sway, niri and
river need a short-lived replacement with all three modes, a dependency set that
can be audited, and an interface that looks like one thing.

## What Changes

- A new Rust project, `windhover` (named after the kestrel: the falcon that
  hovers while it aims): one binary, no daemon, no IPC, no state between
  invocations.
- **pointer-session**: a fullscreen overlay layer through `zwlr_layer_shell_v1`
  with `keyboard_interactivity=exclusive` to own the keyboard; software rendering
  through `wl_shm`; absolute pointer positioning, buttons and the wheel through
  `zwlr_virtual_pointer_v1`. Exiting the process releases everything.
- **mode-normal**: warpd's normal-mode semantics — `hjkl` to move, a numeric
  multiplier prefix, `H`/`M`/`L` to jump to the left edge, the middle and the
  right edge, `m` `,` `.` for the left, middle and right buttons, `v` to drag,
  and `g`/`x` to switch to grid or hint mode.
- **mode-grid**: waynav's grid-mode semantics — start at 2x2, press a key to
  shrink into that quadrant, land the pointer in the centre of the region, and
  support panning and undo.
- **mode-hint**: warpd's hint-mode semantics — lay labelled targets over the
  screen, filter by typed prefix, and on a unique match warp and return to normal
  mode (optionally clicking). Element-level detection (the accessibility tree, as
  stochos does) is an opt-in cargo feature so the default build pulls in no
  D-Bus dependency.
- **configuration**: warpd-style `~/.config/windhover/config`, one
  `<option>: <value>` per line, with keys that can be rebound or unbound.
- Documentation: the README carries hotkey bindings for sway, niri and river
  (`Mod4+Mod1+x/c/g` → `windhover --hint/--normal/--grid`).
- Appearance: one macOS-flavoured visual language — rounded corners, translucency,
  a system font, restrained strokes and shadows — shared by grid, hint and normal
  mode.
- Version control uses jj (jujutsu), not git.

## Capabilities

### New Capabilities

- `pointer-session`: the lifecycle of one overlay session — startup arguments,
  Wayland protocol negotiation, presenting the overlay, holding the keyboard,
  absolute pointer positioning, injecting buttons and the wheel, and exiting with
  nothing left behind.
- `mode-normal`: movement, the multiplier, edge jumps, button actions and mode
  switching in normal mode.
- `mode-grid`: how grid mode partitions, shrinks, pans and undoes a region, and
  how it hands over.
- `mode-hint`: how hint mode generates labels, filters by prefix, positions on a
  unique match, and degrades when the element source has nothing to offer.
- `configuration`: where the file lives, its syntax, which options exist, and how
  the command line, the file and the defaults take precedence.

### Modified Capabilities

(None; the project has no existing specs.)

## Impact

- New code: the whole crate, from nothing.
- Direct dependencies, each audited against the RustSec database and against
  crates.io maintenance status, with no unfixed advisory and no public security
  incident: `wayland-client`, `wayland-protocols-wlr`, `xkbcommon` (bindings to a
  C library), `tiny-skia`, `swash`, `rustix` (`poll` and `memfd_create` only),
  `libc` (signal handling), `anyhow` (>= 1.0.103).
- Rejected outright: `image` and `imageproc` (real CVEs and unfixed advisories),
  and `ab_glyph`/`fontdb` (their underlying `ttf-parser` is declared unmaintained
  with no fixed release).
- The opt-in `atspi` feature pulls in `atspi`/`zbus`, roughly 65 further crates;
  it is off by default.
- Runtime requirements: a compositor implementing `zwlr_layer_shell_v1` and
  `zwlr_virtual_pointer_v1` (sway, niri and river all do); no root, no
  `/dev/uinput` permission.
- Out of scope: X11, macOS and Windows backends, a resident daemon, and any state
  shared between invocations.
