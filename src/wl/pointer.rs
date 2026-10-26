//! Virtual pointer: absolute motion, buttons, wheel.

use wayland_client::{
    protocol::{wl_pointer, wl_seat},
    QueueHandle,
};
use wayland_protocols_wlr::virtual_pointer::v1::client::{
    zwlr_virtual_pointer_manager_v1, zwlr_virtual_pointer_v1,
};

use super::State;

pub const BTN_LEFT: u32 = 0x110;
pub const BTN_MIDDLE: u32 = 0x112;
pub const BTN_RIGHT: u32 = 0x111;

pub struct VirtualPointer {
    proxy: zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1,
}

impl VirtualPointer {
    /// Absolute motion is in the compositor's layout, not on one output.
    ///
    /// `create_virtual_pointer_with_output` would address a single output, which
    /// is the smaller space, but the three compositors do not agree on it: niri
    /// maps a bound pointer's `y` through the output's transform and a warp lands
    /// reflected. The layout space is what a pointer created without an output
    /// gets on all of them, so the caller translates into it instead.
    pub fn create(
        manager: &zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1,
        seat: Option<&wl_seat::WlSeat>,
        qh: &QueueHandle<State>,
    ) -> Self {
        Self {
            proxy: manager.create_virtual_pointer(seat, qh, ()),
        }
    }

    /// The last addressable coordinate is `extent - 1`: `motion_absolute`
    /// carries a normalised position, and asking for exactly `extent` normalises
    /// to 1.0, which compositors treat as past the edge — niri answers it by
    /// moving the pointer off the surface entirely.
    pub fn move_absolute(&self, x: f64, y: f64, extent_w: u32, extent_h: u32) {
        let time = timestamp_ms();
        let last_x = extent_w.saturating_sub(1) as f64;
        let last_y = extent_h.saturating_sub(1) as f64;
        let x = x.clamp(0.0, last_x).round() as u32;
        let y = y.clamp(0.0, last_y).round() as u32;
        self.proxy.motion_absolute(time, x, y, extent_w, extent_h);
        self.proxy.frame();
    }

    pub fn button(&self, button: u32, pressed: bool) {
        let state = if pressed {
            wl_pointer::ButtonState::Pressed
        } else {
            wl_pointer::ButtonState::Released
        };
        self.proxy.button(timestamp_ms(), button, state);
        self.proxy.frame();
    }

    /// `axis` is the event that carries the scroll. `axis_discrete` is only
    /// extra information about the same axis value, and the protocol has it
    /// never occur alone: wlroots drops a discrete event with no `axis` behind
    /// it, and Smithay adds the two values together and scrolls twice as far.
    /// Positive steps scroll down, matching the sign convention of `wl_pointer`.
    pub fn scroll(&self, steps: i32) {
        if steps == 0 {
            return;
        }
        let value = steps as f64 * 15.0;
        self.proxy.axis_source(wl_pointer::AxisSource::Wheel);
        self.proxy.axis(
            timestamp_ms(),
            wl_pointer::Axis::VerticalScroll,
            value,
        );
        self.proxy.frame();
    }

    pub fn destroy(self) {
        self.proxy.destroy();
    }
}

pub fn timestamp_ms() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u32)
        .unwrap_or(0)
}
