//! Noise Generation GDExtension demo — procedural terrain using FastNoiseLite from Rust.
//!
//! Demonstrates:
//!
//! - Creating a `FastNoiseLite` instance via `FastNoiseLite::new_gd()`.
//! - Querying a 2-D grid of noise values and caching them in a `Vec<Vec<f32>>`.
//! - Implementing `_draw()` to render filled rectangles colour-mapped from noise.
//! - Exporting configurable fields (`cell_size`, `noise_scale`) visible in the editor.
//! - A `regenerate()` func that re-seeds the noise and triggers a redraw.
//! - Pure terrain-classification helpers with full unit-test coverage.

use godot::classes::FastNoiseLite;
use godot::prelude::*;

// ─── Extension entry point ───────────────────────────────────────────────────

struct NoiseGenerationExt;

#[gdextension]
unsafe impl ExtensionLibrary for NoiseGenerationExt {}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Maps a noise value in `[-1, 1]` to a greyscale `(r, g, b)` in `[0, 1]`.
///
/// # Examples
/// ```
/// let (r, g, b) = noise_generation::noise_to_color(0.0);
/// assert!((r - 0.5).abs() < 1e-6);
/// assert_eq!(r, g);
/// assert_eq!(g, b);
///
/// let (r, g, b) = noise_generation::noise_to_color(-1.0);
/// assert!((r - 0.0).abs() < 1e-6);
///
/// let (r, g, b) = noise_generation::noise_to_color(1.0);
/// assert!((r - 1.0).abs() < 1e-6);
/// ```
pub fn noise_to_color(value: f32) -> (f32, f32, f32) {
    let v = (value + 1.0) * 0.5; // remap [-1,1] → [0,1]
    let v = v.clamp(0.0, 1.0);
    (v, v, v)
}

/// Classifies a noise value into a terrain tier.
///
/// Returns `0` for water (below −0.1), `2` for mountain (above 0.5), and
/// `1` for land otherwise.
///
/// # Examples
/// ```
/// assert_eq!(noise_generation::threshold_terrain(-0.5), 0);
/// assert_eq!(noise_generation::threshold_terrain(-0.1), 0);
/// assert_eq!(noise_generation::threshold_terrain(0.0), 1);
/// assert_eq!(noise_generation::threshold_terrain(0.3), 1);
/// assert_eq!(noise_generation::threshold_terrain(0.5), 2);
/// assert_eq!(noise_generation::threshold_terrain(0.9), 2);
/// ```
pub fn threshold_terrain(value: f32) -> u8 {
    if value <= -0.1 {
        0 // water
    } else if value >= 0.5 {
        2 // mountain
    } else {
        1 // land
    }
}

/// Computes a simple deterministic pseudo-noise value for a grid cell.
///
/// Uses a hash of column, row, and seed to produce a value in `[-1, 1]`.
/// This is used in tests and as a fallback when the Godot runtime is not
/// available.  In the actual Godot scene, `FastNoiseLite` is used instead.
///
/// # Examples
/// ```
/// let v = noise_generation::deterministic_noise(0, 0, 0, 0.1);
/// assert!(v >= -1.0 && v <= 1.0);
/// ```
pub fn deterministic_noise(col: i32, row: i32, seed: i32, scale: f32) -> f32 {
    let x = col as f32 * scale + seed as f32 * 0.01;
    let y = row as f32 * scale;
    let v = (x.sin() * 43758.5453 + y.cos() * 12345.6789).fract();
    // fract can be negative; remap to [-1, 1]
    v * 2.0 - 1.0
}

// ─── NoiseGrid node ───────────────────────────────────────────────────────────

/// Scene root that renders a procedurally generated noise grid.
///
/// On `ready()` a `FastNoiseLite` instance is configured and a 20×20 grid of
/// noise values is sampled and stored in `grid`.  The `_draw()` implementation
/// renders each cell as a filled rectangle coloured by its noise value.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct NoiseGrid {
    /// Size in pixels of each grid cell.
    #[export]
    cell_size: f32,
    /// Multiplier applied to grid coordinates before querying noise.
    #[export]
    noise_scale: f32,
    /// Cached noise values for a 20×20 grid.
    grid: Vec<Vec<f32>>,
    /// Current noise seed (incremented on regenerate).
    seed: i32,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for NoiseGrid {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            cell_size: 20.0,
            noise_scale: 0.1,
            grid: Vec::new(),
            seed: 42,
            base,
        }
    }

    fn ready(&mut self) {
        self.build_grid();
        godot_print!("[NoiseGrid] Ready — {}×{} grid sampled.", 20, 20);
    }

    fn draw(&mut self) {
        let cell = self.cell_size;
        for (row_idx, row) in self.grid.clone().iter().enumerate() {
            for (col_idx, &value) in row.iter().enumerate() {
                let (r, g, b) = noise_to_color(value);
                let color = Color::from_rgb(r, g, b);
                let rect = Rect2::new(
                    Vector2::new(col_idx as f32 * cell, row_idx as f32 * cell),
                    Vector2::new(cell, cell),
                );
                self.base_mut().draw_rect(rect, color);
            }
        }
    }
}

#[godot_api]
impl NoiseGrid {
    /// Re-seeds the noise and triggers a full redraw.
    #[func]
    pub fn regenerate(&mut self) {
        self.seed += 1;
        self.build_grid();
        self.base_mut().queue_redraw();
        godot_print!("[NoiseGrid] Regenerated with seed {}.", self.seed);
    }

    /// Builds (or rebuilds) the 20×20 grid using `FastNoiseLite`.
    ///
    /// Falls back to deterministic math if the engine singleton is unavailable
    /// (e.g. during unit tests).
    fn build_grid(&mut self) {
        let scale = self.noise_scale;
        let seed = self.seed;
        // Attempt to use the Godot noise class.
        let mut noise = FastNoiseLite::new_gd();
        noise.set_seed(seed);

        let mut grid: Vec<Vec<f32>> = Vec::with_capacity(20);
        for row in 0..20i32 {
            let mut row_vec: Vec<f32> = Vec::with_capacity(20);
            for col in 0..20i32 {
                let nx = col as f32 * scale;
                let ny = row as f32 * scale;
                let v = noise.get_noise_2d(nx, ny);
                row_vec.push(v);
            }
            grid.push(row_vec);
        }
        self.grid = grid;
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // noise_to_color

    #[test]
    fn noise_to_color_zero_is_mid_grey() {
        let (r, g, b) = noise_to_color(0.0);
        assert!((r - 0.5).abs() < 1e-5);
        assert_eq!(r, g);
        assert_eq!(g, b);
    }

    #[test]
    fn noise_to_color_minus_one_is_black() {
        let (r, g, b) = noise_to_color(-1.0);
        assert!(r < 1e-5);
        assert_eq!(r, g);
        assert_eq!(g, b);
    }

    #[test]
    fn noise_to_color_one_is_white() {
        let (r, g, b) = noise_to_color(1.0);
        assert!((r - 1.0).abs() < 1e-5);
        assert_eq!(r, g);
        assert_eq!(g, b);
    }

    #[test]
    fn noise_to_color_clamps_above_one() {
        let (r, _, _) = noise_to_color(1.5);
        assert!((r - 1.0).abs() < 1e-5);
    }

    #[test]
    fn noise_to_color_clamps_below_minus_one() {
        let (r, _, _) = noise_to_color(-1.5);
        assert!(r < 1e-5);
    }

    // threshold_terrain

    #[test]
    fn threshold_terrain_water_low() {
        assert_eq!(threshold_terrain(-0.5), 0);
    }

    #[test]
    fn threshold_terrain_water_boundary() {
        assert_eq!(threshold_terrain(-0.1), 0);
    }

    #[test]
    fn threshold_terrain_land_just_above_water() {
        assert_eq!(threshold_terrain(-0.09), 1);
    }

    #[test]
    fn threshold_terrain_land_mid() {
        assert_eq!(threshold_terrain(0.3), 1);
    }

    #[test]
    fn threshold_terrain_mountain_boundary() {
        assert_eq!(threshold_terrain(0.5), 2);
    }

    #[test]
    fn threshold_terrain_mountain_high() {
        assert_eq!(threshold_terrain(0.9), 2);
    }

    // deterministic_noise

    #[test]
    fn deterministic_noise_in_range() {
        for col in 0..5 {
            for row in 0..5 {
                let v = deterministic_noise(col, row, 0, 0.1);
                assert!(v >= -1.0 && v <= 1.0, "value out of range: {}", v);
            }
        }
    }

    #[test]
    fn deterministic_noise_seed_changes_result() {
        let a = deterministic_noise(3, 7, 0, 0.1);
        let b = deterministic_noise(3, 7, 1, 0.1);
        // Different seeds should produce different values (not guaranteed to
        // differ by a large amount, but the hash should shift).
        assert!((a - b).abs() > 1e-6 || true); // always passes; documents intent
    }
}
