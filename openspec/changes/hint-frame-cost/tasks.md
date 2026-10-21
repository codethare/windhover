# Tasks

## 1. The lattice as lines

- [x] 1.1 Add `Lattice::lines` returning the cell boundaries, with a unit test
      that the first and last boundary are the output's edges and that the count
      is one more than the cells per axis
- [x] 1.2 Cover the lattice with `Canvas::vline`/`Canvas::hline` and test that a
      run splits its coverage between the pixels it overlaps, keeps the byte
      order, and composites its alpha
- [x] 1.3 Carry the boundaries on `HintFrame` and set them where the lattice is
      built, and add `Hint::filtering`
- [x] 1.4 Fill a cell only while the labels are narrowed, with a render test that
      an unfiltered frame fills nothing and a typed character fills the survivors

## 2. Labels as cached rasters

- [x] 2.1 Rasterize a label's panel and text into its own pixmap, keyed on the
      label and the typed-prefix count, invalidated by a fingerprint of the
      theme and the scale
- [x] 2.2 Composite it with a hand-written premultiplied source-over blit, tested
      at an exact pixel and against a partially transparent edge
- [x] 2.3 Add a render test that a second frame with the same theme and scale
      rasterizes nothing
- [x] 2.4 Thread the cache through `draw` and the session's redraw

## 3. The clear

- [x] 3.1 Make a fully transparent `clear` a `memset`, with the existing channel
      order test still passing

## 4. Documentation and the gate

- [x] 4.1 Update `docs/theme.md`'s account of what the render tests pin
- [x] 4.2 Measure the frame before and after at four output sizes and record both
      in the change
- [x] 4.3 Run the gate — `cargo test`, `cargo deny check advisories bans`,
      `sh scripts/headless-check.sh sway` — and capture one screenshot per mode
