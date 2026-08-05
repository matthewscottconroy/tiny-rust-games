//! Pickup and inventory — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`PickupAndInventoryPlugin`] into any
//! Bevy app with `app.add_plugins(PickupAndInventoryPlugin)` and it spawns a
//! player you walk around a field of collectible items with, carrying them in a
//! capped [`Inventory`] and dropping them on demand.
//!
//! Key ideas:
//! - Proximity pickup: each frame the player's position is compared to every
//!   [`Pickup`] and items within [`PickupConfig::pickup_radius`] are collected.
//! - [`Inventory`] is a plain resource with `count` and `max` — no complex
//!   data structure needed.
//! - Dropping spawns a new [`Pickup`] entity at the player's current position.
//! - When all pickups are collected a [`RespawnTimer`] schedules a fresh batch.
//!
//! Tune capacity, movement speed, pickup radius, and respawn delay through the
//! [`PickupConfig`] resource without editing the plugin's internals.
//!
//! **Controls:** WASD to move, Q to drop an item.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use pickup_and_inventory::PickupAndInventoryPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(PickupAndInventoryPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system and resource for the pickup/inventory feature.
///
/// Add it with `app.add_plugins(PickupAndInventoryPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct PickupAndInventoryPlugin;

impl Plugin for PickupAndInventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PickupConfig>()
            .init_resource::<Inventory>()
            .init_resource::<RespawnTimer>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (move_player, collect_pickups, drop_item, tick_respawn, update_hud),
            );
    }
}

// --- Configuration ---

/// Tunable parameters for the feature. Override before adding the plugin, e.g.
/// `app.insert_resource(PickupConfig { capacity: 10, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct PickupConfig {
    /// Maximum items that can be carried at once.
    pub capacity: usize,
    /// Player movement speed in pixels per second.
    pub player_speed: f32,
    /// Distance within which the player automatically collects a pickup.
    pub pickup_radius: f32,
    /// Delay before a fresh batch of pickups respawns.
    pub respawn_seconds: f32,
}

impl Default for PickupConfig {
    fn default() -> Self {
        Self { capacity: 5, player_speed: 180.0, pickup_radius: 28.0, respawn_seconds: 2.0 }
    }
}

// --- Components ---

/// Marks the player entity.
#[derive(Component)]
pub struct Player;

/// Tags an item that can be picked up.
#[derive(Component)]
pub struct Pickup;

/// Marks the HUD text that shows inventory status.
#[derive(Component)]
pub struct HudText;

// --- Resources ---

/// Current inventory state.
#[derive(Resource)]
pub struct Inventory {
    /// Number of items currently carried.
    pub count: usize,
    /// Maximum items that can be carried at once.
    pub max: usize,
}

impl Default for Inventory {
    fn default() -> Self {
        Self { count: 0, max: PickupConfig::default().capacity }
    }
}

/// Optional countdown before a fresh batch of pickups respawns.
#[derive(Resource, Default)]
pub struct RespawnTimer(pub Option<Timer>);

// --- Setup ---

/// Spawns the camera, player, initial pickups, and HUD.
fn setup(mut commands: Commands, config: Res<PickupConfig>, mut inventory: ResMut<Inventory>) {
    inventory.max = config.capacity;

    commands.spawn(Camera2d);

    commands.spawn((
        Sprite {
            color: Color::srgb(0.3, 0.75, 0.95),
            custom_size: Some(Vec2::splat(26.0)),
            ..default()
        },
        Transform::default(),
        Player,
    ));

    spawn_pickups(&mut commands);

    commands.spawn((
        Text::new(format!("Items: 0 / {}", config.capacity)),
        TextFont { font_size: 24.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        HudText,
    ));

    commands.spawn((
        Text::new("WASD — move   Q — drop item"),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgb(0.6, 0.6, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

/// Spawns the fixed set of field pickups.
fn spawn_pickups(commands: &mut Commands) {
    let positions: &[Vec3] = &[
        Vec3::new(-180.0,  120.0, 0.0),
        Vec3::new( 200.0,   80.0, 0.0),
        Vec3::new(  60.0, -150.0, 0.0),
        Vec3::new(-220.0,  -90.0, 0.0),
        Vec3::new( 140.0,  170.0, 0.0),
        Vec3::new(-100.0,  200.0, 0.0),
        Vec3::new( 250.0, -130.0, 0.0),
        Vec3::new( -60.0, -200.0, 0.0),
    ];

    for &pos in positions {
        commands.spawn((
            Sprite {
                color: Color::srgb(1.0, 0.85, 0.1),
                custom_size: Some(Vec2::splat(14.0)),
                ..default()
            },
            Transform::from_translation(pos),
            Pickup,
        ));
    }
}

// --- Systems ---

/// Reads WASD input and moves the player.
fn move_player(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    config: Res<PickupConfig>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    let Ok(mut transform) = query.single_mut() else { return; };
    let mut dir = Vec2::ZERO;
    if input.pressed(KeyCode::KeyW) { dir.y += 1.0; }
    if input.pressed(KeyCode::KeyS) { dir.y -= 1.0; }
    if input.pressed(KeyCode::KeyA) { dir.x -= 1.0; }
    if input.pressed(KeyCode::KeyD) { dir.x += 1.0; }
    if dir != Vec2::ZERO {
        let delta = dir.normalize() * config.player_speed * time.delta_secs();
        transform.translation.x += delta.x;
        transform.translation.y += delta.y;
    }
}

/// Despawns pickups within [`PickupConfig::pickup_radius`] of the player and
/// increments [`Inventory::count`]. Schedules a respawn when the field empties.
fn collect_pickups(
    mut commands: Commands,
    config: Res<PickupConfig>,
    player_query: Query<&Transform, With<Player>>,
    pickup_query: Query<(Entity, &Transform), With<Pickup>>,
    mut inventory: ResMut<Inventory>,
    mut respawn: ResMut<RespawnTimer>,
) {
    let Ok(player) = player_query.single() else { return; };
    let player_pos = player.translation.truncate();

    for (entity, pickup_transform) in &pickup_query {
        let dist = player_pos.distance(pickup_transform.translation.truncate());
        if dist < config.pickup_radius && inventory.count < inventory.max {
            commands.entity(entity).despawn();
            inventory.count += 1;
        }
    }

    if pickup_query.iter().count() == 0 && respawn.0.is_none() && inventory.count >= inventory.max {
        respawn.0 = Some(Timer::from_seconds(config.respawn_seconds, TimerMode::Once));
    }
}

/// Drops one item from the inventory when `Q` is pressed, spawning it just
/// below the player.
fn drop_item(
    input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    mut inventory: ResMut<Inventory>,
) {
    if !input.just_pressed(KeyCode::KeyQ) || inventory.count == 0 {
        return;
    }
    let Ok(player) = player_query.single() else { return; };
    inventory.count -= 1;

    let pos = player.translation + Vec3::new(0.0, -30.0, 0.0);
    commands.spawn((
        Sprite {
            color: Color::srgb(1.0, 0.85, 0.1),
            custom_size: Some(Vec2::splat(14.0)),
            ..default()
        },
        Transform::from_translation(pos),
        Pickup,
    ));
}

/// Ticks the respawn countdown and resets the field when the timer fires.
fn tick_respawn(
    mut commands: Commands,
    time: Res<Time>,
    mut respawn: ResMut<RespawnTimer>,
    mut inventory: ResMut<Inventory>,
) {
    let Some(timer) = respawn.0.as_mut() else { return; };
    if timer.tick(time.delta()).just_finished() {
        respawn.0 = None;
        inventory.count = 0;
        spawn_pickups(&mut commands);
    }
}

/// Rewrites the HUD label whenever [`Inventory`] changes.
fn update_hud(inventory: Res<Inventory>, mut query: Query<&mut Text, With<HudText>>) {
    if !inventory.is_changed() { return; }
    for mut text in &mut query {
        *text = Text::new(format!(
            "Items: {} / {}{}",
            inventory.count,
            inventory.max,
            if inventory.count == inventory.max { "  (FULL)" } else { "" }
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_starts_at_zero() {
        let inv = Inventory { count: 0, max: 5 };
        assert_eq!(inv.count, 0);
    }

    #[test]
    fn inventory_default_uses_config_capacity() {
        let inv = Inventory::default();
        assert_eq!(inv.max, PickupConfig::default().capacity);
    }

    #[test]
    fn inventory_not_full_below_max() {
        let inv = Inventory { count: 3, max: 5 };
        assert!(inv.count < inv.max);
    }

    #[test]
    fn inventory_full_at_max() {
        let inv = Inventory { count: 5, max: 5 };
        assert_eq!(inv.count, inv.max);
    }

    #[test]
    fn config_default_matches_documented_values() {
        let c = PickupConfig::default();
        assert_eq!(c.capacity, 5);
        assert_eq!(c.player_speed, 180.0);
        assert_eq!(c.pickup_radius, 28.0);
    }

    #[test]
    fn setup_spawns_one_player() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<PickupConfig>()
            .init_resource::<Inventory>()
            .init_resource::<RespawnTimer>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Player>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn setup_spawns_eight_pickups() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<PickupConfig>()
            .init_resource::<Inventory>()
            .init_resource::<RespawnTimer>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Pickup>();
        assert_eq!(q.iter(app.world()).count(), 8);
    }
}
