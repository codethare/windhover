# Design

## Context

See `proposal.md` for the measurements. Three facts shaped the fix:

- A frame is drawn from scratch on every keystroke, into a shared-memory buffer
  the compositor takes a full-size copy of: there is no damage model to lean on.
- Hint mode's content is static apart from the surviving labels: the lattice and
  the label set never change within a session.
- `tiny-skia`'s cost here is per path, not per pixel: 390 filled and stroked
  cells covering the whole screen, 780 glyph outlines, and 43 full-length lines
  all cost far more than the area they touch.

## Goals / Non-Goals

**Goals:**

- Keep the lattice visible — it is what makes a cell a region — without paying
  for the screen's area on every frame.
- Make a keystroke's redraw cheap enough that a two-character label never feels
  like waiting.

**Non-Goals:**

- A background layer cache, or any other scheme that trades memory for the
  remaining full-buffer copy. The copy is the compositor's requirement, not ours.
- Configurable line weight or cell appearance. The theme already owns both.

## Decisions

### Lines for the lattice, a fill only for the cells still in play

The fill is what made the cost follow the output: 390 cells tiling the screen is
the screen, filled twice with the hairline stroke. Lines are the same information
about where the regions are, at a fortieth of the pixels, and once the user has
typed a character there are few enough surviving cells that filling them costs
nothing.

Alternative: keeping the fill and caching the whole rendered lattice. Rejected —
the cached layer would still have to be composited into the frame buffer over
every pixel, which is the cost we were trying to avoid.

### Labels cached as whole rasters, keyed on the label and the typed prefix

A label is two glyphs and a box; caching at that grain means one blit per label
instead of four paths. The key carries the typed-prefix count because the prefix
is drawn in the matched colour, and the cache is dropped wholesale when the
theme or the scale changes, which is what a fingerprint of those values detects.

Alternative: caching glyphs and composing boxes per frame. Smaller cache, but the
box would have to be drawn per label per frame, which is a third of what the
frame cost.

### The composite is hand-written, and so are the grid runs

Both were measured, not assumed:

- `Pixmap::draw_pixmap` fills the destination rectangle with a pattern instead of
  copying a sprite; 390 labels cost about 100 ns a pixel through it, 0.4 ms
  through a premultiplied source-over loop over the same pixels.
- `stroke_path` for a full-width line costs about 30 ns per pixel of length; a
  vertical run of the same length composites a few millionths of a second through
  `Canvas::vline`.

Both loops are short, and both blend in the same premultiplied, byte-swapped
space as `clear`, which is the one place that order is decided.

### A transparent clear is a `memset`

The frame starts fully transparent, which in premultiplied terms is all zeroes.
`Vec::fill` is the same result without interpreting the colour, and at the scale-2
4K buffer it is 20 ms instead of 48.

## Risks / Trade-offs

- **The first frame got slower at small sizes**: 95 ms against 60 at 1280x720,
  because it now rasterizes 390 label pixmaps instead of drawing them once
  directly. Every larger size improved, and the steady state — which is the
  keystroke path this change is about — improved everywhere.
  → Accepted. A glyph-level cache would fix it and is the next step if startup
  latency matters more than keystroke latency.
- **Two hand-written blenders**: they do not handle clipping to a rounded box or
  a mask, so they are only correct for axis-aligned runs and opaque-cornered
  rectangles, which is all that calls them. → Noted at both sites.
- **The cache grows with the typed prefixes seen**: at most three per label, and a
  session is short.
- **A theme or scale change must invalidate the cache.** The fingerprint covers
  the colours, the type metrics and the scale, but a new theme field that affects
  a badge has to be added to it. → The render test draws the same frame twice and
  checks the cache is only filled once, so a miss shows up as a re-rasterization
  rather than as a stale pixel.
