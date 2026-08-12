//! virtual-joystick: An on-screen joystick Control drawn entirely in Rust.
//! Reports a clamped unit-vector axis value that other nodes can read.
//!
//! Teaches: an on-screen joystick drawn with `draw_arc` / `draw_circle`, including a
//! dead zone so a resting thumb reads as no input.

use godot::classes::{Control, IControl, InputEvent, InputEventMouseButton, InputEventMouseMotion};
use godot::prelude::*;

struct VirtualJoystickExtension;
#[gdextension]
unsafe impl ExtensionLibrary for VirtualJoystickExtension {}

#[derive(GodotClass)]
#[class(base=Control)]
struct VirtualJoystick {
    /// Outer ring radius in pixels.
    #[export]
    radius: f32,
    /// Dead zone threshold (0–1). Inputs below this magnitude report zero.
    #[export]
    dead_zone: f32,

    active: bool,
    origin_x: f32,
    origin_y: f32,
    current_x: f32,
    current_y: f32,

    base: Base<Control>,
}

#[godot_api]
impl IControl for VirtualJoystick {
    fn init(base: Base<Control>) -> Self {
        Self {
            radius: 60.0,
            dead_zone: 0.1,
            active: false,
            origin_x: 0.0,
            origin_y: 0.0,
            current_x: 0.0,
            current_y: 0.0,
            base,
        }
    }

    fn ready(&mut self) {
        godot_print!("VirtualJoystick ready (radius={})", self.radius);
    }

    fn gui_input(&mut self, event: Gd<InputEvent>) {
        if let Ok(btn) = event.clone().try_cast::<InputEventMouseButton>() {
            if btn.get_button_index() == godot::global::MouseButton::LEFT {
                if btn.is_pressed() {
                    let pos = btn.get_position();
                    self.active = true;
                    self.origin_x = pos.x;
                    self.origin_y = pos.y;
                    self.current_x = pos.x;
                    self.current_y = pos.y;
                } else {
                    self.active = false;
                    self.current_x = self.origin_x;
                    self.current_y = self.origin_y;
                }
                self.base_mut().queue_redraw();
            }
        } else if let Ok(motion) = event.try_cast::<InputEventMouseMotion>()
            && self.active
        {
            let pos = motion.get_position();
            let (cx, cy) =
                clamp_to_circle(pos.x - self.origin_x, pos.y - self.origin_y, self.radius);
            self.current_x = self.origin_x + cx;
            self.current_y = self.origin_y + cy;
            self.base_mut().queue_redraw();
        }
    }

    fn draw(&mut self) {
        // Copy fields to locals to avoid borrow conflicts with base_mut().
        let ox = self.origin_x;
        let oy = self.origin_y;
        let r = self.radius;
        let knob_r = r * 0.4;
        let (ax, ay) = self.axis_tuple();

        // Outer ring (draw_arc takes 6 args in gdext 0.5: center, radius, start, end, point_count, color)
        self.base_mut().draw_arc(
            Vector2::new(ox, oy),
            r,
            0.0,
            std::f32::consts::TAU,
            32,
            Color::from_rgba(1.0, 1.0, 1.0, 0.4),
        );

        // Inner knob
        let kx = ox + ax * r;
        let ky = oy + ay * r;
        self.base_mut()
            .draw_circle(Vector2::new(kx, ky), knob_r, Color::from_rgb(0.9, 0.9, 0.9));
    }
}

#[godot_api]
impl VirtualJoystick {
    /// Returns the current joystick axis as a clamped unit Vector2.
    #[func]
    pub fn get_axis(&self) -> Vector2 {
        let (ax, ay) = self.axis_tuple();
        Vector2::new(ax, ay)
    }

    fn axis_tuple(&self) -> (f32, f32) {
        compute_axis(
            self.origin_x,
            self.origin_y,
            self.current_x,
            self.current_y,
            self.radius,
            self.dead_zone,
        )
    }
}

// ---------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------

/// Compute a normalised, dead-zone-applied axis from origin and current touch points.
pub fn compute_axis(ox: f32, oy: f32, cx: f32, cy: f32, radius: f32, dead_zone: f32) -> (f32, f32) {
    let dx = cx - ox;
    let dy = cy - oy;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-6 {
        return (0.0, 0.0);
    }
    let normalised = len / radius;
    if normalised < dead_zone {
        return (0.0, 0.0);
    }
    let clamped = normalised.min(1.0);
    let nx = dx / len;
    let ny = dy / len;
    (nx * clamped, ny * clamped)
}

/// Clamp a displacement vector (dx, dy) so its length does not exceed `r`.
pub fn clamp_to_circle(dx: f32, dy: f32, r: f32) -> (f32, f32) {
    let len = (dx * dx + dy * dy).sqrt();
    if len <= r {
        (dx, dy)
    } else {
        (dx / len * r, dy / len * r)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_zero_at_origin() {
        let (ax, ay) = compute_axis(0.0, 0.0, 0.0, 0.0, 60.0, 0.1);
        assert_eq!(ax, 0.0);
        assert_eq!(ay, 0.0);
    }

    #[test]
    fn axis_zero_in_dead_zone() {
        // 5 pixels out of 60 radius = 0.083 < dead_zone 0.1
        let (ax, ay) = compute_axis(0.0, 0.0, 5.0, 0.0, 60.0, 0.1);
        assert_eq!(ax, 0.0);
        assert_eq!(ay, 0.0);
    }

    #[test]
    fn axis_full_right() {
        let (ax, ay) = compute_axis(0.0, 0.0, 60.0, 0.0, 60.0, 0.1);
        assert!((ax - 1.0).abs() < 0.001);
        assert!(ay.abs() < 0.001);
    }

    #[test]
    fn axis_clamped_past_radius() {
        let (ax, _ay) = compute_axis(0.0, 0.0, 200.0, 0.0, 60.0, 0.0);
        assert!((ax - 1.0).abs() < 0.001);
    }

    #[test]
    fn clamp_inside_circle() {
        let (x, y) = clamp_to_circle(30.0, 0.0, 60.0);
        assert_eq!(x, 30.0);
        assert_eq!(y, 0.0);
    }

    #[test]
    fn clamp_outside_circle() {
        let (x, y) = clamp_to_circle(120.0, 0.0, 60.0);
        assert!((x - 60.0).abs() < 0.001);
        assert!(y.abs() < 0.001);
    }

    #[test]
    fn clamp_diagonal() {
        let r = 60.0_f32;
        let raw = r * 2.0_f32.sqrt(); // distance = 2r at 45°
        let (x, y) = clamp_to_circle(raw, raw, r);
        let len = (x * x + y * y).sqrt();
        assert!((len - r).abs() < 0.01);
    }
}
