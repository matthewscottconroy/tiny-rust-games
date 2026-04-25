//! Movable sprite demo — WASD movement applied to a sprite entity.
//!
//! Key idea: the movement system reads from `ButtonInput<KeyCode>` (held keys)
//! and applies a delta each frame using `time.delta_secs()` for frame-rate
//! independence.

use bevy::prelude::*;

/// Top-speed of the player sprite in world units per second.
const SPEED: f32 = 300.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, player_movement_system)
        .run();
}

/// Spawns a camera and the player sprite from disk.
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    commands.spawn((
        Sprite {
            image: asset_server.load("sprite.png"),
            ..default()
        },
        Player,
    ));
}

/// Tags the player entity.
#[derive(Component)]
struct Player;

/// Reads WASD input and moves the player at [`SPEED`] world units/s.
fn player_movement_system(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<&mut Transform, With<Player>>,
) {
    for mut player_transform in player_query.iter_mut() {
        let mut direction = Vec3::ZERO;

        if keyboard_input.pressed(KeyCode::KeyW) { direction.y += 1.0; }
        if keyboard_input.pressed(KeyCode::KeyS) { direction.y -= 1.0; }
        if keyboard_input.pressed(KeyCode::KeyA) { direction.x -= 1.0; }
        if keyboard_input.pressed(KeyCode::KeyD) { direction.x += 1.0; }

        if direction != Vec3::ZERO {
            player_transform.translation += direction.normalize() * SPEED * time.delta_secs();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_is_positive() {
        assert!(SPEED > 0.0);
    }
}
