//! Enemy chase AI demo.
//!
//! Key ideas:
//! - Enemies spawn periodically at the screen edge using a repeating [`Timer`].
//! - Each enemy queries the player's position and steers toward it each frame.
//! - No rand crate needed: golden-ratio angle stepping (`2π/φ²`) distributes
//!   spawn points uniformly around a circle with no clustering.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(SpawnTimer {
            timer: Timer::from_seconds(1.5, TimerMode::Repeating),
            angle: 0.0,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (move_player, spawn_enemies, chase_player, move_entities))
        .run();
}

// --- Components ---

/// Marks the player entity.
#[derive(Component)]
struct Player;

/// Marks an enemy entity.
#[derive(Component)]
struct Enemy;

/// 2D linear velocity in world units/second.
#[derive(Component)]
struct Velocity(Vec2);

// --- Resources ---

/// Drives periodic enemy spawning and tracks the current golden-ratio angle.
#[derive(Resource)]
struct SpawnTimer {
    timer: Timer,
    /// Current spawn angle on the circle, advanced each tick by the golden-ratio step.
    angle: f32,
}

// --- Constants ---

const PLAYER_SPEED: f32 = 200.0;
const ENEMY_SPEED:  f32 = 90.0;

/// Radius of the circle on which enemies spawn.
const SPAWN_RADIUS: f32 = 360.0;

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
        TextFont { font_size: 16.0, ..default() },
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
    mut query: Query<&mut Velocity, With<Player>>,
) {
    let Ok(mut vel) = query.single_mut() else { return; };

    let mut dir = Vec2::ZERO;
    if input.pressed(KeyCode::KeyW) { dir.y += 1.0; }
    if input.pressed(KeyCode::KeyS) { dir.y -= 1.0; }
    if input.pressed(KeyCode::KeyA) { dir.x -= 1.0; }
    if input.pressed(KeyCode::KeyD) { dir.x += 1.0; }

    vel.0 = if dir != Vec2::ZERO { dir.normalize() * PLAYER_SPEED } else { Vec2::ZERO };
}

/// Ticks the spawn timer and spawns a new enemy on each interval.
///
/// The spawn angle is advanced by `2π/φ²` (the golden-ratio angle) to spread
/// enemies evenly around the spawn circle without repeating patterns.
fn spawn_enemies(
    mut commands: Commands,
    time: Res<Time>,
    mut spawn: ResMut<SpawnTimer>,
) {
    if !spawn.timer.tick(time.delta()).just_finished() {
        return;
    }

    spawn.angle = (spawn.angle + 2.399_963) % (2.0 * std::f32::consts::PI);
    let x = spawn.angle.cos() * SPAWN_RADIUS;
    let y = spawn.angle.sin() * SPAWN_RADIUS;

    commands.spawn((
        Sprite {
            color: Color::srgb(0.85, 0.2, 0.2),
            custom_size: Some(Vec2::splat(22.0)),
            ..default()
        },
        Transform::from_xyz(x, y, 0.0),
        Enemy,
        Velocity(Vec2::ZERO),
    ));
}

/// Sets each enemy's velocity to point directly at the player.
fn chase_player(
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemy_query: Query<(&Transform, &mut Velocity), (With<Enemy>, Without<Player>)>,
) {
    let Ok(player_transform) = player_query.single() else { return; };

    for (enemy_transform, mut vel) in &mut enemy_query {
        let diff = player_transform.translation - enemy_transform.translation;
        vel.0 = diff.truncate().normalize_or_zero() * ENEMY_SPEED;
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

    #[test]
    fn setup_spawns_one_player() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(SpawnTimer {
                timer: Timer::from_seconds(1.5, TimerMode::Repeating),
                angle: 0.0,
            })
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Player>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn no_enemies_at_startup() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(SpawnTimer {
                timer: Timer::from_seconds(1.5, TimerMode::Repeating),
                angle: 0.0,
            })
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Enemy>();
        assert_eq!(q.iter(app.world()).count(), 0);
    }

    #[test]
    fn spawn_radius_is_positive() {
        assert!(SPAWN_RADIUS > 0.0);
    }

    #[test]
    fn golden_ratio_angle_stays_in_range() {
        let mut angle = 0.0f32;
        for _ in 0..100 {
            angle = (angle + 2.399_963) % (2.0 * std::f32::consts::PI);
            assert!(angle >= 0.0 && angle < 2.0 * std::f32::consts::PI);
        }
    }
}
