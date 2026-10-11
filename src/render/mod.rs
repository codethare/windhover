//! Software rendering onto shared-memory buffers.
//!
//! One entry point ([`draw`]) composes the whole frame for whichever mode is
//! active, using only the primitives in [`canvas`] and the values in [`theme`].
//! That is the whole reason the three modes look like one program.

pub mod canvas;
pub mod text;
pub mod theme;

use crate::mode::grid::Grid;

pub use canvas::Canvas;
pub use text::Fonts;
pub use theme::{Rgba, Theme};

/// Everything the frame needs, in logical coordinates.
pub struct Frame<'a> {
    /// Buffer pixels per logical pixel.
    pub scale: f32,
    pub cursor: (f64, f64),
    pub multiplier: Option<u32>,
    pub dragging: bool,
    pub grid: Option<GridFrame<'a>>,
    pub hint: Option<HintFrame<'a>>,
}

pub struct GridFrame<'a> {
    pub grid: &'a Grid,
    /// Label for each cell, row major, matching [`Grid::cell`] indices.
    pub cell_labels: &'a [String],
}

pub struct HintFrame<'a> {
    pub hint: &'a crate::mode::hint::Hint,
}

impl<'a> Frame<'a> {
    pub fn normal(scale: f32, cursor: (f64, f64)) -> Self {
        Self {
            scale,
            cursor,
            multiplier: None,
            dragging: false,
            grid: None,
            hint: None,
        }
    }
}

pub fn draw(canvas: &mut Canvas, theme: &Theme, fonts: &mut Option<Fonts>, frame: &Frame<'_>) {
    // An overlay that dims the screen would defeat hint mode's purpose: the
    // user has to be able to read what is underneath.
    canvas.clear(Rgba::rgba(0, 0, 0, 0));

    if let Some(grid) = &frame.grid {
        draw_grid(canvas, theme, fonts, frame.scale, grid);
    }
    if let Some(hint) = &frame.hint {
        draw_hint(canvas, theme, fonts, frame.scale, hint);
    }

    draw_cursor(canvas, theme, frame.scale, frame.cursor);
    draw_status(canvas, theme, fonts, frame);
}

fn draw_grid(
    canvas: &mut Canvas,
    theme: &Theme,
    fonts: &mut Option<Fonts>,
    scale: f32,
    view: &GridFrame<'_>,
) {
    let region = view.grid.region();
    let (columns, rows) = (view.grid.columns(), view.grid.rows());

    for column in 1..columns {
        let x = (region.x + region.w * column as f64 / columns as f64) as f32 * scale;
        canvas.line(
            (x, region.y as f32 * scale),
            (x, (region.y + region.h) as f32 * scale),
            theme.grid_line_width * scale,
            theme.grid_line(),
        );
    }
    for row in 1..rows {
        let y = (region.y + region.h * row as f64 / rows as f64) as f32 * scale;
        canvas.line(
            (region.x as f32 * scale, y),
            ((region.x + region.w) as f32 * scale, y),
            theme.grid_line_width * scale,
            theme.grid_line(),
        );
    }

    canvas.rounded_rect_stroke(
        region.x as f32 * scale,
        region.y as f32 * scale,
        region.w as f32 * scale,
        region.h as f32 * scale,
        theme.corner_radius * scale,
        theme.region_edge_width * scale,
        theme.region_edge(),
    );

    for (index, label) in view.cell_labels.iter().enumerate() {
        let Some(cell) = view.grid.cell(index) else {
            break;
        };
        let (x, y) = cell.center();
        badge(canvas, theme, fonts, (x as f32, y as f32), label, 0, scale);
    }
}

fn draw_hint(
    canvas: &mut Canvas,
    theme: &Theme,
    fonts: &mut Option<Fonts>,
    scale: f32,
    view: &HintFrame<'_>,
) {
    for (target, matched) in view.hint.visible() {
        badge(
            canvas,
            theme,
            fonts,
            (target.x as f32, target.y as f32),
            &target.label,
            matched,
            scale,
        );
    }
}

/// A label in a translucent rounded panel. Used by both grid and hint mode so
/// the two cannot drift apart.
fn badge(
    canvas: &mut Canvas,
    theme: &Theme,
    fonts: &mut Option<Fonts>,
    centre: (f32, f32),
    label: &str,
    matched: usize,
    scale: f32,
) {
    let Some(fonts) = fonts.as_mut() else {
        return;
    };
    let size = theme.badge_font_size;
    let text_width = fonts.measure(label, size);
    let (ascent, descent) = fonts.ascent_descent(size);
    let padding = (theme.badge_padding_x, theme.badge_padding_y);
    let width = (text_width + padding.0 * 2.0) * scale;
    let height = (ascent + descent + padding.1 * 2.0) * scale;
    let x = centre.0 * scale - width / 2.0;
    let y = centre.1 * scale - height / 2.0;

    // No drop shadow on a badge: `soft_shadow` is concentric rectangles, and on a
    // box this small the rings read as an artifact rather than as depth. The
    // panel fill and the hairline are what separate it from the screen, which is
    // also what makes a small pill legible on any background.
    canvas.panel_with_edge(
        x,
        y,
        width,
        height,
        theme.badge_radius * scale,
        theme.surface(),
        theme.panel_edge,
        theme.hairline * scale,
    );

    let baseline = y + (padding.1 + ascent) * scale;
    let text_left = x + padding.0 * scale;
    let matched_chars = label.chars().count().min(matched);
    if matched_chars > 0 {
        let prefix: String = label.chars().take(matched_chars).collect();
        let prefix_width = fonts.measure(&prefix, size) * scale;
        fonts.draw(canvas, &prefix, text_left, baseline, size * scale, theme.matched());
        fonts.draw(
            canvas,
            &label.chars().skip(matched_chars).collect::<String>(),
            text_left + prefix_width,
            baseline,
            size * scale,
            theme.text,
        );
    } else {
        fonts.draw(canvas, label, text_left, baseline, size * scale, theme.text);
    }
}

/// A ring around the cursor. Normal mode has to show where the pointer is
/// without covering what is underneath it.
fn draw_cursor(canvas: &mut Canvas, theme: &Theme, scale: f32, cursor: (f64, f64)) {
    let radius = 9.0 * scale;
    let centre = (cursor.0 as f32 * scale, cursor.1 as f32 * scale);
    let ring = 1.5 * scale;
    canvas.rounded_rect_stroke(
        centre.0 - radius,
        centre.1 - radius,
        radius * 2.0,
        radius * 2.0,
        radius,
        ring,
        theme.region_edge(),
    );
    canvas.rounded_rect(
        centre.0 - ring,
        centre.1 - ring,
        ring * 2.0,
        ring * 2.0,
        ring,
        theme.accent,
    );
}

/// The one status element: only there when it says something the cursor ring
/// does not.
fn draw_status(canvas: &mut Canvas, theme: &Theme, fonts: &mut Option<Fonts>, frame: &Frame<'_>) {
    let text = match (frame.dragging, frame.multiplier) {
        (true, Some(multiplier)) => format!("drag ×{multiplier}"),
        (true, None) => "drag".to_string(),
        (false, Some(multiplier)) => format!("×{multiplier}"),
        (false, None) => return,
    };
    let Some(fonts) = fonts.as_mut() else {
        return;
    };
    let scale = frame.scale;
    let size = theme.status_font_size;
    let (ascent, descent) = fonts.ascent_descent(size);
    let width = (fonts.measure(&text, size) + theme.badge_padding_x * 2.0) * scale;
    let height = (ascent + descent + theme.badge_padding_y * 2.0) * scale;
    let x = (frame.cursor.0 as f32 * scale + 16.0 * scale).min(canvas.width() as f32 - width - 8.0);
    let y = (frame.cursor.1 as f32 * scale + 16.0 * scale).min(canvas.height() as f32 - height - 8.0);

    canvas.soft_shadow(x, y, width, height, theme.corner_radius * scale, 4.0 * scale, theme.shadow);
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
    let baseline = y + theme.badge_padding_y * scale + ascent * scale;
    let centre = x + width / 2.0;
    fonts.draw_centred(
        canvas,
        &text,
        centre,
        baseline,
        size * scale,
        theme.text,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::grid::Grid;
    use crate::mode::hint::Hint;

    /// The example in `docs/theme.md`, so the documentation cannot drift from
    /// the primitives it tells a reader to use.
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

    fn buffer(width: u32, height: u32) -> Vec<u8> {
        vec![0u8; width as usize * height as usize * 4]
    }

    fn alpha_at(bytes: &[u8], width: u32, x: u32, y: u32) -> u8 {
        bytes[((y * width + x) * 4 + 3) as usize]
    }

    /// The bar in the documentation example is 120x8 logical pixels at `at`.
    fn inside_the_bar(at: (f32, f32), scale: f32) -> (u32, u32) {
        (
            ((at.0 + 60.0) * scale) as u32,
            ((at.1 + 4.0) * scale) as u32,
        )
    }

    #[test]
    fn the_theme_documentation_example_draws_something() {
        let mut bytes = buffer(200, 60);
        let mut canvas = Canvas::new(&mut bytes, 200, 60).unwrap();
        canvas.clear(Rgba::rgba(0, 0, 0, 0));
        draw_bar(&mut canvas, &Theme::default(), 1.0, (10.0, 10.0));
        // The panel is translucent, so the interior is its alpha, not opaque.
        let (x, y) = inside_the_bar((10.0, 10.0), 1.0);
        assert_eq!(alpha_at(&bytes, 200, x, y), 0xE0, "the bar was not painted");
        assert_eq!(alpha_at(&bytes, 200, x, y + 27), 0, "the bar is too tall");
    }

    #[test]
    fn the_overlay_never_obscures_what_is_under_it() {
        // Every frame starts transparent: the marks a mode needs are drawn on
        // top of it, and nothing else is filled in.
        let mut bytes = buffer(200, 200);
        bytes.fill(0xAA);
        {
            let mut canvas = Canvas::new(&mut bytes, 200, 200).unwrap();
            canvas.clear(Rgba::rgba(0, 0, 0, 0));
        }
        assert!(bytes.iter().all(|byte| *byte == 0), "the clear left pixels");

        {
            let mut canvas = Canvas::new(&mut bytes, 200, 200).unwrap();
            draw_bar(&mut canvas, &Theme::default(), 1.0, (10.0, 10.0));
        }
        assert_eq!(alpha_at(&bytes, 200, 150, 150), 0);
    }

    #[test]
    fn a_grid_frame_draws_lines_badges_and_nothing_else() {
        let mut bytes = buffer(1280, 720);
        let mut canvas = Canvas::new(&mut bytes, 1280, 720).unwrap();
        let theme = Theme::default();
        let (mut fonts, _) = Fonts::load("sans-serif");
        let grid = Grid::new((1280, 720), 2, 2, 40.0);
        let labels = ["q", "w", "a", "s"].map(String::from);
        let mut frame = Frame::normal(1.0, (640.0, 360.0));
        frame.grid = Some(GridFrame {
            grid: &grid,
            cell_labels: &labels,
        });
        draw(&mut canvas, &theme, &mut fonts, &frame);

        // The region edge, the interior lines, and a badge at a cell centre.
        // Thresholds rather than exact values: everything is antialiased.
        assert!(alpha_at(&bytes, 1280, 640, 2) > 100, "no region edge");
        assert!(alpha_at(&bytes, 1280, 640, 360) > 100, "no interior line");
        assert!(alpha_at(&bytes, 1280, 320, 180) > 200, "no badge in the cell");
        // And nothing between the marks.
        assert_eq!(alpha_at(&bytes, 1280, 100, 100), 0, "the region was filled");
    }

    #[test]
    fn a_hint_frame_puts_a_badge_on_every_target() {
        let mut bytes = buffer(640, 360);
        let mut canvas = Canvas::new(&mut bytes, 640, 360).unwrap();
        let theme = Theme::default();
        let (mut fonts, _) = Fonts::load("sans-serif");
        let hint = Hint::new(vec![(100.0, 100.0), (400.0, 200.0)], &"as".chars().collect::<Vec<_>>());
        let mut frame = Frame::normal(1.0, (0.0, 0.0));
        frame.hint = Some(HintFrame { hint: &hint });
        draw(&mut canvas, &theme, &mut fonts, &frame);

        assert!(alpha_at(&bytes, 640, 100, 100) > 200, "no badge on target 0");
        assert!(alpha_at(&bytes, 640, 400, 200) > 200, "no badge on target 1");
        assert_eq!(alpha_at(&bytes, 640, 250, 300), 0, "something was drawn elsewhere");
    }

    #[test]
    fn the_scale_factor_sizes_everything() {
        // Drawing at 2x must cover twice the pixels, or a fractional-scaled
        // output would show a correctly coloured but half-size overlay.
        let mut single = buffer(400, 400);
        let mut double = buffer(800, 800);
        let theme = Theme::default();
        let mut canvas = Canvas::new(&mut single, 400, 400).unwrap();
        draw_bar(&mut canvas, &theme, 1.0, (10.0, 10.0));
        let mut canvas = Canvas::new(&mut double, 800, 800).unwrap();
        draw_bar(&mut canvas, &theme, 2.0, (10.0, 10.0));

        let (x, y) = inside_the_bar((10.0, 10.0), 1.0);
        assert_eq!(alpha_at(&single, 400, x, y), 0xE0, "1x bar missing");
        let (x, y) = inside_the_bar((10.0, 10.0), 2.0);
        assert_eq!(alpha_at(&double, 800, x, y), 0xE0, "2x bar missing");
        // Just past the bar at 1x, and just past it at 2x.
        let (x, y) = inside_the_bar((10.0, 10.0), 1.0);
        assert_eq!(alpha_at(&single, 400, x, y + 12), 0);
        let (x, y) = inside_the_bar((10.0, 10.0), 2.0);
        assert_eq!(alpha_at(&double, 800, x, y + 24), 0);
        // The 2x bar reaches further than the 1x one would have.
        assert!(alpha_at(&double, 800, 159, 26) > 0, "the 2x bar is not wider");
    }
}
