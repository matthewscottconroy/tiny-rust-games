//! Particles 2D demo — `CPUParticles2D` emission control driven from Rust.
//!
//! Teaches: finding a `CpuParticles2D` child with `get_node_as`; configuring
//! emission amount, lifetime, direction, and spread from Rust; toggling
//! emission on/off; and scheduling a one-shot burst via a `Timer` child.
//!
//! The scene requires two children of the root `ParticleController` node:
//! - `"CPUParticles2D"` — a `CPUParticles2D` node
//! - `"StatusLabel"` — a `Label` for displaying the current emission state

use godot::classes::cpu_particles_2d::Parameter as CpuParam;
use godot::classes::{CpuParticles2D, INode2D, Label, Node, Node2D, Timer};
use godot::prelude::*;

// ---------------------------------------------------------------------------
// Extension entry-point
// ---------------------------------------------------------------------------

struct Particles2DExtension;
#[gdextension]
unsafe impl ExtensionLibrary for Particles2DExtension {}

// ---------------------------------------------------------------------------
// ParticleController node
// ---------------------------------------------------------------------------

/// Scene root that configures and controls a child `CPUParticles2D` from Rust.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct ParticleController {
    /// Emission rate (particles per second).  Exported so the Godot inspector
    /// can tweak it before the scene starts.
    #[export]
    emission_rate: f32,

    /// How long (in seconds) each particle lives.
    #[export]
    particle_lifetime: f32,

    /// If true, emission starts automatically in `ready()`.
    #[export]
    emit_on_start: bool,

    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for ParticleController {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            emission_rate: 20.0,
            particle_lifetime: 2.0,
            emit_on_start: true,
            base,
        }
    }

    fn ready(&mut self) {
        let lifetime = self.particle_lifetime as f64;
        let emit = self.emit_on_start;

        let mut particles = self
            .base()
            .get_node_as::<CpuParticles2D>("CPUParticles2D");

        particles.set_amount(100);
        particles.set_lifetime(lifetime);
        // Direction: straight up.
        particles.set_direction(Vector2::new(0.0, -1.0));
        // Spread in degrees around the direction vector.
        particles.set_spread(45.0);
        // Initial linear velocity range uses the generic param API.
        particles.set_param_min(CpuParam::INITIAL_LINEAR_VELOCITY, 50.0);
        particles.set_param_max(CpuParam::INITIAL_LINEAR_VELOCITY, 150.0);

        if emit {
            particles.set_emitting(true);
        }

        self.refresh_label();
        godot_print!("[ParticleController] ready — emitting={}", emit);
    }
}

#[godot_api]
impl ParticleController {
    /// Starts particle emission.
    #[func]
    pub fn start_emission(&mut self) {
        let mut particles = self
            .base()
            .get_node_as::<CpuParticles2D>("CPUParticles2D");
        particles.set_emitting(true);
        self.refresh_label();
        godot_print!("[ParticleController] emission started");
    }

    /// Stops particle emission.
    #[func]
    pub fn stop_emission(&mut self) {
        let mut particles = self
            .base()
            .get_node_as::<CpuParticles2D>("CPUParticles2D");
        particles.set_emitting(false);
        self.refresh_label();
        godot_print!("[ParticleController] emission stopped");
    }

    /// Enables emission for 0.5 seconds then automatically stops it.
    #[func]
    pub fn burst(&mut self) {
        {
            let mut particles = self
                .base()
                .get_node_as::<CpuParticles2D>("CPUParticles2D");
            particles.set_emitting(true);
        }

        // Create a one-shot timer to stop emission after the burst window.
        let mut timer = Timer::new_alloc();
        timer.set_wait_time(0.5);
        timer.set_one_shot(true);
        let callable = self.base().callable("stop_emission");
        timer.connect("timeout", &callable);

        let timer_node = timer.upcast::<Node>();
        self.base_mut().add_child(&timer_node);

        // Start the timer immediately after it enters the tree.
        let mut timer_ref = self.base().get_node_as::<Timer>("Timer");
        timer_ref.start();

        self.refresh_label();
        godot_print!("[ParticleController] burst started — stopping in 0.5 s");
    }

    /// Changes the particle colour to the given RGB values.
    #[func]
    pub fn set_color(&mut self, r: f32, g: f32, b: f32) {
        let mut particles = self
            .base()
            .get_node_as::<CpuParticles2D>("CPUParticles2D");
        particles.set_color(Color::from_rgb(r, g, b));
        godot_print!("[ParticleController] color set to ({r}, {g}, {b})");
    }

    // Internal: update the StatusLabel with the current emission state.
    fn refresh_label(&mut self) {
        if let Some(mut label) = self.base().try_get_node_as::<Label>("StatusLabel") {
            let emitting = self
                .base()
                .try_get_node_as::<CpuParticles2D>("CPUParticles2D")
                .map(|p| p.is_emitting())
                .unwrap_or(false);
            let rate = self.emission_rate;
            let text = format_emission_state(emitting, rate);
            label.set_text(text.as_str());
        }
    }
}

// ---------------------------------------------------------------------------
// Pure helper functions
// ---------------------------------------------------------------------------

/// Average number of particles alive at any given moment = rate × lifetime.
///
/// # Examples
/// ```
/// assert!((particles_2d::particle_density(20.0, 2.0) - 40.0).abs() < 1e-5);
/// assert!((particles_2d::particle_density(10.0, 0.5) - 5.0).abs() < 1e-5);
/// ```
pub fn particle_density(rate: f32, lifetime: f32) -> f32 {
    rate * lifetime
}

/// Formats a human-readable emission state string for the HUD.
///
/// # Examples
/// ```
/// let s = particles_2d::format_emission_state(true, 20.0);
/// assert!(s.contains("Emitting"));
/// let s2 = particles_2d::format_emission_state(false, 20.0);
/// assert!(s2.contains("Stopped"));
/// ```
pub fn format_emission_state(emitting: bool, rate: f32) -> String {
    if emitting {
        format!("Emitting — {:.0} particles/s", rate)
    } else {
        format!("Stopped — rate: {:.0} particles/s", rate)
    }
}

/// Estimates how long (in seconds) a burst of `count` particles takes at
/// `rate` particles per second.
///
/// # Examples
/// ```
/// assert!((particles_2d::burst_duration(20.0, 100) - 5.0).abs() < 1e-5);
/// ```
pub fn burst_duration(rate: f32, count: i32) -> f32 {
    if rate <= 0.0 {
        return 0.0;
    }
    count as f32 / rate
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // particle_density --------------------------------------------------------

    #[test]
    fn particle_density_normal() {
        assert!((particle_density(20.0, 2.0) - 40.0).abs() < 1e-5);
    }

    #[test]
    fn particle_density_zero_lifetime() {
        assert!((particle_density(20.0, 0.0) - 0.0).abs() < 1e-5);
    }

    #[test]
    fn particle_density_zero_rate() {
        assert!((particle_density(0.0, 5.0) - 0.0).abs() < 1e-5);
    }

    // format_emission_state ---------------------------------------------------

    #[test]
    fn format_emission_state_emitting() {
        let s = format_emission_state(true, 20.0);
        assert!(s.contains("Emitting"));
        assert!(s.contains("20"));
    }

    #[test]
    fn format_emission_state_stopped() {
        let s = format_emission_state(false, 10.0);
        assert!(s.contains("Stopped"));
        assert!(s.contains("10"));
    }

    // burst_duration ----------------------------------------------------------

    #[test]
    fn burst_duration_normal() {
        assert!((burst_duration(20.0, 100) - 5.0).abs() < 1e-5);
    }

    #[test]
    fn burst_duration_zero_rate_returns_zero() {
        assert!((burst_duration(0.0, 50) - 0.0).abs() < 1e-5);
    }

    #[test]
    fn burst_duration_single_particle() {
        assert!((burst_duration(10.0, 1) - 0.1).abs() < 1e-5);
    }

    #[test]
    fn burst_duration_large_count() {
        assert!((burst_duration(100.0, 1000) - 10.0).abs() < 1e-4);
    }
}
