//! Template demo — the canonical shape every demo in this workspace follows.
//!
//! Copy this crate to start a new demo. It is a *building block*: the reusable
//! feature lives here as [`TemplatePlugin`], tuned by [`TemplateConfig`], and
//! [`main.rs`](../src/main.rs) is just a thin runner that boots the engine and
//! adds the plugin.
//!
//! See `DEMO_ANATOMY.md` for the conventions this file demonstrates:
//! 1. all app wiring lives in the plugin, never in `main`;
//! 2. `main` owns `DefaultPlugins` / window setup;
//! 3. the plugin, config, and queryable components are `pub`;
//! 4. tunables live in a `Config` resource with a `Default`;
//! 5. self-contained logic is a `pub fn` so it is unit-testable.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use template::TemplatePlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(TemplatePlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Registers everything the feature needs. Add with
/// `app.add_plugins(TemplatePlugin)`. Never adds `DefaultPlugins`.
pub struct TemplatePlugin;

impl Plugin for TemplatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TemplateConfig>()
            .add_systems(Startup, setup)
            .add_systems(Update, move_player);
    }
}

/// Tunables the host can override before adding the plugin, e.g.
/// `app.insert_resource(TemplateConfig { speed: 400.0 })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct TemplateConfig {
    /// Player movement speed in pixels per second.
    pub speed: f32,
}

impl Default for TemplateConfig {
    fn default() -> Self {
        Self { speed: 200.0 }
    }
}

/// Marks the player-controlled sprite. Public so a host app can query it.
#[derive(Component)]
pub struct Player;

/// Reads WASD/arrow keys into a normalized direction vector.
///
/// Kept as a `pub fn` so the movement rule is unit-testable without a World.
pub fn input_direction(input: &ButtonInput<KeyCode>) -> Vec2 {
    let mut dir = Vec2::ZERO;
    if input.pressed(KeyCode::KeyW) || input.pressed(KeyCode::ArrowUp) {
        dir.y += 1.0;
    }
    if input.pressed(KeyCode::KeyS) || input.pressed(KeyCode::ArrowDown) {
        dir.y -= 1.0;
    }
    if input.pressed(KeyCode::KeyA) || input.pressed(KeyCode::ArrowLeft) {
        dir.x -= 1.0;
    }
    if input.pressed(KeyCode::KeyD) || input.pressed(KeyCode::ArrowRight) {
        dir.x += 1.0;
    }
    dir.normalize_or_zero()
}

/// Advances a position by a direction at a given speed over `dt` seconds.
pub fn advance(pos: Vec3, dir: Vec2, speed: f32, dt: f32) -> Vec3 {
    pos + Vec3::new(dir.x, dir.y, 0.0) * speed * dt
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        Sprite {
            color: Color::srgb(0.3, 0.7, 0.9),
            custom_size: Some(Vec2::splat(48.0)),
            ..default()
        },
        Transform::default(),
        Player,
    ));
}

fn move_player(
    input: Res<ButtonInput<KeyCode>>,
    config: Res<TemplateConfig>,
    time: Res<Time>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    let dir = input_direction(&input);
    for mut transform in &mut query {
        transform.translation =
            advance(transform.translation, dir, config.speed, time.delta_secs());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_moves_along_direction() {
        let p = advance(Vec3::ZERO, Vec2::new(1.0, 0.0), 100.0, 0.5);
        assert!((p.x - 50.0).abs() < 1e-6);
        assert_eq!(p.y, 0.0);
    }

    #[test]
    fn advance_is_noop_without_direction() {
        let start = Vec3::new(5.0, 5.0, 0.0);
        assert_eq!(advance(start, Vec2::ZERO, 100.0, 0.5), start);
    }

    #[test]
    fn config_default_speed() {
        assert_eq!(TemplateConfig::default().speed, 200.0);
    }

    #[test]
    fn plugin_spawns_one_player() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TemplatePlugin));
        // The plugin's Update systems read input; MinimalPlugins omits it.
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.update();
        let mut q = app.world_mut().query::<&Player>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }
}
