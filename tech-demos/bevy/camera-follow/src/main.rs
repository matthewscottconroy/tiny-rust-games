//! Camera-follow demo.
//!
//! Key idea: the camera smoothly lerps toward the player each frame.
//! Using disjoint query filters (`Without<>`) avoids borrow conflicts when
//! two systems both need to read or write `Transform` on different entity sets.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (move_player, follow_camera))
        .run();
}

/// Marks the entity the camera should track.
#[derive(Component)]
struct Player;

/// Top-speed of the player in world units per second.
const SPEED: f32 = 220.0;

/// Fraction of the gap to close each second (higher = snappier).
const LERP_SPEED: f32 = 6.0;

/// Spawns a checkerboard background (makes camera motion visible), the player,
/// and an instruction label.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    for row in -10..=10 {
        for col in -12..=12 {
            let dark = (row + col) % 2 == 0;
            let color = if dark {
                Color::srgb(0.13, 0.13, 0.13)
            } else {
                Color::srgb(0.22, 0.22, 0.22)
            };
            commands.spawn((
                Sprite { color, custom_size: Some(Vec2::splat(64.0)), ..default() },
                Transform::from_xyz(col as f32 * 64.0, row as f32 * 64.0, -1.0),
            ));
        }
    }

    commands.spawn((
        Sprite {
            color: Color::srgb(0.25, 0.85, 0.35),
            custom_size: Some(Vec2::splat(32.0)),
            ..default()
        },
        Transform::default(),
        Player,
    ));

    commands.spawn((
        Text::new("WASD — move   (camera lerp-follows)"),
        TextFont { font_size: 16.0, ..default() },
        TextColor(Color::srgb(0.85, 0.85, 0.85)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

/// Reads WASD input and moves the player at [`SPEED`] units/s.
fn move_player(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    let Ok(mut transform) = query.single_mut() else { return; };

    let mut dir = Vec2::ZERO;
    if input.pressed(KeyCode::KeyW) { dir.y += 1.0; }
    if input.pressed(KeyCode::KeyS) { dir.y -= 1.0; }
    if input.pressed(KeyCode::KeyA) { dir.x -= 1.0; }
    if input.pressed(KeyCode::KeyD) { dir.x += 1.0; }

    if dir != Vec2::ZERO {
        let delta = dir.normalize() * SPEED * time.delta_secs();
        transform.translation.x += delta.x;
        transform.translation.y += delta.y;
    }
}

/// Smoothly moves the camera toward the player using exponential lerp.
///
/// `Without<Camera2d>` and `Without<Player>` make the two queries disjoint,
/// which is required when both touch `Transform`.
fn follow_camera(
    time: Res<Time>,
    player_query: Query<&Transform, (With<Player>, Without<Camera2d>)>,
    mut cam_query: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
) {
    let Ok(player) = player_query.single() else { return; };
    let Ok(mut cam) = cam_query.single_mut() else { return; };

    let target = player.translation;
    cam.translation = cam.translation.lerp(target, LERP_SPEED * time.delta_secs());
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
    fn setup_spawns_one_camera() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Camera2d>();
        assert_eq!(q.iter(app.world()).count(), 1);
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
}
