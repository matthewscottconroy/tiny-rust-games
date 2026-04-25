//! Two-player demo — two independent sprites controlled by separate key bindings.
//!
//! Key idea: marker components (`Player1`, `Player2`) let two systems each
//! target their own entity without sharing any mutable state.
//! Player 1 uses WASD; Player 2 uses IJKL.

use bevy::prelude::*;

/// Shared movement speed in world units per second.
const SPEED: f32 = 300.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (player1_movement_system, player2_movement_system))
        .run();
}

/// Tags the first player entity.
#[derive(Component)]
struct Player1;

/// Tags the second player entity.
#[derive(Component)]
struct Player2;

/// Spawns a camera and two player sprites side-by-side.
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let texture: Handle<Image> = asset_server.load("sprite.png");

    commands.spawn((
        Sprite { image: texture.clone(), ..default() },
        Transform::from_xyz(-100.0, 0.0, 0.0),
        Player1,
    ));

    commands.spawn((
        Sprite { image: texture, ..default() },
        Transform::from_xyz(100.0, 0.0, 0.0),
        Player2,
    ));
}

/// Moves Player 1 with WASD.
fn player1_movement_system(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<&mut Transform, With<Player1>>,
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

/// Moves Player 2 with IJKL.
fn player2_movement_system(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<&mut Transform, With<Player2>>,
) {
    for mut player_transform in player_query.iter_mut() {
        let mut direction = Vec3::ZERO;

        if keyboard_input.pressed(KeyCode::KeyI) { direction.y += 1.0; }
        if keyboard_input.pressed(KeyCode::KeyK) { direction.y -= 1.0; }
        if keyboard_input.pressed(KeyCode::KeyJ) { direction.x -= 1.0; }
        if keyboard_input.pressed(KeyCode::KeyL) { direction.x += 1.0; }

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
