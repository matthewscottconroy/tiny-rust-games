//! Stealth AI demo — FOV cone detection with Patrol / Alert / Chase states.
//!
//! Key ideas:
//! - `in_fov_cone` is a pure function that checks whether a target point falls
//!   inside a directional cone defined by an origin, a normalised forward
//!   vector, a half-angle, and a maximum range.
//! - The guard cycles through three states:
//!     - **Patrol** — walks between two waypoints.
//!     - **Alert**  — stops and looks toward where the player was spotted;
//!                    transitions to Chase if the player stays in view, or
//!                    back to Patrol after a timeout.
//!     - **Chase**  — runs directly toward the last known player position.
//! - A coloured rectangle behind the guard visualises the detection state.
//! - The player moves freely; the guard's FOV cone is drawn as a fan of thin
//!   sprites approximating an arc.
//!
//! **Controls:** WASD / Arrow keys — move player.

use bevy::prelude::*;
use bevy::window::WindowResolution;

const GUARD_SPEED: f32 = 90.0;
const CHASE_SPEED: f32 = 140.0;
const FOV_HALF_ANGLE: f32 = std::f32::consts::FRAC_PI_4; // 45 °
const FOV_RANGE: f32 = 200.0;
const ALERT_TIMEOUT: f32 = 2.5;
const PLAYER_SPEED: f32 = 160.0;

// ─── Pure helpers ────────────────────────────────────────────────────────────

/// Returns `true` when `target` lies inside the FOV cone.
///
/// The cone is defined by:
/// - `origin`      — tip of the cone (guard position)
/// - `forward`     — unit vector pointing the guard's facing direction
/// - `half_angle`  — half-aperture of the cone in radians
/// - `range`       — maximum detection distance
pub fn in_fov_cone(target: Vec2, origin: Vec2, forward: Vec2, half_angle: f32, range: f32) -> bool {
    let to = target - origin;
    let dist = to.length();
    if dist > range || dist < 1e-6 {
        return dist < 1e-6; // at origin, always detected
    }
    let dir = to / dist;
    let cos_half = half_angle.cos();
    dir.dot(forward.normalize()) >= cos_half
}

// ─── Guard state ─────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
enum GuardState {
    Patrol { waypoint_idx: usize },
    Alert { timer: f32 },
    Chase { last_known: Vec2 },
}

// ─── Waypoints ───────────────────────────────────────────────────────────────

const WAYPOINTS: &[Vec2] = &[
    Vec2::new(-280.0, 60.0),
    Vec2::new(280.0, 60.0),
    Vec2::new(280.0, -100.0),
    Vec2::new(-280.0, -100.0),
];

// ─── Components & resources ──────────────────────────────────────────────────

/// Guard entity with its AI state and facing direction.
#[derive(Component)]
struct Guard {
    state: GuardState,
    facing: Vec2,
}

#[derive(Component)]
struct PlayerMarker;

/// Background status tile that shows the guard's current state colour.
#[derive(Component)]
struct StatusTile;

/// One segment of the FOV fan visualisation.
#[derive(Component)]
struct FovSegment(usize);

// ─── Bevy app ────────────────────────────────────────────────────────────────

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Stealth AI — WASD to move, avoid the guard".to_string(),
                resolution: (800u32, 500u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (move_player, update_guard, draw_fov, update_status))
        .run();
}

const FOV_RAYS: usize = 24;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Player
    commands.spawn((
        Sprite { color: Color::srgb(0.3, 0.85, 1.0), custom_size: Some(Vec2::splat(20.0)), ..default() },
        Transform::from_translation(Vec3::new(-200.0, -200.0, 1.0)),
        PlayerMarker,
    ));

    // Guard
    commands.spawn((
        Sprite { color: Color::srgb(0.85, 0.55, 0.1), custom_size: Some(Vec2::splat(22.0)), ..default() },
        Transform::from_translation(WAYPOINTS[0].extend(1.0)),
        Guard { state: GuardState::Patrol { waypoint_idx: 0 }, facing: Vec2::X },
    ));

    // Status tile behind guard (z = 0.5)
    commands.spawn((
        Sprite { color: Color::srgb(0.1, 0.8, 0.1), custom_size: Some(Vec2::splat(36.0)), ..default() },
        Transform::from_translation(WAYPOINTS[0].extend(0.5)),
        StatusTile,
    ));

    // FOV fan segments (very thin lines)
    for i in 0..FOV_RAYS {
        commands.spawn((
            Sprite { color: Color::srgba(1.0, 1.0, 0.0, 0.18), custom_size: Some(Vec2::new(FOV_RANGE, 2.0)), ..default() },
            Transform::default(),
            FovSegment(i),
        ));
    }

    // Legend
    commands.spawn((
        Text::new("WASD / Arrow keys — move\nGreen = Patrol   Orange = Alert   Red = Chase"),
        TextFont { font_size: 16.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
    ));
}

fn move_player(
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut q: Query<&mut Transform, With<PlayerMarker>>,
) {
    let Ok(mut t) = q.single_mut() else { return };
    let mut dir = Vec2::ZERO;
    if input.pressed(KeyCode::KeyW) || input.pressed(KeyCode::ArrowUp)    { dir.y += 1.0; }
    if input.pressed(KeyCode::KeyS) || input.pressed(KeyCode::ArrowDown)   { dir.y -= 1.0; }
    if input.pressed(KeyCode::KeyA) || input.pressed(KeyCode::ArrowLeft)   { dir.x -= 1.0; }
    if input.pressed(KeyCode::KeyD) || input.pressed(KeyCode::ArrowRight)  { dir.x += 1.0; }
    if dir != Vec2::ZERO {
        t.translation += (dir.normalize() * PLAYER_SPEED * time.delta_secs()).extend(0.0);
    }
}

fn update_guard(
    time: Res<Time>,
    player_q: Query<&Transform, With<PlayerMarker>>,
    mut guard_q: Query<(&mut Guard, &mut Transform), Without<PlayerMarker>>,
    mut status_q: Query<&mut Transform, (With<StatusTile>, Without<Guard>, Without<PlayerMarker>)>,
) {
    let Ok(player_t) = player_q.single() else { return };
    let Ok((mut guard, mut guard_t)) = guard_q.single_mut() else { return };
    let dt = time.delta_secs();
    let player_pos = player_t.translation.truncate();
    let guard_pos = guard_t.translation.truncate();

    let sees_player = in_fov_cone(player_pos, guard_pos, guard.facing, FOV_HALF_ANGLE, FOV_RANGE);

    let new_state = match guard.state.clone() {
        GuardState::Patrol { waypoint_idx } => {
            if sees_player {
                GuardState::Alert { timer: ALERT_TIMEOUT }
            } else {
                let target = WAYPOINTS[waypoint_idx];
                let to = target - guard_pos;
                if to.length() < 4.0 {
                    let next = (waypoint_idx + 1) % WAYPOINTS.len();
                    GuardState::Patrol { waypoint_idx: next }
                } else {
                    let dir = to.normalize();
                    guard.facing = dir;
                    guard_t.translation += (dir * GUARD_SPEED * dt).extend(0.0);
                    GuardState::Patrol { waypoint_idx }
                }
            }
        }
        GuardState::Alert { timer } => {
            if sees_player {
                guard.facing = (player_pos - guard_pos).normalize_or_zero();
                GuardState::Chase { last_known: player_pos }
            } else {
                let t = timer - dt;
                if t <= 0.0 {
                    GuardState::Patrol { waypoint_idx: 0 }
                } else {
                    GuardState::Alert { timer: t }
                }
            }
        }
        GuardState::Chase { last_known } => {
            let target = if sees_player { player_pos } else { last_known };
            let to = target - guard_pos;
            if to.length() < 6.0 {
                GuardState::Alert { timer: ALERT_TIMEOUT }
            } else {
                let dir = to.normalize();
                guard.facing = dir;
                guard_t.translation += (dir * CHASE_SPEED * dt).extend(0.0);
                GuardState::Chase { last_known: target }
            }
        }
    };
    guard.state = new_state;

    // Sync status tile
    if let Ok(mut st) = status_q.single_mut() {
        st.translation = guard_t.translation - Vec3::Z * 0.5;
    }
}

/// Repositions FOV fan sprite segments to visualise the cone.
fn draw_fov(
    guard_q: Query<(&Guard, &Transform)>,
    mut segments: Query<(&FovSegment, &mut Transform), Without<Guard>>,
) {
    let Ok((guard, gt)) = guard_q.single() else { return };
    let origin = gt.translation.truncate();
    let base_angle = guard.facing.y.atan2(guard.facing.x);
    let step = (FOV_HALF_ANGLE * 2.0) / FOV_RAYS as f32;

    for (FovSegment(i), mut t) in &mut segments {
        let angle = base_angle - FOV_HALF_ANGLE + step * (*i as f32 + 0.5);
        let dir = Vec2::from_angle(angle);
        let mid = origin + dir * FOV_RANGE * 0.5;
        t.translation = mid.extend(0.8);
        t.rotation = Quat::from_rotation_z(angle);
    }
}

fn update_status(
    guard_q: Query<&Guard>,
    mut status_q: Query<&mut Sprite, With<StatusTile>>,
) {
    let Ok(guard) = guard_q.single() else { return };
    let Ok(mut sprite) = status_q.single_mut() else { return };
    sprite.color = match guard.state {
        GuardState::Patrol { .. } => Color::srgb(0.1, 0.8, 0.1),
        GuardState::Alert { .. }  => Color::srgb(0.9, 0.6, 0.0),
        GuardState::Chase { .. }  => Color::srgb(0.9, 0.1, 0.1),
    };
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_directly_ahead_is_detected() {
        let origin = Vec2::ZERO;
        let forward = Vec2::X;
        let target = Vec2::new(100.0, 0.0);
        assert!(in_fov_cone(target, origin, forward, FOV_HALF_ANGLE, FOV_RANGE));
    }

    #[test]
    fn target_behind_is_not_detected() {
        let origin = Vec2::ZERO;
        let forward = Vec2::X;
        let target = Vec2::new(-100.0, 0.0);
        assert!(!in_fov_cone(target, origin, forward, FOV_HALF_ANGLE, FOV_RANGE));
    }

    #[test]
    fn target_out_of_range_is_not_detected() {
        let origin = Vec2::ZERO;
        let forward = Vec2::X;
        let target = Vec2::new(FOV_RANGE + 1.0, 0.0);
        assert!(!in_fov_cone(target, origin, forward, FOV_HALF_ANGLE, FOV_RANGE));
    }

    #[test]
    fn target_at_exact_half_angle_edge_is_detected() {
        let origin = Vec2::ZERO;
        let forward = Vec2::X;
        // Slightly inside half-angle boundary.
        let angle = FOV_HALF_ANGLE - 0.01;
        let target = Vec2::new(angle.cos() * 50.0, angle.sin() * 50.0);
        assert!(in_fov_cone(target, origin, forward, FOV_HALF_ANGLE, FOV_RANGE));
    }

    #[test]
    fn target_beyond_half_angle_is_not_detected() {
        let origin = Vec2::ZERO;
        let forward = Vec2::X;
        let angle = FOV_HALF_ANGLE + 0.1;
        let target = Vec2::new(angle.cos() * 50.0, angle.sin() * 50.0);
        assert!(!in_fov_cone(target, origin, forward, FOV_HALF_ANGLE, FOV_RANGE));
    }
}
