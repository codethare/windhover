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
}

impl OutputInfo {
    pub fn logical_size(&self) -> (u32, u32) {
        (self.logical_w, self.logical_h)
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
