# Tasks

## 1. The twenty-character alphabet

- [x] 1.1 Change the default `hint_charset` to `asdfjkl;ghqwertyuiop` in
      `src/config.rs`, and add a unit test that the default parses to twenty
      characters including `;`
- [x] 1.2 Relax the `hint_charset` validation from alphanumeric-only to any
      printable non-whitespace ASCII, keeping the empty-charset error, and extend
      the existing `config.rs` tests: `;` accepted, a space and a multi-byte
      non-ASCII character still rejected
- [x] 1.3 Update the alphabet row in `docs/configuration.md` and the sample
      configuration's `hint_charset` line to the new default

## 2. Cells, and a spacing that follows from the alphabet

- [x] 2.1 Add `Lattice::cells(screen)` in `src/hint/lattice.rs` returning one
      `(x, y, w, h)` per cell, derived from the same `dimensions()` the targets
      use, and unit-test that the cells tile the output with no gaps or overlap
      and that each cell's centre is the corresponding target
- [x] 2.2 Change `Target` in `src/mode/hint.rs` to carry the cell rectangle beside
      its centre, threading it through `Hint::new`, and keep the existing
      `mode::hint` tests passing unchanged
- [x] 2.3 Add `Config::hint_cell_for(screen)` mirroring `step_for`: the configured
      value when set, otherwise `sqrt(width * height / len(charset)^2)`; use it in
      `reset_hint`, and unit-test that the default yields a label length of two
      for 1920x1080, 2560x1440 and 3840x2160
- [x] 2.4 Update the `hint_cell` row in `docs/configuration.md` to say the default
      is derived from the alphabet and the output

## 3. The label-length budget

- [x] 3.1 Lower `labels::assign`'s length cap to 3 and adjust its unit tests: a
      count that fits in two characters gets two, a count past `charset^2` gets
      three, and the prefix-free property still holds
- [x] 3.2 Refuse a configuration whose lattice needs more than three characters:
      `reset_hint` returns an error naming `hint_cell` and the cell count, wired
      to the existing start-up error path, with a unit test for both the refusal
      and the message
- [x] 3.3 Add the refusal to `docs/configuration.md`'s `hint_cell` row and to
      `scripts/headless-check.sh` as a failing-config case beside the existing
      `grid_cells` refusal

## 4. The translucent cell grid

- [x] 4.1 Add the cell fill colour to `src/render/theme.rs` next to `panel` and
      `panel_edge` (translucent, the same hue family), with the `accent`/`text`
      overrides unaffected
- [x] 4.2 Replace `draw_hint`'s badge call with a cell: translucent fill, hairline
      edge, label centred, matched-prefix colouring preserved; update the
      `render` tests to assert one cell per visible target and that the fill is
      translucent rather than opaque
- [x] 4.3 Update the hint label bullet in `README.md`'s mode table for the new
      rendering

## 5. End to end

- [x] 5.1 Update `scripts/headless-check.sh`: the hint key sequences become two
      characters again, and the expected first-cell centre becomes
      `W/26/2, H/15/2` for the headless 1280x720 output
- [x] 5.2 Run the full gate — `cargo test`, `cargo deny check advisories bans`,
      `sh scripts/headless-check.sh sway` — and report each result
- [x] 5.3 Capture one screenshot per mode with `sh scripts/capture.sh` and check
      the hint grid against a light and a dark region, adjusting the cell alpha if
      the labels or the edges do not read
