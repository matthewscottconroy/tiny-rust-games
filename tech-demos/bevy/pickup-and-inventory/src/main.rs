//! Pickup and inventory demo.
//!
//! Key ideas:
//! - Proximity pickup: each frame the player's position is compared to every
//!   [`Pickup`] and items within [`PICKUP_RADIUS`] are collected.
//! - [`Inventory`] is a plain resource with `count` and `max` — no complex
//!   data structure needed.
//! - Dropping spawns a new [`Pickup`] entity at the player's current position.
//! - When all pickups are collected a [`RespawnTimer`] schedules a fresh batch.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Inventory { count: 0, max: 5 })
        .insert_resource(RespawnTimer(None))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (move_player, collect_pickups, drop_item, tick_respawn, update_hud),
        )
        .run();
}

// --- Components ---

/// Marks the player entity.
#[derive(Component)]
struct Player;

/// Tags an item that can be picked up.
#[derive(Component)]
struct Pickup;

/// Marks the HUD text that shows inventory status.
#[derive(Component)]
struct HudText;

// --- Resources ---

/// Current inventory state.
#[derive(Resource)]
struct Inventory {
    /// Number of items currently carried.
    count: usize,
    /// Maximum items that can be carried at once.
    max: usize,
}

/// Optional countdown before a fresh batch of pickups respawns.
#[derive(Resource)]
struct RespawnTimer(Option<Timer>);

/// Distance within which the player automatically collects a pickup.
const PICKUP_RADIUS: f32 = 28.0;

const PLAYER_SPEED: f32 = 180.0;

// --- Setup ---

/// Spawns the camera, player, initial pickups, and HUD.
fn setup(mut commands: Commands) {
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
        Text::new("Items: 0 / 5"),
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
    mut query: Query<&mut Transform, With<Player>>,
) {
    let Ok(mut transform) = query.single_mut() else { return; };
    let mut dir = Vec2::ZERO;
    if input.pressed(KeyCode::KeyW) { dir.y += 1.0; }
    if input.pressed(KeyCode::KeyS) { dir.y -= 1.0; }
    if input.pressed(KeyCode::KeyA) { dir.x -= 1.0; }
    if input.pressed(KeyCode::KeyD) { dir.x += 1.0; }
    if dir != Vec2::ZERO {
        let delta = dir.normalize() * PLAYER_SPEED * time.delta_secs();
        transform.translation.x += delta.x;
        transform.translation.y += delta.y;
    }
}

/// Despawns pickups within [`PICKUP_RADIUS`] of the player and increments
/// [`Inventory::count`].  Schedules a respawn when the field empties.
fn collect_pickups(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    pickup_query: Query<(Entity, &Transform), With<Pickup>>,
    mut inventory: ResMut<Inventory>,
    mut respawn: ResMut<RespawnTimer>,
) {
    let Ok(player) = player_query.single() else { return; };
    let player_pos = player.translation.truncate();

    for (entity, pickup_transform) in &pickup_query {
        let dist = player_pos.distance(pickup_transform.translation.truncate());
        if dist < PICKUP_RADIUS && inventory.count < inventory.max {
            commands.entity(entity).despawn();
            inventory.count += 1;
        }
    }

    if pickup_query.iter().count() == 0 && respawn.0.is_none() && inventory.count >= inventory.max {
        respawn.0 = Some(Timer::from_seconds(2.0, TimerMode::Once));
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
    fn setup_spawns_one_player() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(Inventory { count: 0, max: 5 })
            .insert_resource(RespawnTimer(None))
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Player>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn setup_spawns_eight_pickups() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(Inventory { count: 0, max: 5 })
            .insert_resource(RespawnTimer(None))
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Pickup>();
        assert_eq!(q.iter(app.world()).count(), 8);
    }
}
