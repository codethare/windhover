//! Single source of truth for colours, geometry, and typography.
//!
//! Every mode draws through these values and through the primitives in
//! [`super::canvas`], which is what keeps the three modes looking like one
//! thing. Nothing outside this module should name a colour or a radius.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }

}

#[derive(Debug, Clone)]
pub struct Theme {
    /// The one accent everything is tinted with.
    pub accent: Rgba,
    pub text: Rgba,
    /// Label characters that have already been typed away.
    pub text_matched: Rgba,
    pub panel: Rgba,
    /// The hairline that gives translucent panels an edge on any background.
    pub panel_edge: Rgba,
    pub shadow: Rgba,

    pub corner_radius: f32,
    pub badge_radius: f32,
    pub hairline: f32,
    pub grid_line_width: f32,
    pub region_edge_width: f32,
    pub badge_padding_x: f32,
    pub badge_padding_y: f32,

    pub font_family: String,
    pub badge_font_size: f32,
    pub status_font_size: f32,
}

impl Default for Theme {
    /// macOS dark: a single system blue, translucent neutral panels, small
    /// radii, and hairlines rather than heavy borders.
    fn default() -> Self {
        Self {
            accent: Rgba::rgb(0x0A, 0x84, 0xFF),
            text: Rgba::rgb(0xFF, 0xFF, 0xFF),
            text_matched: Rgba::rgba(0xFF, 0xFF, 0xFF, 0x66),
            panel: Rgba::rgba(0x1C, 0x1C, 0x1E, 0xE0),
            panel_edge: Rgba::rgba(0xFF, 0xFF, 0xFF, 0x26),
            shadow: Rgba::rgba(0x00, 0x00, 0x00, 0x59),

            corner_radius: 6.0,
            badge_radius: 5.0,
            hairline: 1.0,
            grid_line_width: 1.5,
            region_edge_width: 2.0,
            badge_padding_x: 7.0,
            badge_padding_y: 3.0,

            font_family: "sans-serif".to_string(),
            badge_font_size: 14.0,
            status_font_size: 13.0,
        }
    }
}

impl Theme {
    /// A translucent surface that needs an edge to stay legible: used by badges
    /// and panels alike.
    pub fn surface(&self) -> Rgba {
        self.panel
    }

    /// The character positions a hint label has already consumed.
    pub fn matched(&self) -> Rgba {
        self.text_matched
    }

    /// A hint cell's fill. Quiet enough that what the cell covers stays the
    /// thing being read, which is the whole point of hint mode.
    pub fn hint_cell(&self) -> Rgba {
        self.panel.with_alpha(0x28)
    }

    /// The lattice's own lines: quieter than grid mode's, because they are a map
    /// of the whole screen rather than the edges of one region.
    pub fn hint_line(&self) -> Rgba {
        self.panel_edge.with_alpha(0x5A)
    }

    /// Everything tinted deriv[ies] from [`Theme::accent`] rather than naming a
    /// colour of its own, so a single configuration line moves the whole
    /// interface together.
    pub fn grid_line(&self) -> Rgba {
        self.accent.with_alpha(0x99)
    }

    pub fn region_edge(&self) -> Rgba {
        self.accent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_variants_keep_the_colour() {
        let base = Rgba::rgb(10, 20, 30);
        let faded = base.with_alpha(128);
        assert_eq!((faded.r, faded.g, faded.b), (10, 20, 30));
        assert_eq!(faded.a, 128);
    }

    #[test]
    fn the_palette_derives_every_tint_from_the_accent() {
        // The point of the rule: one accent moves the lines, the borders and the
        // cursor together.
        let theme = Theme {
            accent: Rgba::rgb(0xFF, 0x95, 0x00),
            ..Default::default()
        };
        assert_eq!(theme.region_edge(), theme.accent);
        assert_eq!(theme.grid_line().r, 0xFF);
        assert_eq!(theme.grid_line().g, 0x95);
        assert_eq!(theme.grid_line().b, 0x00);
        assert_eq!(theme.grid_line().a, 0x99, "grid lines should be quieter");
    }

    #[test]
    fn a_colour_is_carried_by_its_alpha_variants() {
        let theme = Theme::default();
        assert_eq!(theme.grid_line().a < theme.accent.a, true);
        assert_eq!(theme.matched().a < theme.text.a, true);
    }

    #[test]
    fn a_hint_cell_is_quieter_than_a_badge() {
        // The cell covers a whole region, so it has to be far more transparent
        // than the panel a label sits in.
        let theme = Theme::default();
        assert!(theme.hint_cell().a < theme.surface().a / 2);
        assert_eq!(theme.hint_cell().a > 0, true, "a cell has to be visible");
    }
}
