//! Procedural dungeon demo — BSP room splitting and corridor carving.
//!
//! Key ideas:
//! - A `Rect` represents a region of the grid; `split_room` bisects it along a
//!   chosen axis, returning two child rects only when both children meet the
//!   minimum size constraint.
//! - `build_dungeon` iteratively applies BSP splits, shrinks each leaf region
//!   into a room (`room_interior`), and connects adjacent rooms with an
//!   L-shaped corridor.
//! - A lightweight LCG (`lcg_next`) produces all random decisions without any
//!   external dependency.
//! - Press **SPACE** to regenerate with a new seed.
//!
//! **Controls:** SPACE — regenerate dungeon.

use bevy::prelude::*;
use bevy::window::WindowResolution;

const COLS: usize = 48;
const ROWS: usize = 28;
const TILE_PX: f32 = 18.0;
const MIN_ROOM: usize = 5;

// ─── Tile types ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Tile {
    Wall,
    Floor,
}

// ─── Geometry ────────────────────────────────────────────────────────────────

/// An axis-aligned rectangle in grid coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

/// Splits `room` at grid offset `pos` along the chosen axis.
///
/// `vertical = true` splits left/right; `false` splits top/bottom.
/// Returns `None` if either child would be narrower/shorter than `min_size`.
pub fn split_room(room: Rect, vertical: bool, pos: usize, min_size: usize) -> Option<(Rect, Rect)> {
    if vertical {
        if pos < min_size || room.w.saturating_sub(pos) < min_size {
            return None;
        }
        Some((
            Rect { x: room.x, y: room.y, w: pos, h: room.h },
            Rect { x: room.x + pos, y: room.y, w: room.w - pos, h: room.h },
        ))
    } else {
        if pos < min_size || room.h.saturating_sub(pos) < min_size {
            return None;
        }
        Some((
            Rect { x: room.x, y: room.y, w: room.w, h: pos },
            Rect { x: room.x, y: room.y + pos, w: room.w, h: room.h - pos },
        ))
    }
}

/// Returns the interior of `rect` shrunk by `margin` on all sides, or `None`
/// if the margin would collapse either dimension to zero.
pub fn room_interior(rect: Rect, margin: usize) -> Option<Rect> {
    let double = margin * 2;
    if rect.w <= double || rect.h <= double {
        return None;
    }
    Some(Rect {
        x: rect.x + margin,
        y: rect.y + margin,
        w: rect.w - double,
        h: rect.h - double,
    })
}

/// One step of a 64-bit linear congruential generator.
pub fn lcg_next(seed: u64) -> u64 {
    seed.wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407)
}

// ─── Dungeon builder ─────────────────────────────────────────────────────────

fn build_dungeon(seed: u64) -> Vec<Vec<Tile>> {
    let mut grid = vec![vec![Tile::Wall; COLS]; ROWS];
    let root = Rect { x: 1, y: 1, w: COLS - 2, h: ROWS - 2 };
    let mut regions = vec![root];
    let mut rng = seed;

    // BSP: repeatedly split regions until they are small enough.
    for _ in 0..16 {
        let mut next = Vec::new();
        let mut any_split = false;
        for region in &regions {
            let big_w = region.w > MIN_ROOM * 2 + 2;
            let big_h = region.h > MIN_ROOM * 2 + 2;
            if !big_w && !big_h {
                next.push(*region);
                continue;
            }
            rng = lcg_next(rng);
            let vertical = if big_w && big_h { rng & 1 == 0 } else { big_w };
            let span = if vertical { region.w } else { region.h };
            rng = lcg_next(rng);
            let lo = MIN_ROOM;
            let hi = span.saturating_sub(MIN_ROOM);
            let pos = if hi > lo { lo + (rng as usize % (hi - lo)) } else { lo };
            if let Some((a, b)) = split_room(*region, vertical, pos, MIN_ROOM) {
                next.push(a);
                next.push(b);
                any_split = true;
            } else {
                next.push(*region);
            }
        }
        regions = next;
        if !any_split {
            break;
        }
    }

    // Carve rooms and collect their centres.
    let mut centres: Vec<(usize, usize)> = Vec::new();
    for region in &regions {
        if let Some(room) = room_interior(*region, 1) {
            for ry in room.y..(room.y + room.h).min(ROWS) {
                for rx in room.x..(room.x + room.w).min(COLS) {
                    grid[ry][rx] = Tile::Floor;
                }
            }
            centres.push((room.x + room.w / 2, room.y + room.h / 2));
        }
    }

    // Connect consecutive rooms with L-shaped corridors.
    for i in 0..centres.len().saturating_sub(1) {
        let (ax, ay) = centres[i];
        let (bx, by) = centres[i + 1];
        let (x0, x1) = (ax.min(bx), ax.max(bx));
        let (y0, y1) = (ay.min(by), ay.max(by));
        for x in x0..=x1 {
            grid[ay][x] = Tile::Floor;
        }
        for y in y0..=y1 {
            grid[y][bx] = Tile::Floor;
        }
    }

    grid
}

// ─── Resources & components ──────────────────────────────────────────────────

#[derive(Resource)]
struct DungeonSeed(u64);

/// Marks tile sprite entities so they can be rebuilt on regeneration.
#[derive(Component)]
struct TileSprite;

// ─── Bevy app ────────────────────────────────────────────────────────────────

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Procedural Dungeon — SPACE to regenerate".to_string(),
                resolution: (864u32, 534u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(DungeonSeed(42))
        .add_systems(Startup, (setup_camera, spawn_dungeon).chain())
        .add_systems(Update, (handle_regen, label_system))
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// Builds and renders the dungeon grid.
fn spawn_dungeon(mut commands: Commands, seed: Res<DungeonSeed>) {
    let grid = build_dungeon(seed.0);
    let offset_x = -(COLS as f32 * TILE_PX) / 2.0 + TILE_PX / 2.0;
    let offset_y = (ROWS as f32 * TILE_PX) / 2.0 - TILE_PX / 2.0;

    for (r, row) in grid.iter().enumerate() {
        for (c, tile) in row.iter().enumerate() {
            let color = match tile {
                Tile::Wall => Color::srgb(0.18, 0.18, 0.22),
                Tile::Floor => Color::srgb(0.55, 0.5, 0.42),
            };
            let pos = Vec3::new(
                offset_x + c as f32 * TILE_PX,
                offset_y - r as f32 * TILE_PX,
                0.0,
            );
            commands.spawn((
                Sprite { color, custom_size: Some(Vec2::splat(TILE_PX - 1.0)), ..default() },
                Transform::from_translation(pos),
                TileSprite,
            ));
        }
    }

    commands.spawn((
        Text::new(format!("Seed: {}  |  SPACE to regenerate", seed.0)),
        TextFont { font_size: 16.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(4.0),
            left: Val::Px(8.0),
            ..default()
        },
    ));
}

/// Regenerates the dungeon on SPACE, despawning old tiles first.
fn handle_regen(
    input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut seed: ResMut<DungeonSeed>,
    tiles: Query<Entity, With<TileSprite>>,
) {
    if !input.just_pressed(KeyCode::Space) {
        return;
    }
    for e in &tiles {
        commands.entity(e).despawn();
    }
    seed.0 = lcg_next(seed.0);

    let grid = build_dungeon(seed.0);
    let offset_x = -(COLS as f32 * TILE_PX) / 2.0 + TILE_PX / 2.0;
    let offset_y = (ROWS as f32 * TILE_PX) / 2.0 - TILE_PX / 2.0;
    for (r, row) in grid.iter().enumerate() {
        for (c, tile) in row.iter().enumerate() {
            let color = match tile {
                Tile::Wall => Color::srgb(0.18, 0.18, 0.22),
                Tile::Floor => Color::srgb(0.55, 0.5, 0.42),
            };
            let pos = Vec3::new(
                offset_x + c as f32 * TILE_PX,
                offset_y - r as f32 * TILE_PX,
                0.0,
            );
            commands.spawn((
                Sprite { color, custom_size: Some(Vec2::splat(TILE_PX - 1.0)), ..default() },
                Transform::from_translation(pos),
                TileSprite,
            ));
        }
    }
}

fn label_system(seed: Res<DungeonSeed>, mut query: Query<&mut Text, Without<TileSprite>>) {
    for mut text in &mut query {
        text.0 = format!("Seed: {}  |  SPACE to regenerate", seed.0);
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_vertical_produces_correct_widths() {
        let r = Rect { x: 0, y: 0, w: 20, h: 10 };
        let (a, b) = split_room(r, true, 8, 4).unwrap();
        assert_eq!(a.w, 8);
        assert_eq!(b.w, 12);
        assert_eq!(a.x, 0);
        assert_eq!(b.x, 8);
    }

    #[test]
    fn split_horizontal_produces_correct_heights() {
        let r = Rect { x: 0, y: 0, w: 10, h: 20 };
        let (a, b) = split_room(r, false, 7, 4).unwrap();
        assert_eq!(a.h, 7);
        assert_eq!(b.h, 13);
    }

    #[test]
    fn split_returns_none_when_child_too_small() {
        let r = Rect { x: 0, y: 0, w: 10, h: 10 };
        assert!(split_room(r, true, 2, 5).is_none());
        assert!(split_room(r, true, 9, 5).is_none());
    }

    #[test]
    fn room_interior_shrinks_correctly() {
        let r = Rect { x: 2, y: 3, w: 10, h: 8 };
        let inner = room_interior(r, 1).unwrap();
        assert_eq!(inner.x, 3);
        assert_eq!(inner.y, 4);
        assert_eq!(inner.w, 8);
        assert_eq!(inner.h, 6);
    }

    #[test]
    fn room_interior_collapses_returns_none() {
        let r = Rect { x: 0, y: 0, w: 2, h: 2 };
        assert!(room_interior(r, 1).is_none());
    }

    #[test]
    fn lcg_next_advances_seed() {
        let s0 = 1234u64;
        let s1 = lcg_next(s0);
        assert_ne!(s0, s1);
        assert_ne!(s1, lcg_next(s1));
    }

    #[test]
    fn build_dungeon_produces_some_floor_tiles() {
        let grid = build_dungeon(99);
        let floors = grid.iter().flat_map(|r| r.iter()).filter(|&&t| t == Tile::Floor).count();
        assert!(floors > 20, "expected some floor tiles, got {}", floors);
    }
}
