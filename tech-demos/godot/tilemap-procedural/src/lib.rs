//! Tilemap Procedural demo — fills a TileMap from Rust using a cellular-automata
//! cave generator. Demonstrates how to drive Godot's TileMap API entirely from
//! Rust code, including per-cell writes and multi-step smoothing.

use godot::classes::{INode2D, Node2D, TileMap};
use godot::prelude::*;

struct TilemapProceduralExtension;
#[gdextension]
unsafe impl ExtensionLibrary for TilemapProceduralExtension {}

/// Root node that owns the procedural map data and writes it to a TileMap child.
#[derive(GodotClass)]
#[class(base=Node2D)]
struct TilemapProcedural {
    /// Width of the tile grid.
    #[export]
    map_width: i32,

    /// Height of the tile grid.
    #[export]
    map_height: i32,

    /// Probability (0–1) that any cell starts as a wall.
    #[export]
    fill_probability: f32,

    /// Number of cellular-automata smoothing passes.
    #[export]
    smoothing_steps: i32,

    /// Flat cell buffer: true = wall, false = floor.
    cells: Vec<bool>,

    /// Running seed so each regeneration is different.
    seed: u64,

    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for TilemapProcedural {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            map_width: 30,
            map_height: 20,
            fill_probability: 0.45,
            smoothing_steps: 4,
            cells: Vec::new(),
            seed: 12345,
            base,
        }
    }

    fn ready(&mut self) {
        self.generate_and_apply();
    }
}

#[godot_api]
impl TilemapProcedural {
    /// Re-run the cave generator with a new seed and update the TileMap.
    #[func]
    pub fn regenerate(&mut self) {
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.generate_and_apply();
    }
}

impl TilemapProcedural {
    /// Run the full CA pipeline and push results to the TileMap child.
    fn generate_and_apply(&mut self) {
        let w = self.map_width;
        let h = self.map_height;
        let prob = self.fill_probability;
        let steps = self.smoothing_steps;
        let seed = self.seed;

        let mut cells = random_fill(w, h, prob, seed);
        for _ in 0..steps {
            cells = smooth_step(&cells, w, h);
        }
        self.cells = cells;

        // Write to TileMap child (layer 0)
        let cells_snapshot = self.cells.clone();
        if let Some(mut tilemap) = self.base().try_get_node_as::<TileMap>("TileMap") {
            for y in 0..h {
                for x in 0..w {
                    let idx = (y * w + x) as usize;
                    let coords = Vector2i::new(x, y);
                    if cells_snapshot[idx] {
                        // Wall tile: source 0, atlas cell (0,0)
                        tilemap.set_cell_ex(0, coords)
                            .source_id(0)
                            .atlas_coords(Vector2i::new(0, 0))
                            .done();
                    } else {
                        // Floor: erase by using source_id -1
                        tilemap.set_cell_ex(0, coords)
                            .source_id(-1)
                            .atlas_coords(Vector2i::new(0, 0))
                            .done();
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Pure helper functions — fully testable without Godot runtime
// ---------------------------------------------------------------------------

/// LCG-based random fill. Returns a flat `w*h` buffer; border cells are always walls.
pub fn random_fill(w: i32, h: i32, prob: f32, seed: u64) -> Vec<bool> {
    let size = (w * h) as usize;
    let mut cells = vec![false; size];
    let mut state = seed;

    for y in 0..h {
        for x in 0..w {
            // Advance LCG
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);

            let is_border = x == 0 || x == w - 1 || y == 0 || y == h - 1;
            let rand_val = (state >> 33) as f32 / (u32::MAX as f32);
            let is_wall = is_border || rand_val < prob;
            cells[(y * w + x) as usize] = is_wall;
        }
    }
    cells
}

/// Count the 8-connected wall neighbours of (x, y). Out-of-bounds cells count as walls.
pub fn count_wall_neighbors(cells: &[bool], x: i32, y: i32, w: i32, h: i32) -> i32 {
    let mut count = 0;
    for dy in -1..=1_i32 {
        for dx in -1..=1_i32 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x + dx;
            let ny = y + dy;
            if nx < 0 || nx >= w || ny < 0 || ny >= h {
                // Treat out-of-bounds as wall
                count += 1;
            } else if cells[(ny * w + nx) as usize] {
                count += 1;
            }
        }
    }
    count
}

/// Apply one CA smoothing pass. Returns a new buffer.
/// Rule: >4 wall neighbours → wall; <4 → floor; ==4 → unchanged.
/// Border cells are always walls.
pub fn smooth_step(cells: &[bool], w: i32, h: i32) -> Vec<bool> {
    let mut next = vec![false; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            let is_border = x == 0 || x == w - 1 || y == 0 || y == h - 1;
            if is_border {
                next[idx] = true;
                continue;
            }
            let neighbours = count_wall_neighbors(cells, x, y, w, h);
            next[idx] = match neighbours.cmp(&4) {
                std::cmp::Ordering::Greater => true,
                std::cmp::Ordering::Less => false,
                std::cmp::Ordering::Equal => cells[idx],
            };
        }
    }
    next
}

/// Fraction of cells that are walls (0.0–1.0).
pub fn wall_fraction(cells: &[bool]) -> f32 {
    if cells.is_empty() {
        return 0.0;
    }
    let walls = cells.iter().filter(|&&c| c).count();
    walls as f32 / cells.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // random_fill
    // -----------------------------------------------------------------------

    #[test]
    fn random_fill_correct_size() {
        let cells = random_fill(10, 8, 0.5, 42);
        assert_eq!(cells.len(), 80);
    }

    #[test]
    fn random_fill_borders_are_walls() {
        let w = 10;
        let h = 8;
        let cells = random_fill(w, h, 0.0, 99); // prob=0 so interior would be floor
        for x in 0..w {
            assert!(cells[(0 * w + x) as usize], "top row x={x} should be wall");
            assert!(
                cells[((h - 1) * w + x) as usize],
                "bottom row x={x} should be wall"
            );
        }
        for y in 0..h {
            assert!(cells[(y * w + 0) as usize], "left col y={y} should be wall");
            assert!(
                cells[(y * w + (w - 1)) as usize],
                "right col y={y} should be wall"
            );
        }
    }

    #[test]
    fn random_fill_prob_one_all_walls() {
        let cells = random_fill(5, 5, 1.0, 1);
        assert!(cells.iter().all(|&c| c));
    }

    #[test]
    fn random_fill_prob_zero_only_border_walls() {
        let cells = random_fill(5, 5, 0.0, 1);
        // Interior cells (1..3) x (1..3) should be floor
        for y in 1..4 {
            for x in 1..4 {
                assert!(!cells[(y * 5 + x) as usize], "interior ({x},{y}) should be floor");
            }
        }
    }

    // -----------------------------------------------------------------------
    // count_wall_neighbors
    // -----------------------------------------------------------------------

    #[test]
    fn count_wall_neighbors_all_walls() {
        // 3×3 grid, all walls; center cell (1,1) has 8 wall neighbours
        let cells = vec![true; 9];
        assert_eq!(count_wall_neighbors(&cells, 1, 1, 3, 3), 8);
    }

    #[test]
    fn count_wall_neighbors_no_walls() {
        // 3×3 grid, all floor; center has 0 wall neighbours
        let cells = vec![false; 9];
        assert_eq!(count_wall_neighbors(&cells, 1, 1, 3, 3), 0);
    }

    #[test]
    fn count_wall_neighbors_corner_counts_oob_as_wall() {
        // Top-left corner (0,0) of a 3×3 all-floor grid.
        // 5 out-of-bounds neighbours are treated as walls.
        let cells = vec![false; 9];
        assert_eq!(count_wall_neighbors(&cells, 0, 0, 3, 3), 5);
    }

    // -----------------------------------------------------------------------
    // smooth_step
    // -----------------------------------------------------------------------

    #[test]
    fn smooth_step_border_always_wall() {
        // Start with all-floor interior (only border is wall via smooth_step rules)
        let cells = vec![false; 6 * 4];
        let next = smooth_step(&cells, 6, 4);
        for x in 0..6 {
            assert!(next[x], "top border x={x}");
            assert!(next[3 * 6 + x], "bottom border x={x}");
        }
        for y in 0..4 {
            assert!(next[y * 6], "left border y={y}");
            assert!(next[y * 6 + 5], "right border y={y}");
        }
    }

    #[test]
    fn smooth_step_all_walls_stays_all_walls() {
        let cells = vec![true; 10 * 10];
        let next = smooth_step(&cells, 10, 10);
        assert!(next.iter().all(|&c| c));
    }

    #[test]
    fn wall_fraction_empty_returns_zero() {
        assert!((wall_fraction(&[]) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn wall_fraction_all_walls() {
        let cells = vec![true; 20];
        assert!((wall_fraction(&cells) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn wall_fraction_half() {
        let cells: Vec<bool> = (0..10).map(|i| i % 2 == 0).collect();
        assert!((wall_fraction(&cells) - 0.5).abs() < 1e-6);
    }
}
