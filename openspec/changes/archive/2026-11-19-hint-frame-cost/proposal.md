# Proposal

## Why

A hint frame cost 60 ms at 1280x720, 134 ms at 2560x1440 and 488 ms on a 4K
output at scale 2, and every typed character redraws the whole frame — so
choosing a two-character label spent three of those. The labels themselves were
never the cost: assigning all 390 took 0.066 ms. Nine tenths of a frame went on
the rasterizer's per-path work and on filling the screen cell by cell.

Measured, release build, on one machine, median of ten:

| | before | after |
|---|---|---|
1280x720 | 60 ms | 14 ms |
2560x1440 | 134 ms | 12 ms |
3840x2160 | 142 ms | 21 ms |
3840x2160 at scale 2 | 488 ms | 73 ms |

## What Changes

- The hint lattice is drawn as grid lines instead of a translucent fill per cell.
  The whole screen was being filled once per cell, which is what made the frame
  cost follow the output size rather than the work in it.
- A cell is filled only once the user has typed a character, when only the
  surviving cells are left to look at.
- Each label's panel and text is rasterized once per session and blitted
  afterwards, instead of being drawn again on every frame.
- Fragments are composited by hand: `Pixmap::draw_pixmap` fills its destination
  with a pattern and measured at about 100 ns a pixel, and the rasterizer's path
  stroke costs about 30 ns per pixel of length, which a full-width grid line pays
  in full.
- A fully transparent clear is a `memset` rather than a pixel loop.

The first frame rasterizes every label, so it is unimproved at the smallest size
and better at every larger one: 95 ms at 1280x720 against 60 before, 72 at
2560x1440 against 134.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `mode-hint`: the lattice is drawn as lines, with a cell filled only while the
  labels are being narrowed.

## Impact

- `src/render/canvas.rs`, `src/render/mod.rs`, `src/render/theme.rs`
- `src/hint/lattice.rs`, `src/mode/hint.rs`, `src/session.rs`
- `docs/theme.md`
