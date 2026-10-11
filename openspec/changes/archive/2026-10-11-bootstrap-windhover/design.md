# Design

## Context

An empty repository. The three reference implementations in `/tmp/refs` (waynav
in C with meson, warpd in C, stochos in Rust) were read in full, and the
constraints of this machine and of the target environments were confirmed:

- Wayland lets a client neither register global hotkeys nor grab the keyboard.
  The only workable shape is **a compositor hotkey launching a short-lived
  process** plus a `zwlr_layer_shell_v1` overlay layer with
  `keyboard_interactivity=exclusive` for the keyboard. All three references work
  this way.
- There are two ways to position the pointer absolutely:
  `zwlr_virtual_pointer_v1::motion_absolute` (no privileges) and a uinput device
  with absolute axes (needs `/dev/uinput`, and libinput applies pointer
  acceleration on top, so the result lands somewhere other than requested). warpd
  takes the uinput route and documents the limitations; waynav and stochos use the
  protocol.
- sway, niri and river are wlroots-based or implement the wlr protocols;
  `zwlr_virtual_pointer_manager_v1` is version 2, so
  `create_virtual_pointer_with_output` is available but the binding version still
  has to be checked.
- This machine: rustc 1.98.1, Arch Linux, jj 0.45.1.

## Goals / Non-Goals

**Goals:**

- One binary, no daemon, no IPC, no state between invocations; process lifetime is
  one interaction.
- The three modes share one event loop, one renderer, one theme and one state
  machine, so the interaction and the appearance are consistent by construction.
- Every direct dependency auditable: no unfixed RustSec advisory, no public
  security incident, no C or C++ runtime dependency beyond `libxkbcommon`.
- No blur at niri's fractional scale, since niri is the primary target and enables
  it by default.

**Non-Goals:**

- X11, macOS or Windows backends.
- A resident hotkey listener (the compositor binding owns that).
- Covering every output at once: the overlay is created on the output the pointer
  is on at startup.
- Building a text layout engine from scratch (no kerning, bidi or ligatures).
- Element-level hints in the default build.
- Any target discovery other than the accessibility tree (no computer vision, no
  compositor-specific IPC).

## Decisions

### D1. Session model: a short-lived process with an in-process mode machine

One process runs one session, but may move between normal, grid and hint mode
freely. The compositor hotkey chooses only the **entry mode**.

- Alternative A: one short-lived process per mode (warpd's Wayland usage).
  Rejected: a hint match has to hand over to normal mode for fine adjustment, and
  separate processes cannot keep the overlay and the keyboard grab continuous;
  rebuilding the overlay on every switch also flickers.
- Alternative B: a daemon with IPC (warpd's X11 usage). Rejected: on Wayland the
  hotkey still has to be bound in the compositor, so a daemon buys only caching
  while costing a resident process, an IPC attack surface and lifecycle
  management.

### D2. Protocols and backend choices

| need | chosen | alternative and why not |
|---|---|---|
| overlay layer | `zwlr_layer_shell_v1` (Overlay layer, all four anchors, `exclusive_zone=-1`, `keyboard_interactivity=Exclusive`) | `ext_layer_shell` is not implemented widely enough in the target compositors yet |
| absolute pointer positioning, buttons, wheel | `zwlr_virtual_pointer_v1` | uinput: needs privileges and is subject to pointer acceleration |
| pixel transport | `wl_shm`, double buffered | dma-buf or Vulkan: a GPU stack is not worth it for a few strings of labels |
| key resolution | `wl_keyboard`'s keymap plus `xkbcommon` | hard-coded keycodes do not survive a layout change |
| fractional scaling | `wp_fractional_scale_v1` plus `wp_viewporter` | the integer `wl_output.scale` alone (stochos' approach) is visibly blurry at niri's default scale |

The version of `zwlr_virtual_pointer_manager_v1` that was bound decides how the
pointer is created: at version 2 or above,
`create_virtual_pointer_with_output` binds it to the target output, which is what
makes positioning correct with more than one screen; below that it falls back to
`create_virtual_pointer`.

### D3. Coordinate space and scaling

There is exactly one coordinate space outside the renderer: **logical pixels**,
which is what the configuration and the specs are written in. The overlay's
buffer is sized `ceil(logical × fractional scale)` and the overhang is trimmed by
the viewporter's destination rectangle, so the pixel mapping stays 1:1. If the
compositor offers neither fractional-scale nor viewporter, the session falls back
to the integer scale and warns on standard error: the picture may be slightly
soft, the positioning is unaffected.

### D4. Rendering

`tiny-skia` (a `Pixmap` with rounded rectangles, strokes and antialiasing) plus
`swash` for glyph rasterization and metrics. The family name is resolved through
the system's own font database: `fc-match -f %{file} <family>` first, and a probe
list of well-known paths when `fc-match` is not installed.

- `tiny-skia::Pixmap` is premultiplied RGBA8888, while the buffer is
  `wl_shm::Format::Argb8888` — a `0xAARRGGBB` word, which on little-endian is the
  bytes B, G, R, A. Red and blue are therefore swapped relative to the renderer,
  and `canvas::to_canvas` is the single place that reconciles them.
- The theme (colours, corner radius, padding, stroke widths, font family and
  sizes) lives in one place, and the three modes call only those primitives. That
  is where "the design is consistent" actually happens.
- Alternative: `ab_glyph` plus `fontdb` — both are built on `ttf-parser`, which
  the author has declared unmaintained (RUSTSEC-2026-0192), so they fail
  dependency admission.
- Alternative: `fontique` — it also removes `ttf-parser`, but pulls in ICU and
  `memmap2`; resolving a family name to a file is one step, and the system's own
  `fc-match` is smaller.
- Alternative: cairo (warpd's choice, brings a C dependency), `cosmic-text` (a
  whole layout stack), `font8x8` (bitmap glyphs, nothing like the intended
  appearance).

### D5. Layering of hint target sources

The default source is a **regular lattice** (equivalent to warpd's hint mode:
labelled targets laid out across the output at a configured spacing) and needs no
extra dependency. The element source sits behind the opt-in `atspi` feature and
implements the same `HintSource` trait:

- When the element source returns candidates, their bounding-box centres are used;
  when it returns nothing, cannot connect, or exceeds its time budget, hint mode
  falls back to the lattice silently. The fallback path shares the label
  assignment and filtering.
- Alternative: computer vision (stochos' `hint/cv.rs`) needs image-processing
  dependencies, which the dependency policy rejects. Alternative: compositor IPC
  (sway's `get_tree`, `niri msg`) reports windows rather than elements and ties the
  implementation to one compositor.

**Label assignment**: fixed-length, prefix-free labels — alphabet
`asdfghjkl` by default, length `L = ceil(log_|alphabet|(N))` for `N` targets,
generated in target order. A fixed length is prefix-free for free, and filtering
keeps every label the typed prefix is a prefix of. Alternative: mixed lengths
(warpd's approach), where filtering has to keep both "an exact match" and "still a
prefix of something", which is easier to get wrong.

### D6. Grid algorithm

The active region is a `Rect`. Each cell key shrinks the region to that cell,
places the cursor at its centre and pushes the previous region onto a history
stack. Once the region is narrower than the configured minimum in either
direction, grid mode hands over to normal mode. The movement keys shift the whole
region by its own size, clamped inside the output.

### D7. Event loop and key repeat

Single-threaded. `wayland-client`'s `EventQueue::blocking_dispatch` cannot take a
timeout, so the loop drives the connection itself: `prepare_read()` exposes the
connection fd, `rustix::event::poll` waits on it with a timeout derived from the
next due key repeat, and `dispatch_pending()` drains what arrived.

Key repeat is timed here rather than taken from the compositor's
`wl_keyboard.repeat_info`, because that only arrives when focus moves and because
grid and hint keys must not repeat while normal-mode movement keys must.

A modifier may still be held from the hotkey when the session starts. The held
keys reported by `wl_keyboard::Enter` are seeded into the key state so that the
hotkey's own modifiers are not mistaken for input.

### D8. Configuration and CLI parsing: by hand, no parsing crate

The configuration is a line-oriented `<option>: <value>` file and the command line
is four flags plus `--help` and `--version`. Both together are about 100 lines by
hand, where `serde` + `toml` + `clap` would bring in a dozen transitive
dependencies. With "dependencies under strict control" as a hard constraint, hand
writing is the smaller total cost.

### D9. Dependency list

Runtime: `wayland-client`, `wayland-protocols` (`client`), `wayland-protocols-wlr`
(`client`), `xkbcommon` (`default-features = false`: only parsing is needed, not
its `memmap2`-based fd convenience API), `tiny-skia` (`default-features = false`
to drop the unused PNG codec), `swash`, `rustix` (`event` and `fs`), `libc`
(signal handlers), `anyhow`.
Opt-in `atspi` feature: `atspi` (and through it `zbus`) plus `futures-executor`.

### D10. Module layout

```
src/
  main.rs           argument parsing, wiring, exit codes
  config.rs         line-oriented parsing, defaults, binding tables
  keys.rs           keysym name to action
  session.rs        the mode state machine and event dispatch
  mode/{normal,grid,hint}.rs
  hint/{lattice,labels,source}.rs   (+ atspi.rs, behind the feature)
  wl/{mod,overlay,seat,pointer,output}.rs
  render/{canvas,theme,text}.rs
```

### D11. Project name

`windhover` — the kestrel, the falcon that hovers in the wind, locks onto its
target and drops. It belongs to the same family of names as `stochos` (Greek for
"target"), `waynav` and `warpd`: one word that describes aiming, rather than a
compound like `keyboardmouse`. The name was free on crates.io, checked against
the crates.io API candidate by candidate. Renaming touches the crate name, the
configuration path and the documentation only; no spec behaviour changes.

### D12. Version control

jj. Once the first change was in place a bookmark was created so later work has
somewhere to hang.

## Risks / Trade-offs

- [Whether river honours `keyboard_interactivity=exclusive` was unverified] →
  measured with a minimal probe under river with a target window manager. It
  works, so river needs no special casing; had it not, river would have been
  documented as experimental rather than described as supported.
- [`motion_absolute`'s coordinate space (relative to the output, or the global
  extents) differs between compositors] → the session derived extents from the
  output record and got a 1×1 extent when the compositor chose the output for it,
  which pinned the pointer to a corner. Extents now come from the layer surface
  configure, and landing positions are asserted on all three compositors.
- [The fractional-scale path needs two protocols at once] → with either missing
  the session falls back to the integer scale and warns; positioning accuracy is
  unaffected, only sharpness.
- [A fullscreen shm buffer is about 30 MB × 2 at 4K with 1.5x scaling] → buffers
  are allocated only when a mode needs them, and a small ring is reused across
  frames rather than re-mapped per key.
- [AT-SPI pulls in roughly 65 transitive crates] → off by default, so only builds
  that ask for it are affected.
- [Hand-written CLI and config parsing miss details easily] → both have unit
  tests (parse results, error handling), and the scenarios in the specs cover
  them.
- [Modifiers held from the hotkey at startup] → see D7: the held keys from
  `wl_keyboard::Enter` are seeded into the state, or the first `h` after
  `Mod4+Mod1+x` could be misread.
- [Destroying the overlay before a click causes a focus change and a possible
  pointer-enter race] → the order stochos already validated is used: destroy the
  layer surface, roundtrip, inject motion and the button, then rebuild the
  surface. The rebuild path is shared rather than duplicated.

## Migration Plan

A new project, so there is nothing to migrate. Order of work: scaffold and
dependency baseline → the smallest interactive session (normal-mode movement) →
grid → hint (lattice) → configuration and theme → documentation and the
compositor runs. Rolling back means discarding the change.

## Open Questions

- Whether output hotplug during a session needs handling at all. The design
  assumes the output layout is stable for one invocation; if it changes, the
  session is rebuilt afterwards.
