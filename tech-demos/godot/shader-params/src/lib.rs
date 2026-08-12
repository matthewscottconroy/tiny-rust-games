//! Shader Params demo — drives a ShaderMaterial's uniform parameters from Rust,
//! animating a time value each frame.

use godot::classes::{INode2D, MeshInstance2D, Node2D, ShaderMaterial};
use godot::prelude::*;

struct ShaderParamsExtension;
#[gdextension]
unsafe impl ExtensionLibrary for ShaderParamsExtension {}

#[derive(GodotClass)]
#[class(base=Node2D)]
struct ShaderParamsDemo {
    #[export]
    speed: f32,

    elapsed: f32,
    shader_material: Option<Gd<ShaderMaterial>>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for ShaderParamsDemo {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            speed: 1.0,
            elapsed: 0.0,
            shader_material: None,
            base,
        }
    }

    fn ready(&mut self) {
        let mesh = self.base().get_node_as::<MeshInstance2D>("MeshInstance2D");
        if let Some(mat) = mesh.get_material()
            && let Ok(shader_mat) = mat.try_cast::<ShaderMaterial>()
        {
            self.shader_material = Some(shader_mat);
        }
    }

    fn process(&mut self, delta: f64) {
        let speed = self.speed;
        self.elapsed += delta as f32 * speed;

        let elapsed = self.elapsed;
        if let Some(ref mut mat) = self.shader_material {
            mat.set_shader_parameter("time", &Variant::from(elapsed));
        }
    }
}

#[godot_api]
impl ShaderParamsDemo {
    #[func]
    pub fn set_color_param(&mut self, r: f32, g: f32, b: f32) {
        if let Some(ref mut mat) = self.shader_material {
            let color = Color::from_rgb(r, g, b);
            mat.set_shader_parameter("color", &Variant::from(color));
        }
    }
}

// ---------------------------------------------------------------------------
// Pure helper functions
// ---------------------------------------------------------------------------

/// Compute a sine wave value for the given time, frequency, and amplitude.
pub fn wave_value(t: f32, frequency: f32, amplitude: f32) -> f32 {
    (t * frequency * std::f32::consts::TAU).sin() * amplitude
}

/// Convert accumulated elapsed time and speed into a phase [0, 1).
pub fn elapsed_to_phase(elapsed: f32, speed: f32) -> f32 {
    (elapsed * speed).fract()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wave_value_at_zero_is_zero() {
        assert!((wave_value(0.0, 1.0, 1.0)).abs() < 1e-6);
    }

    #[test]
    fn wave_value_amplitude_scales_output() {
        let v = wave_value(0.25, 1.0, 2.0);
        assert!(
            (v - 2.0).abs() < 1e-5,
            "quarter period should give amplitude: {v}"
        );
    }

    #[test]
    fn wave_value_frequency_zero_stays_zero() {
        assert!((wave_value(0.5, 0.0, 5.0)).abs() < 1e-6);
    }

    #[test]
    fn elapsed_to_phase_zero_elapsed() {
        assert!((elapsed_to_phase(0.0, 2.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn elapsed_to_phase_wraps_at_one() {
        let phase = elapsed_to_phase(1.0, 1.0);
        assert!(
            phase < 1e-6,
            "phase should wrap to ~0 at elapsed=1, speed=1: {phase}"
        );
    }

    #[test]
    fn elapsed_to_phase_half_way() {
        let phase = elapsed_to_phase(0.5, 1.0);
        assert!((phase - 0.5).abs() < 1e-6);
    }

    #[test]
    fn wave_value_is_bounded_by_amplitude() {
        for i in 0..100 {
            let t = i as f32 * 0.01;
            let v = wave_value(t, 3.0, 1.5);
            assert!(v.abs() <= 1.5 + 1e-5, "wave out of bounds: {v}");
        }
    }
}
