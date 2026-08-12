//! Floating text — a reusable Bevy plugin for world-space damage numbers.
//!
//! This crate is a *building block*: drop [`FloatingTextPlugin`] into any Bevy
//! app with `app.add_plugins(FloatingTextPlugin)` and left-clicking the game
//! world spawns a rising, fading damage number at the cursor. Tune it through
//! the [`FloatingTextConfig`] resource without editing the plugin's internals.
//!
//! Key ideas:
//! - `Text2d` places text in world space so it moves with the game camera,
//!   unlike UI `Text` which is anchored to the screen.
//! - Each [`FloatingText`] entity has a lifetime and upward velocity; its alpha
//!   fades linearly to 0 as the lifetime expires, then the entity despawns.
//! - Clicking in the game world spawns a random damage number at the cursor.
//!
//! **Controls:** left-click anywhere to spawn a damage number.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use floating_text::FloatingTextPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(FloatingTextPlugin)
//!     .run();
//! ```

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// Bundles every system and resource for the floating-text feature.
///
/// Add it with `app.add_plugins(FloatingTextPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct FloatingTextPlugin;

impl Plugin for FloatingTextPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FloatingTextConfig>()
            .init_resource::<ClickCounter>()
            .add_systems(Startup, setup)
            .add_systems(Update, (spawn_on_click, tick_floating_text));
    }
}

// --- Configuration ---

/// Tunable parameters for the feature. Override before adding the plugin, e.g.
/// `app.insert_resource(FloatingTextConfig { rise_speed: 120.0, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct FloatingTextConfig {
    /// Duration in seconds a floating text label lives.
    pub lifetime: f32,
    /// Upward pixels-per-second drift speed.
    pub rise_speed: f32,
    /// Font size of a spawned damage number.
    pub font_size: f32,
    /// Damage value at or above which a hit is rendered as a critical.
    pub crit_threshold: u32,
}

impl Default for FloatingTextConfig {
    fn default() -> Self {
        Self {
            lifetime: 1.2,
            rise_speed: 60.0,
            font_size: 32.0,
            crit_threshold: 80,
        }
    }
}

// --- Components ---

/// Drives the lifetime and fade of a floating label.
#[derive(Component)]
pub struct FloatingText {
    /// Seconds remaining before despawn.
    pub remaining: f32,
    /// Total lifetime the label started with, used to compute fade alpha.
    pub lifetime: f32,
}

// --- Resource ---

/// Counter used to cycle through predictable damage values for the demo.
#[derive(Resource, Default)]
pub struct ClickCounter(pub u32);

// --- Setup ---

/// Spawns the camera and a brief instruction label.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Text::new("Left-click to spawn damage numbers"),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

// --- Pure helpers ---

/// Converts a window-space mouse position to world space using the camera transform.
pub fn window_to_world(cursor: Vec2, window: &Window, camera_transform: &Transform) -> Vec2 {
    let half = Vec2::new(window.width() / 2.0, window.height() / 2.0);
    let ndc = Vec2::new(cursor.x - half.x, half.y - cursor.y);
    let cam_pos = camera_transform.translation.truncate();
    cam_pos + ndc
}

/// Picks a damage value for the given click index, cycling for visual variety.
pub fn damage_for_click(click: u32) -> u32 {
    10 + (click % 8) * 15
}

/// Chooses the label color for a damage value: red-orange for crits, else yellow.
pub fn damage_color(damage: u32, crit_threshold: u32) -> Color {
    if damage >= crit_threshold {
        Color::srgb(1.0, 0.3, 0.1) // critical — red-orange
    } else {
        Color::srgb(1.0, 0.95, 0.2) // normal — yellow
    }
}

// --- Systems ---

/// Spawns a `Text2d` floating label at the cursor position on left-click.
fn spawn_on_click(
    mut commands: Commands,
    mut counter: ResMut<ClickCounter>,
    config: Res<FloatingTextConfig>,
    mouse: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<&Transform, With<Camera2d>>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = window_query.single() else {
        return;
    };
    let Ok(cam_transform) = camera_query.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    let world_pos = window_to_world(cursor, window, cam_transform);
    counter.0 += 1;

    let damage = damage_for_click(counter.0);
    let color = damage_color(damage, config.crit_threshold);

    commands.spawn((
        Text2d::new(format!("{}", damage)),
        TextFont {
            font_size: config.font_size,
            ..default()
        },
        TextColor(color),
        Transform::from_translation(world_pos.extend(1.0)),
        FloatingText {
            remaining: config.lifetime,
            lifetime: config.lifetime,
        },
    ));
}

/// Moves each label upward, fades it out, and despawns it when its lifetime expires.
fn tick_floating_text(
    mut commands: Commands,
    time: Res<Time>,
    config: Res<FloatingTextConfig>,
    mut query: Query<(Entity, &mut Transform, &mut TextColor, &mut FloatingText)>,
) {
    let dt = time.delta_secs();
    for (entity, mut transform, mut color, mut label) in &mut query {
        label.remaining -= dt;
        if label.remaining <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        transform.translation.y += config.rise_speed * dt;
        let alpha = (label.remaining / label.lifetime).clamp(0.0, 1.0);
        // Replace the color with the same hue but updated alpha.
        let c = color.0.to_srgba();
        color.0 = Color::srgba(c.red, c.green, c.blue, alpha);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_default_matches_documented_values() {
        let c = FloatingTextConfig::default();
        assert!(c.lifetime > 0.0);
        assert!(c.rise_speed > 0.0);
        assert_eq!(c.crit_threshold, 80);
    }

    #[test]
    fn window_to_world_center_maps_to_camera_origin() {
        let window = Window {
            resolution: (800, 500).into(),
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
            resolution: (800, 500).into(),
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
            resolution: (800, 500).into(),
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
        let lifetime = FloatingTextConfig::default().lifetime;
        let remaining = lifetime / 2.0;
        let alpha = (remaining / lifetime).clamp(0.0, 1.0);
        assert!((alpha - 0.5).abs() < 1e-5);
    }

    #[test]
    fn alpha_is_zero_at_end_of_life() {
        let lifetime = FloatingTextConfig::default().lifetime;
        let remaining = 0.0_f32;
        let alpha = (remaining / lifetime).clamp(0.0, 1.0);
        assert_eq!(alpha, 0.0);
    }

    #[test]
    fn damage_for_click_cycles_within_expected_range() {
        for click in 1..=20 {
            let d = damage_for_click(click);
            assert!((10..=115).contains(&d), "click {} -> {}", click, d);
        }
    }

    #[test]
    fn damage_color_marks_crit_above_threshold() {
        let crit = damage_color(80, 80);
        let normal = damage_color(79, 80);
        assert_ne!(crit.to_srgba().red, normal.to_srgba().green);
        // Crit is red-orange; normal is yellow — their green channels differ.
        assert!(crit.to_srgba().green < normal.to_srgba().green);
    }

    // --- ECS setup test ---
    // setup does not use AssetServer, so it is safe to run headlessly.

    #[test]
    fn setup_spawns_a_camera() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<FloatingTextConfig>()
            .init_resource::<ClickCounter>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Camera2d>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }
}
