//! Projectiles — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`ProjectilesPlugin`] into any Bevy
//! app with `app.add_plugins(ProjectilesPlugin)` and it manages a player that
//! moves with WASD and fires [`Bullet`] entities with the arrow keys.
//!
//! Key ideas:
//! - Bullets are spawned at runtime as plain ECS entities; no object pool needed.
//! - Each bullet carries a [`Velocity`] component and moves every `Update` frame.
//! - Off-screen bullets are despawned each frame to keep entity count bounded.
//! - Tunables live in [`ProjectilesConfig`]; override it before adding the plugin.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use projectiles::ProjectilesPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(ProjectilesPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system and resource for the projectiles feature.
///
/// Add it with `app.add_plugins(ProjectilesPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct ProjectilesPlugin;

impl Plugin for ProjectilesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProjectilesConfig>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (move_player, fire_bullets, move_bullets, despawn_offscreen),
            );
    }
}

// --- Configuration ---

/// Tunable parameters for the feature. Override before adding the plugin, e.g.
/// `app.insert_resource(ProjectilesConfig { bullet_speed: 800.0, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct ProjectilesConfig {
    /// Player movement speed in world units/second.
    pub player_speed: f32,
    /// Bullet travel speed in world units/second.
    pub bullet_speed: f32,
    /// Bullets are despawned once their position exceeds this distance from origin.
    pub offscreen_margin: f32,
}

impl Default for ProjectilesConfig {
    fn default() -> Self {
        Self {
            player_speed: 200.0,
            bullet_speed: 500.0,
            offscreen_margin: 600.0,
        }
    }
}

// --- Components ---

/// Player state — tracks the last movement direction so bullets inherit it.
#[derive(Component)]
pub struct Player {
    /// Unit vector of the player's last movement direction.
    pub facing: Vec2,
}

/// Tags a bullet entity.
#[derive(Component)]
pub struct Bullet;

/// 2D linear velocity in world units/second.
#[derive(Component)]
pub struct Velocity(pub Vec2);

// --- Pure helpers ---

/// Returns `true` when a 2D position lies outside the square arena defined by `margin`.
pub fn is_offscreen(pos: Vec2, margin: f32) -> bool {
    pos.x.abs() > margin || pos.y.abs() > margin
}

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
        TextFont {
            font_size: 16.0,
            ..default()
        },
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
    config: Res<ProjectilesConfig>,
    mut query: Query<(&mut Transform, &mut Player)>,
) {
    let Ok((mut transform, mut player)) = query.single_mut() else {
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
        let normalized = dir.normalize();
        player.facing = normalized;
        let delta = normalized * config.player_speed * time.delta_secs();
        transform.translation.x += delta.x;
        transform.translation.y += delta.y;
    }
}

/// Spawns a bullet in a cardinal direction when an arrow key is just pressed.
fn fire_bullets(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    config: Res<ProjectilesConfig>,
    player_query: Query<&Transform, With<Player>>,
) {
    let Ok(transform) = player_query.single() else {
        return;
    };

    let mut directions: Vec<Vec2> = Vec::new();
    if input.just_pressed(KeyCode::ArrowUp) {
        directions.push(Vec2::Y);
    }
    if input.just_pressed(KeyCode::ArrowDown) {
        directions.push(Vec2::NEG_Y);
    }
    if input.just_pressed(KeyCode::ArrowLeft) {
        directions.push(Vec2::NEG_X);
    }
    if input.just_pressed(KeyCode::ArrowRight) {
        directions.push(Vec2::X);
    }

    for dir in directions {
        commands.spawn((
            Sprite {
                color: Color::srgb(1.0, 0.85, 0.1),
                custom_size: Some(Vec2::new(6.0, 16.0)),
                ..default()
            },
            Transform::from_translation(transform.translation),
            Bullet,
            Velocity(dir * config.bullet_speed),
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

/// Despawns bullets that have travelled beyond [`ProjectilesConfig::offscreen_margin`].
fn despawn_offscreen(
    mut commands: Commands,
    config: Res<ProjectilesConfig>,
    query: Query<(Entity, &Transform), With<Bullet>>,
) {
    for (entity, transform) in &query {
        if is_offscreen(transform.translation.truncate(), config.offscreen_margin) {
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
        app.add_plugins(MinimalPlugins).add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Player>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn no_bullets_at_startup() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Bullet>();
        assert_eq!(q.iter(app.world()).count(), 0);
    }

    #[test]
    fn player_starts_at_origin() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<(&Player, &Transform)>();
        for (_, t) in q.iter(app.world()) {
            assert_eq!(t.translation, Vec3::ZERO);
        }
    }

    #[test]
    fn config_default_matches_documented_values() {
        let c = ProjectilesConfig::default();
        assert_eq!(c.player_speed, 200.0);
        assert_eq!(c.bullet_speed, 500.0);
        assert_eq!(c.offscreen_margin, 600.0);
    }

    #[test]
    fn offscreen_margin_is_positive() {
        assert!(ProjectilesConfig::default().offscreen_margin > 0.0);
    }

    #[test]
    fn is_offscreen_inside_arena_returns_false() {
        assert!(!is_offscreen(Vec2::new(100.0, -200.0), 600.0));
    }

    #[test]
    fn is_offscreen_too_far_right_returns_true() {
        assert!(is_offscreen(Vec2::new(700.0, 0.0), 600.0));
    }

    #[test]
    fn is_offscreen_too_far_up_returns_true() {
        assert!(is_offscreen(Vec2::new(0.0, 650.0), 600.0));
    }

    #[test]
    fn is_offscreen_exactly_at_margin_returns_false() {
        // boundary is exclusive (> not >=)
        assert!(!is_offscreen(Vec2::new(600.0, 0.0), 600.0));
    }

    #[test]
    fn player_default_facing_is_up() {
        let p = Player { facing: Vec2::Y };
        assert_eq!(p.facing, Vec2::Y);
    }

    #[test]
    fn bullet_speed_exceeds_player_speed() {
        let c = ProjectilesConfig::default();
        assert!(c.bullet_speed > c.player_speed);
    }
}
