//! Tilemap Basic GDExtension demo — reading and writing TileMap cells from Rust.
//!
//! Demonstrates:
//!
//! - Storing tile data in a `Vec<u8>` in Rust (0=empty, 1=floor, 2=wall).
//! - Generating a simple bordered room procedurally: walls on edges, floors inside.
//! - Looking up a `TileMap` child with `try_get_node_as` and calling `set_cell`.
//! - Exposing `get_tile_at` and `set_tile_at` funcs for GDScript interaction.
//! - Displaying tile count on a `Label` child.

use godot::classes::{INode2D, Label, Node2D, TileMap};
use godot::prelude::*;

// ─── Extension entry point ───────────────────────────────────────────────────

struct TilemapBasicExt;

#[gdextension]
unsafe impl ExtensionLibrary for TilemapBasicExt {}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Generates a bordered room layout as a flat `Vec<u8>`.
///
/// Cells on the border (x==0, x==w-1, y==0, y==h-1) are walls (`2`).
/// Interior cells are floors (`1`).
///
/// # Examples
/// ```
/// let data = tilemap_basic::generate_room(5, 4);
/// assert_eq!(data.len(), 20);
/// // Corners are walls.
/// assert_eq!(data[0], 2);
/// // Interior is floor.
/// assert_eq!(data[tilemap_basic::flat_idx(1, 1, 5)], 1);
/// ```
pub fn generate_room(w: i32, h: i32) -> Vec<u8> {
    let mut data = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            if x == 0 || x == w - 1 || y == 0 || y == h - 1 {
                data.push(2); // wall
            } else {
                data.push(1); // floor
            }
        }
    }
    data
}

/// Returns a human-readable name for a tile type byte.
///
/// # Examples
/// ```
/// assert_eq!(tilemap_basic::tile_name(0), "empty");
/// assert_eq!(tilemap_basic::tile_name(1), "floor");
/// assert_eq!(tilemap_basic::tile_name(2), "wall");
/// assert_eq!(tilemap_basic::tile_name(99), "unknown");
/// ```
pub fn tile_name(t: u8) -> &'static str {
    match t {
        0 => "empty",
        1 => "floor",
        2 => "wall",
        _ => "unknown",
    }
}

/// Returns `true` if `(x, y)` is within the map bounds.
///
/// # Examples
/// ```
/// assert!(tilemap_basic::coords_in_bounds(0, 0, 10, 10));
/// assert!(tilemap_basic::coords_in_bounds(9, 9, 10, 10));
/// assert!(!tilemap_basic::coords_in_bounds(10, 0, 10, 10));
/// assert!(!tilemap_basic::coords_in_bounds(0, -1, 10, 10));
/// ```
pub fn coords_in_bounds(x: i32, y: i32, w: i32, h: i32) -> bool {
    x >= 0 && y >= 0 && x < w && y < h
}

/// Converts a 2D grid coordinate to a flat array index.
///
/// # Examples
/// ```
/// assert_eq!(tilemap_basic::flat_idx(3, 2, 10), 23);
/// assert_eq!(tilemap_basic::flat_idx(0, 0, 10), 0);
/// ```
pub fn flat_idx(x: i32, y: i32, w: i32) -> usize {
    (y * w + x) as usize
}

// ─── TilemapDemo node ─────────────────────────────────────────────────────────

/// Scene root that generates a bordered room and syncs it to a `TileMap` child.
///
/// The tile data is stored as a `Vec<u8>` in Rust for fast pure-logic queries.
/// The `TileMap` child (named `"TileMap"`) is updated via `set_cell`.
///
/// Tile type mapping: `0`=empty, `1`=floor (source_id=0), `2`=wall (source_id=1).
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct TilemapDemo {
    /// Width of the tile grid in cells.
    #[export]
    map_width: i32,
    /// Height of the tile grid in cells.
    #[export]
    map_height: i32,
    /// Flat row-major tile data: 0=empty, 1=floor, 2=wall.
    tile_data: Vec<u8>,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for TilemapDemo {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            map_width: 20,
            map_height: 15,
            tile_data: Vec::new(),
            base,
        }
    }

    fn ready(&mut self) {
        let w = self.map_width;
        let h = self.map_height;
        self.tile_data = generate_room(w, h);

        // Sync tile_data to TileMap child.
        self.sync_tilemap();

        let total = w * h;
        let text = format!("Tiles: {}", total);
        if let Some(mut label) = self.base_mut().try_get_node_as::<Label>("Label") {
            label.set_text(text.as_str());
        }

        godot_print!("[TilemapDemo] Ready — {}×{} grid generated.", w, h);
    }
}

#[godot_api]
impl TilemapDemo {
    /// Returns the tile type at `(x, y)`, or `-1` if out of bounds.
    #[func]
    pub fn get_tile_at(&self, x: i32, y: i32) -> i32 {
        if !coords_in_bounds(x, y, self.map_width, self.map_height) {
            return -1;
        }
        let idx = flat_idx(x, y, self.map_width);
        self.tile_data.get(idx).copied().unwrap_or(0) as i32
    }

    /// Sets the tile at `(x, y)` to `tile_type` and updates the TileMap child.
    #[func]
    pub fn set_tile_at(&mut self, x: i32, y: i32, tile_type: i32) {
        if !coords_in_bounds(x, y, self.map_width, self.map_height) {
            return;
        }
        let idx = flat_idx(x, y, self.map_width);
        if let Some(cell) = self.tile_data.get_mut(idx) {
            *cell = tile_type.clamp(0, 2) as u8;
        }
        let source_id = if tile_type > 0 { tile_type - 1 } else { -1 };
        if let Some(mut tilemap) = self.base_mut().try_get_node_as::<TileMap>("TileMap") {
            tilemap.set_cell_ex(0, Vector2i::new(x, y))
                .source_id(source_id)
                .done();
        }
    }

    /// Writes all `tile_data` entries to the `TileMap` child.
    fn sync_tilemap(&mut self) {
        let w = self.map_width;
        let h = self.map_height;
        let data = self.tile_data.clone();

        if let Some(mut tilemap) = self.base_mut().try_get_node_as::<TileMap>("TileMap") {
            for y in 0..h {
                for x in 0..w {
                    let idx = flat_idx(x, y, w);
                    let tile_type = data.get(idx).copied().unwrap_or(0);
                    // source_id: -1 = erase, 0 = floor, 1 = wall
                    let source_id: i32 = match tile_type {
                        1 => 0,
                        2 => 1,
                        _ => -1,
                    };
                    tilemap.set_cell_ex(0, Vector2i::new(x, y))
                        .source_id(source_id)
                        .done();
                }
            }
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // generate_room ───────────────────────────────────────────────────────────

    #[test]
    fn generate_room_correct_size() {
        let data = generate_room(10, 8);
        assert_eq!(data.len(), 80);
    }

    #[test]
    fn generate_room_top_edge_is_wall() {
        let data = generate_room(5, 4);
        for x in 0..5 {
            assert_eq!(data[flat_idx(x, 0, 5)], 2, "top edge x={} should be wall", x);
        }
    }

    #[test]
    fn generate_room_bottom_edge_is_wall() {
        let data = generate_room(5, 4);
        for x in 0..5 {
            assert_eq!(data[flat_idx(x, 3, 5)], 2, "bottom edge x={} should be wall", x);
        }
    }

    #[test]
    fn generate_room_left_edge_is_wall() {
        let data = generate_room(5, 4);
        for y in 0..4 {
            assert_eq!(data[flat_idx(0, y, 5)], 2, "left edge y={} should be wall", y);
        }
    }

    #[test]
    fn generate_room_interior_is_floor() {
        let data = generate_room(6, 5);
        assert_eq!(data[flat_idx(1, 1, 6)], 1);
        assert_eq!(data[flat_idx(2, 2, 6)], 1);
        assert_eq!(data[flat_idx(4, 3, 6)], 1);
    }

    // tile_name ───────────────────────────────────────────────────────────────

    #[test]
    fn tile_name_empty() {
        assert_eq!(tile_name(0), "empty");
    }

    #[test]
    fn tile_name_floor() {
        assert_eq!(tile_name(1), "floor");
    }

    #[test]
    fn tile_name_wall() {
        assert_eq!(tile_name(2), "wall");
    }

    #[test]
    fn tile_name_unknown() {
        assert_eq!(tile_name(99), "unknown");
    }

    // coords_in_bounds ────────────────────────────────────────────────────────

    #[test]
    fn coords_in_bounds_origin() {
        assert!(coords_in_bounds(0, 0, 10, 10));
    }

    #[test]
    fn coords_in_bounds_max_corner() {
        assert!(coords_in_bounds(9, 9, 10, 10));
    }

    #[test]
    fn coords_out_of_bounds_x() {
        assert!(!coords_in_bounds(10, 5, 10, 10));
    }

    #[test]
    fn coords_out_of_bounds_negative() {
        assert!(!coords_in_bounds(-1, 0, 10, 10));
    }

    // flat_idx ────────────────────────────────────────────────────────────────

    #[test]
    fn flat_idx_origin() {
        assert_eq!(flat_idx(0, 0, 10), 0);
    }

    #[test]
    fn flat_idx_second_row() {
        assert_eq!(flat_idx(3, 2, 10), 23);
    }

    #[test]
    fn flat_idx_end_of_row() {
        assert_eq!(flat_idx(9, 0, 10), 9);
    }
}
