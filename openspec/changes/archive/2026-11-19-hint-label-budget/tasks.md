# Tasks

## 1. The cap

- [x] 1.1 Give `Lattice` a per-axis cap and a `capped` constructor, with a unit
      test that a wide screen's columns fall back to the alphabet and the cells
      widen to compensate
- [x] 1.2 Cap the lattice at the alphabet's length where the session builds it,
      with a session test on a 4K output
- [x] 1.3 Remove the density refusal and its test now that a lattice cannot be
      too dense, and drop the now-unreachable exceed-capacity scenario from the
      spec delta

## 2. The label budget

- [x] 2.1 Rewrite `labels::assign` to the short-then-long rule, with tests for
      400 targets (no short labels), 300 (five, and `ka` after them), and 25 over
      a nine-character alphabet
- [x] 2.2 Keep the prefix-free and uniqueness tests passing unchanged
- [x] 2.3 Handle a one-character alphabet without looping

## 3. The type size

- [x] 3.1 Carry the label size on `Frame`, derived from the theme's reference and
      the output height, and use it in `draw_grid`, `draw_hint`, the badge cache
      and its fingerprint
- [x] 3.2 Session test: 14 at 720 high, 28 at 1440

## 4. Documentation and the gate

- [x] 4.1 Update the `hint_cell` row in `docs/configuration.md` and the hint row
      in `README.md`
- [x] 4.2 Pin an alphabet in `scripts/headless-check.sh`'s hint checks so their
      labels are deterministic, and remove the density refusal check
- [x] 4.3 Run the gate — `cargo test`, `cargo deny check advisories bans`,
      `sh scripts/headless-check.sh sway`
