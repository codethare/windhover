//! Drawing primitives: rounded rectangles, strokes, hairlines, soft shadows.
//!
//! `tiny-skia` works in premultiplied RGBA8888, but the buffer is
//! `wl_shm::Format::Argb8888`: a `0xAARRGGBB` word, which on little-endian is
//! stored as the bytes B, G, R, A. Red and blue are therefore swapped relative
//! to the renderer, and [`to_canvas`] is the one place that reconciles them —
//! drawing an accent of `0A84FF` without it comes out orange.

use tiny_skia::{FillRule, Paint, Path, PathBuilder, PixmapMut, Stroke, Transform};

use super::theme::Rgba;

/// Reorder a colour for the canvas. Blending is per channel, so swapping the two
/// consistently at both ends is all that is needed.
fn to_canvas(colour: Rgba) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba8(colour.b, colour.g, colour.r, colour.a)
}

pub struct Canvas<'a> {
    bytes: &'a mut [u8],
    width: u32,
    height: u32,
}

impl<'a> Canvas<'a> {
    pub fn new(bytes: &'a mut [u8], width: u32, height: u32) -> Option<Self> {
        if bytes.len() < width as usize * height as usize * 4 {
            return None;
        }
        Some(Self {
            bytes,
            width,
            height,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    fn pixmap(&mut self) -> PixmapMut<'_> {
        PixmapMut::from_bytes(self.bytes, self.width, self.height)
            .expect("canvas size was checked on construction")
    }

    pub fn clear(&mut self, colour: Rgba) {
        let (r, g, b, a) = premultiply(colour);
        for pixel in self.bytes.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[r, g, b, a]);
        }
    }

    /// Filled rounded rectangle; `radius` is clamped so a small box stays sane.
    pub fn rounded_rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, colour: Rgba) {
        let Some(path) = rounded_rect_path(x, y, w, h, radius) else {
            return;
        };
        let mut paint = Paint::default();
        paint.set_color(to_canvas(colour));
        paint.anti_alias = true;
        self.pixmap()
            .fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
    }

    /// Outlined rounded rectangle, drawn inside the given box.
    pub fn rounded_rect_stroke(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        width: f32,
        colour: Rgba,
    ) {
        let inset = width / 2.0;
        let Some(path) = rounded_rect_path(
            x + inset,
            y + inset,
            (w - width).max(0.0),
            (h - width).max(0.0),
            (radius - inset).max(0.0),
        ) else {
            return;
        };
        let mut paint = Paint::default();
        paint.set_color(to_canvas(colour));
        paint.anti_alias = true;
        let stroke = Stroke {
            width,
            ..Default::default()
        };
        self.pixmap()
            .stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }

    /// A translucent surface with an edge hairline: the panel and badge
    /// treatment, kept in one place so the modes cannot drift apart.
    pub fn panel_with_edge(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        fill: Rgba,
        edge: Rgba,
        edge_width: f32,
    ) {
        self.rounded_rect(x, y, w, h, radius, fill);
        self.rounded_rect_stroke(x, y, w, h, radius, edge_width, edge);
    }

    pub fn line(&mut self, from: (f32, f32), to: (f32, f32), width: f32, colour: Rgba) {
        let mut builder = PathBuilder::new();
        builder.move_to(from.0, from.1);
        builder.line_to(to.0, to.1);
        let Some(path) = builder.finish() else {
            return;
        };
        let mut paint = Paint::default();
        paint.set_color(to_canvas(colour));
        paint.anti_alias = true;
        let stroke = Stroke {
            width,
            ..Default::default()
        };
        self.pixmap()
            .stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }

    /// A cheap soft shadow: concentric rounded rectangles fading outwards. There
    /// is no blur to be had from `tiny-skia`, and a shadow is what stops a
    /// translucent panel from looking pasted onto the screen.
    #[allow(clippy::too_many_arguments)]
    pub fn soft_shadow(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        spread: f32,
        colour: Rgba,
    ) {
        let steps = spread.round().max(1.0) as u32;
        for step in (1..=steps).rev() {
            let grow = step as f32;
            let alpha = (colour.a as f32 * 0.18 / step as f32).round() as u8;
            if alpha == 0 {
                continue;
            }
            self.rounded_rect(
                x - grow,
                y - grow + grow * 0.5,
                w + grow * 2.0,
                h + grow * 2.0,
                radius + grow,
                colour.with_alpha(alpha),
            );
        }
    }

    /// Fill an arbitrary path. Glyph outlines arrive this way, already converted
    /// into canvas coordinates by the text layer.
    pub fn fill_path(&mut self, path: &Path, colour: Rgba) {
        let mut paint = Paint::default();
        paint.set_color(to_canvas(colour));
        paint.anti_alias = true;
        self.pixmap()
            .fill_path(path, &paint, FillRule::Winding, Transform::identity(), None);
    }
}

/// Premultiplied bytes in the buffer's own channel order. A transparent clear
/// must be all zeroes, not `00 00 00 ff`, or the overlay would paint an opaque
/// black sheet.
fn premultiply(colour: Rgba) -> (u8, u8, u8, u8) {
    let alpha = colour.a as f32 / 255.0;
    (
        (colour.b as f32 * alpha).round() as u8,
        (colour.g as f32 * alpha).round() as u8,
        (colour.r as f32 * alpha).round() as u8,
        colour.a,
    )
}

fn rounded_rect_path(x: f32, y: f32, w: f32, h: f32, radius: f32) -> Option<Path> {
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let radius = radius.min(w / 2.0).min(h / 2.0).max(0.0);
    if radius <= 0.0 {
        return Some(PathBuilder::from_rect(tiny_skia::Rect::from_xywh(x, y, w, h)?));
    }
    let mut builder = PathBuilder::new();
    builder.move_to(x + radius, y);
    builder.line_to(x + w - radius, y);
    builder.quad_to(x + w, y, x + w, y + radius);
    builder.line_to(x + w, y + h - radius);
    builder.quad_to(x + w, y + h, x + w - radius, y + h);
    builder.line_to(x + radius, y + h);
    builder.quad_to(x, y + h, x, y + h - radius);
    builder.line_to(x, y + radius);
    builder.quad_to(x, y, x + radius, y);
    builder.close();
    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas_bytes(width: u32, height: u32) -> Vec<u8> {
        vec![0u8; width as usize * height as usize * 4]
    }

    fn pixel(bytes: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
        let offset = ((y * width + x) * 4) as usize;
        bytes[offset..offset + 4].try_into().unwrap()
    }

    #[test]
    fn the_buffer_is_argb8888_so_the_bytes_are_bgra() {
        // The contract with the compositor: `wl_shm::Format::Argb8888` is a
        // native-endian 0xAARRGGBB word, which is B, G, R, A in memory.
        let mut bytes = canvas_bytes(1, 1);
        let mut canvas = Canvas::new(&mut bytes, 1, 1).unwrap();
        canvas.clear(Rgba::rgb(0x0A, 0x84, 0xFF));
        assert_eq!(pixel(&bytes, 1, 0, 0), [0xFF, 0x84, 0x0A, 0xFF]);
    }

    #[test]
    fn a_drawn_rectangle_keeps_the_channel_order_too() {
        let mut bytes = canvas_bytes(4, 4);
        let mut canvas = Canvas::new(&mut bytes, 4, 4).unwrap();
        canvas.clear(Rgba::rgba(0, 0, 0, 0));
        canvas.rounded_rect(0.0, 0.0, 4.0, 4.0, 0.0, Rgba::rgb(0x0A, 0x84, 0xFF));
        assert_eq!(pixel(&bytes, 4, 2, 2), [0xFF, 0x84, 0x0A, 0xFF]);
    }

    #[test]
    fn filling_a_path_paints_inside_it() {
        let mut bytes = canvas_bytes(8, 8);
        let mut canvas = Canvas::new(&mut bytes, 8, 8).unwrap();
        canvas.clear(Rgba::rgba(0, 0, 0, 0));
        let mut builder = PathBuilder::new();
        builder.move_to(2.0, 2.0);
        builder.line_to(6.0, 2.0);
        builder.line_to(6.0, 6.0);
        builder.line_to(2.0, 6.0);
        builder.close();
        let path = builder.finish().unwrap();
        canvas.fill_path(&path, Rgba::rgb(0x0A, 0x84, 0xFF));
        assert_eq!(pixel(&bytes, 8, 4, 4), [0xFF, 0x84, 0x0A, 0xFF], "channel order swapped");
        assert_eq!(pixel(&bytes, 8, 0, 0)[3], 0, "painted outside the path");
    }

    #[test]
    fn clearing_transparent_leaves_an_empty_overlay() {
        let mut bytes = canvas_bytes(4, 4);
        bytes.fill(0xAA);
        let mut canvas = Canvas::new(&mut bytes, 4, 4).unwrap();
        canvas.clear(Rgba::rgba(0, 0, 0, 0));
        assert_eq!(pixel(&bytes, 4, 1, 1), [0, 0, 0, 0]);
    }

    #[test]
    fn clearing_opaque_writes_premultiplied_bytes() {
        let mut bytes = canvas_bytes(2, 2);
        let mut canvas = Canvas::new(&mut bytes, 2, 2).unwrap();
        canvas.clear(Rgba::rgb(0x33, 0x66, 0xCC));
        assert_eq!(pixel(&bytes, 2, 0, 0), [0xCC, 0x66, 0x33, 0xFF]);
    }

    #[test]
    fn translucent_fill_is_premultiplied() {
        let mut bytes = canvas_bytes(2, 2);
        let mut canvas = Canvas::new(&mut bytes, 2, 2).unwrap();
        canvas.clear(Rgba::rgba(255, 255, 255, 128));
        let [r, g, b, a] = pixel(&bytes, 2, 0, 0);
        assert_eq!(a, 128);
        assert_eq!((r, g, b), (128, 128, 128));
    }

    #[test]
    fn a_rounded_rect_leaves_the_corners_alone() {
        let mut bytes = canvas_bytes(20, 20);
        let mut canvas = Canvas::new(&mut bytes, 20, 20).unwrap();
        canvas.clear(Rgba::rgba(0, 0, 0, 0));
        canvas.rounded_rect(2.0, 2.0, 16.0, 16.0, 6.0, Rgba::rgb(255, 255, 255));
        // Well inside: filled.
        assert_eq!(pixel(&bytes, 20, 10, 10)[3], 255);
        // The very corner of the box, outside the radius: untouched.
        assert_eq!(pixel(&bytes, 20, 2, 2)[3], 0);
    }

    #[test]
    fn a_stroke_draws_an_edge_and_leaves_the_middle() {
        let mut bytes = canvas_bytes(20, 20);
        let mut canvas = Canvas::new(&mut bytes, 20, 20).unwrap();
        canvas.clear(Rgba::rgba(0, 0, 0, 0));
        canvas.rounded_rect_stroke(1.0, 1.0, 18.0, 18.0, 4.0, 2.0, Rgba::rgb(255, 0, 0));
        assert!(pixel(&bytes, 20, 10, 1)[3] > 0, "top edge missing");
        assert_eq!(pixel(&bytes, 20, 10, 10)[3], 0, "middle should be empty");
    }

    #[test]
    fn a_soft_shadow_darkens_just_outside_the_box() {
        let mut bytes = canvas_bytes(40, 40);
        let mut canvas = Canvas::new(&mut bytes, 40, 40).unwrap();
        canvas.clear(Rgba::rgb(255, 255, 255));
        canvas.soft_shadow(15.0, 15.0, 10.0, 10.0, 2.0, 4.0, Rgba::rgb(0, 0, 0));
        // Below the box: darkened by the shadow.
        assert!(pixel(&bytes, 40, 20, 27)[0] < 255);
        // Far from the box: untouched.
        assert_eq!(pixel(&bytes, 40, 1, 1)[0], 255);
    }

    #[test]
    fn a_canvas_smaller_than_its_geometry_is_rejected() {
        let mut bytes = canvas_bytes(2, 2);
        assert!(Canvas::new(&mut bytes, 100, 100).is_none());
    }
}
