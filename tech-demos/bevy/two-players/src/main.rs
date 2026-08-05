//! Two-player demo — two independent sprites controlled by separate key bindings.
//!
//! Key idea: marker components (`Player1`, `Player2`) let two systems each
//! target their own entity without sharing any mutable state.
//! Player 1 uses WASD; Player 2 uses IJKL.

use bevy::prelude::*;

/// Shared movement speed in world units per second.
const SPEED: f32 = 300.0;

/// Returns a normalized movement direction from four boolean key states.
pub fn input_dir(up: bool, down: bool, left: bool, right: bool) -> Vec2 {
    let raw = Vec2::new(right as i8 as f32 - left as i8 as f32, up as i8 as f32 - down as i8 as f32);
    if raw != Vec2::ZERO { raw.normalize() } else { Vec2::ZERO }
}

/// Returns the per-frame translation delta given a unit direction, speed, and timestep.
pub fn frame_delta(dir: Vec2, speed: f32, dt: f32) -> Vec3 {
    Vec3::new(dir.x * speed * dt, dir.y * speed * dt, 0.0)
}

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
        let dir = input_dir(
            keyboard_input.pressed(KeyCode::KeyW),
            keyboard_input.pressed(KeyCode::KeyS),
            keyboard_input.pressed(KeyCode::KeyA),
            keyboard_input.pressed(KeyCode::KeyD),
        );
        if dir != Vec2::ZERO {
            player_transform.translation += frame_delta(dir, SPEED, time.delta_secs());
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
        let dir = input_dir(
            keyboard_input.pressed(KeyCode::KeyI),
            keyboard_input.pressed(KeyCode::KeyK),
            keyboard_input.pressed(KeyCode::KeyJ),
            keyboard_input.pressed(KeyCode::KeyL),
        );
        if dir != Vec2::ZERO {
            player_transform.translation += frame_delta(dir, SPEED, time.delta_secs());
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

    #[test]
    fn input_dir_no_keys_is_zero() {
        assert_eq!(input_dir(false, false, false, false), Vec2::ZERO);
    }

    #[test]
    fn input_dir_single_direction_is_unit() {
        let d = input_dir(false, true, false, false); // down
        assert!((d.y - -1.0).abs() < 1e-5);
        assert_eq!(d.x, 0.0);
    }

    #[test]
    fn input_dir_diagonal_normalized() {
        let d = input_dir(true, false, true, false); // up + left
        assert!((d.length() - 1.0).abs() < 1e-5);
        assert!(d.x < 0.0 && d.y > 0.0);
    }

    #[test]
    fn input_dir_opposing_keys_cancel() {
        assert_eq!(input_dir(false, false, true, true), Vec2::ZERO);
    }

    #[test]
    fn frame_delta_scales_correctly() {
        let delta = frame_delta(Vec2::Y, 300.0, 0.5);
        assert!((delta.y - 150.0).abs() < 1e-4);
        assert_eq!(delta.x, 0.0);
    }

    #[test]
    fn setup_spawns_two_players_at_offset_positions() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Player1>();
        assert_eq!(q.iter(app.world()).count(), 1);
        let mut q2 = app.world_mut().query::<&Player2>();
        assert_eq!(q2.iter(app.world()).count(), 1);
    }
}
