# Theme

The overlay has one visual language, and [`src/render/theme.rs`](../src/render/theme.rs)
is the only place that names a colour, a radius or a font size. Everything the
three modes draw goes through it and through the primitives in
[`src/render/canvas.rs`](../src/render/canvas.rs). That is the whole mechanism
behind "the three modes look like one program": there is nowhere else for them
to drift apart.

## The rules

1. **No colour literals outside `theme.rs`.** A mode module hands
   [`render::draw`](../src/render/mod.rs) its state and gets a frame back; it
   never draws.
2. **Tints derive from one accent.** Grid lines are the accent at a lower alpha,
   the region border is the accent itself, and the cursor ring is the accent.
   One configuration line (`accent:`) moves all three modes together.
3. **Small things get a fill and a hairline, not a shadow.** `soft_shadow` is
   concentric rounded rectangles: there is no blur to be had from `tiny-skia`,
   and on a 26-pixel badge the rings read as an artifact. The hairline is what
   keeps a pill legible on a white window and on a black one.
4. **The surface is never dimmed.** The overlay clears to transparent so what is
   underneath stays readable; only the marks the mode needs are drawn.

## Adding a drawn element

Say a new mode needs a bar. Add the values it needs to `Theme` (or derive them
from the accent), then draw it with the existing primitives:

```rust
use crate::render::{Canvas, Theme};

fn draw_bar(canvas: &mut Canvas, theme: &Theme, scale: f32, at: (f32, f32)) {
    let (x, y) = (at.0 * scale, at.1 * scale);
    let (width, height) = (120.0 * scale, 8.0 * scale);
    canvas.panel_with_edge(
        x,
        y,
        width,
        height,
        theme.corner_radius * scale,
        theme.surface(),
        theme.panel_edge,
        theme.hairline * scale,
    );
}
```

Notes that are easy to get wrong:

- **Everything is in logical pixels and multiplied by `scale`.** The frame is
  drawn into a buffer that may be 1.5x or 2x the logical size, and `scale` is
  `Frame::scale`.
- **The buffer is ARGB8888, which is BGRA in memory on little-endian.** `canvas`
  reconciles that in one place (`to_canvas`); do not pass a `tiny_skia::Color`
  built by hand.
- **Text is drawn through `Fonts`**, which is `None` when no font could be
  loaded; every drawing function that wants text has to cope with that rather
  than unwrapping.

## Tests

The snippet above is also a test: `render::tests::draw_bar` is the same code, so
a change to the primitives breaks the build rather than the documentation.

`canvas.rs` pins the format contract (the byte order of an `Argb8888` buffer),
`theme.rs` pins the colour conversion, and `text.rs` pins font resolution,
measurement monotonicity and the fallback when the requested family is missing.
`render/mod.rs` checks that a grid frame draws its edge, its interior lines and
its badges and leaves everything else transparent.
`scripts/capture.sh` renders one frame per mode against a headless compositor
and writes PNGs to look at, which is how the channel-order and unstyled-accent
bugs were found.
