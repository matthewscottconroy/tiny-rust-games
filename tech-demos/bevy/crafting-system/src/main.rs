//! Crafting System — inventory collection and recipe matching.
//!
//! Key ideas:
//! - Items are scattered around the arena as pickup entities.
//! - `can_craft` checks whether the inventory satisfies a recipe's ingredient list —
//!   a pure function over two plain slices.
//! - `consume_ingredients` removes items from an inventory `HashMap`, also pure.
//! - Walking into a pickup collects it; pressing C at a workbench opens the recipe
//!   list and pressing 1–4 attempts to craft the highlighted recipe.
//!
//! **Controls:** WASD — move   C near workbench — craft   1–4 — choose recipe

use bevy::prelude::*;
use std::collections::HashMap;

const WINDOW_W: f32 = 800.0;
const WINDOW_H: f32 = 500.0;
const PLAYER_SPEED: f32 = 150.0;
const PICKUP_RADIUS: f32 = 24.0;
const CRAFT_RADIUS: f32 = 50.0;

// ── Pure crafting model ───────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Item { Wood, Stone, Iron, Herb, Coal }

impl Item {
    pub fn label(self) -> &'static str {
        match self {
            Item::Wood  => "Wood",
            Item::Stone => "Stone",
            Item::Iron  => "Iron",
            Item::Herb  => "Herb",
            Item::Coal  => "Coal",
        }
    }
    pub fn color(self) -> Color {
        match self {
            Item::Wood  => Color::srgb(0.6, 0.4, 0.2),
            Item::Stone => Color::srgb(0.55, 0.55, 0.6),
            Item::Iron  => Color::srgb(0.7, 0.75, 0.8),
            Item::Herb  => Color::srgb(0.2, 0.8, 0.35),
            Item::Coal  => Color::srgb(0.18, 0.18, 0.2),
        }
    }
}

pub struct Recipe {
    pub name: &'static str,
    pub inputs: &'static [(Item, u32)],
    pub output: Item,
}

/// True when `inventory` contains at least the required quantity of every ingredient.
pub fn can_craft(inventory: &HashMap<Item, u32>, recipe: &Recipe) -> bool {
    recipe.inputs.iter().all(|(item, qty)| inventory.get(item).copied().unwrap_or(0) >= *qty)
}

/// Remove the recipe's ingredients from the inventory. Panics if quantities are insufficient
/// (call `can_craft` first).
pub fn consume_ingredients(inventory: &mut HashMap<Item, u32>, recipe: &Recipe) {
    for (item, qty) in recipe.inputs {
        *inventory.entry(*item).or_insert(0) -= qty;
    }
}

const RECIPES: &[Recipe] = &[
    Recipe { name: "Torch",  inputs: &[(Item::Wood, 1), (Item::Coal, 1)],              output: Item::Coal  },
    Recipe { name: "Sword",  inputs: &[(Item::Iron, 2), (Item::Wood, 1)],              output: Item::Iron  },
    Recipe { name: "Potion", inputs: &[(Item::Herb, 2)],                               output: Item::Herb  },
    Recipe { name: "Armor",  inputs: &[(Item::Iron, 2), (Item::Stone, 1)],             output: Item::Stone },
];

// ── ECS ───────────────────────────────────────────────────────────────────────

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Pickup(Item);

#[derive(Component)]
struct Workbench;

#[derive(Resource)]
struct Inventory(HashMap<Item, u32>);

#[derive(Resource)]
struct CraftingOpen(bool);

#[derive(Component)]
struct HudText;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Crafting System".into(),
                resolution: (800u32, 500u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(Inventory(HashMap::new()))
        .insert_resource(CraftingOpen(false))
        .add_systems(Startup, setup)
        .add_systems(Update, (move_player, collect_pickups, handle_craft, update_hud).chain())
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Player.
    commands.spawn((
        Player,
        Sprite { color: Color::srgb(0.3, 0.6, 1.0), custom_size: Some(Vec2::splat(20.0)), ..default() },
        Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
    ));

    // Workbench.
    commands.spawn((
        Workbench,
        Sprite { color: Color::srgb(0.8, 0.65, 0.3), custom_size: Some(Vec2::splat(32.0)), ..default() },
        Transform::from_translation(Vec3::new(0.0, 170.0, 0.5)),
    ));
    commands.spawn((
        Text::new("Workbench"),
        TextFont { font_size: 11.0, ..default() },
        TextColor(Color::srgb(0.8, 0.65, 0.3)),
        Node { position_type: PositionType::Absolute, top: Val::Px(36.0), left: Val::Px(335.0), ..default() },
    ));

    // Pickups scattered around.
    let pickups: &[(Item, f32, f32)] = &[
        (Item::Wood,  -280.0,  120.0),
        (Item::Wood,  -200.0, -100.0),
        (Item::Stone,  250.0,  130.0),
        (Item::Stone,  180.0,  -90.0),
        (Item::Iron,  -240.0,   30.0),
        (Item::Iron,   260.0,  -30.0),
        (Item::Herb,   -90.0,  160.0),
        (Item::Herb,   120.0, -160.0),
        (Item::Coal,  -150.0, -170.0),
        (Item::Coal,   200.0,  170.0),
    ];
    for (item, x, y) in pickups {
        commands.spawn((
            Pickup(*item),
            Sprite { color: item.color(), custom_size: Some(Vec2::splat(18.0)), ..default() },
            Transform::from_translation(Vec3::new(*x, *y, 0.5)),
        ));
    }

    commands.spawn((
        HudText,
        Text::new(""),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::WHITE),
        Node { position_type: PositionType::Absolute, top: Val::Px(10.0), left: Val::Px(10.0), ..default() },
    ));

    commands.spawn((
        Text::new("WASD — move   walk into items to collect   C near workbench to craft (1–4)"),
        TextFont { font_size: 13.0, ..default() },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.5)),
        Node { position_type: PositionType::Absolute, bottom: Val::Px(10.0), left: Val::Px(10.0), ..default() },
    ));
}

fn move_player(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut q: Query<&mut Transform, With<Player>>,
) {
    let mut dir = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp)    { dir.y += 1.0; }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown)  { dir.y -= 1.0; }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft)  { dir.x -= 1.0; }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) { dir.x += 1.0; }
    if dir == Vec2::ZERO { return; }
    let Ok(mut tf) = q.single_mut() else { return };
    tf.translation += (dir.normalize() * PLAYER_SPEED * time.delta_secs()).extend(0.0);
    tf.translation.x = tf.translation.x.clamp(-WINDOW_W / 2.0 + 14.0, WINDOW_W / 2.0 - 14.0);
    tf.translation.y = tf.translation.y.clamp(-WINDOW_H / 2.0 + 14.0, WINDOW_H / 2.0 - 14.0);
}

fn collect_pickups(
    mut commands: Commands,
    player_q: Query<&Transform, With<Player>>,
    pickup_q: Query<(Entity, &Pickup, &Transform), Without<Player>>,
    mut inventory: ResMut<Inventory>,
) {
    let Ok(ptf) = player_q.single() else { return };
    for (entity, pickup, ptf2) in &pickup_q {
        if ptf.translation.distance(ptf2.translation) < PICKUP_RADIUS {
            *inventory.0.entry(pickup.0).or_insert(0) += 1;
            commands.entity(entity).despawn();
        }
    }
}

fn handle_craft(
    keys: Res<ButtonInput<KeyCode>>,
    player_q: Query<&Transform, With<Player>>,
    bench_q: Query<&Transform, (With<Workbench>, Without<Player>)>,
    mut inventory: ResMut<Inventory>,
    mut open: ResMut<CraftingOpen>,
) {
    let Ok(ptf) = player_q.single() else { return };
    let Ok(btf) = bench_q.single() else { return };
    let near_bench = ptf.translation.distance(btf.translation) < CRAFT_RADIUS;

    if keys.just_pressed(KeyCode::KeyC) && near_bench {
        open.0 = !open.0;
    }
    if !open.0 { return; }

    let digit_keys = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4];
    for (i, &key) in digit_keys.iter().enumerate() {
        if keys.just_pressed(key) {
            if let Some(recipe) = RECIPES.get(i) {
                if can_craft(&inventory.0, recipe) {
                    consume_ingredients(&mut inventory.0, recipe);
                    *inventory.0.entry(recipe.output).or_insert(0) += 1;
                }
            }
        }
    }
}

fn update_hud(
    inventory: Res<Inventory>,
    open: Res<CraftingOpen>,
    player_q: Query<&Transform, With<Player>>,
    bench_q: Query<&Transform, (With<Workbench>, Without<Player>)>,
    mut text_q: Query<&mut Text, With<HudText>>,
) {
    let Ok(mut text) = text_q.single_mut() else { return };
    let Ok(ptf) = player_q.single() else { return };
    let Ok(btf) = bench_q.single() else { return };
    let near = ptf.translation.distance(btf.translation) < CRAFT_RADIUS;

    let all_items = [Item::Wood, Item::Stone, Item::Iron, Item::Herb, Item::Coal];
    let inv: String = all_items.iter().map(|item| {
        format!("{}: {}  ", item.label(), inventory.0.get(item).copied().unwrap_or(0))
    }).collect();

    let mut lines = format!("Inventory: {inv}\n");

    if near {
        lines.push_str("[ Workbench — press C to ");
        lines.push_str(if open.0 { "close ]" } else { "open ]" });
        if open.0 {
            lines.push('\n');
            for (i, recipe) in RECIPES.iter().enumerate() {
                let ready = if can_craft(&inventory.0, recipe) { "READY" } else { "     " };
                let inputs: String = recipe.inputs.iter()
                    .map(|(item, qty)| format!("{}×{}", qty, item.label()))
                    .collect::<Vec<_>>().join(" + ");
                lines.push_str(&format!("  {} [{}] → {}: {inputs}\n", i + 1, ready, recipe.name));
            }
        }
    }

    text.0 = lines;
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn inv(items: &[(Item, u32)]) -> HashMap<Item, u32> {
        items.iter().cloned().collect()
    }

    #[test]
    fn can_craft_torch_with_correct_items() {
        let inventory = inv(&[(Item::Wood, 1), (Item::Coal, 1)]);
        assert!(can_craft(&inventory, &RECIPES[0]));
    }

    #[test]
    fn cannot_craft_when_missing_ingredient() {
        let inventory = inv(&[(Item::Wood, 1)]);
        assert!(!can_craft(&inventory, &RECIPES[0]));
    }

    #[test]
    fn cannot_craft_when_quantity_insufficient() {
        let inventory = inv(&[(Item::Iron, 1), (Item::Wood, 1)]);
        assert!(!can_craft(&inventory, &RECIPES[1])); // Sword needs Iron×2
    }

    #[test]
    fn consume_removes_correct_amounts() {
        let mut inventory = inv(&[(Item::Wood, 3), (Item::Coal, 2)]);
        consume_ingredients(&mut inventory, &RECIPES[0]); // Torch: Wood×1 + Coal×1
        assert_eq!(inventory[&Item::Wood], 2);
        assert_eq!(inventory[&Item::Coal], 1);
    }

    #[test]
    fn potion_crafted_from_two_herbs() {
        let inventory = inv(&[(Item::Herb, 2)]);
        assert!(can_craft(&inventory, &RECIPES[2]));
        let inventory_one = inv(&[(Item::Herb, 1)]);
        assert!(!can_craft(&inventory_one, &RECIPES[2]));
    }

    #[test]
    fn empty_inventory_cannot_craft_anything() {
        let inventory = HashMap::new();
        for recipe in RECIPES {
            assert!(!can_craft(&inventory, recipe));
        }
    }
}
