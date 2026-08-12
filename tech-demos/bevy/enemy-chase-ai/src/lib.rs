//! Enemy chase AI — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`EnemyChaseAiPlugin`] into any Bevy
//! app with `app.add_plugins(EnemyChaseAiPlugin)` and it spawns a player, then
//! periodically spawns enemies that steer toward the player each frame. Tune it
//! through the [`EnemyChaseAiConfig`] resource without editing the internals.
//!
//! Key ideas:
//! - Enemies spawn periodically at the screen edge using a repeating [`Timer`].
//! - Each [`Enemy`] queries the player's position and steers toward it each frame.
//! - No rand crate needed: golden-ratio angle stepping (`2π/φ²`) distributes
//!   spawn points uniformly around a circle with no clustering. See
//!   [`next_spawn_angle`] and [`spawn_point`].
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use enemy_chase_ai::EnemyChaseAiPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(EnemyChaseAiPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system and resource for the enemy-chase feature.
///
/// Add it with `app.add_plugins(EnemyChaseAiPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct EnemyChaseAiPlugin;

impl Plugin for EnemyChaseAiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnemyChaseAiConfig>()
            .init_resource::<SpawnTimer>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (move_player, spawn_enemies, chase_player, move_entities),
            );
    }
}

// --- Configuration ---

/// Tunable parameters for the feature. Override before adding the plugin, e.g.
/// `app.insert_resource(EnemyChaseAiConfig { enemy_speed: 150.0, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct EnemyChaseAiConfig {
    /// Player movement speed in world units/second.
    pub player_speed: f32,
    /// Enemy chase speed in world units/second.
    pub enemy_speed: f32,
    /// Radius of the circle on which enemies spawn.
    pub spawn_radius: f32,
    /// Seconds between enemy spawns.
    pub spawn_interval: f32,
}

impl Default for EnemyChaseAiConfig {
    fn default() -> Self {
        Self {
            player_speed: 200.0,
            enemy_speed: 90.0,
            spawn_radius: 360.0,
            spawn_interval: 1.5,
        }
    }
}

/// The golden-ratio angle (`2π/φ²` ≈ 2.399963 radians) used to step spawn
/// points evenly around a circle without repeating patterns.
pub const GOLDEN_ANGLE: f32 = 2.399_963;

// --- Components ---

/// Marks the player entity.
#[derive(Component)]
pub struct Player;

/// Marks an enemy entity.
#[derive(Component)]
pub struct Enemy;

/// 2D linear velocity in world units/second.
#[derive(Component)]
pub struct Velocity(pub Vec2);

// --- Resources ---

/// Drives periodic enemy spawning and tracks the current golden-ratio angle.
#[derive(Resource)]
pub struct SpawnTimer {
    /// Repeating timer that fires once per spawn interval.
    pub timer: Timer,
    /// Current spawn angle on the circle, advanced each tick by the golden-ratio step.
    pub angle: f32,
}

impl Default for SpawnTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(
                EnemyChaseAiConfig::default().spawn_interval,
                TimerMode::Repeating,
            ),
            angle: 0.0,
        }
    }
}

// --- Pure functions ---

/// Advances a spawn angle by the golden-ratio step, wrapped into `[0, 2π)`.
pub fn next_spawn_angle(angle: f32) -> f32 {
    (angle + GOLDEN_ANGLE) % (2.0 * std::f32::consts::PI)
}

/// Returns the point on a circle of the given radius at the given angle.
pub fn spawn_point(angle: f32, radius: f32) -> Vec2 {
    Vec2::new(angle.cos() * radius, angle.sin() * radius)
}

// --- Setup ---

/// Spawns the camera, player, and instruction label.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Sprite {
            color: Color::srgb(0.25, 0.8, 0.35),
            custom_size: Some(Vec2::splat(28.0)),
            ..default()
        },
        Transform::default(),
        Player,
        Velocity(Vec2::ZERO),
    ));

    commands.spawn((
        Text::new("WASD — move   survive the horde"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.8, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

// --- Systems ---

/// Reads WASD and sets the player's velocity (stops when no key held).
fn move_player(
    input: Res<ButtonInput<KeyCode>>,
    config: Res<EnemyChaseAiConfig>,
    mut query: Query<&mut Velocity, With<Player>>,
) {
    let Ok(mut vel) = query.single_mut() else {
        return;
    };

    let mut dir = Vec2::ZERO;
    if input.pressed(KeyCode::KeyW) {
        dir.y += 1.0;
    }
    if input.pressed(KeyCode::KeyS) {
        dir.y -= 1.0;
    }
    if input.pressed(KeyCode::KeyA) {
        dir.x -= 1.0;
    }
    if input.pressed(KeyCode::KeyD) {
        dir.x += 1.0;
    }

    vel.0 = if dir != Vec2::ZERO {
        dir.normalize() * config.player_speed
    } else {
        Vec2::ZERO
    };
}

/// Ticks the spawn timer and spawns a new enemy on each interval.
///
/// The spawn angle is advanced by [`GOLDEN_ANGLE`] (the golden-ratio angle) to
/// spread enemies evenly around the spawn circle without repeating patterns.
fn spawn_enemies(
    mut commands: Commands,
    time: Res<Time>,
    config: Res<EnemyChaseAiConfig>,
    mut spawn: ResMut<SpawnTimer>,
) {
    if !spawn.timer.tick(time.delta()).just_finished() {
        return;
    }

    spawn.angle = next_spawn_angle(spawn.angle);
    let pos = spawn_point(spawn.angle, config.spawn_radius);

    commands.spawn((
        Sprite {
            color: Color::srgb(0.85, 0.2, 0.2),
            custom_size: Some(Vec2::splat(22.0)),
            ..default()
        },
        Transform::from_xyz(pos.x, pos.y, 0.0),
        Enemy,
        Velocity(Vec2::ZERO),
    ));
}

/// Sets each enemy's velocity to point directly at the player.
fn chase_player(
    config: Res<EnemyChaseAiConfig>,
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemy_query: Query<(&Transform, &mut Velocity), (With<Enemy>, Without<Player>)>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    for (enemy_transform, mut vel) in &mut enemy_query {
        let diff = player_transform.translation - enemy_transform.translation;
        vel.0 = diff.truncate().normalize_or_zero() * config.enemy_speed;
    }
}

/// Moves every entity with a [`Velocity`] component.
fn move_entities(time: Res<Time>, mut query: Query<(&mut Transform, &Velocity)>) {
    for (mut transform, vel) in &mut query {
        transform.translation.x += vel.0.x * time.delta_secs();
        transform.translation.y += vel.0.y * time.delta_secs();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Pure function tests ---

    #[test]
    fn spawn_radius_default_is_positive() {
        assert!(EnemyChaseAiConfig::default().spawn_radius > 0.0);
    }

    #[test]
    fn config_default_matches_documented_values() {
        let c = EnemyChaseAiConfig::default();
        assert_eq!(c.player_speed, 200.0);
        assert_eq!(c.enemy_speed, 90.0);
        assert_eq!(c.spawn_radius, 360.0);
        assert_eq!(c.spawn_interval, 1.5);
    }

    #[test]
    fn golden_ratio_angle_stays_in_range() {
        let mut angle = 0.0f32;
        for _ in 0..100 {
            angle = next_spawn_angle(angle);
            assert!((0.0..2.0 * std::f32::consts::PI).contains(&angle));
        }
    }

    #[test]
    fn spawn_point_lies_on_circle() {
        let r = 360.0;
        for step in 0..8 {
            let angle = step as f32 * 0.5;
            let p = spawn_point(angle, r);
            assert!((p.length() - r).abs() < 1e-2);
        }
    }

    #[test]
    fn spawn_point_at_zero_angle() {
        let p = spawn_point(0.0, 360.0);
        assert!((p.x - 360.0).abs() < 1e-2);
        assert!(p.y.abs() < 1e-2);
    }

    // --- ECS setup tests ---

    #[test]
    fn setup_spawns_one_player() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<EnemyChaseAiConfig>()
            .init_resource::<SpawnTimer>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Player>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn no_enemies_at_startup() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<EnemyChaseAiConfig>()
            .init_resource::<SpawnTimer>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Enemy>();
        assert_eq!(q.iter(app.world()).count(), 0);
    }
}
