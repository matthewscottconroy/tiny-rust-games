//! Y-sort (depth sorting) — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`YSortPlugin`] into any Bevy app with
//! `app.add_plugins(YSortPlugin)` and it renders a top-down scene where sprites
//! lower on screen draw in front. Tune it through the [`YSortConfig`] resource
//! without editing the plugin's internals.
//!
//! Key idea: for top-down or isometric views, sprites lower on screen are
//! visually "closer" and should render in front. Setting `Z = -Y * scale`
//! each frame achieves this with a single line of math — see [`y_to_z`].
//!
//! Move the player over the wandering characters to see them pop in front of
//! or behind the player based on their relative vertical positions.
//!
//! **Controls:** WASD — move
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use y_sort::YSortPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(YSortPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system and resource for the y-sort feature.
///
/// Add it with `app.add_plugins(YSortPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct YSortPlugin;

impl Plugin for YSortPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<YSortConfig>()
            .add_systems(Startup, setup)
            .add_systems(Update, (move_player, wander_characters, y_sort, update_hud));
    }
}

// --- Configuration ---

/// Tunable parameters for the feature. Override before adding the plugin, e.g.
/// `app.insert_resource(YSortConfig { z_scale: 0.002, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct YSortConfig {
    /// Player movement speed in pixels per second.
    pub player_speed: f32,
    /// Factor mapping world-space Y to Z depth (`z = -y * z_scale`).
    pub z_scale: f32,
    /// Half-width of the wander arena; characters bounce past `±wander_bound_x`.
    pub wander_bound_x: f32,
    /// Half-height of the wander arena; characters bounce past `±wander_bound_y`.
    pub wander_bound_y: f32,
}

impl Default for YSortConfig {
    fn default() -> Self {
        Self {
            player_speed: 160.0,
            z_scale: 0.001,
            wander_bound_x: 350.0,
            wander_bound_y: 240.0,
        }
    }
}

// --- Components ---

/// Marks the player entity.
#[derive(Component)]
pub struct Player;

/// A wandering non-player character with an autonomous velocity.
#[derive(Component)]
pub struct Character {
    /// Current velocity in pixels per second.
    pub velocity: Vec2,
}

/// Tags an entity whose Z coordinate should be derived from its Y position.
#[derive(Component)]
pub struct YSorted;

/// Marks the position / depth display text.
#[derive(Component)]
pub struct HudText;

// --- Setup ---

/// Spawns a checkerboard ground, four wandering characters, the player, and HUD.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    for row in -4..=4 {
        for col in -6..=6 {
            let shade = if (row + col) % 2 == 0 { 0.12 } else { 0.16 };
            commands.spawn((
                Sprite {
                    color: Color::srgb(shade, shade * 1.2, shade),
                    custom_size: Some(Vec2::splat(72.0)),
                    ..default()
                },
                Transform::from_xyz(col as f32 * 72.0, row as f32 * 72.0, -100.0),
            ));
        }
    }

    let chars: &[(Vec3, Vec2, Color)] = &[
        (
            Vec3::new(-150.0, 60.0, 0.0),
            Vec2::new(55.0, 30.0),
            Color::srgb(0.85, 0.35, 0.2),
        ),
        (
            Vec3::new(80.0, -40.0, 0.0),
            Vec2::new(-40.0, 65.0),
            Color::srgb(0.2, 0.6, 0.9),
        ),
        (
            Vec3::new(180.0, 100.0, 0.0),
            Vec2::new(-70.0, -45.0),
            Color::srgb(0.8, 0.75, 0.1),
        ),
        (
            Vec3::new(-200.0, -100.0, 0.0),
            Vec2::new(60.0, -55.0),
            Color::srgb(0.5, 0.85, 0.5),
        ),
    ];

    for &(pos, vel, color) in chars {
        commands.spawn((
            Sprite {
                color,
                custom_size: Some(Vec2::new(22.0, 32.0)),
                ..default()
            },
            Transform::from_translation(pos),
            Character { velocity: vel },
            YSorted,
        ));
    }

    commands.spawn((
        Sprite {
            color: Color::srgb(0.95, 0.95, 0.95),
            custom_size: Some(Vec2::new(22.0, 32.0)),
            ..default()
        },
        Transform::default(),
        Player,
        YSorted,
    ));

    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.8, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        HudText,
    ));

    commands.spawn((
        Text::new("WASD - move   walk behind/in-front of colored characters"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.55, 0.55, 0.55)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

// --- Pure helper ---

/// Converts a world-space Y coordinate to a Z depth for Y-sorting.
///
/// Lower Y (further down on screen) yields a higher Z (rendered in front),
/// matching the top-down perspective convention that objects at the bottom of
/// the screen appear closest to the viewer. `scale` is the [`YSortConfig::z_scale`]
/// tunable.
pub fn y_to_z(y: f32, scale: f32) -> f32 {
    -y * scale
}

// --- Systems ---

/// Reads WASD input and moves the player.
fn move_player(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    config: Res<YSortConfig>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    let Ok(mut t) = query.single_mut() else {
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
    if dir != Vec2::ZERO {
        let d = dir.normalize() * config.player_speed * time.delta_secs();
        t.translation.x += d.x;
        t.translation.y += d.y;
    }
}

/// Moves each character along its velocity and bounces it at arena edges.
fn wander_characters(
    time: Res<Time>,
    config: Res<YSortConfig>,
    mut query: Query<(&mut Transform, &mut Character)>,
) {
    for (mut transform, mut ch) in &mut query {
        transform.translation.x += ch.velocity.x * time.delta_secs();
        transform.translation.y += ch.velocity.y * time.delta_secs();
        if transform.translation.x.abs() > config.wander_bound_x {
            ch.velocity.x *= -1.0;
        }
        if transform.translation.y.abs() > config.wander_bound_y {
            ch.velocity.y *= -1.0;
        }
    }
}

/// Overwrites the Z coordinate of every [`YSorted`] entity using [`y_to_z`].
fn y_sort(config: Res<YSortConfig>, mut query: Query<&mut Transform, With<YSorted>>) {
    for mut transform in &mut query {
        transform.translation.z = y_to_z(transform.translation.y, config.z_scale);
    }
}

/// Updates the HUD with the player's current Y and derived Z.
fn update_hud(
    player_query: Query<&Transform, With<Player>>,
    mut hud_query: Query<&mut Text, With<HudText>>,
) {
    let Ok(t) = player_query.single() else {
        return;
    };
    for mut text in &mut hud_query {
        *text = Text::new(format!(
            "Player Y={:.0}  Z={:.4} (higher Z = in front)",
            t.translation.y, t.translation.z,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scale() -> f32 {
        YSortConfig::default().z_scale
    }

    // --- y_to_z ---

    #[test]
    fn y_to_z_negative_y_gives_positive_z() {
        assert!((y_to_z(-100.0, scale()) - 0.1).abs() < 1e-6);
    }

    #[test]
    fn y_to_z_positive_y_gives_negative_z() {
        assert!((y_to_z(100.0, scale()) + 0.1).abs() < 1e-6);
    }

    #[test]
    fn y_to_z_zero_gives_zero() {
        assert_eq!(y_to_z(0.0, scale()), 0.0);
    }

    #[test]
    fn lower_y_has_higher_z() {
        assert!(
            y_to_z(-200.0, scale()) > y_to_z(200.0, scale()),
            "sprite lower on screen should have higher Z"
        );
    }

    #[test]
    fn y_to_z_is_linear() {
        let ratio = y_to_z(50.0, scale()) / y_to_z(25.0, scale());
        assert!((ratio - 2.0).abs() < 1e-5);
    }

    #[test]
    fn config_default_matches_documented_values() {
        let c = YSortConfig::default();
        assert_eq!(c.player_speed, 160.0);
        assert_eq!(c.z_scale, 0.001);
    }

    // --- ECS ---

    #[test]
    fn setup_spawns_one_player() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Player>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn setup_spawns_four_characters() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Character>();
        assert_eq!(q.iter(app.world()).count(), 4);
    }

    #[test]
    fn setup_spawns_five_y_sorted_entities() {
        // 4 characters + 1 player
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&YSorted>();
        assert_eq!(q.iter(app.world()).count(), 5);
    }
}
