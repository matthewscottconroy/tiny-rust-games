//! Destructible Terrain — grid tiles with hit points that can be broken open.
//!
//! Key ideas:
//! - Wall tiles carry a `Tile { hp, max_hp }` component. `damage_tile` is a pure
//!   function that returns `Some(new_hp)` or `None` (destroyed).
//! - `tile_color` maps the hp fraction to a visual crack gradient.
//! - The player can walk freely on floor tiles and press SPACE to attack the wall
//!   tile directly in front of their facing direction.
//! - Destroying a wall despawns the entity, permanently opening that cell.
//!
//! **Controls:** WASD / Arrows — move   SPACE — attack facing wall

use bevy::prelude::*;
use std::collections::HashSet;

const COLS: usize = 22;
const ROWS: usize = 14;
const TILE_PX: f32 = 34.0;
const PLAYER_SPEED: f32 = 140.0;
const ATTACK_CD: f32 = 0.3;
const ATTACK_DAMAGE: i32 = 1;

// ── Pure helpers ──────────────────────────────────────────────────────────────

/// Apply `damage` to a tile's HP. Returns `None` if the tile is destroyed.
pub fn damage_tile(hp: i32, damage: i32) -> Option<i32> {
    let new_hp = hp - damage;
    if new_hp <= 0 { None } else { Some(new_hp) }
}

/// Colour a wall tile from intact (grey-brown) to crumbling (dark red).
pub fn tile_color(hp: i32, max_hp: i32) -> Color {
    let frac = (hp as f32 / max_hp as f32).clamp(0.0, 1.0);
    Color::srgb(0.38 * frac + 0.22, 0.30 * frac + 0.15, 0.25 * frac + 0.12)
}

/// World-space centre of the grid cell at `(col, row)`.
pub fn cell_center(col: usize, row: usize) -> Vec2 {
    let ox = -(COLS as f32 * TILE_PX) / 2.0 + TILE_PX / 2.0;
    let oy = -(ROWS as f32 * TILE_PX) / 2.0 + TILE_PX / 2.0;
    Vec2::new(ox + col as f32 * TILE_PX, oy + row as f32 * TILE_PX)
}

/// Grid column and row from a world position, clamped to grid bounds.
pub fn world_to_cell(pos: Vec2) -> IVec2 {
    let ox = -(COLS as f32 * TILE_PX) / 2.0;
    let oy = -(ROWS as f32 * TILE_PX) / 2.0;
    let col = ((pos.x - ox) / TILE_PX).floor() as i32;
    let row = ((pos.y - oy) / TILE_PX).floor() as i32;
    IVec2::new(col.clamp(0, COLS as i32 - 1), row.clamp(0, ROWS as i32 - 1))
}

// ── ECS ───────────────────────────────────────────────────────────────────────

#[derive(Component)]
struct Player { facing: IVec2, attack_cd: f32 }

#[derive(Component)]
struct Tile { hp: i32, max_hp: i32, cell: IVec2 }

/// Set of wall cells (used for collision).
#[derive(Resource, Default)]
struct WallSet(HashSet<IVec2>);

#[derive(Component)]
struct HudText;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Destructible Terrain".into(),
                resolution: (800u32, 500u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(WallSet::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (move_player, handle_attack, update_tile_colors).chain())
        .run();
}

fn is_wall_cell(col: usize, row: usize) -> bool {
    // Border always wall.
    if col == 0 || row == 0 || col == COLS - 1 || row == ROWS - 1 { return true; }
    // Interior pillars at even positions.
    col % 4 == 0 && row % 3 == 0
}

fn setup(mut commands: Commands, mut walls: ResMut<WallSet>) {
    commands.spawn(Camera2d);

    for row in 0..ROWS {
        for col in 0..COLS {
            let center = cell_center(col, row);
            let cell = IVec2::new(col as i32, row as i32);
            if is_wall_cell(col, row) {
                let max_hp = if col == 0 || row == 0 || col == COLS - 1 || row == ROWS - 1 { 99 } else { 3 };
                commands.spawn((
                    Tile { hp: max_hp, max_hp, cell },
                    Sprite {
                        color: tile_color(max_hp, max_hp),
                        custom_size: Some(Vec2::splat(TILE_PX - 1.0)),
                        ..default()
                    },
                    Transform::from_translation(center.extend(0.0)),
                ));
                walls.0.insert(cell);
            } else {
                // Floor tile (not interactive).
                commands.spawn((
                    Sprite {
                        color: Color::srgb(0.12, 0.11, 0.10),
                        custom_size: Some(Vec2::splat(TILE_PX - 1.0)),
                        ..default()
                    },
                    Transform::from_translation(center.extend(0.0)),
                ));
            }
        }
    }

    // Start the player in the centre floor cell.
    let start = cell_center(COLS / 2, ROWS / 2);
    commands.spawn((
        Player { facing: IVec2::new(0, 1), attack_cd: 0.0 },
        Sprite { color: Color::srgb(0.3, 0.6, 1.0), custom_size: Some(Vec2::splat(TILE_PX - 6.0)), ..default() },
        Transform::from_translation(start.extend(1.0)),
    ));

    commands.spawn((
        HudText,
        Text::new("SPACE — attack facing wall"),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::WHITE),
        Node { position_type: PositionType::Absolute, top: Val::Px(8.0), left: Val::Px(8.0), ..default() },
    ));

    commands.spawn((
        Text::new("WASD / Arrows — move   SPACE — break wall"),
        TextFont { font_size: 13.0, ..default() },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.5)),
        Node { position_type: PositionType::Absolute, bottom: Val::Px(8.0), left: Val::Px(8.0), ..default() },
    ));
}

fn move_player(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    walls: Res<WallSet>,
    mut q: Query<(&mut Player, &mut Transform)>,
) {
    let Ok((mut player, mut tf)) = q.single_mut() else { return };
    player.attack_cd = (player.attack_cd - time.delta_secs()).max(0.0);

    let mut dir = IVec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp)    { dir.y += 1; }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown)  { dir.y -= 1; }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft)  { dir.x -= 1; }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) { dir.x += 1; }
    if dir == IVec2::ZERO { return; }

    // Only allow cardinal movement (pick one axis).
    let dir = if dir.x != 0 { IVec2::new(dir.x, 0) } else { IVec2::new(0, dir.y) };
    player.facing = dir;

    let current_cell = world_to_cell(tf.translation.truncate());
    let target_cell = current_cell + dir;
    if !walls.0.contains(&target_cell) {
        let target_pos = cell_center(target_cell.x as usize, target_cell.y as usize);
        let dt = time.delta_secs();
        let move_vec = (target_pos - tf.translation.truncate()).normalize_or_zero() * PLAYER_SPEED * dt;
        // Snap when very close to avoid jitter.
        if tf.translation.truncate().distance(target_pos) < PLAYER_SPEED * dt + 1.0 {
            tf.translation = target_pos.extend(1.0);
        } else {
            tf.translation += move_vec.extend(0.0);
        }
    }
}

fn handle_attack(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut walls: ResMut<WallSet>,
    mut player_q: Query<(&mut Player, &Transform)>,
    mut tile_q: Query<(Entity, &mut Tile, &mut Sprite)>,
) {
    let Ok((mut player, ptf)) = player_q.single_mut() else { return };
    if !keys.just_pressed(KeyCode::Space) || player.attack_cd > 0.0 { return; }

    let current_cell = world_to_cell(ptf.translation.truncate());
    let target_cell = current_cell + player.facing;

    for (entity, mut tile, mut sprite) in &mut tile_q {
        if tile.cell == target_cell && tile.max_hp < 99 {
            player.attack_cd = ATTACK_CD;
            match damage_tile(tile.hp, ATTACK_DAMAGE) {
                Some(new_hp) => {
                    tile.hp = new_hp;
                    sprite.color = tile_color(new_hp, tile.max_hp);
                }
                None => {
                    walls.0.remove(&target_cell);
                    commands.entity(entity).despawn();
                }
            }
            break;
        }
    }
}

fn update_tile_colors(mut q: Query<(&Tile, &mut Sprite)>) {
    for (tile, mut sprite) in &mut q {
        sprite.color = tile_color(tile.hp, tile.max_hp);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_tile_reduces_hp() {
        assert_eq!(damage_tile(3, 1), Some(2));
    }

    #[test]
    fn damage_tile_exact_zero_destroys() {
        assert_eq!(damage_tile(1, 1), None);
    }

    #[test]
    fn damage_tile_overkill_destroys() {
        assert_eq!(damage_tile(1, 5), None);
    }

    #[test]
    fn tile_color_full_hp_is_lighter() {
        let full = tile_color(3, 3);
        let half = tile_color(1, 3);
        // Full HP should be brighter (higher red channel).
        let Color::Srgba(full_c) = full else { panic!() };
        let Color::Srgba(half_c) = half else { panic!() };
        assert!(full_c.red > half_c.red);
    }

    #[test]
    fn world_to_cell_centre_maps_back() {
        let centre = cell_center(COLS / 2, ROWS / 2);
        let cell = world_to_cell(centre);
        assert_eq!(cell, IVec2::new(COLS as i32 / 2, ROWS as i32 / 2));
    }
}
