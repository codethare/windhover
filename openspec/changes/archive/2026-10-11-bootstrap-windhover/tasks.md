# Tasks

## 1. Repository scaffold and dependency baseline

- [x] 1.1 Create the crate with `cargo init --name windhover`, with the repository root under jj (`jj status` works and `cargo build` passes).
- [x] 1.2 Write the dependencies from design D9 into `Cargo.toml` (`wayland-client`, `wayland-protocols`, `wayland-protocols-wlr`, `xkbcommon` with `default-features = false`, `tiny-skia` with `default-features = false`, `swash`, `rustix`, `anyhow`, `libc`) and declare an empty `atspi` feature. Verify: `cargo build` passes and `cargo tree -e normal` contains no `image`, `imageproc`, `memmap2`, `png` or `ttf-parser`.
- [x] 1.3 Create the module skeleton from design D10 (`config`, `keys`, `session`, `mode/*`, `hint/*`, `wl/*`, `render/*`). Verify: `cargo build` passes and the files under `src/` match the design.
- [x] 1.4 Establish the dependency audit gate: record the audit outcome (each direct dependency's advisory status) in `docs/dependencies.md` and a reproducible command for it. Verify: running the documented command reproduces the documented conclusion.
- [x] 1.5 Write `README.md`: the target compositors and the protocols they need, why a compositor hotkey is unavoidable, and bindings for sway, niri and river (`Mod4+Mod1+x/c/g` → `windhover --hint/--normal/--grid`). Verify: every command in the README can be copied and run on the target machine.

## 2. pointer-session: the smallest interactive session

- [x] 2.1 Implement the `wl` connection and registry discovery (compositor, shm, seat, output, layer shell, virtual pointer), reporting a named error and exiting non-zero when a required capability is missing. Verify: unit tests cover "missing capability gives an error naming the capability", and on a compositor lacking it the exit status is non-zero (matching `pointer-session`'s "the compositor lacks a required protocol").
- [x] 2.2 Implement the overlay layer: Overlay layer, all four anchors, `exclusive_zone=-1`, `keyboard_interactivity=Exclusive`. Verify: the overlay covers the screen, other windows do not change size, and keys no longer reach other windows (matching "workspace layout is untouched" and "keys are not passed through").
- [x] 2.3 Implement the shared-memory buffer (`rustix::fs::memfd_create` plus double buffering) and a flat fill. Verify: the overlay's colour is right at 4K with no tearing.
- [x] 2.4 Implement the key path: `wl_keyboard` keymap to xkb state to keysym name, seeding the modifiers that are already held when the session starts. Verify: with the hotkey's Super+Alt still held, keys still resolve, and releasing the modifiers performs no action (matching "bindings independent of layout" and "a key that cannot be resolved"). The keysym-to-action tables land with each mode in groups 3 to 5 and are checked key by key.
- [x] 2.5 Implement the virtual pointer: `motion_absolute` for positioning, `button` for press and release, `axis_source=Wheel` plus `axis_discrete` for the wheel, falling back between `create_virtual_pointer_with_output` and `create_virtual_pointer` by the bound version. Verify: unit tests cover the version branch, and the pointer can be positioned, clicked with any of the three buttons, and scrolled.
- [x] 2.6 Implement the exit path: the quit key, and destroying the layer surface and the virtual pointer before exiting on SIGTERM. Verify: after exiting, the overlay is gone, no layer is left behind, and the exit status is 0 (matching "the user exits" and "killed by a signal").
- [x] 2.7 Implement scaling: `wp_fractional_scale_v1` plus `wp_viewporter`, rounding the buffer size to the fractional scale and trimming with the viewporter; when a protocol is missing, fall back to the integer `wl_output.scale` and warn on standard error. Verify: unit tests cover the buffer size arithmetic, and text is sharp on niri at 1.5x while 1x is unchanged (matching "fractional scaling").

## 3. mode-normal

- [x] 3.1 Implement four-directional movement with the step and edge clamping. Verify: unit tests cover all four directions, no wrapping at the screen edge, and an unchanged coordinate when already at the edge (matching "directional movement").
- [x] 3.2 Implement the numeric multiplier prefix (accumulation, clamping, cleared after use, dangling prefix discarded). Verify: unit tests cover `10j` moving ten steps, two consecutive presses moving one step each, and an over-long prefix being clamped (matching "numeric multiplier prefix").
- [x] 3.3 Implement the `H`/`M`/`L` edge jumps keeping the vertical coordinate. Verify: unit tests cover the x coordinate of each jump (matching "screen edge jumps").
- [x] 3.4 Implement key repeat with its own timing (initial delay plus a fixed rate, for movement keys only). Verify: a unit test with an injectable clock asserts the repeat count and the first delay (matching "holding a movement key").
- [x] 3.5 Implement left, middle and right clicks, destroying the overlay before injection and rebuilding it after. Verify: the click reaches the layer below the overlay, at the expected position, and the session keeps running (matching "mouse button actions" and "the overlay does not swallow the click").
- [x] 3.6 Implement the drag toggle (`v`): holding the left button, releasing on a second press or on any button injection, and releasing when the session exits. Verify: unit tests cover the state transitions and the release on exit, and a drag in a real session works (matching "dragging").
- [x] 3.7 Implement `g`/`x` mode switching and the return from grid or hint mode to normal mode without rebuilding the overlay or the keyboard grab. Verify: the pointer position is continuous across a switch and the keyboard does not have to be re-acquired (matching "mode switching and exit").
- [x] 3.8 Show the multiplier and the step on the normal-mode overlay, leaving only a minimal set of elements when idle. Verify: the overlay shows the multiplier after typing a digit, and idle mode does not hinder reading what is underneath (matching "normal mode reports its own state"); confirmed by `scripts/capture.sh`.

## 4. mode-grid

- [x] 4.1 Implement the grid model (the active region, partitioning, shrinking to a cell, centring the pointer). Verify: unit tests cover the region and the pointer coordinate after one and after several shrinks (matching "initial partition" and "region shrinking").
- [x] 4.2 Implement the cell key table (2x2 → `q w a s` by default) and its validation. Verify: unit tests cover refusing to start with a diagnosable error when there are too few keys (matching "a custom partition" and "a partition with no matching key table").
- [x] 4.3 Implement the minimum-size threshold and the automatic handover to normal mode. Verify: unit tests cover the threshold, and in a real session shrinking to the end returns to normal mode where the user can keep adjusting (matching "reaching the minimum size").
- [x] 4.4 Implement region panning (shifting by the region's own size, clamped inside the output). Verify: unit tests cover panning and clamping at the edge (matching "region panning").
- [x] 4.5 Implement the region history stack and `BackSpace` undo (unchanged with nothing to undo). Verify: unit tests cover undo and the empty stack (matching "region history").
- [x] 4.6 Implement the finishing actions: `Return` positions only, `Space` positions and clicks, `Escape` exits. Verify: the behaviour and the pointer position of all three, and that `Return`/`Space` return to normal mode (matching "region actions and exit").
- [x] 4.7 Draw the active region's border, the cell dividing lines and the key hints with the theme primitives. Verify: content outside the region is not covered by an opaque block, and the hints are readable over light and dark backgrounds (matching "region visibility"); confirmed by `scripts/capture.sh`.

## 5. mode-hint (lattice source)

- [x] 5.1 Implement prefix-free label assignment (fixed length, `ceil(log_|alphabet|(N))`, alphabet from the configuration). Verify: unit tests assert that N labels are distinct, that no label is a prefix of another, and that the length grows when N exceeds the capacity (matching "the lattice target source" and "more targets than the alphabet can label").
- [x] 5.2 Implement lattice target generation (spacing from the configuration, target centres as both the drawing and the positioning basis). Verify: unit tests cover the target count and centre coordinates as the spacing changes (matching "density is configurable").
- [x] 5.3 Implement prefix filtering and the ignoring of unmatched input. Verify: unit tests cover converging character by character, an invalid character leaving state alone, and the remaining candidates being correct (matching "prefix filtering").
- [x] 5.4 Implement the unique-match path: position at the target's centre, switch the overlay back to normal mode, and with `click_after_warp` enabled remove the overlay and click the left button first. Verify: after a match the pointer lands within one logical pixel and the session is in normal mode (matching "behaviour after a match" and "positioning accuracy").
- [x] 5.5 Implement `BackSpace` editing (removing a character when something is typed, returning to normal mode without exiting when nothing is) and `Escape`. Verify: both `BackSpace` behaviours and the exit (matching "label editing and exit").
- [x] 5.6 Draw hint labels with the same badge primitive as grid mode, not covering the target centre and legible on any background. Verify: readability over a black and over a white background (matching "consistent hint badges"); confirmed by `scripts/capture.sh`, which renders a white fullscreen layer underneath.

## 6. Rendering and theme consistency

- [x] 6.1 Establish the tiny-skia canvas abstraction and a single `theme` structure (colours, radii, padding, strokes, font family and sizes). Verify: no literal colour values appear in `src/mode` or `src/hint`; everything comes from the theme.
- [x] 6.2 Implement font loading, text measurement and centred drawing: the family name resolves through `fc-match` to a file path, with a fixed probe list when `fc-match` is unavailable, warning on standard error and moving to the next candidate when a family is missing. Verify: unit tests assert that measurement is monotonic in the size, and label text is centred and not clipped (matching "the badge does not cover the target centre").
- [x] 6.3 Implement the four drawing primitives — rounded corners, translucent fill, inner stroke and shadow — and use them for badges and panels. Verify: the three modes look like one thing, comparable by eye in the captured frames.
- [x] 6.4 Write `docs/theme.md` explaining the theme as the single source of the visual language and how to add a drawn element. Verify: the example code in the document compiles into a `render` module test.

## 7. configuration

- [x] 7.1 Implement line-by-line parsing (`<option>: <value>`, ignoring blank lines and `#` comments, tolerating whitespace, taking everything after the first `:` as the value). Verify: unit tests cover comments, blank lines, a value containing whitespace and trailing whitespace (matching "syntax").
- [x] 7.2 Implement error handling: an unknown option warns with the line's content and is ignored, an illegal value warns and keeps the default, and a named file that cannot be read exits non-zero. Verify: unit tests capture standard error and assert the warning text and the exit status (matching "handling configuration errors" and "a named file that is missing").
- [x] 7.3 Implement every option the design and the spec list, with its default (key bindings, step, partition and key table, hint density and alphabet, colours and font). Verify: a unit test asserts each option's default and that one override takes effect (matching "configurable options").
- [x] 7.4 Implement the binding semantics: several lines coexisting, `unbind`, and the later definition winning. Verify: unit tests cover all three, including two lines for one option both binding (matching "binding and unbinding keys").
- [x] 7.5 Implement XDG path resolution (`$XDG_CONFIG_HOME` then `~/.config`), `--config` overriding it, and a missing default file not being an error. Verify: unit tests cover the three path sources and the missing cases (matching "location and precedence").
- [x] 7.6 Provide a commented sample configuration and an option table in the README. Verify: every line of the sample is accepted by the parser without a warning (asserted by a test that reads the documented sample).

## 8. The opt-in AT-SPI element source

- [x] 8.1 Extract the `HintSource` trait and connect the lattice to it, so the default build pulls in no accessibility dependency. Verify: `cargo tree -e normal` contains no `zbus` or `atspi`, and hint mode behaves as it did before.
- [x] 8.2 Under the `atspi` feature, implement candidate extraction: walking the accessibility tree and taking the centres of interactive elements' bounding boxes, with a time budget for connecting and walking. Verify: it returns non-empty candidates for an application publishing a tree, and returns or times out within the budget (matching "element targets and degradation").
- [x] 8.3 Implement the fallback: empty candidates, an unavailable service or a timeout silently switch to the lattice source, with a bounded total wait and without the session failing. Verify: unit tests cover the empty and timeout fallback paths, and hint mode still works on a machine with no accessibility service (matching "an empty accessibility tree" and "the accessibility service is unavailable").
- [x] 8.4 Document the feature in the README and `docs/dependencies.md`: how to enable it, what it costs in dependencies, and which applications it applies to. Verify: `cargo build --features atspi` succeeds and the documented dependencies match `cargo tree`.

## 9. Compositor verification and wrap-up

- [x] 9.1 sway checklist: session establishment, positioning accuracy, left/middle/right buttons, the wheel, dragging, all three mode switches, and 1x and 1.5x scaling, each recorded with a conclusion. Verify: every item has an explicit pass or fail.
- [x] 9.2 niri checklist: as 9.1, plus text sharpness at the default fractional scale and the overlay landing on the output under the pointer. Verify: every item has an explicit conclusion.
- [x] 9.3 river checklist: as 9.1. Verify: `keyboard_interactivity=exclusive` holds on river, or the README marks river experimental with the known limitation stated.
- [x] 9.4 Dependency audit regression: re-check every direct dependency and locked version against the documented command, including the opt-in feature's transitive dependencies. Verify: no unfixed advisory, and any that appears is recorded and handled before merging.
- [x] 9.5 Consistency review and jj wrap-up: check the README and `docs/*` against the implementation (option table, key table, protocol requirements), and write the change's jj description with a bookmark. Verify: `jj log` shows the change, and every key in the documented table is exercised by a test.
