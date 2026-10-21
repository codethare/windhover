//! Software rendering onto shared-memory buffers.
//!
//! One entry point ([`draw`]) composes the whole frame for whichever mode is
//! active, using only the primitives in [`canvas`] and the values in [`theme`].
//! That is the whole reason the three modes look like one program.

pub mod canvas;
pub mod text;
pub mod theme;

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use tiny_skia::Pixmap;

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
    /// The speed preset the session is moving at. `1.0` is unlabelled.
    pub speed: f64,
    /// The label type size for this output, in logical pixels.
    pub label_size: f32,
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
    /// The cell boundaries, as x and y positions, for the grid the labels sit on.
    pub lines: &'a (Vec<f64>, Vec<f64>),
}

impl<'a> Frame<'a> {
    pub fn normal(scale: f32, cursor: (f64, f64)) -> Self {
        Self {
            scale,
            cursor,
            multiplier: None,
            dragging: false,
            speed: 1.0,
            label_size: 14.0,
            grid: None,
            hint: None,
        }
    }
}

pub fn draw(
    canvas: &mut Canvas,
    theme: &Theme,
    fonts: &mut Option<Fonts>,
    badges: &mut Badges,
    frame: &Frame<'_>,
) {
    // An overlay that dims the screen would defeat hint mode's purpose: the
    // user has to be able to read what is underneath.
    canvas.clear(Rgba::rgba(0, 0, 0, 0));

    if let Some(grid) = &frame.grid {
        draw_grid(canvas, theme, fonts, badges, frame.scale, frame.label_size, grid);
    }
    if let Some(hint) = &frame.hint {
        draw_hint(canvas, theme, fonts, badges, frame.scale, frame.label_size, hint);
    }

    draw_cursor(canvas, theme, frame.scale, frame.cursor);
    draw_status(canvas, theme, fonts, frame);
}

fn draw_grid(
    canvas: &mut Canvas,
    theme: &Theme,
    fonts: &mut Option<Fonts>,
    badges: &mut Badges,
    scale: f32,
    size: f32,
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
        badge(canvas, theme, fonts, badges, (x as f32, y as f32), label, 0, scale, size);
    }
}

fn draw_hint(
    canvas: &mut Canvas,
    theme: &Theme,
    fonts: &mut Option<Fonts>,
    badges: &mut Badges,
    scale: f32,
    size: f32,
    view: &HintFrame<'_>,
) {
    // The lattice is a map of regions, and a map of regions is lines: filling
    // every cell costs as much as filling the screen, which at 4K was most of a
    // frame. A cell is only painted once the user has narrowed the labels, when
    // the few survivors are what is left to look at.
    if view.hint.filtering() {
        for (target, _) in view.hint.visible() {
            let cell = target.cell;
            canvas.rounded_rect(
                cell.x as f32 * scale,
                cell.y as f32 * scale,
                cell.w as f32 * scale,
                cell.h as f32 * scale,
                theme.badge_radius * scale,
                theme.hint_cell(),
            );
        }
    }

    let width = canvas.width() as f32;
    let height = canvas.height() as f32;
    let _ = (width, height);
    for x in &view.lines.0 {
        canvas.vline(*x as f32 * scale, theme.hairline * scale, theme.hint_line());
    }
    for y in &view.lines.1 {
        canvas.hline(*y as f32 * scale, theme.hairline * scale, theme.hint_line());
    }

    for (target, matched) in view.hint.visible() {
        let cell = target.cell;
        // The label keeps a panel of its own: white text over a line grid is
        // unreadable on a light background, and a fill opaque enough to carry it
        // would hide the thing being aimed at.
        badge(
            canvas,
            theme,
            fonts,
            badges,
            (
                (cell.x + cell.w / 2.0) as f32,
                (cell.y + cell.h / 2.0) as f32,
            ),
            &target.label,
            matched,
            scale,
            size,
        );
    }
}

/// One label's panel and text, rasterized once.
struct Cached {
    pixmap: Pixmap,
    /// Where the panel's top left sits relative to the point it was centred on.
    offset: (f32, f32),
}

/// Labels already rasterized for this theme and scale.
///
/// A hint frame draws the same few hundred labels again on every keystroke, and
/// what that cost was the rasterizer's per-path work — two glyphs and a filled,
/// stroked box each — not the area they cover, so it did not improve when the
/// area did. Keyed on the label and how much of it is typed, cleared whenever
/// the values a badge is drawn from change.
#[derive(Default)]
pub struct Badges {
    key: Option<u64>,
    labels: HashMap<(String, usize), Cached>,
}

impl Badges {
    /// The label's raster, drawn into the cache if this is its first appearance.
    fn get(
        &mut self,
        theme: &Theme,
        fonts: &mut Fonts,
        label: &str,
        matched: usize,
        scale: f32,
        size: f32,
    ) -> &Cached {
        let key = fingerprint(theme, scale, size);
        if self.key != Some(key) {
            self.labels.clear();
            self.key = Some(key);
        }
        let entry = (label.to_string(), matched);
        if !self.labels.contains_key(&entry) {
            let cached = rasterize(theme, fonts, label, matched, scale, size);
            self.labels.insert(entry.clone(), cached);
        }
        &self.labels[&entry]
    }

    #[cfg(test)]
    fn cached(&self) -> usize {
        self.labels.len()
    }
}

fn fingerprint(theme: &Theme, scale: f32, size: f32) -> u64 {
    let mut hasher = DefaultHasher::new();
    for colour in [
        theme.text,
        theme.text_matched,
        theme.panel,
        theme.panel_edge,
    ] {
        (colour.r, colour.g, colour.b, colour.a).hash(&mut hasher);
    }
    for value in [
        size,
        theme.badge_padding_x,
        theme.badge_padding_y,
        theme.badge_radius,
        theme.hairline,
        scale,
    ] {
        value.to_bits().hash(&mut hasher);
    }
    hasher.finish()
}

/// Draw a label into a pixmap of its own, centred on nothing yet: the offset the
/// frame places it by comes back with it.
fn rasterize(
    theme: &Theme,
    fonts: &mut Fonts,
    label: &str,
    matched: usize,
    scale: f32,
    size: f32,
) -> Cached {
    let text_width = fonts.measure(label, size);
    let (ascent, descent) = fonts.ascent_descent(size);
    let padding = (theme.badge_padding_x, theme.badge_padding_y);
    let width = (text_width + padding.0 * 2.0) * scale;
    let height = (ascent + descent + padding.1 * 2.0) * scale;
    let (w, h) = (
        (width.ceil() as u32).max(1),
        (height.ceil() as u32).max(1),
    );
    let mut pixmap = Pixmap::new(w, h).expect("a label box is never empty");
    {
        let mut canvas = Canvas::new(pixmap.data_mut(), w, h).expect("the box fits its own pixmap");
        canvas.panel_with_edge(
            0.0,
            0.0,
            width,
            height,
            theme.badge_radius * scale,
            theme.surface(),
            theme.panel_edge,
            theme.hairline * scale,
        );

        let baseline = padding.1 * scale + ascent * scale;
        let text_left = padding.0 * scale;
        let matched_chars = label.chars().count().min(matched);
        if matched_chars > 0 {
            let prefix: String = label.chars().take(matched_chars).collect();
            let prefix_width = fonts.measure(&prefix, size) * scale;
            fonts.draw(
                &mut canvas,
                &prefix,
                text_left,
                baseline,
                size * scale,
                theme.matched(),
            );
            fonts.draw(
                &mut canvas,
                &label.chars().skip(matched_chars).collect::<String>(),
                text_left + prefix_width,
                baseline,
                size * scale,
                theme.text,
            );
        } else {
            fonts.draw(&mut canvas, label, text_left, baseline, size * scale, theme.text);
        }
    }
    Cached {
        pixmap,
        offset: (-width / 2.0, -height / 2.0),
    }
}

/// A label in a translucent rounded panel, from the cache when it has been drawn
/// before. Used by both grid and hint mode so the two cannot drift apart.
fn badge(
    canvas: &mut Canvas,
    theme: &Theme,
    fonts: &mut Option<Fonts>,
    badges: &mut Badges,
    centre: (f32, f32),
    label: &str,
    matched: usize,
    scale: f32,
    size: f32,
) {
    let Some(fonts) = fonts.as_mut() else {
        return;
    };
    let cached = badges.get(theme, fonts, label, matched, scale, size);
    let x = (centre.0 * scale + cached.offset.0).round() as i32;
    let y = (centre.1 * scale + cached.offset.1).round() as i32;
    canvas.blit(x, y, &cached.pixmap);
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
    let mut parts: Vec<String> = Vec::new();
    if frame.dragging {
        parts.push("drag".to_string());
    }
    if let Some(multiplier) = frame.multiplier {
        parts.push(format!("×{multiplier}"));
    }
    if frame.speed != 1.0 {
        parts.push(format!("{}×", frame.speed));
    }
    if parts.is_empty() {
        return;
    }
    let text = parts.join(" ");
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
    use crate::hint::lattice::Cell;
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
        draw(&mut canvas, &theme, &mut fonts, &mut Badges::default(), &frame);

        // The region edge, the interior lines, and a badge at a cell centre.
        // Thresholds rather than exact values: everything is antialiased.
        assert!(alpha_at(&bytes, 1280, 640, 2) > 100, "no region edge");
        assert!(alpha_at(&bytes, 1280, 640, 360) > 100, "no interior line");
        assert!(alpha_at(&bytes, 1280, 320, 180) > 200, "no badge in the cell");
        // And nothing between the marks.
        assert_eq!(alpha_at(&bytes, 1280, 100, 100), 0, "the region was filled");
    }

    fn hint_frame<'a>(
        hint: &'a Hint,
        lines: &'a (Vec<f64>, Vec<f64>),
    ) -> Frame<'a> {
        let mut frame = Frame::normal(1.0, (0.0, 0.0));
        frame.hint = Some(HintFrame { hint, lines });
        frame
    }

    #[test]
    fn a_hint_frame_draws_the_lattice_as_lines() {
        let mut bytes = buffer(640, 400);
        let mut canvas = Canvas::new(&mut bytes, 640, 400).unwrap();
        let theme = Theme::default();
        let (mut fonts, _) = Fonts::load("sans-serif");
        let lattice = crate::hint::lattice::Lattice::new(200.0);
        let cells = lattice.cells((640, 400));
        let lines = lattice.lines((640, 400));
        let hint = Hint::new(cells, &"as".chars().collect::<Vec<_>>());
        draw(
            &mut canvas,
            &theme,
            &mut fonts,
            &mut Badges::default(),
            &hint_frame(&hint, &lines),
        );

        // The lattice is a map of regions, so it is lines, not filled cells.
        assert!(alpha_at(&bytes, 640, 213, 30) > 0, "no cell boundary");
        assert_eq!(
            alpha_at(&bytes, 640, 20, 20),
            0,
            "an unfiltered cell was filled"
        );
        // And every label is drawn, in its panel.
        let label = (80..140)
            .map(|x| alpha_at(&bytes, 640, x, 100))
            .max()
            .unwrap_or(0);
        assert!(label > 200, "no label in the first cell");
    }

    #[test]
    fn only_the_surviving_cells_are_filled_once_a_label_is_typed() {
        let mut bytes = buffer(640, 400);
        let mut canvas = Canvas::new(&mut bytes, 640, 400).unwrap();
        let theme = Theme::default();
        let (mut fonts, _) = Fonts::load("sans-serif");
        let lattice = crate::hint::lattice::Lattice::new(200.0);
        let lines = lattice.lines((640, 400));
        let mut hint = Hint::new(lattice.cells((640, 400)), &"as".chars().collect::<Vec<_>>());
        // Six cells over a two-letter alphabet are labelled in three characters,
        // so one typed letter narrows without completing a label.
        assert!(hint.on_char('a', false).is_empty());
        draw(
            &mut canvas,
            &theme,
            &mut fonts,
            &mut Badges::default(),
            &hint_frame(&hint, &lines),
        );

        let fill = alpha_at(&bytes, 640, 20, 20);
        assert_eq!(fill, theme.hint_cell().a, "a surviving cell was not filled");
        assert!(fill < 0x80, "the fill is not translucent: {fill}");
        // A cell whose label does not start with `a` is still a boundary in the
        // lattice, and no longer a filled region.
        let gone = alpha_at(&bytes, 640, 620, 380);
        assert_eq!(gone, 0, "a filtered-out cell was filled");
    }

    #[test]
    fn a_label_is_rasterized_once_per_frame_and_reused() {
        let mut bytes = buffer(640, 400);
        let mut canvas = Canvas::new(&mut bytes, 640, 400).unwrap();
        let theme = Theme::default();
        let (mut fonts, _) = Fonts::load("sans-serif");
        let lattice = crate::hint::lattice::Lattice::new(200.0);
        let lines = lattice.lines((640, 400));
        let hint = Hint::new(lattice.cells((640, 400)), &"as".chars().collect::<Vec<_>>());
        let mut badges = Badges::default();

        draw(&mut canvas, &theme, &mut fonts, &mut badges, &hint_frame(&hint, &lines));
        let first = badges.cached();
        assert!(first > 0, "no labels were cached");
        canvas.clear(Rgba::rgba(0, 0, 0, 0));
        draw(&mut canvas, &theme, &mut fonts, &mut badges, &hint_frame(&hint, &lines));
        assert_eq!(badges.cached(), first, "the second frame rasterized again");
    }

    #[test]
    fn a_speed_preset_is_named_beside_the_cursor() {
        // The status element is the only thing that can tell the user a session
        // was left slow, so it has to appear for a preset and stay away for the
        // normal one.
        let painted = |speed: f64| -> usize {
            let mut bytes = buffer(400, 200);
            let mut canvas = Canvas::new(&mut bytes, 400, 200).unwrap();
            let theme = Theme::default();
            let (mut fonts, _) = Fonts::load("sans-serif");
            let mut frame = Frame::normal(1.0, (50.0, 50.0));
            frame.speed = speed;
            draw(&mut canvas, &theme, &mut fonts, &mut Badges::default(), &frame);
            // The status sits down and to the right of the cursor ring, which
            // never reaches past (60, 60).
            (60..200)
                .map(|y| {
                    (60..400)
                        .filter(|x| alpha_at(&bytes, 400, *x, y) > 0)
                        .count()
                })
                .sum()
        };
        assert!(painted(0.1) > 0, "the fine preset is not shown");
        assert_eq!(painted(1.0), 0, "the normal preset is labelled anyway");
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
