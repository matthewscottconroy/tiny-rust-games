//! Parallax background demo.
//!
//! Key ideas:
//! - Each background layer stores a `scroll_factor` (0 = static, 1 = moves
//!   with the world at full speed).
//! - Every frame each layer's world X is set to `camera_x * scroll_factor`.
//! - Layers with lower factors appear to be farther away because they move
//!   less as the camera pans.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        // Chain ensures camera position is updated before parallax reads it.
        .add_systems(Update, (move_player, follow_camera, update_parallax).chain())
        .run();
}

// --- Components ---

/// Marks the player entity.
#[derive(Component)]
struct Player;

/// A background layer whose X position tracks a fraction of the camera's X.
#[derive(Component)]
struct ParallaxLayer {
    /// Fraction of the camera displacement this layer follows.
    /// `0.0` = stationary; `1.0` = moves with world; values in between appear at depth.
    scroll_factor: f32,
}

const PLAYER_SPEED: f32 = 220.0;
const CAM_LERP: f32 = 8.0;

// --- Setup ---

/// Spawns five parallax background layers (back to front), the player, and a HUD label.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let layers: &[(f32, Color, f32, f32)] = &[
        (0.05, Color::srgb(0.10, 0.10, 0.22), 600.0, -10.0), // distant sky
        (0.15, Color::srgb(0.15, 0.12, 0.30), 200.0,  -9.0), // far mountains
        (0.35, Color::srgb(0.18, 0.22, 0.18), 120.0,  -8.0), // mid hills
        (0.60, Color::srgb(0.13, 0.25, 0.13),  70.0,  -7.0), // near treeline
        (0.85, Color::srgb(0.25, 0.18, 0.08),  40.0,  -6.0), // ground strip
    ];

    for &(scroll_factor, color, height, z) in layers {
        commands.spawn((
            Sprite {
                color,
                custom_size: Some(Vec2::new(4000.0, height)),
                ..default()
            },
            Transform::from_xyz(0.0, -300.0 + height / 2.0, z),
            ParallaxLayer { scroll_factor },
        ));
    }

    commands.spawn((
        Sprite {
            color: Color::srgb(0.9, 0.75, 0.2),
            custom_size: Some(Vec2::new(20.0, 30.0)),
            ..default()
        },
        Transform::from_xyz(0.0, -270.0, 0.0),
        Player,
    ));

    commands.spawn((
        Text::new("WASD — move   notice layers scroll at different speeds"),
        TextFont { font_size: 15.0, ..default() },
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

/// Reads horizontal input (A/D or arrow keys) and moves the player.
///
/// Vertical movement is intentionally disabled to keep focus on horizontal parallax.
fn move_player(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    let Ok(mut transform) = query.single_mut() else { return; };

    let mut dir = Vec2::ZERO;
    if input.pressed(KeyCode::KeyA) || input.pressed(KeyCode::ArrowLeft)  { dir.x -= 1.0; }
    if input.pressed(KeyCode::KeyD) || input.pressed(KeyCode::ArrowRight) { dir.x += 1.0; }

    if dir != Vec2::ZERO {
        transform.translation.x += dir.normalize().x * PLAYER_SPEED * time.delta_secs();
    }
}

/// Smoothly moves the camera toward the player's horizontal position.
fn follow_camera(
    time: Res<Time>,
    player_query: Query<&Transform, (With<Player>, Without<Camera2d>)>,
    mut cam_query: Query<&mut Transform, (With<Camera2d>, Without<Player>, Without<ParallaxLayer>)>,
) {
    let Ok(player) = player_query.single() else { return; };
    let Ok(mut cam) = cam_query.single_mut() else { return; };

    let target_x = player.translation.x;
    cam.translation.x += (target_x - cam.translation.x) * CAM_LERP * time.delta_secs();
}

/// Repositions each layer so it lags behind the camera by its `scroll_factor`.
fn update_parallax(
    cam_query: Query<&Transform, (With<Camera2d>, Without<ParallaxLayer>)>,
    mut layer_query: Query<(&mut Transform, &ParallaxLayer)>,
) {
    let Ok(cam) = cam_query.single() else { return; };

    for (mut transform, layer) in &mut layer_query {
        transform.translation.x = cam.translation.x * layer.scroll_factor;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_spawns_five_parallax_layers() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&ParallaxLayer>();
        assert_eq!(q.iter(app.world()).count(), 5);
    }

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
    fn scroll_factors_are_in_unit_interval() {
        for &(scroll_factor, _, _, _) in &[
            (0.05f32, (), (), ()),
            (0.15, (), (), ()),
            (0.35, (), (), ()),
            (0.60, (), (), ()),
            (0.85, (), (), ()),
        ] {
            assert!(scroll_factor >= 0.0 && scroll_factor <= 1.0,
                "scroll_factor {scroll_factor} out of [0, 1]");
        }
    }

    #[test]
    fn scroll_factors_are_ascending() {
        let factors = [0.05f32, 0.15, 0.35, 0.60, 0.85];
        for w in factors.windows(2) {
            assert!(w[1] > w[0], "factors should increase back-to-front");
        }
    }
}
