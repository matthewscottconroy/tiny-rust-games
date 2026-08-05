//! Stealth AI — a reusable Bevy plugin: FOV cone detection with
//! Patrol / Alert / Chase guard states.
//!
//! This crate is a *building block*: drop [`StealthAiPlugin`] into any Bevy app
//! with `app.add_plugins(StealthAiPlugin)` and it spawns a player, a patrolling
//! guard with a field-of-view cone, and a status tile that changes color as the
//! guard cycles through its AI states. Tune it through the [`StealthAiConfig`]
//! resource without editing the plugin's internals.
//!
//! Key ideas:
//! - [`in_fov_cone`] is a pure function that checks whether a target point falls
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
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use stealth_ai::StealthAiPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(StealthAiPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system and resource for the stealth-AI feature.
///
/// Add it with `app.add_plugins(StealthAiPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct StealthAiPlugin;

impl Plugin for StealthAiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StealthAiConfig>()
            .add_systems(Startup, setup)
            .add_systems(Update, (move_player, update_guard, draw_fov, update_status));
    }
}

// --- Configuration ---

/// Tunable parameters for the feature. Override before adding the plugin, e.g.
/// `app.insert_resource(StealthAiConfig { guard_speed: 120.0, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct StealthAiConfig {
    /// Patrol walking speed in pixels per second.
    pub guard_speed: f32,
    /// Chase running speed in pixels per second.
    pub chase_speed: f32,
    /// Half-aperture of the FOV cone in radians.
    pub fov_half_angle: f32,
    /// Maximum detection distance in pixels.
    pub fov_range: f32,
    /// Seconds the guard stays alert before returning to patrol.
    pub alert_timeout: f32,
    /// Player movement speed in pixels per second.
    pub player_speed: f32,
    /// Number of thin sprite segments used to draw the FOV fan.
    pub fov_rays: usize,
}

impl Default for StealthAiConfig {
    fn default() -> Self {
        Self {
            guard_speed: 90.0,
            chase_speed: 140.0,
            fov_half_angle: std::f32::consts::FRAC_PI_4, // 45 °
            fov_range: 200.0,
            alert_timeout: 2.5,
            player_speed: 160.0,
            fov_rays: 24,
        }
    }
}

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

/// The guard's AI state machine.
#[derive(Clone, PartialEq)]
pub enum GuardState {
    /// Walking between waypoints.
    Patrol { waypoint_idx: usize },
    /// Stopped and looking after spotting the player.
    Alert { timer: f32 },
    /// Running toward the last known player position.
    Chase { last_known: Vec2 },
}

// ─── Waypoints ───────────────────────────────────────────────────────────────

const WAYPOINTS: &[Vec2] = &[
    Vec2::new(-280.0, 60.0),
    Vec2::new(280.0, 60.0),
    Vec2::new(280.0, -100.0),
    Vec2::new(-280.0, -100.0),
];

// ─── Components ──────────────────────────────────────────────────────────────

/// Guard entity with its AI state and facing direction.
#[derive(Component)]
pub struct Guard {
    /// Current AI state.
    pub state: GuardState,
    /// Unit vector the guard is facing.
    pub facing: Vec2,
}

/// Marks the player-controlled entity.
#[derive(Component)]
pub struct PlayerMarker;

/// Background status tile that shows the guard's current state colour.
#[derive(Component)]
pub struct StatusTile;

/// One segment of the FOV fan visualisation.
#[derive(Component)]
pub struct FovSegment(pub usize);

// ─── Setup ───────────────────────────────────────────────────────────────────

fn setup(mut commands: Commands, config: Res<StealthAiConfig>) {
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
    for i in 0..config.fov_rays {
        commands.spawn((
            Sprite { color: Color::srgba(1.0, 1.0, 0.0, 0.18), custom_size: Some(Vec2::new(config.fov_range, 2.0)), ..default() },
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

// ─── Systems ─────────────────────────────────────────────────────────────────

fn move_player(
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    config: Res<StealthAiConfig>,
    mut q: Query<&mut Transform, With<PlayerMarker>>,
) {
    let Ok(mut t) = q.single_mut() else { return };
    let mut dir = Vec2::ZERO;
    if input.pressed(KeyCode::KeyW) || input.pressed(KeyCode::ArrowUp)    { dir.y += 1.0; }
    if input.pressed(KeyCode::KeyS) || input.pressed(KeyCode::ArrowDown)   { dir.y -= 1.0; }
    if input.pressed(KeyCode::KeyA) || input.pressed(KeyCode::ArrowLeft)   { dir.x -= 1.0; }
    if input.pressed(KeyCode::KeyD) || input.pressed(KeyCode::ArrowRight)  { dir.x += 1.0; }
    if dir != Vec2::ZERO {
        t.translation += (dir.normalize() * config.player_speed * time.delta_secs()).extend(0.0);
    }
}

fn update_guard(
    time: Res<Time>,
    config: Res<StealthAiConfig>,
    player_q: Query<&Transform, With<PlayerMarker>>,
    mut guard_q: Query<(&mut Guard, &mut Transform), Without<PlayerMarker>>,
    mut status_q: Query<&mut Transform, (With<StatusTile>, Without<Guard>, Without<PlayerMarker>)>,
) {
    let Ok(player_t) = player_q.single() else { return };
    let Ok((mut guard, mut guard_t)) = guard_q.single_mut() else { return };
    let dt = time.delta_secs();
    let player_pos = player_t.translation.truncate();
    let guard_pos = guard_t.translation.truncate();

    let sees_player = in_fov_cone(player_pos, guard_pos, guard.facing, config.fov_half_angle, config.fov_range);

    let new_state = match guard.state.clone() {
        GuardState::Patrol { waypoint_idx } => {
            if sees_player {
                GuardState::Alert { timer: config.alert_timeout }
            } else {
                let target = WAYPOINTS[waypoint_idx];
                let to = target - guard_pos;
                if to.length() < 4.0 {
                    let next = (waypoint_idx + 1) % WAYPOINTS.len();
                    GuardState::Patrol { waypoint_idx: next }
                } else {
                    let dir = to.normalize();
                    guard.facing = dir;
                    guard_t.translation += (dir * config.guard_speed * dt).extend(0.0);
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
                GuardState::Alert { timer: config.alert_timeout }
            } else {
                let dir = to.normalize();
                guard.facing = dir;
                guard_t.translation += (dir * config.chase_speed * dt).extend(0.0);
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
    config: Res<StealthAiConfig>,
    guard_q: Query<(&Guard, &Transform)>,
    mut segments: Query<(&FovSegment, &mut Transform), Without<Guard>>,
) {
    let Ok((guard, gt)) = guard_q.single() else { return };
    let origin = gt.translation.truncate();
    let base_angle = guard.facing.y.atan2(guard.facing.x);
    let step = (config.fov_half_angle * 2.0) / config.fov_rays as f32;

    for (FovSegment(i), mut t) in &mut segments {
        let angle = base_angle - config.fov_half_angle + step * (*i as f32 + 0.5);
        let dir = Vec2::from_angle(angle);
        let mid = origin + dir * config.fov_range * 0.5;
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

    fn cfg() -> StealthAiConfig {
        StealthAiConfig::default()
    }

    #[test]
    fn target_directly_ahead_is_detected() {
        let c = cfg();
        let origin = Vec2::ZERO;
        let forward = Vec2::X;
        let target = Vec2::new(100.0, 0.0);
        assert!(in_fov_cone(target, origin, forward, c.fov_half_angle, c.fov_range));
    }

    #[test]
    fn target_behind_is_not_detected() {
        let c = cfg();
        let origin = Vec2::ZERO;
        let forward = Vec2::X;
        let target = Vec2::new(-100.0, 0.0);
        assert!(!in_fov_cone(target, origin, forward, c.fov_half_angle, c.fov_range));
    }

    #[test]
    fn target_out_of_range_is_not_detected() {
        let c = cfg();
        let origin = Vec2::ZERO;
        let forward = Vec2::X;
        let target = Vec2::new(c.fov_range + 1.0, 0.0);
        assert!(!in_fov_cone(target, origin, forward, c.fov_half_angle, c.fov_range));
    }

    #[test]
    fn target_at_exact_half_angle_edge_is_detected() {
        let c = cfg();
        let origin = Vec2::ZERO;
        let forward = Vec2::X;
        // Slightly inside half-angle boundary.
        let angle = c.fov_half_angle - 0.01;
        let target = Vec2::new(angle.cos() * 50.0, angle.sin() * 50.0);
        assert!(in_fov_cone(target, origin, forward, c.fov_half_angle, c.fov_range));
    }

    #[test]
    fn target_beyond_half_angle_is_not_detected() {
        let c = cfg();
        let origin = Vec2::ZERO;
        let forward = Vec2::X;
        let angle = c.fov_half_angle + 0.1;
        let target = Vec2::new(angle.cos() * 50.0, angle.sin() * 50.0);
        assert!(!in_fov_cone(target, origin, forward, c.fov_half_angle, c.fov_range));
    }

    #[test]
    fn config_default_matches_documented_values() {
        let c = StealthAiConfig::default();
        assert_eq!(c.guard_speed, 90.0);
        assert_eq!(c.chase_speed, 140.0);
        assert_eq!(c.fov_range, 200.0);
        assert_eq!(c.fov_rays, 24);
    }

    #[test]
    fn setup_spawns_one_guard() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<StealthAiConfig>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Guard>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn plugin_spawns_fov_segments() {
        // Building-block path: the plugin composes onto a headless app.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StealthAiPlugin));
        // The plugin's Update systems read input; MinimalPlugins omits it.
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.update();

        let expected = StealthAiConfig::default().fov_rays;
        let mut q = app.world_mut().query::<&FovSegment>();
        assert_eq!(q.iter(app.world()).count(), expected);
    }
}
