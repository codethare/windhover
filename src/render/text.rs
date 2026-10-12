//! Font loading, text measurement, and drawing.
//!
//! Glyph outlines come from `skrifa` (the Google Fonts `fontations` parser) and
//! are filled with `tiny-skia`, which is the same rasterizer the rest of the
//! overlay draws with. The family name is resolved through the system font
//! database with `fc-match`, falling back to a list of well-known paths when
//! fontconfig is not installed.

use std::path::{Path, PathBuf};
use std::process::Command;

use skrifa::outline::OutlinePen;
use skrifa::prelude::*;
use tiny_skia::PathBuilder;

use super::canvas::Canvas;
use super::theme::Rgba;

/// Tried in order when `fc-match` cannot answer. Regular weights first: a bold
/// face would break the restrained look the theme is after.
const FALLBACK_PATHS: &[&str] = &[
    "/usr/share/fonts/Inter-Regular.ttf",
    "/usr/share/fonts/TTF/Inter-Regular.ttf",
    "/usr/share/fonts/inter/Inter-Regular.ttf",
    "/usr/share/fonts/noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/TTF/DejaVuSans.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/liberation/LiberationSans-Regular.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
];

/// Resolve a family name to a font file and say whether it came from the
/// fallback list rather than the system's own font database. A generic name
/// like `sans-serif` resolving to a concrete file is the system answering the
/// question, not a substitution, so it is not worth a warning.
pub fn find_font(family: &str) -> Option<(PathBuf, bool)> {
    if let Some(path) = fc_match(family) {
        return Some((path, false));
    }
    FALLBACK_PATHS
        .iter()
        .map(PathBuf::from)
        .find(|path| path.is_file())
        .map(|path| (path, true))
}

fn fc_match(family: &str) -> Option<PathBuf> {
    // The family is passed as its own argument, never through a shell, so a
    // configuration value cannot become a command.
    let output = Command::new("fc-match")
        .args(["-f", "%{file}"])
        .arg(family)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let path = Path::new(&path);
    path.is_file().then(|| path.to_path_buf())
}

pub struct Fonts {
    data: Vec<u8>,
}

impl Fonts {
    /// Loads `family`, or the first fallback that exists. Returns the fonts and
    /// a warning to print when the requested family could not be used.
    pub fn load(family: &str) -> (Option<Self>, Option<String>) {
        let Some((path, substituted)) = find_font(family) else {
            return (
                None,
                Some(format!(
                    "windhover: found no usable font for `{family}`; labels will not be drawn"
                )),
            );
        };
        let Ok(data) = std::fs::read(&path) else {
            return (
                None,
                Some(format!(
                    "windhover: could not read {}; labels will not be drawn",
                    path.display()
                )),
            );
        };
        if FontRef::from_index(&data, 0).is_err() {
            return (
                None,
                Some(format!("windhover: {} is not a font file", path.display())),
            );
        }
        let warning = substituted.then(|| {
            format!(
                "windhover: no fontconfig answer for `{family}`; falling back to {}",
                path.display()
            )
        });
        (Some(Self { data }), warning)
    }

    /// A parsed view of the loaded file. Cheap: it validates the table
    /// directory, nothing more.
    fn font(&self) -> Option<FontRef<'_>> {
        FontRef::from_index(&self.data, 0).ok()
    }

    /// Distance above and below the baseline, both positive, so centring a line
    /// on `y` is `y + (ascent - descent) / 2` for the baseline.
    pub fn ascent_descent(&self, size: f32) -> (f32, f32) {
        let Some(metrics) = self.font().map(|font| font.metrics(Size::new(size), LocationRef::default())) else {
            // A readable default rather than a zero-height badge.
            return (size * 0.8, size * 0.2);
        };
        // skrifa reports the descent as a negative distance from the baseline.
        (metrics.ascent, -metrics.descent)
    }

    /// Width of `text` at `size`, in logical pixels.
    pub fn measure(&self, text: &str, size: f32) -> f32 {
        let Some(font) = self.font() else {
            return 0.0;
        };
        let charmap = font.charmap();
        let metrics = font.glyph_metrics(Size::new(size), LocationRef::default());
        text.chars()
            .map(|ch| glyph_for(&charmap, ch))
            .filter_map(|glyph| metrics.advance_width(glyph))
            .sum()
    }

    /// Draw `text` with its baseline at `baseline`, centred on `centre_x`.
    pub fn draw_centred(
        &mut self,
        canvas: &mut Canvas,
        text: &str,
        centre_x: f32,
        baseline: f32,
        size: f32,
        colour: Rgba,
    ) {
        let width = self.measure(text, size);
        self.draw(canvas, text, centre_x - width / 2.0, baseline, size, colour);
    }

    pub fn draw(
        &mut self,
        canvas: &mut Canvas,
        text: &str,
        left: f32,
        baseline: f32,
        size: f32,
        colour: Rgba,
    ) {
        let Some(font) = self.font() else {
            return;
        };
        let size = Size::new(size);
        let location = LocationRef::default();
        let charmap = font.charmap();
        let metrics = font.glyph_metrics(size, location);
        let outlines = font.outline_glyphs();

        let mut pen_x = left;
        for ch in text.chars() {
            let glyph = glyph_for(&charmap, ch);
            if let Some(outline) = outlines.get(glyph) {
                let mut builder = PathBuilder::new();
                let mut pen = GlyphPen {
                    builder: &mut builder,
                    origin: pen_x,
                    baseline,
                };
                // A glyph whose outline cannot be drawn still advances the pen.
                if outline.draw(size, &mut pen).is_ok() {
                    if let Some(path) = builder.finish() {
                        canvas.fill_path(&path, colour);
                    }
                }
            }
            pen_x += metrics.advance_width(glyph).unwrap_or(0.0);
        }
    }
}

/// A character the font has no mapping for is drawn as `.notdef`, which is what
/// every other text stack does with it.
fn glyph_for(charmap: &skrifa::charmap::Charmap<'_>, ch: char) -> GlyphId {
    charmap.map(ch).unwrap_or(GlyphId::new(0))
}

/// Forwards a glyph outline into a `tiny-skia` path.
///
/// Font outlines are laid out with y increasing upwards from the baseline; the
/// canvas has y increasing downwards, so every y is mirrored about the baseline.
struct GlyphPen<'a> {
    builder: &'a mut PathBuilder,
    origin: f32,
    baseline: f32,
}

impl GlyphPen<'_> {
    fn point(&self, x: f32, y: f32) -> (f32, f32) {
        (self.origin + x, self.baseline - y)
    }
}

impl OutlinePen for GlyphPen<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.point(x, y);
        self.builder.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.point(x, y);
        self.builder.line_to(x, y);
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        let (cx0, cy0) = self.point(cx0, cy0);
        let (x, y) = self.point(x, y);
        self.builder.quad_to(cx0, cy0, x, y);
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        let (cx0, cy0) = self.point(cx0, cy0);
        let (cx1, cy1) = self.point(cx1, cy1);
        let (x, y) = self.point(x, y);
        self.builder.cubic_to(cx0, cy0, cx1, cy1, x, y);
    }

    fn close(&mut self) {
        self.builder.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_generic_family_is_answered_without_a_warning() {
        // `sans-serif` must resolve on any system that has a font at all, and
        // the system answering is not a substitution.
        let (path, substituted) = find_font("sans-serif").expect("no font found for sans-serif");
        assert!(path.is_file());
        assert!(!substituted, "fontconfig answered but we called it a fallback");
    }

    #[test]
    fn an_unknown_family_still_falls_back_to_something_readable() {
        let (fonts, warning) = Fonts::load("A Family That Does Not Exist");
        let fonts = fonts.expect("fallback font");
        if let Some(warning) = warning {
            assert!(warning.contains("A Family That Does Not Exist"), "{warning}");
        }
        assert!(fonts.measure("W", 20.0) > 0.0);
    }

    #[test]
    fn measurement_is_monotonic_in_the_font_size() {
        let (Some(fonts), _) = Fonts::load("sans-serif") else {
            panic!("no font");
        };
        let small = fonts.measure("windhover", 10.0);
        let large = fonts.measure("windhover", 20.0);
        assert!(large > small, "{large} !> {small}");
        let (ascent_small, descent_small) = fonts.ascent_descent(10.0);
        let (ascent_large, descent_large) = fonts.ascent_descent(20.0);
        assert!(ascent_large + descent_large > ascent_small + descent_small);
    }

    #[test]
    fn a_longer_string_measures_wider() {
        let (Some(fonts), _) = Fonts::load("sans-serif") else {
            panic!("no font");
        };
        assert!(fonts.measure("aaaa", 16.0) > fonts.measure("aa", 16.0));
    }

    #[test]
    fn measurement_survives_characters_the_font_lacks() {
        let (Some(fonts), _) = Fonts::load("sans-serif") else {
            panic!("no font");
        };
        // U+10FFFF is unassigned; it must measure like .notdef rather than panic.
        assert!(fonts.measure("\u{10FFFF}", 16.0) > 0.0);
    }

    #[test]
    fn the_ascent_and_descent_are_both_positive_distances() {
        let (Some(fonts), _) = Fonts::load("sans-serif") else {
            panic!("no font");
        };
        let (ascent, descent) = fonts.ascent_descent(20.0);
        assert!(ascent > 0.0, "ascent = {ascent}");
        assert!(descent > 0.0, "descent = {descent}");
        // A line of text is taller than it is wide for a single character.
        assert!(ascent + descent > fonts.measure("M", 20.0));
    }
}
