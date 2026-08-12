//! Grid-based inventory UI demo — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`InventoryUiPlugin`] into any Bevy
//! app with `app.add_plugins(InventoryUiPlugin)` and it renders an 8×5 item
//! grid you can pick up, place, and swap items in. Tune the layout through the
//! [`InventoryUiConfig`] resource without editing the plugin's internals.
//!
//! Key ideas:
//! - An 8×5 [`ItemGrid`] resource maps `(col, row)` slots to `Option<Item>`.
//! - [`slot_world_pos`] converts a grid coordinate to a world-space position,
//!   keeping layout logic out of systems.
//! - [`find_slot_at`] does the reverse: maps a cursor position back to `(col, row)`.
//! - [`first_empty_slot`] scans for an open slot — used when picking up a drop.
//! - Left-clicking a filled slot picks up the item; clicking an empty slot drops
//!   the held item there.  The HUD shows what's currently held.
//!
//! **Controls:** left-click — pick up / place item;  R — reset grid.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use inventory_ui::InventoryUiPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(InventoryUiPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system and resource for the inventory UI feature.
///
/// Add it with `app.add_plugins(InventoryUiPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct InventoryUiPlugin;

impl Plugin for InventoryUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryUiConfig>()
            .init_resource::<ItemGrid>()
            .insert_resource(HeldItem(None))
            .add_systems(Startup, setup)
            .add_systems(Update, (handle_click, sync_display).chain());
    }
}

// ── Configuration ─────────────────────────────────────────────────────────────

/// Tunable layout parameters for the grid. Override before adding the plugin,
/// e.g. `app.insert_resource(InventoryUiConfig { cell: 60.0, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct InventoryUiConfig {
    /// Number of columns in the grid.
    pub cols: usize,
    /// Number of rows in the grid.
    pub rows: usize,
    /// Size of one square cell in world units.
    pub cell: f32,
}

impl Default for InventoryUiConfig {
    fn default() -> Self {
        Self {
            cols: COLS,
            rows: ROWS,
            cell: CELL,
        }
    }
}

// ── Pure layout helpers ───────────────────────────────────────────────────────

/// Number of inventory columns.
pub const COLS: usize = 8;
/// Number of inventory rows.
pub const ROWS: usize = 5;
/// Width and height of one slot, in pixels.
pub const CELL: f32 = 52.0;

/// World-space centre of grid slot `(col, row)`.
/// The grid is centred at the origin.
pub fn slot_world_pos(col: usize, row: usize) -> Vec2 {
    let origin = Vec2::new(
        -(COLS as f32 - 1.0) * CELL * 0.5,
        -(ROWS as f32 - 1.0) * CELL * 0.5 - 60.0,
    );
    origin + Vec2::new(col as f32 * CELL, row as f32 * CELL)
}

/// Maps `cursor` world position back to `(col, row)`, or `None` if outside grid.
pub fn find_slot_at(cursor: Vec2) -> Option<(usize, usize)> {
    let origin = Vec2::new(
        -(COLS as f32 - 1.0) * CELL * 0.5 - CELL * 0.5,
        -(ROWS as f32 - 1.0) * CELL * 0.5 - CELL * 0.5 - 60.0,
    );
    let rel = cursor - origin;
    if rel.x < 0.0 || rel.y < 0.0 {
        return None;
    }
    let col = (rel.x / CELL) as usize;
    let row = (rel.y / CELL) as usize;
    if col < COLS && row < ROWS {
        Some((col, row))
    } else {
        None
    }
}

/// Returns the index of the first `None` slot, scanning row-major.
pub fn first_empty_slot(slots: &[Option<Item>]) -> Option<usize> {
    slots.iter().position(|s| s.is_none())
}

// ── Item ──────────────────────────────────────────────────────────────────────

/// The item kinds that can occupy a slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    /// A weapon.
    Sword,
    /// Defensive gear.
    Shield,
    /// A consumable.
    Potion,
    /// A valuable.
    Gem,
    /// A quest item.
    Key,
}

impl Item {
    /// The swatch colour drawn for this item.
    pub fn color(self) -> Color {
        match self {
            Item::Sword => Color::srgb(0.7, 0.75, 0.95),
            Item::Shield => Color::srgb(0.55, 0.45, 0.85),
            Item::Potion => Color::srgb(0.3, 0.9, 0.4),
            Item::Gem => Color::srgb(0.9, 0.3, 0.8),
            Item::Key => Color::srgb(1.0, 0.85, 0.2),
        }
    }
    /// The glyph drawn on the item's tile.
    pub fn label(self) -> &'static str {
        match self {
            Item::Sword => "⚔",
            Item::Shield => "🛡",
            Item::Potion => "⚗",
            Item::Gem => "💎",
            Item::Key => "🗝",
        }
    }
}

// ── ECS ───────────────────────────────────────────────────────────────────────

/// The inventory contents, one entry per slot.
#[derive(Resource)]
pub struct ItemGrid {
    /// Row-major slots; index with `row * COLS + col`.
    pub slots: Vec<Option<Item>>,
}

impl Default for ItemGrid {
    fn default() -> Self {
        let mut slots = vec![None; COLS * ROWS];
        // Pre-populate some items
        let items = [
            (0, 0, Item::Sword),
            (1, 0, Item::Shield),
            (2, 0, Item::Potion),
            (4, 2, Item::Gem),
            (6, 4, Item::Key),
        ];
        for (col, row, item) in items {
            slots[row * COLS + col] = Some(item);
        }
        Self { slots }
    }
}

/// The item currently picked up and following the cursor, if any.
#[derive(Resource)]
pub struct HeldItem(pub Option<Item>);

#[derive(Component)]
struct SlotSprite(usize, usize);

#[derive(Component)]
struct HeldLabel;

fn setup(mut commands: Commands, grid: Res<ItemGrid>) {
    commands.spawn(Camera2d);

    commands.spawn((
        Text::new("left-click: pick up / place   R: reset"),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::srgb(0.55, 0.55, 0.55)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));

    commands.spawn((
        Text::new("Holding: nothing"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.9, 0.6)),
        HeldLabel,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(28.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));

    for row in 0..ROWS {
        for col in 0..COLS {
            let pos = slot_world_pos(col, row);
            let item = grid.slots[row * COLS + col];
            commands
                .spawn((
                    Sprite {
                        color: slot_color(item, false),
                        custom_size: Some(Vec2::splat(CELL - 4.0)),
                        ..default()
                    },
                    Transform::from_xyz(pos.x, pos.y, 0.0),
                    SlotSprite(col, row),
                ))
                .with_children(|p| {
                    p.spawn((
                        Text2d::new(item.map(|i| i.label()).unwrap_or("")),
                        TextFont {
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        Transform::from_xyz(0.0, 0.0, 1.0),
                    ));
                });
        }
    }
}

fn slot_color(item: Option<Item>, hovered: bool) -> Color {
    let base = if let Some(it) = item {
        it.color()
    } else {
        Color::srgb(0.18, 0.18, 0.22)
    };
    if hovered {
        Color::srgb(
            (base.to_linear().red + 0.2).min(1.0),
            (base.to_linear().green + 0.2).min(1.0),
            (base.to_linear().blue + 0.2).min(1.0),
        )
    } else {
        base
    }
}

fn handle_click(
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    windows: Query<&Window>,
    mut grid: ResMut<ItemGrid>,
    mut held: ResMut<HeldItem>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        *grid = ItemGrid::default();
        held.0 = None;
        return;
    }
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok((camera, cam_tf)) = camera_q.single() else {
        return;
    };
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok(world) = camera.viewport_to_world_2d(cam_tf, cursor) else {
        return;
    };

    let Some((col, row)) = find_slot_at(world) else {
        return;
    };
    let idx = row * COLS + col;

    match (held.0, grid.slots[idx]) {
        (Some(item), None) => {
            // Drop held item into empty slot
            grid.slots[idx] = Some(item);
            held.0 = None;
        }
        (None, Some(item)) => {
            // Pick up item from slot
            grid.slots[idx] = None;
            held.0 = Some(item);
        }
        (Some(held_item), Some(slot_item)) => {
            // Swap
            grid.slots[idx] = Some(held_item);
            held.0 = Some(slot_item);
        }
        (None, None) => {}
    }
}

fn sync_display(
    grid: Res<ItemGrid>,
    held: Res<HeldItem>,
    mut slots: Query<(&SlotSprite, &mut Sprite, &Children)>,
    mut texts: Query<&mut Text2d>,
    mut held_label: Query<&mut Text, With<HeldLabel>>,
) {
    if !grid.is_changed() && !held.is_changed() {
        return;
    }

    for (ss, mut sprite, children) in &mut slots {
        let item = grid.slots[ss.1 * COLS + ss.0];
        sprite.color = slot_color(item, false);
        for child in children.iter() {
            if let Ok(mut t) = texts.get_mut(child) {
                *t = Text2d::new(item.map(|i| i.label()).unwrap_or(""));
            }
        }
    }

    if let Ok(mut t) = held_label.single_mut() {
        *t = Text::new(match held.0 {
            Some(item) => format!("Holding: {}", item.label()),
            None => "Holding: nothing".to_string(),
        });
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_world_pos_col0_row0_is_bottom_left_ish() {
        let p = slot_world_pos(0, 0);
        let p_right = slot_world_pos(1, 0);
        assert!(p_right.x > p.x);
    }

    #[test]
    fn slot_world_pos_spacing_equals_cell() {
        let p0 = slot_world_pos(0, 0);
        let p1 = slot_world_pos(1, 0);
        assert!((p1.x - p0.x - CELL).abs() < 1e-3);
    }

    #[test]
    fn slot_world_pos_vertical_spacing_equals_cell() {
        let p0 = slot_world_pos(0, 0);
        let p1 = slot_world_pos(0, 1);
        assert!((p1.y - p0.y - CELL).abs() < 1e-3);
    }

    #[test]
    fn find_slot_at_round_trips_origin_slot() {
        let pos = slot_world_pos(0, 0);
        let result = find_slot_at(pos);
        assert_eq!(result, Some((0, 0)), "round-trip failed: {pos:?}");
    }

    #[test]
    fn find_slot_at_round_trips_several_slots() {
        for col in 0..COLS {
            for row in 0..ROWS {
                let pos = slot_world_pos(col, row);
                assert_eq!(find_slot_at(pos), Some((col, row)), "({col},{row})");
            }
        }
    }

    #[test]
    fn find_slot_at_far_outside_returns_none() {
        assert_eq!(find_slot_at(Vec2::new(9999.0, 9999.0)), None);
    }

    #[test]
    fn first_empty_slot_finds_first_none() {
        let slots = vec![Some(Item::Sword), None, Some(Item::Gem)];
        assert_eq!(first_empty_slot(&slots), Some(1));
    }

    #[test]
    fn first_empty_slot_all_full_returns_none() {
        let slots = vec![Some(Item::Key); 4];
        assert_eq!(first_empty_slot(&slots), None);
    }

    #[test]
    fn item_grid_default_has_correct_count() {
        let grid = ItemGrid::default();
        let filled = grid.slots.iter().filter(|s| s.is_some()).count();
        assert_eq!(filled, 5);
    }

    #[test]
    fn config_default_matches_constants() {
        let c = InventoryUiConfig::default();
        assert_eq!(c.cols, COLS);
        assert_eq!(c.rows, ROWS);
        assert_eq!(c.cell, CELL);
    }
}
