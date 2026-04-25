//! Floating-text demo — world-space damage numbers.
//!
//! Key ideas:
//! - `Text2d` places text in world space so it moves with the game camera,
//!   unlike UI `Text` which is anchored to the screen.
//! - Each `FloatingText` entity has a lifetime, upward velocity, and starting
//!   alpha; the alpha fades linearly to 0 as the lifetime expires, then the
//!   entity despawns.
//! - Clicking in the game world spawns a random damage number at the cursor.
//!
//! **Controls:** left-click anywhere to spawn a damage number.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Floating Text — click to spawn damage numbers".to_string(),
                resolution: (800.0, 500.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (spawn_on_click, tick_floating_text))
        .run();
}

/// Duration in seconds a floating text label lives.
const LIFETIME: f32 = 1.2;
/// Upward pixels-per-second drift speed.
const RISE_SPEED: f32 = 60.0;

/// Drives the lifetime and fade of a floating label.
#[derive(Component)]
struct FloatingText {
    /// Seconds remaining before despawn.
    remaining: f32,
}

/// Counter used to cycle through predictable damage values for the demo.
#[derive(Resource, Default)]
struct ClickCounter(u32);

/// Spawns the camera and a brief instruction label.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Text::new("Left-click to spawn damage numbers"),
        TextFont { font_size: 20.0, ..default() },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
    commands.init_resource::<ClickCounter>();
}

/// Converts a window-space mouse position to world space using the camera transform.
pub fn window_to_world(cursor: Vec2, window: &Window, camera_transform: &Transform) -> Vec2 {
    let half = Vec2::new(window.width() / 2.0, window.height() / 2.0);
    let ndc = Vec2::new(cursor.x - half.x, half.y - cursor.y);
    let cam_pos = camera_transform.translation.truncate();
    cam_pos + ndc
}

/// Spawns a `Text2d` floating label at the cursor position on left-click.
fn spawn_on_click(
    mut commands: Commands,
    mut counter: ResMut<ClickCounter>,
    mouse: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<&Transform, With<Camera2d>>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = window_query.get_single() else { return };
    let Ok(cam_transform) = camera_query.get_single() else { return };
    let Some(cursor) = window.cursor_position() else { return };

    let world_pos = window_to_world(cursor, window, cam_transform);
    counter.0 += 1;

    // Cycle through a range of damage values for visual variety.
    let damage = 10 + (counter.0 % 8) * 15;
    let color = if damage >= 80 {
        Color::srgb(1.0, 0.3, 0.1) // critical — red-orange
    } else {
        Color::srgb(1.0, 0.95, 0.2) // normal — yellow
    };

    commands.spawn((
        Text2d::new(format!("{}", damage)),
        TextFont { font_size: 32.0, ..default() },
        TextColor(color),
        Transform::from_translation(world_pos.extend(1.0)),
        FloatingText { remaining: LIFETIME },
    ));
}

/// Moves each label upward, fades it out, and despawns it when its lifetime expires.
fn tick_floating_text(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut TextColor, &mut FloatingText)>,
) {
    let dt = time.delta_secs();
    for (entity, mut transform, mut color, mut label) in &mut query {
        label.remaining -= dt;
        if label.remaining <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        transform.translation.y += RISE_SPEED * dt;
        let alpha = (label.remaining / LIFETIME).clamp(0.0, 1.0);
        // Replace the color with the same hue but updated alpha.
        let c = color.0.to_srgba();
        color.0 = Color::srgba(c.red, c.green, c.blue, alpha);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifetime_and_rise_constants_are_positive() {
        assert!(LIFETIME > 0.0);
        assert!(RISE_SPEED > 0.0);
    }

    #[test]
    fn window_to_world_center_maps_to_camera_origin() {
        let window = Window {
            resolution: (800.0, 500.0).into(),
            ..default()
        };
        let cam = Transform::from_translation(Vec3::ZERO);
        let center = Vec2::new(400.0, 250.0);
        let world = window_to_world(center, &window, &cam);
        assert!((world.x).abs() < 1e-3, "x={}", world.x);
        assert!((world.y).abs() < 1e-3, "y={}", world.y);
    }

    #[test]
    fn window_to_world_top_left_is_negative_x_positive_y() {
        let window = Window {
            resolution: (800.0, 500.0).into(),
            ..default()
        };
        let cam = Transform::from_translation(Vec3::ZERO);
        let top_left = Vec2::new(0.0, 0.0);
        let world = window_to_world(top_left, &window, &cam);
        assert!(world.x < 0.0, "expected negative x, got {}", world.x);
        assert!(world.y > 0.0, "expected positive y, got {}", world.y);
    }

    #[test]
    fn window_to_world_respects_camera_offset() {
        let window = Window {
            resolution: (800.0, 500.0).into(),
            ..default()
        };
        let cam = Transform::from_translation(Vec3::new(100.0, 50.0, 0.0));
        let center = Vec2::new(400.0, 250.0);
        let world = window_to_world(center, &window, &cam);
        assert!((world.x - 100.0).abs() < 1e-3, "x={}", world.x);
        assert!((world.y - 50.0).abs() < 1e-3, "y={}", world.y);
    }

    #[test]
    fn alpha_fade_formula_at_half_life() {
        let remaining = LIFETIME / 2.0;
        let alpha = (remaining / LIFETIME).clamp(0.0, 1.0);
        assert!((alpha - 0.5).abs() < 1e-5);
    }

    #[test]
    fn alpha_is_zero_at_end_of_life() {
        let remaining = 0.0_f32;
        let alpha = (remaining / LIFETIME).clamp(0.0, 1.0);
        assert_eq!(alpha, 0.0);
    }
}
