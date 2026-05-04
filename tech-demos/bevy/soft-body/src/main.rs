//! # Soft-Body Simulation
//!
//! A spring-mass network that simulates an elastic soft body without any
//! physics plugin.  25 nodes arranged in a 5×5 grid are each an ECS entity
//! with a position (via `Transform`) and a velocity (`NodeVelocity`).
//! Springs are stored in a `Springs` resource rather than as entities.
//!
//! Each `FixedUpdate` tick:
//! 1. Compute spring forces for every spring (immutable read).
//! 2. Accumulate force deltas per node in a `HashMap` (no aliasing).
//! 3. Apply accumulated forces and integrate: `pos += vel * dt`.
//! 4. Apply damping and gravity to all non-pinned nodes.
//!
//! ## Controls
//! - **Left-click drag** near a node to grab and pull it.
//! - Release to drop.
//!
//! The top-left and top-right corner nodes are pinned and never move.

use bevy::prelude::*;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const GRID_N: usize = 5;
const NODE_SPACING: f32 = 60.0;
const NODE_SIZE: f32 = 8.0;
const STIFFNESS: f32 = 400.0;
const DAMPING: f32 = 8.0;
const GRAVITY: f32 = 200.0;
/// How close the cursor must be to a node to grab it.
const GRAB_RADIUS: f32 = 20.0;

// ---------------------------------------------------------------------------
// Pure physics functions
// ---------------------------------------------------------------------------

/// Computes the spring force vector acting on `node_a` given the positions
/// of both endpoints, the spring's rest length, and its stiffness constant.
///
/// Positive (attractive) when the nodes are farther apart than `rest_length`;
/// negative (repulsive) when they are too close.
pub fn spring_force(pos_a: Vec2, pos_b: Vec2, rest_length: f32, stiffness: f32) -> Vec2 {
    let delta = pos_b - pos_a;
    let dist = delta.length();
    if dist < 1e-6 {
        return Vec2::ZERO;
    }
    let extension = dist - rest_length;
    let magnitude = stiffness * extension;
    delta / dist * magnitude
}

/// Applies Euler integration: returns the new position given current position,
/// velocity, and time-step `dt`.
pub fn integrate(pos: Vec2, vel: Vec2, dt: f32) -> Vec2 {
    pos + vel * dt
}

/// Applies velocity damping over `dt` seconds.
///
/// Returns the new velocity after the damping factor is applied.
pub fn damp_velocity(vel: Vec2, damping: f32, dt: f32) -> Vec2 {
    vel * (1.0 - damping * dt).max(0.0)
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Linear velocity of a soft-body node (world units/second).
#[derive(Component, Clone, Copy)]
pub struct NodeVelocity(pub Vec2);

/// Marker component — this node is pinned and never moves.
#[derive(Component, Clone, Copy)]
pub struct Pinned;

/// Index into the node grid `(col, row)` — used for layout only.
#[derive(Component, Clone, Copy)]
struct NodeIndex(#[allow(dead_code)] usize, #[allow(dead_code)] usize);

/// Marks the grabbed node entity during a mouse-drag.
#[derive(Component)]
struct Grabbed;

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// A single spring connecting two node entities.
#[derive(Clone)]
pub struct SpringDef {
    pub node_a: Entity,
    pub node_b: Entity,
    pub rest_length: f32,
}

/// Collection of all springs in the soft body.
#[derive(Resource)]
pub struct Springs(pub Vec<SpringDef>);

/// Tracks which entity (if any) the user is currently dragging.
#[derive(Resource, Default)]
struct GrabState {
    entity: Option<Entity>,
}

// ---------------------------------------------------------------------------
// Bevy App
// ---------------------------------------------------------------------------

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Soft-Body Simulation".into(),
                resolution: (800u32, 600u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(Springs(Vec::new()))
        .insert_resource(GrabState::default())
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, physics_step)
        .add_systems(Update, (handle_grab_input, apply_grab_drag))
        .run();
}

// ---------------------------------------------------------------------------
// Startup
// ---------------------------------------------------------------------------

fn setup(mut commands: Commands, mut springs: ResMut<Springs>) {
    commands.spawn(Camera2d);

    // Compute grid top-left so the whole net is centered
    let half = (GRID_N as f32 - 1.0) * NODE_SPACING * 0.5;
    let origin = Vec2::new(-half, half); // top-left node world pos

    // Spawn nodes
    let mut entities = [[Entity::PLACEHOLDER; GRID_N]; GRID_N];
    for row in 0..GRID_N {
        for col in 0..GRID_N {
            let pos = Vec2::new(
                origin.x + col as f32 * NODE_SPACING,
                origin.y - row as f32 * NODE_SPACING,
            );
            let pinned = (col == 0 && row == 0) || (col == GRID_N - 1 && row == 0);
            let color = if pinned {
                Color::srgb(0.9, 0.3, 0.3)
            } else {
                Color::srgb(0.9, 0.9, 0.9)
            };

            let mut ec = commands.spawn((
                NodeIndex(col, row),
                NodeVelocity(Vec2::ZERO),
                Sprite {
                    color,
                    custom_size: Some(Vec2::splat(NODE_SIZE)),
                    ..default()
                },
                Transform::from_translation(pos.extend(0.0)),
            ));
            if pinned {
                ec.insert(Pinned);
            }
            entities[row][col] = ec.id();
        }
    }

    // Build springs: horizontal, vertical, diagonal connections
    let mut defs: Vec<SpringDef> = Vec::new();
    for row in 0..GRID_N {
        for col in 0..GRID_N {
            let a = entities[row][col];
            // Horizontal neighbor
            if col + 1 < GRID_N {
                let b = entities[row][col + 1];
                defs.push(SpringDef { node_a: a, node_b: b, rest_length: NODE_SPACING });
            }
            // Vertical neighbor
            if row + 1 < GRID_N {
                let b = entities[row + 1][col];
                defs.push(SpringDef { node_a: a, node_b: b, rest_length: NODE_SPACING });
            }
            // Diagonal ↘
            if col + 1 < GRID_N && row + 1 < GRID_N {
                let b = entities[row + 1][col + 1];
                defs.push(SpringDef {
                    node_a: a,
                    node_b: b,
                    rest_length: NODE_SPACING * std::f32::consts::SQRT_2,
                });
            }
            // Diagonal ↙
            if col > 0 && row + 1 < GRID_N {
                let b = entities[row + 1][col - 1];
                defs.push(SpringDef {
                    node_a: a,
                    node_b: b,
                    rest_length: NODE_SPACING * std::f32::consts::SQRT_2,
                });
            }
        }
    }
    springs.0 = defs;

    // HUD
    commands.spawn((
        Text::new("Left-click drag: grab node"),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(6.0),
            left: Val::Px(8.0),
            ..default()
        },
    ));
}

// ---------------------------------------------------------------------------
// FixedUpdate physics
// ---------------------------------------------------------------------------

fn physics_step(
    springs: Res<Springs>,
    time: Res<Time>,
    mut nodes: Query<(Entity, &mut Transform, &mut NodeVelocity, Option<&Pinned>)>,
) {
    let dt = time.delta_secs();

    // Phase 1 — collect spring force deltas (read-only borrows, dropped each iteration)
    let mut force_map: HashMap<Entity, Vec2> = HashMap::new();
    for spring in &springs.0 {
        let (pos_a, pos_b) = {
            let Ok((_, t_a, _, _)) = nodes.get(spring.node_a) else { continue };
            let Ok((_, t_b, _, _)) = nodes.get(spring.node_b) else { continue };
            (t_a.translation.truncate(), t_b.translation.truncate())
        };
        let force = spring_force(pos_a, pos_b, spring.rest_length, STIFFNESS);
        *force_map.entry(spring.node_a).or_default() += force;
        *force_map.entry(spring.node_b).or_default() -= force;
    }

    // Phase 2 — apply forces to velocities
    for (entity, _, mut vel, _) in &mut nodes {
        if let Some(delta) = force_map.get(&entity) {
            vel.0 += *delta * dt;
        }
    }

    // Phase 3 — integrate positions, apply damping + gravity (skip pinned)
    for (_, mut transform, mut vel, pinned) in &mut nodes {
        if pinned.is_some() {
            continue;
        }
        vel.0 = damp_velocity(vel.0, DAMPING, dt);
        vel.0.y -= GRAVITY * dt;
        transform.translation =
            integrate(transform.translation.truncate(), vel.0, dt).extend(transform.translation.z);
    }
}

// ---------------------------------------------------------------------------
// Mouse grab
// ---------------------------------------------------------------------------

fn handle_grab_input(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut grab: ResMut<GrabState>,
    nodes: Query<(Entity, &Transform), (With<NodeVelocity>, Without<Pinned>)>,
) {
    let Ok(window) = windows.single() else { return };
    let Ok((camera, cam_tf)) = camera_q.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let Ok(world) = camera.viewport_to_world_2d(cam_tf, cursor) else { return };

    if buttons.just_pressed(MouseButton::Left) {
        // Find closest node within grab radius
        let mut best: Option<(Entity, f32)> = None;
        for (entity, tf) in &nodes {
            let d = tf.translation.truncate().distance(world);
            if d < GRAB_RADIUS {
                if best.map_or(true, |(_, bd)| d < bd) {
                    best = Some((entity, d));
                }
            }
        }
        if let Some((entity, _)) = best {
            grab.entity = Some(entity);
            commands.entity(entity).insert(Grabbed);
        }
    }

    if buttons.just_released(MouseButton::Left) {
        if let Some(entity) = grab.entity.take() {
            commands.entity(entity).remove::<Grabbed>();
        }
    }
}

fn apply_grab_drag(
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    grab: Res<GrabState>,
    mut nodes: Query<(&mut Transform, &mut NodeVelocity), With<Grabbed>>,
) {
    let Some(_grabbed_entity) = grab.entity else { return };
    let Ok(window) = windows.single() else { return };
    let Ok((camera, cam_tf)) = camera_q.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let Ok(world) = camera.viewport_to_world_2d(cam_tf, cursor) else { return };

    for (mut tf, mut vel) in &mut nodes {
        tf.translation = world.extend(tf.translation.z);
        vel.0 = Vec2::ZERO;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spring_force_at_rest_is_zero() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(100.0, 0.0);
        let force = spring_force(a, b, 100.0, 400.0);
        assert!(force.length() < 1e-4, "spring at rest should produce near-zero force");
    }

    #[test]
    fn spring_force_when_stretched() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(150.0, 0.0); // 50 units beyond rest
        let force = spring_force(a, b, 100.0, 400.0);
        // Force on A points toward B (positive x)
        assert!(force.x > 0.0, "stretched spring should pull a toward b");
        assert!((force.x - 400.0 * 50.0).abs() < 1e-3);
    }

    #[test]
    fn spring_force_when_compressed() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(50.0, 0.0); // 50 units inside rest length
        let force = spring_force(a, b, 100.0, 400.0);
        // Force on A points away from B (negative x) — spring pushes a back
        assert!(force.x < 0.0, "compressed spring should push a away from b");
    }

    #[test]
    fn spring_force_coincident_nodes_is_zero() {
        let force = spring_force(Vec2::ZERO, Vec2::ZERO, 100.0, 400.0);
        assert_eq!(force, Vec2::ZERO);
    }

    #[test]
    fn integrate_basic_case() {
        let pos = Vec2::new(0.0, 0.0);
        let vel = Vec2::new(10.0, 5.0);
        let result = integrate(pos, vel, 0.1);
        assert!((result.x - 1.0).abs() < 1e-5);
        assert!((result.y - 0.5).abs() < 1e-5);
    }

    #[test]
    fn integrate_zero_dt_returns_same_position() {
        let pos = Vec2::new(3.0, 7.0);
        let vel = Vec2::new(100.0, 200.0);
        let result = integrate(pos, vel, 0.0);
        assert_eq!(result, pos);
    }

    #[test]
    fn damping_reduces_magnitude() {
        let vel = Vec2::new(10.0, 0.0);
        let damped = damp_velocity(vel, 8.0, 0.016);
        assert!(damped.length() < vel.length(), "damped velocity must be smaller");
        assert!(damped.x > 0.0, "damped velocity should still point in same direction");
    }

    #[test]
    fn damping_zero_dt_returns_unchanged() {
        let vel = Vec2::new(5.0, 3.0);
        let result = damp_velocity(vel, 8.0, 0.0);
        assert!((result - vel).length() < 1e-5);
    }

    #[test]
    fn damping_large_dt_clamps_to_zero() {
        // With dt so large that (1 - damping * dt) < 0 it should clamp to 0, not go negative
        let vel = Vec2::new(10.0, 0.0);
        let result = damp_velocity(vel, 8.0, 10.0); // factor would be -79 without clamp
        assert!(result.length() >= 0.0);
        assert!(result.x >= 0.0, "damped velocity should not reverse direction");
    }
}
