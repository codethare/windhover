//! Virtual pointer: absolute motion, buttons, wheel.

use wayland_client::{
    protocol::{wl_output, wl_pointer, wl_seat},
    Proxy, QueueHandle,
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
    /// `create_virtual_pointer_with_output` is version 2; compositors that only
    /// advertise version 1 get the plain constructor and therefore position the
    /// pointer in the global coordinate space.
    pub fn create(
        manager: &zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1,
        seat: Option<&wl_seat::WlSeat>,
        output: Option<&wl_output::WlOutput>,
        qh: &QueueHandle<State>,
    ) -> Self {
        let proxy = if wants_output_variant(manager.version()) {
            manager.create_virtual_pointer_with_output(seat, output, qh, ())
        } else {
            manager.create_virtual_pointer(seat, qh, ())
        };
        Self { proxy }
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

    /// `axis_source` plus `axis_discrete`: a bare `axis` is frequently ignored
    /// by compositors that expect wheel events to carry a notch count.
    /// Positive steps scroll down, matching the sign convention of `wl_pointer`.
    pub fn scroll(&self, steps: i32) {
        if steps == 0 {
            return;
        }
        self.proxy.axis_source(wl_pointer::AxisSource::Wheel);
        self.proxy.axis_discrete(
            timestamp_ms(),
            wl_pointer::Axis::VerticalScroll,
            steps as f64 * 15.0,
            steps,
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

/// `create_virtual_pointer_with_output` arrived in version 2; binding it on a
/// version 1 manager is a protocol error, so the constructor has to be chosen
/// from the version the compositor actually advertised.
fn wants_output_variant(manager_version: u32) -> bool {
    manager_version >= 2
}

#[cfg(test)]
mod tests {
    use super::wants_output_variant;

    #[test]
    fn version_one_manager_uses_the_plain_constructor() {
        assert!(!wants_output_variant(1));
    }

    #[test]
    fn version_two_manager_binds_the_pointer_to_an_output() {
        assert!(wants_output_variant(2));
        assert!(wants_output_variant(3));
    }
}
