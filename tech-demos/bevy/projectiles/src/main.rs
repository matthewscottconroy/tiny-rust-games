//! Projectiles demo.
//!
//! Key ideas:
//! - Bullets are spawned at runtime as plain ECS entities; no object pool needed.
//! - Each bullet carries a [`Velocity`] component and moves every `Update` frame.
//! - Off-screen bullets are despawned each frame to keep entity count bounded.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (move_player, fire_bullets, move_bullets, despawn_offscreen))
        .run();
}

// --- Components ---

/// Player state — tracks the last movement direction so bullets inherit it.
#[derive(Component)]
struct Player {
    /// Unit vector of the player's last movement direction.
    facing: Vec2,
}

/// Tags a bullet entity.
#[derive(Component)]
struct Bullet;

/// 2D linear velocity in world units/second.
#[derive(Component)]
struct Velocity(Vec2);

// --- Constants ---

const PLAYER_SPEED: f32 = 200.0;
const BULLET_SPEED: f32 = 500.0;

/// Bullets are despawned once their position exceeds this distance from origin.
const OFFSCREEN_MARGIN: f32 = 600.0;

// --- Setup ---

/// Spawns the camera, player sprite, and instruction label.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Sprite {
            color: Color::srgb(0.3, 0.8, 0.3),
            custom_size: Some(Vec2::new(28.0, 36.0)),
            ..default()
        },
        Transform::default(),
        Player { facing: Vec2::Y },
    ));

    commands.spawn((
        Text::new("WASD — move   Arrow keys — fire"),
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

/// Reads WASD input, moves the player, and updates `Player::facing`.
fn move_player(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut Player)>,
) {
    let Ok((mut transform, mut player)) = query.single_mut() else { return; };

    let mut dir = Vec2::ZERO;
    if input.pressed(KeyCode::KeyW) { dir.y += 1.0; }
    if input.pressed(KeyCode::KeyS) { dir.y -= 1.0; }
    if input.pressed(KeyCode::KeyA) { dir.x -= 1.0; }
    if input.pressed(KeyCode::KeyD) { dir.x += 1.0; }

    if dir != Vec2::ZERO {
        let normalized = dir.normalize();
        player.facing = normalized;
        let delta = normalized * PLAYER_SPEED * time.delta_secs();
        transform.translation.x += delta.x;
        transform.translation.y += delta.y;
    }
}

/// Spawns a bullet in a cardinal direction when an arrow key is just pressed.
fn fire_bullets(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    player_query: Query<&Transform, With<Player>>,
) {
    let Ok(transform) = player_query.single() else { return; };

    let mut directions: Vec<Vec2> = Vec::new();
    if input.just_pressed(KeyCode::ArrowUp)    { directions.push(Vec2::Y); }
    if input.just_pressed(KeyCode::ArrowDown)  { directions.push(Vec2::NEG_Y); }
    if input.just_pressed(KeyCode::ArrowLeft)  { directions.push(Vec2::NEG_X); }
    if input.just_pressed(KeyCode::ArrowRight) { directions.push(Vec2::X); }

    for dir in directions {
        commands.spawn((
            Sprite {
                color: Color::srgb(1.0, 0.85, 0.1),
                custom_size: Some(Vec2::new(6.0, 16.0)),
                ..default()
            },
            Transform::from_translation(transform.translation),
            Bullet,
            Velocity(dir * BULLET_SPEED),
        ));
    }
}

/// Advances all bullets along their velocity.
fn move_bullets(time: Res<Time>, mut query: Query<(&mut Transform, &Velocity), With<Bullet>>) {
    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.0.x * time.delta_secs();
        transform.translation.y += velocity.0.y * time.delta_secs();
    }
}

/// Despawns bullets that have travelled beyond [`OFFSCREEN_MARGIN`].
fn despawn_offscreen(mut commands: Commands, query: Query<(Entity, &Transform), With<Bullet>>) {
    for (entity, transform) in &query {
        let p = transform.translation;
        if p.x.abs() > OFFSCREEN_MARGIN || p.y.abs() > OFFSCREEN_MARGIN {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_spawns_one_player() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Player>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn no_bullets_at_startup() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Bullet>();
        assert_eq!(q.iter(app.world()).count(), 0);
    }

    #[test]
    fn player_starts_at_origin() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<(&Player, &Transform)>();
        for (_, t) in q.iter(app.world()) {
            assert_eq!(t.translation, Vec3::ZERO);
        }
    }

    #[test]
    fn offscreen_margin_is_positive() {
        assert!(OFFSCREEN_MARGIN > 0.0);
    }
}
