//! Required components — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`RequiredComponentsPlugin`] into any
//! Bevy app with `app.add_plugins(RequiredComponentsPlugin)`. It demonstrates
//! Bevy 0.15+'s `#[require(...)]` attribute, which automatically inserts
//! required components with their `Default` values whenever a root component is
//! spawned — eliminating boilerplate bundle types.
//!
//! Three entity archetypes are defined:
//! - [`Soldier`]: requires [`Health`], [`Team`], and [`Speed`]
//! - [`Mage`]: requires [`Health`], [`ManaPool`], and [`Speed`]
//! - [`Turret`]: requires [`Health`] and [`Team`] (no `Speed` — stationary)
//!
//! Each archetype is spawned with a single component; the others arrive
//! automatically. A drain system ticks health down over time (tunable via
//! [`RequiredComponentsConfig`]), and a HUD displays live counts and average
//! health per type.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use required_components::RequiredComponentsPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(RequiredComponentsPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system and resource for the required-components demo.
///
/// Add it with `app.add_plugins(RequiredComponentsPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct RequiredComponentsPlugin;

impl Plugin for RequiredComponentsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RequiredComponentsConfig>()
            .add_systems(Startup, setup)
            .add_systems(Update, (drain_health, update_hud));
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Tunable parameters for the demo. Override before adding the plugin, e.g.
/// `app.insert_resource(RequiredComponentsConfig { drain_per_second: 10.0 })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct RequiredComponentsConfig {
    /// Hit points drained from every entity each second.
    pub drain_per_second: f32,
}

impl Default for RequiredComponentsConfig {
    fn default() -> Self {
        Self { drain_per_second: 5.0 }
    }
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Root component for soldier entities.
///
/// Spawning a `Soldier` automatically adds [`Health`], [`Team`], and [`Speed`]
/// via Bevy's required-components mechanism.
#[derive(Component, Default)]
#[require(Health, Team, Speed)]
pub struct Soldier;

/// Root component for mage entities.
///
/// Spawning a `Mage` automatically adds [`Health`], [`ManaPool`], and [`Speed`].
#[derive(Component, Default)]
#[require(Health, ManaPool, Speed)]
pub struct Mage;

/// Root component for turret entities.
///
/// Turrets are stationary, so only [`Health`] and [`Team`] are required.
#[derive(Component, Default)]
#[require(Health, Team)]
pub struct Turret;

/// Current hit points of an entity.
#[derive(Component)]
pub struct Health(pub f32);

impl Default for Health {
    fn default() -> Self {
        Self(100.0)
    }
}

/// Team identifier for an entity (0 = red, 1 = blue, etc.).
#[derive(Component)]
pub struct Team(pub u8);

impl Default for Team {
    fn default() -> Self {
        Self(0)
    }
}

/// Movement speed of an entity in units per second.
#[derive(Component)]
pub struct Speed(pub f32);

impl Default for Speed {
    fn default() -> Self {
        Self(100.0)
    }
}

/// Available mana for mage entities.
#[derive(Component)]
pub struct ManaPool(pub f32);

impl Default for ManaPool {
    fn default() -> Self {
        Self(80.0)
    }
}

/// Marker component for the HUD text entity.
#[derive(Component)]
pub struct HudText;

// ---------------------------------------------------------------------------
// Pure functions
// ---------------------------------------------------------------------------

/// Returns the arithmetic mean of a slice, or `0.0` if the slice is empty.
pub fn average(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f32>() / values.len() as f32
}

/// Returns `current / max` clamped to `[0.0, 1.0]`.
///
/// Returns `0.0` when `max` is zero to avoid division by zero.
pub fn health_fraction(current: f32, max: f32) -> f32 {
    if max <= 0.0 {
        return 0.0;
    }
    (current / max).clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Spawns the camera, initial entities, and the HUD text node.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // --- Soldiers (blue) ---
    let soldier_positions = [
        Vec3::new(-300.0, 150.0, 0.0),
        Vec3::new(-250.0, 150.0, 0.0),
        Vec3::new(-200.0, 150.0, 0.0),
    ];
    for pos in soldier_positions {
        commands.spawn((
            Soldier,
            Sprite {
                color: Color::srgb(0.2, 0.4, 0.9),
                custom_size: Some(Vec2::new(40.0, 40.0)),
                ..default()
            },
            Transform::from_translation(pos),
        ));
    }

    // --- Mages (purple) ---
    let mage_positions = [
        Vec3::new(-50.0, -50.0, 0.0),
        Vec3::new(0.0, -50.0, 0.0),
        Vec3::new(50.0, -50.0, 0.0),
    ];
    for pos in mage_positions {
        commands.spawn((
            Mage,
            Sprite {
                color: Color::srgb(0.7, 0.2, 0.9),
                custom_size: Some(Vec2::new(40.0, 40.0)),
                ..default()
            },
            Transform::from_translation(pos),
        ));
    }

    // --- Turrets (gray) ---
    let turret_positions = [
        Vec3::new(200.0, 100.0, 0.0),
        Vec3::new(270.0, 100.0, 0.0),
    ];
    for pos in turret_positions {
        commands.spawn((
            Turret,
            Sprite {
                color: Color::srgb(0.5, 0.5, 0.5),
                custom_size: Some(Vec2::new(50.0, 50.0)),
                ..default()
            },
            Transform::from_translation(pos),
        ));
    }

    // --- HUD ---
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
        HudText,
    ));
}

/// Drains health per second from every entity that has a [`Health`] component.
fn drain_health(
    mut query: Query<&mut Health>,
    time: Res<Time>,
    config: Res<RequiredComponentsConfig>,
) {
    for mut health in &mut query {
        health.0 = (health.0 - config.drain_per_second * time.delta_secs()).max(0.0);
    }
}

/// Updates the HUD text with live counts and average health for each type.
fn update_hud(
    soldiers: Query<&Health, With<Soldier>>,
    mages: Query<&Health, With<Mage>>,
    turrets: Query<&Health, With<Turret>>,
    mut hud: Query<&mut Text, With<HudText>>,
) {
    let Ok(mut text) = hud.single_mut() else {
        return;
    };

    let soldier_healths: Vec<f32> = soldiers.iter().map(|h| h.0).collect();
    let mage_healths: Vec<f32> = mages.iter().map(|h| h.0).collect();
    let turret_healths: Vec<f32> = turrets.iter().map(|h| h.0).collect();

    text.0 = format!(
        "Required Components Demo\n\
         Soldiers: {} | avg HP: {:.1}\n\
         Mages:    {} | avg HP: {:.1}\n\
         Turrets:  {} | avg HP: {:.1}\n\n\
         Spawning a Soldier automatically\n\
         adds Health, Team, and Speed.",
        soldier_healths.len(),
        average(&soldier_healths),
        mage_healths.len(),
        average(&mage_healths),
        turret_healths.len(),
        average(&turret_healths),
    );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn average_empty_slice_returns_zero() {
        assert_eq!(average(&[]), 0.0);
    }

    #[test]
    fn average_single_value() {
        assert!((average(&[42.0]) - 42.0).abs() < f32::EPSILON);
    }

    #[test]
    fn average_multiple_values() {
        let result = average(&[10.0, 20.0, 30.0]);
        assert!((result - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn health_fraction_normal() {
        let frac = health_fraction(50.0, 100.0);
        assert!((frac - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn health_fraction_clamped_above_one() {
        assert!((health_fraction(150.0, 100.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn health_fraction_zero_max() {
        assert_eq!(health_fraction(50.0, 0.0), 0.0);
    }

    #[test]
    fn health_fraction_zero_current() {
        assert_eq!(health_fraction(0.0, 100.0), 0.0);
    }

    #[test]
    fn soldier_gets_required_components() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.world_mut().spawn(Soldier);
        let count = app.world_mut().query::<&Health>().iter(app.world()).count();
        assert_eq!(count, 1);
    }

    #[test]
    fn soldier_gets_team_and_speed() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.world_mut().spawn(Soldier);
        let team_count = app.world_mut().query::<&Team>().iter(app.world()).count();
        let speed_count = app.world_mut().query::<&Speed>().iter(app.world()).count();
        assert_eq!(team_count, 1);
        assert_eq!(speed_count, 1);
    }

    #[test]
    fn mage_gets_mana_pool_but_not_team() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.world_mut().spawn(Mage);
        let mana_count = app
            .world_mut()
            .query::<&ManaPool>()
            .iter(app.world())
            .count();
        assert_eq!(mana_count, 1);
        // Mage has no Team requirement
        let team_count = app.world_mut().query::<&Team>().iter(app.world()).count();
        assert_eq!(team_count, 0);
    }

    #[test]
    fn turret_has_no_speed() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.world_mut().spawn(Turret);
        let speed_count = app.world_mut().query::<&Speed>().iter(app.world()).count();
        assert_eq!(speed_count, 0);
    }

    #[test]
    fn default_health_is_100() {
        assert!((Health::default().0 - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn default_speed_is_100() {
        assert!((Speed::default().0 - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn default_mana_pool_is_80() {
        assert!((ManaPool::default().0 - 80.0).abs() < f32::EPSILON);
    }

    #[test]
    fn config_default_drain_is_5() {
        assert!((RequiredComponentsConfig::default().drain_per_second - 5.0).abs() < f32::EPSILON);
    }
}
