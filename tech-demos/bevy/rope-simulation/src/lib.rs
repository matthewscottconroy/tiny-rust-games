//! Rope simulation — Verlet integration with iterative distance constraints.
//!
//! This crate is a *building block*: drop [`RopeSimulationPlugin`] into any
//! Bevy app and it simulates a mouse-pinned rope of point masses.
//!
//! Key ideas:
//! - Each node stores its current and previous position; velocity is implicit
//!   in the difference `pos − prev` (Störmer–Verlet scheme).
//! - [`verlet_step`] integrates one timestep: `new = 2·pos − prev + accel·dt²`.
//! - [`constrain_segment`] enforces the rest-length between adjacent nodes
//!   using equal-mass Jacobi relaxation; multiple passes per frame produce a
//!   stiffer rope.
//! - Node 0 is pinned to the mouse cursor each frame before the constraint
//!   solve, so the rope hangs from wherever the cursor sits.
//! - All rope state lives in a single [`Rope`] resource; transforms are synced
//!   from it after simulation.
//! - Tunables (node count, rest length, gravity, iterations…) live in
//!   [`RopeConfig`]; override it before adding the plugin.
//!
//! **Controls:** move the mouse — the rope hangs from the cursor position.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use rope_simulation::RopeSimulationPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(RopeSimulationPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

// ── Plugin ─────────────────────────────────────────────────────────────────────

/// Bundles every system and resource for the rope simulation.
///
/// Add it with `app.add_plugins(RopeSimulationPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app keeps control of window and rendering setup.
pub struct RopeSimulationPlugin;

impl Plugin for RopeSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RopeConfig>()
            .add_systems(Startup, setup)
            .add_systems(Update, (pin_to_mouse, simulate_rope, sync_transforms).chain());
    }
}

// ── Configuration ──────────────────────────────────────────────────────────────

/// Tunable parameters for the rope. Override before adding the plugin, e.g.
/// `app.insert_resource(RopeConfig { node_count: 40, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct RopeConfig {
    /// Number of beads in the chain (including the pinned anchor).
    pub node_count: usize,
    /// Rest length between adjacent nodes, in world units.
    pub rest_len: f32,
    /// Jacobi relaxation passes per frame — higher is stiffer.
    pub constraint_iters: usize,
    /// Constant acceleration applied to every non-pinned node.
    pub gravity: Vec2,
    /// Radius of each bead sprite, in world units.
    pub node_radius: f32,
}

impl Default for RopeConfig {
    fn default() -> Self {
        Self {
            node_count: 22,
            rest_len: 16.0,
            constraint_iters: 10,
            gravity: Vec2::new(0.0, -500.0),
            node_radius: 5.0,
        }
    }
}

// ── Pure physics helpers ──────────────────────────────────────────────────────

/// Integrates one Verlet timestep for a single point mass under constant acceleration.
///
/// `new_pos = 2·pos − prev + accel·dt²`
pub fn verlet_step(pos: Vec2, prev: Vec2, accel: Vec2, dt: f32) -> Vec2 {
    2.0 * pos - prev + accel * dt * dt
}

/// Solves the distance constraint between two equal-mass Verlet nodes.
///
/// Returns the corrected positions `(a', b')` such that `|a' − b'| == rest_len`.
/// Each node is displaced by half the required correction.
pub fn constrain_segment(a: Vec2, b: Vec2, rest_len: f32) -> (Vec2, Vec2) {
    let delta = b - a;
    let dist = delta.length();
    if dist < 1e-6 {
        return (a, b);
    }
    let correction = delta * ((dist - rest_len) / dist * 0.5);
    (a + correction, b - correction)
}

/// Returns a unit vector from `a` toward `b`.
///
/// Falls back to `Vec2::X` for degenerate (zero-length) input.
pub fn segment_dir(a: Vec2, b: Vec2) -> Vec2 {
    let d = b - a;
    if d.length_squared() < 1e-12 {
        Vec2::X
    } else {
        d.normalize()
    }
}

// ── ECS data ─────────────────────────────────────────────────────────────────

/// All Verlet rope state in one resource — keeps systems simple and avoids
/// per-node archetype overhead for a short chain.
#[derive(Resource)]
pub struct Rope {
    /// Current position of each node.
    pub positions: Vec<Vec2>,
    /// Previous-frame position of each node (implicit velocity).
    pub prev_positions: Vec<Vec2>,
    /// Sprite entity backing each node.
    pub entities: Vec<Entity>,
    /// Target rest length between adjacent nodes.
    pub rest_len: f32,
}

// ── Systems ─────────────────────────────────────────────────────────────────────

/// Spawns a vertical chain of bead sprites and initialises the [`Rope`] resource.
fn setup(mut commands: Commands, config: Res<RopeConfig>) {
    commands.spawn(Camera2d);

    let mut positions = Vec::with_capacity(config.node_count);
    let mut prev_positions = Vec::with_capacity(config.node_count);
    let mut entities = Vec::with_capacity(config.node_count);

    for i in 0..config.node_count {
        let pos = Vec2::new(0.0, -(i as f32) * config.rest_len);
        let color = if i == 0 {
            Color::srgb(1.0, 0.55, 0.1) // anchor bead — orange
        } else {
            Color::srgb(0.5, 0.75, 1.0) // chain beads — blue
        };
        let entity = commands
            .spawn((
                Sprite {
                    color,
                    custom_size: Some(Vec2::splat(config.node_radius * 2.0)),
                    ..default()
                },
                Transform::from_translation(pos.extend(0.0)),
            ))
            .id();
        positions.push(pos);
        prev_positions.push(pos);
        entities.push(entity);
    }

    commands.spawn((
        Text::new("Move the mouse — the rope hangs from the cursor"),
        TextFont { font_size: 15.0, ..default() },
        TextColor(Color::srgb(0.7, 0.7, 0.7)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));

    commands.insert_resource(Rope {
        positions,
        prev_positions,
        entities,
        rest_len: config.rest_len,
    });
}

/// Moves the anchor node (index 0) to the world-space cursor each frame.
fn pin_to_mouse(
    window_query: Query<&Window>,
    cam_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut rope: ResMut<Rope>,
) {
    let Ok(window) = window_query.single() else { return };
    let Ok((cam, cam_tf)) = cam_query.single() else { return };
    if let Some(cursor) = window
        .cursor_position()
        .and_then(|c| cam.viewport_to_world_2d(cam_tf, c).ok())
    {
        rope.prev_positions[0] = rope.positions[0];
        rope.positions[0] = cursor;
    }
}

/// Advances the Verlet simulation and resolves distance constraints.
fn simulate_rope(time: Res<Time>, config: Res<RopeConfig>, mut rope: ResMut<Rope>) {
    let dt = time.delta_secs().min(0.05); // cap dt to prevent instability
    let n = rope.positions.len();

    // Verlet integrate all non-pinned nodes.
    for i in 1..n {
        let new_pos = verlet_step(rope.positions[i], rope.prev_positions[i], config.gravity, dt);
        rope.prev_positions[i] = rope.positions[i];
        rope.positions[i] = new_pos;
    }

    // Jacobi relaxation — resolve distance constraints between adjacent nodes.
    for _ in 0..config.constraint_iters {
        for i in 0..n - 1 {
            if i == 0 {
                // Anchor is pinned; only displace node 1 to enforce rest length.
                let dir = segment_dir(rope.positions[0], rope.positions[1]);
                rope.positions[1] = rope.positions[0] + dir * rope.rest_len;
            } else {
                let (a, b) = constrain_segment(
                    rope.positions[i],
                    rope.positions[i + 1],
                    rope.rest_len,
                );
                rope.positions[i] = a;
                rope.positions[i + 1] = b;
            }
        }
    }
}

/// Writes simulated positions back to entity [`Transform`]s.
fn sync_transforms(rope: Res<Rope>, mut transforms: Query<&mut Transform>) {
    for (i, &entity) in rope.entities.iter().enumerate() {
        if let Ok(mut tf) = transforms.get_mut(entity) {
            tf.translation = rope.positions[i].extend(0.0);
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verlet_step_constant_velocity_no_accel() {
        // Moving right at 10 px/frame: pos=10, prev=0 → new=20
        let new = verlet_step(Vec2::new(10.0, 0.0), Vec2::ZERO, Vec2::ZERO, 1.0);
        assert!((new.x - 20.0).abs() < 1e-4);
        assert!(new.y.abs() < 1e-4);
    }

    #[test]
    fn verlet_step_gravity_pulls_down() {
        let new = verlet_step(Vec2::ZERO, Vec2::ZERO, Vec2::new(0.0, -500.0), 0.016);
        assert!(new.y < 0.0, "gravity should move node downward");
    }

    #[test]
    fn verlet_step_stationary_with_no_accel() {
        let new = verlet_step(Vec2::ONE, Vec2::ONE, Vec2::ZERO, 1.0);
        assert!((new - Vec2::ONE).length() < 1e-5);
    }

    #[test]
    fn constrain_segment_enforces_rest_length() {
        let a = Vec2::ZERO;
        let b = Vec2::new(20.0, 0.0); // 20 px apart, rest = 10
        let (a2, b2) = constrain_segment(a, b, 10.0);
        assert!((a2.distance(b2) - 10.0).abs() < 1e-4);
    }

    #[test]
    fn constrain_segment_preserves_midpoint() {
        let a = Vec2::ZERO;
        let b = Vec2::new(20.0, 0.0);
        let (a2, b2) = constrain_segment(a, b, 10.0);
        let mid_before = (a + b) * 0.5;
        let mid_after = (a2 + b2) * 0.5;
        assert!((mid_before - mid_after).length() < 1e-4);
    }

    #[test]
    fn constrain_segment_already_at_rest_is_noop() {
        let a = Vec2::ZERO;
        let b = Vec2::new(10.0, 0.0);
        let (a2, b2) = constrain_segment(a, b, 10.0);
        assert!((a2 - a).length() < 1e-4);
        assert!((b2 - b).length() < 1e-4);
    }

    #[test]
    fn segment_dir_normalises() {
        let d = segment_dir(Vec2::ZERO, Vec2::new(0.0, 7.0));
        assert!((d.length() - 1.0).abs() < 1e-5);
        assert!((d.y - 1.0).abs() < 1e-5);
    }

    #[test]
    fn segment_dir_degenerate_input_returns_x() {
        assert_eq!(segment_dir(Vec2::ZERO, Vec2::ZERO), Vec2::X);
    }

    #[test]
    fn config_default_matches_documented_values() {
        let c = RopeConfig::default();
        assert!(c.node_count > 1);
        assert!(c.rest_len > 0.0);
        assert!(c.constraint_iters > 0);
        assert_eq!(c.gravity, Vec2::new(0.0, -500.0));
        assert_eq!(c.node_radius, 5.0);
    }
}
