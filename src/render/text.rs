//! Font loading, text measurement, and drawing.
//!
//! `swash` rasterizes; the family name is resolved through the system font
//! database with `fc-match`, falling back to a list of well-known paths when
//! fontconfig is not installed.

use std::path::{Path, PathBuf};
use std::process::Command;

use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::zeno::Format;
use swash::{FontRef, GlyphId};

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

/// Resolve a family name to a font file. `fc-match` first, because it knows
/// about aliases (`sans-serif`) and user-installed fonts; the fixed list is for
/// systems without fontconfig.
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
    offset: u32,
    key: swash::CacheKey,
    context: ScaleContext,
    render: Render<'static>,
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
                Some(format!("windhover: could not read {}; labels will not be drawn", path.display())),
            );
        };
        let Some(font) = FontRef::from_index(&data, 0) else {
            return (
                None,
                Some(format!("windhover: {} is not a font file", path.display())),
            );
        };
        let offset = font.offset;
        let key = font.key;

        let warning = substituted.then(|| {
            format!(
                "windhover: no fontconfig answer for `{family}`; falling back to {}",
                path.display()
            )
        });

        let fonts = Self {
            data,
            offset,
            key,
            context: ScaleContext::new(),
            // Static sources: a colour outline first so emoji and colour fonts
            // keep their colours, then a plain outline.
            render: Render::new(&[
                Source::ColorOutline(0),
                Source::ColorBitmap(StrikeWith::BestFit),
                Source::Outline,
            ]),
        };
        (Some(fonts), warning)
    }

    /// Reusing offset and key keeps swash's caches warm across calls, so the
    /// reference is rebuilt from the stored fields rather than re-parsed.
    fn reference(data: &[u8], offset: u32, key: swash::CacheKey) -> FontRef<'_> {
        FontRef { data, offset, key }
    }

    /// Distance above and below the baseline. Both come back positive, so
    /// centring a line on `y` is `y + (ascent - descent) / 2` for the baseline.
    pub fn ascent_descent(&self, size: f32) -> (f32, f32) {
        let font = Self::reference(&self.data, self.offset, self.key);
        let metrics = font.metrics(&[]);
        let scale = size / metrics.units_per_em as f32;
        (metrics.ascent as f32 * scale, metrics.descent as f32 * scale)
    }

    /// Width of `text` at `size`, in logical pixels.
    pub fn measure(&self, text: &str, size: f32) -> f32 {
        let font = Self::reference(&self.data, self.offset, self.key);
        let metrics = font.glyph_metrics(&[]).scale(size);
        let charmap = font.charmap();
        text.chars()
            .map(|ch| metrics.advance_width(charmap.map(ch)))
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
        // Destructured so the font borrows only the font data while the scale
        // context and the render state are borrowed mutably.
        let Self {
            data,
            offset,
            key,
            context,
            render: render_slot,
            ..
        } = self;
        let font = Self::reference(data, *offset, *key);
        let charmap = font.charmap();
        let mut scaler = context.builder(font).size(size).hint(true).build();
        let mut render = std::mem::replace(render_slot, Render::new(&[Source::Outline]));
        render.format(Format::Alpha);

        let mut pen = left;
        for ch in text.chars() {
            let glyph: GlyphId = charmap.map(ch);
            if let Some(image) = render.render(&mut scaler, glyph) {
                let placement = image.placement;
                canvas.blit_mask(
                    pen + placement.left as f32,
                    baseline - placement.top as f32,
                    placement.width,
                    placement.height,
                    &image.data,
                    colour,
                );
            }
            pen += font
                .glyph_metrics(&[])
                .scale(size)
                .advance_width(glyph);
        }

        *render_slot = render;
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
}
