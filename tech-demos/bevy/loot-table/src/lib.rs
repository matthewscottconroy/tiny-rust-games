//! Loot table — a reusable Bevy plugin for weighted random item drops.
//!
//! This crate is a *building block*: drop [`LootTablePlugin`] into any Bevy app
//! with `app.add_plugins(LootTablePlugin)` and it spawns clickable enemies that
//! drop weighted random loot into an accumulating inventory — no `rand` crate.
//!
//! Key ideas:
//! - A loot table is a slice of `(Item, weight)` pairs.
//! - [`total_weight`] sums all weights; [`roll_loot`] picks an item whose
//!   cumulative bucket contains `roll % total_weight`.
//! - A deterministic LCG ([`lcg_next`]) replaces `rand`, keeping the demo
//!   dependency-free while still producing varied drops.
//! - Enemies are killed by clicking on them; each kill advances the RNG state
//!   and selects a drop.  Drops float upward briefly, then despawn.
//! - The inventory accumulates in a [`Resource`] and is rendered in a label.
//!
//! Tune the drop weights, RNG seed, and drop animation through the
//! [`LootTableConfig`] resource without editing the plugin's internals.
//!
//! **Controls:** left-click a red square to kill the enemy and collect a drop.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use loot_table::LootTablePlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(LootTablePlugin)
//!     .run();
//! ```

use bevy::prelude::*;
use std::collections::HashMap;

/// Bundles every system and resource for the loot-table feature.
///
/// Add it with `app.add_plugins(LootTablePlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct LootTablePlugin;

impl Plugin for LootTablePlugin {
    fn build(&self, app: &mut App) {
        let config = LootTableConfig::default();
        app.insert_resource(RngState(config.rng_seed))
            .insert_resource(config)
            .init_resource::<Inventory>()
            .add_systems(Startup, setup)
            .add_systems(Update, (click_enemy, tick_drops, refresh_inventory));
    }
}

// --- Pure loot model ---

/// One item type that can appear in the loot pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Item {
    /// Currency; the most common drop.
    Gold,
    /// A consumable.
    Potion,
    /// A weapon.
    Sword,
    /// Defensive gear.
    Shield,
    /// The rarest drop.
    Gem,
}

impl Item {
    /// Short display label used in the inventory HUD.
    pub fn label(self) -> &'static str {
        match self {
            Item::Gold => "Gold",
            Item::Potion => "Potion",
            Item::Sword => "Sword",
            Item::Shield => "Shield",
            Item::Gem => "Gem",
        }
    }

    /// Colour for the floating drop text.
    pub fn color(self) -> Color {
        match self {
            Item::Gold => Color::srgb(1.0, 0.85, 0.15),
            Item::Potion => Color::srgb(0.3, 0.9, 0.45),
            Item::Sword => Color::srgb(0.7, 0.75, 0.9),
            Item::Shield => Color::srgb(0.45, 0.6, 1.0),
            Item::Gem => Color::srgb(0.9, 0.3, 0.8),
        }
    }
}

/// Returns the sum of all weights in `table`.
pub fn total_weight(table: &[(Item, u32)]) -> u32 {
    table.iter().map(|(_, w)| *w).sum()
}

/// Selects an item from `table` using `roll` as a uniform random value.
///
/// Returns `None` for an empty or zero-weight table.
/// `roll` is taken modulo `total_weight` so any `u32` is valid input.
pub fn roll_loot(table: &[(Item, u32)], roll: u32) -> Option<&Item> {
    let total = total_weight(table);
    if total == 0 {
        return None;
    }
    let r = roll % total;
    let mut acc = 0u32;
    for (item, weight) in table {
        acc += weight;
        if r < acc {
            return Some(item);
        }
    }
    None
}

/// Linear congruential generator — advances the RNG state one step.
///
/// Uses the same multiplier/increment as Knuth's MMIX LCG.
pub fn lcg_next(state: u64) -> u64 {
    state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407)
}

// --- Loot table ---

/// The default weighted loot pool.
pub const TABLE: &[(Item, u32)] = &[
    (Item::Gold, 50),
    (Item::Potion, 25),
    (Item::Sword, 12),
    (Item::Shield, 10),
    (Item::Gem, 3),
];

// --- Configuration ---

/// Tunable parameters for the feature. Override before adding the plugin.
///
/// Holds a `Vec` so it is `Clone` but not `Copy`.
#[derive(Resource, Clone, Debug)]
pub struct LootTableConfig {
    /// The weighted loot pool rolled on each kill.
    pub table: Vec<(Item, u32)>,
    /// Seed for the deterministic LCG.
    pub rng_seed: u64,
    /// Half the side length (in pixels) of an enemy square (its hit radius).
    pub enemy_half_size: f32,
    /// Upward pixels-per-second a drop label floats.
    pub drop_rise_speed: f32,
    /// Seconds a drop label lives before despawning.
    pub drop_lifetime: f32,
}

impl Default for LootTableConfig {
    fn default() -> Self {
        Self {
            table: TABLE.to_vec(),
            rng_seed: 0xDEAD_BEEF_u64,
            enemy_half_size: 23.0,
            drop_rise_speed: 38.0,
            drop_lifetime: 1.4,
        }
    }
}

// --- ECS ---

/// Marks a clickable enemy entity.
#[derive(Component)]
pub struct Enemy {
    /// Half the side length of the clickable square.
    pub half_size: f32,
}

/// A floating drop label that rises and fades before despawning.
#[derive(Component)]
pub struct DropLabel {
    /// Seconds since the label spawned.
    pub elapsed: f32,
}

/// Global LCG state used to generate drops.
#[derive(Resource)]
pub struct RngState(pub u64);

/// Accumulated item counts for the current session.
#[derive(Resource, Default)]
pub struct Inventory(pub HashMap<Item, u32>);

/// Marks the inventory HUD text entity.
#[derive(Component)]
pub struct InventoryLabel;

/// Spawns the camera, enemy grid, inventory label, and key-hint label.
fn setup(mut commands: Commands, config: Res<LootTableConfig>) {
    commands.spawn(Camera2d);

    let side = config.enemy_half_size * 2.0;
    let positions: &[(f32, f32)] = &[
        (-220.0, 90.0),
        (-80.0, 90.0),
        (80.0, 90.0),
        (220.0, 90.0),
        (-150.0, -30.0),
        (0.0, -30.0),
        (150.0, -30.0),
    ];
    for &(x, y) in positions {
        commands.spawn((
            Sprite {
                color: Color::srgb(0.85, 0.22, 0.22),
                custom_size: Some(Vec2::splat(side)),
                ..default()
            },
            Transform::from_xyz(x, y, 0.0),
            Enemy {
                half_size: config.enemy_half_size,
            },
        ));
    }

    commands.spawn((
        Text::new("Inventory: (nothing yet)"),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::srgb(0.85, 0.85, 0.85)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        InventoryLabel,
    ));

    commands.spawn((
        Text::new("Left-click a red square to kill an enemy and collect loot"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.5, 0.5, 0.5)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

/// On left-click, tests each enemy for a hit; kills the first hit and drops loot.
fn click_enemy(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window>,
    cam_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    enemy_query: Query<(Entity, &Transform, &Enemy)>,
    config: Res<LootTableConfig>,
    mut rng: ResMut<RngState>,
    mut inventory: ResMut<Inventory>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = window_query.single() else {
        return;
    };
    let Ok((cam, cam_tf)) = cam_query.single() else {
        return;
    };
    let Some(cursor) = window
        .cursor_position()
        .and_then(|c| cam.viewport_to_world_2d(cam_tf, c).ok())
    else {
        return;
    };

    for (entity, tf, enemy) in &enemy_query {
        let pos = tf.translation.truncate();
        if (cursor - pos).abs().max_element() < enemy.half_size {
            commands.entity(entity).despawn();
            rng.0 = lcg_next(rng.0);
            if let Some(&item) = roll_loot(&config.table, rng.0 as u32) {
                *inventory.0.entry(item).or_insert(0) += 1;
                let drop_pos = tf.translation + Vec3::Y * 36.0;
                commands.spawn((
                    Text::new(format!("+{}", item.label())),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(item.color()),
                    Transform::from_translation(drop_pos),
                    DropLabel { elapsed: 0.0 },
                ));
            }
            break;
        }
    }
}

/// Floats drop labels upward and removes them after their lifetime elapses.
fn tick_drops(
    time: Res<Time>,
    config: Res<LootTableConfig>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut DropLabel, &mut Transform)>,
) {
    for (entity, mut drop, mut tf) in &mut query {
        drop.elapsed += time.delta_secs();
        tf.translation.y += config.drop_rise_speed * time.delta_secs();
        if drop.elapsed > config.drop_lifetime {
            commands.entity(entity).despawn();
        }
    }
}

/// Rewrites the inventory HUD when the [`Inventory`] resource changes.
fn refresh_inventory(inventory: Res<Inventory>, mut query: Query<&mut Text, With<InventoryLabel>>) {
    if !inventory.is_changed() {
        return;
    }
    let Ok(mut text) = query.single_mut() else {
        return;
    };
    if inventory.0.is_empty() {
        *text = Text::new("Inventory: (nothing yet)");
    } else {
        let mut parts: Vec<String> = inventory
            .0
            .iter()
            .map(|(item, count)| format!("{} x{}", item.label(), count))
            .collect();
        parts.sort();
        *text = Text::new(format!("Inventory: {}", parts.join("  |  ")));
    }
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_weight_matches_expected() {
        assert_eq!(total_weight(TABLE), 100);
    }

    #[test]
    fn roll_loot_zero_is_gold() {
        assert_eq!(roll_loot(TABLE, 0), Some(&Item::Gold));
    }

    #[test]
    fn roll_loot_49_is_still_gold() {
        assert_eq!(roll_loot(TABLE, 49), Some(&Item::Gold));
    }

    #[test]
    fn roll_loot_50_is_potion() {
        assert_eq!(roll_loot(TABLE, 50), Some(&Item::Potion));
    }

    #[test]
    fn roll_loot_99_is_gem() {
        assert_eq!(roll_loot(TABLE, 99), Some(&Item::Gem));
    }

    #[test]
    fn roll_loot_wraps_at_total() {
        assert_eq!(roll_loot(TABLE, 100), roll_loot(TABLE, 0));
    }

    #[test]
    fn roll_loot_empty_table_is_none() {
        assert_eq!(roll_loot(&[], 7), None);
    }

    #[test]
    fn lcg_next_advances_state() {
        let s0 = 1u64;
        assert_ne!(lcg_next(s0), s0);
    }

    #[test]
    fn lcg_produces_variety_over_100_iterations() {
        let mut s = 42u64;
        let mut seen = std::collections::HashSet::new();
        for _ in 0..100 {
            s = lcg_next(s);
            seen.insert(s % 100);
        }
        assert!(seen.len() > 10, "LCG should cover many buckets");
    }

    #[test]
    fn all_table_weights_are_positive() {
        for (_, w) in TABLE {
            assert!(*w > 0);
        }
    }

    #[test]
    fn config_default_matches_documented_values() {
        let c = LootTableConfig::default();
        assert_eq!(c.table, TABLE.to_vec());
        assert_eq!(c.rng_seed, 0xDEAD_BEEF_u64);
        assert_eq!(c.enemy_half_size, 23.0);
        assert_eq!(c.drop_lifetime, 1.4);
    }
}
