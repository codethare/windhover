//! Output discovery, logical geometry, and scale.

use wayland_client::{
    protocol::wl_output::{self, WlOutput},
    Dispatch,
};
use wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_v1::{self, ZxdgOutputV1};

use super::State;

/// What we need to know about one output. Positions and sizes are logical
/// pixels; `scale` is the integer scale of the output.
#[derive(Debug, Clone, Default)]
pub struct OutputInfo {
    pub name: String,
    pub logical_x: i32,
    pub logical_y: i32,
    pub logical_w: u32,
    pub logical_h: u32,
    pub scale: i32,
    /// Set by `zxdg_output_v1::LogicalPosition`. A pointer that cannot be bound
    /// to an output has to translate into the layout, which needs this.
    pub position_known: bool,
}

impl OutputInfo {
    pub fn logical_size(&self) -> (u32, u32) {
        (self.logical_w, self.logical_h)
    }

    /// Whether the compositor told us where this output sits in the layout.
    /// Without `zxdg_output_v1` every output claims (0, 0), which is only
    /// correct for one of them.
    pub fn position_known(&self) -> bool {
        self.position_known
    }

    /// The point this output's top-left corner sits at in the layout.
    pub fn logical_origin(&self) -> (i32, i32) {
        (self.logical_x, self.logical_y)
    }

    fn label(&self, index: usize) -> String {
        if self.name.is_empty() {
            format!("output-{index}")
        } else {
            self.name.clone()
        }
    }
}

#[derive(Default)]
pub struct OutputEntry {
    pub info: OutputInfo,
    pub output: Option<WlOutput>,
    /// Held so the compositor keeps sending this output's geometry.
    pub _xdg: Option<ZxdgOutputV1>,
    pub mode: Option<(i32, i32)>,
}
impl OutputEntry {
    /// The compositor may not provide `zxdg_output_v1`; fall back to the mode
    /// size divided by the integer scale, which is the same thing whenever the
    /// compositor rounds fractional scale the usual way.
    fn resolve(&mut self, index: usize) {
        if self.info.logical_w == 0 || self.info.logical_h == 0 {
            if let Some((w, h)) = self.mode {
                let scale = self.info.scale.max(1);
                self.info.logical_w = (w / scale).max(1) as u32;
                self.info.logical_h = (h / scale).max(1) as u32;
            }
        }
        self.info.name = self.info.label(index);
    }
}

/// The box the compositor's outputs occupy: its top-left corner and its size.
/// An unbound virtual pointer normalises absolute coordinates over this box, not
/// over a box starting at (0, 0), so a point is only addressable relative to it.
pub(super) fn layout_box(outputs: &[OutputEntry], overlay: (u32, u32)) -> ((i32, i32), (u32, u32)) {
    let mut corner: Option<(i32, i32, i32, i32)> = None;
    for entry in outputs {
        let (w, h) = entry.info.logical_size();
        if w == 0 || h == 0 {
            continue;
        }
        let (x, y) = entry.info.logical_origin();
        corner = Some(match corner {
            Some((x0, y0, x1, y1)) => (
                x0.min(x),
                y0.min(y),
                x1.max(x.saturating_add(w as i32)),
                y1.max(y.saturating_add(h as i32)),
            ),
            None => (x, y, x.saturating_add(w as i32), y.saturating_add(h as i32)),
        });
    }
    match corner {
        Some((x0, y0, x1, y1)) => ((x0, y0), ((x1 - x0).max(1) as u32, (y1 - y0).max(1) as u32)),
        None => ((0, 0), overlay),
    }
}

impl Dispatch<WlOutput, usize> for State {
    fn event(
        state: &mut Self,
        _proxy: &WlOutput,
        event: wl_output::Event,
        index: &usize,
        _conn: &wayland_client::Connection,
        _qh: &wayland_client::QueueHandle<Self>,
    ) {
        let Some(entry) = state.outputs.get_mut(*index) else {
            return;
        };
        match event {
            wl_output::Event::Geometry { .. } => {}
            wl_output::Event::Mode { width, height, .. } => entry.mode = Some((width, height)),
            wl_output::Event::Scale { factor } => entry.info.scale = factor,
            wl_output::Event::Name { name } => entry.info.name = name,
            wl_output::Event::Done => entry.resolve(*index),
            _ => {}
        }
    }
}

impl Dispatch<ZxdgOutputV1, usize> for State {
    fn event(
        state: &mut Self,
        _proxy: &ZxdgOutputV1,
        event: zxdg_output_v1::Event,
        index: &usize,
        _conn: &wayland_client::Connection,
        _qh: &wayland_client::QueueHandle<Self>,
    ) {
        let Some(entry) = state.outputs.get_mut(*index) else {
            return;
        };
        match event {
            zxdg_output_v1::Event::LogicalPosition { x, y } => {
                entry.info.logical_x = x;
                entry.info.logical_y = y;
                entry.info.position_known = true;
            }
            zxdg_output_v1::Event::LogicalSize { width, height } => {
                entry.info.logical_w = width.max(0) as u32;
                entry.info.logical_h = height.max(0) as u32;
            }
            zxdg_output_v1::Event::Name { name } => entry.info.name = name,
            zxdg_output_v1::Event::Description { .. } => {}
            zxdg_output_v1::Event::Done => entry.resolve(*index),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(x: i32, y: i32, w: u32, h: u32) -> OutputEntry {
        OutputEntry {
            info: OutputInfo {
                logical_x: x,
                logical_y: y,
                logical_w: w,
                logical_h: h,
                position_known: true,
                ..OutputInfo::default()
            },
            ..OutputEntry::default()
        }
    }

    #[test]
    fn the_layout_starts_at_the_top_left_output() {
        // Two outputs starting at the origin: the box is their union.
        let outputs = [output(0, 0, 1280, 720), output(1280, 0, 1920, 1080)];
        assert_eq!(layout_box(&outputs, (1280, 720)), ((0, 0), (3200, 1080)));
    }

    #[test]
    fn a_layout_that_does_not_start_at_the_origin_is_still_a_box() {
        // No output is at (0, 0): the box starts where the leftmost one does, so
        // a coordinate normalised over it lands on an output rather than in the
        // empty space before them.
        let outputs = [output(1280, 0, 1280, 720), output(2560, 0, 1280, 720)];
        assert_eq!(layout_box(&outputs, (1280, 720)), ((1280, 0), (2560, 720)));
    }

    #[test]
    fn an_output_above_the_origin_extends_the_box_upwards() {
        let outputs = [output(0, 0, 1280, 720), output(0, -720, 1280, 720)];
        assert_eq!(layout_box(&outputs, (1280, 720)), ((0, -720), (1280, 1440)));
    }

    #[test]
    fn without_any_reported_size_the_overlay_is_the_box() {
        assert_eq!(layout_box(&[], (1280, 720)), ((0, 0), (1280, 720)));
        assert_eq!(
            layout_box(&[OutputEntry::default()], (800, 600)),
            ((0, 0), (800, 600))
        );
    }

    #[test]
    fn a_position_is_only_known_once_the_compositor_says_so() {
        assert!(!OutputInfo::default().position_known());
        assert!(output(1280, 0, 1280, 720).info.position_known());
    }
}
