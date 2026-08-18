//! Water-ripple simulation demo — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`WaterRipplePlugin`] into any Bevy
//! app with `app.add_plugins(WaterRipplePlugin)` and it renders a grid of cells
//! running a discrete wave-equation simulation you can disturb by clicking.
//! Tune the grid size, damping, and click strength through [`WaterRippleConfig`].
//!
//! Key ideas:
//! - The discrete wave equation `next[i] = 2·curr[i] − prev[i] + c²·Δh`
//!   propagates height values across a 2-D grid.
//! - [`ripple_step`] computes one cell's next height from its current neighbours,
//!   its previous value, and a damping factor.
//! - [`height_to_color`] maps `h ∈ [-1, 1]` to a blue-tinted colour.
//! - The simulation runs on a fixed-timestep schedule (FixedUpdate) so the wave
//!   speed is frame-rate independent.
//! - Clicking the screen adds a disturbance at the nearest grid cell.
//!
//! **Controls:** left-click — add ripple.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use water_ripple::WaterRipplePlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(WaterRipplePlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system and resource for the water-ripple feature.
///
/// Add it with `app.add_plugins(WaterRipplePlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct WaterRipplePlugin;

impl Plugin for WaterRipplePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WaterRippleConfig>()
            .init_resource::<WaterGrid>()
            .add_systems(Startup, setup)
            .add_systems(FixedUpdate, step_simulation)
            .add_systems(Update, (handle_click, sync_colors).chain());
    }
}

// ── Configuration ─────────────────────────────────────────────────────────────

/// Default grid columns.
pub const COLS: usize = 80;
/// Default grid rows.
pub const ROWS: usize = 60;

/// Tunable parameters for the ripple simulation. Override before adding the
/// plugin, e.g. `app.insert_resource(WaterRippleConfig { damping: 0.95, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct WaterRippleConfig {
    /// Grid columns (horizontal cells).
    pub cols: usize,
    /// Grid rows (vertical cells).
    pub rows: usize,
    /// Damping factor in `[0, 1)`; higher = ripples persist longer.
    pub damping: f32,
    /// Height added to the clicked cell.
    pub disturb_strength: f32,
}

impl Default for WaterRippleConfig {
    fn default() -> Self {
        Self {
            cols: COLS,
            rows: ROWS,
            damping: 0.985,
            disturb_strength: 2.0,
        }
    }
}

// ── Pure simulation ───────────────────────────────────────────────────────────

/// One discrete wave-equation step for a single cell.
///
/// `curr` is the cell's current height; `prev` is its height last step.
/// `n`, `s`, `e`, `w` are the four neighbours' current heights.
/// `damp` in `[0, 1)` bleeds energy away each step; 0.99 gives gentle decay.
pub fn ripple_step(curr: f32, prev: f32, n: f32, s: f32, e: f32, w: f32, damp: f32) -> f32 {
    let laplacian = n + s + e + w - 4.0 * curr;
    (2.0 * curr - prev + 0.5 * laplacian) * damp
}

/// Maps `h ∈ [-1, 1]` to a water colour: deep blue at 0, white crest, dark trough.
pub fn height_to_color(h: f32) -> Color {
    let t = h.clamp(-1.0, 1.0);
    if t >= 0.0 {
        // crest: blue → white
        Color::srgb(t * 0.6, t * 0.75 + 0.25, 0.55 + t * 0.45)
    } else {
        // trough: blue → dark
        let d = 1.0 + t; // 0..1 from deepest to neutral
        Color::srgb(0.0, d * 0.25, d * 0.55)
    }
}

// ── Simulation grid ───────────────────────────────────────────────────────────

/// Height field for the wave simulation, double-buffered across frames.
#[derive(Resource)]
pub struct WaterGrid {
    /// Grid width in cells.
    pub cols: usize,
    /// Grid height in cells.
    pub rows: usize,
    /// Surface height this frame, row-major.
    pub curr: Vec<f32>,
    /// Surface height last frame; the wave equation needs both.
    pub prev: Vec<f32>,
}

impl WaterGrid {
    /// Creates a flat, still surface of the given size.
    pub fn new(cols: usize, rows: usize) -> Self {
        let n = cols * rows;
        Self {
            cols,
            rows,
            curr: vec![0.0; n],
            prev: vec![0.0; n],
        }
    }

    /// Row-major index of a cell.
    pub fn idx(&self, col: usize, row: usize) -> usize {
        row * self.cols + col
    }

    /// Pushes a cell down by `strength`, starting a ripple.
    pub fn disturb(&mut self, col: usize, row: usize, strength: f32) {
        if col < self.cols && row < self.rows {
            let i = self.idx(col, row);
            self.curr[i] += strength;
        }
    }
}

impl FromWorld for WaterGrid {
    fn from_world(world: &mut World) -> Self {
        let config = world
            .get_resource::<WaterRippleConfig>()
            .copied()
            .unwrap_or_default();
        WaterGrid::new(config.cols, config.rows)
    }
}

// ── ECS ───────────────────────────────────────────────────────────────────────

/// Marks a tile sprite with the grid cell it draws, as `(col, row)`.
#[derive(Component)]
pub struct Cell(pub usize, pub usize); // (col, row)

fn setup(mut commands: Commands, grid: Res<WaterGrid>) {
    commands.spawn(Camera2d);

    commands.spawn((
        Text::new("left-click - add ripple"),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::srgb(0.7, 0.7, 0.7)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));

    let cell_w = 800.0 / grid.cols as f32;
    let cell_h = 600.0 / grid.rows as f32;
    let origin_x = -400.0 + cell_w * 0.5;
    let origin_y = -300.0 + cell_h * 0.5;

    for row in 0..grid.rows {
        for col in 0..grid.cols {
            commands.spawn((
                Sprite {
                    color: height_to_color(0.0),
                    custom_size: Some(Vec2::new(cell_w, cell_h)),
                    ..default()
                },
                Transform::from_xyz(
                    origin_x + col as f32 * cell_w,
                    origin_y + row as f32 * cell_h,
                    0.0,
                ),
                Cell(col, row),
            ));
        }
    }
}

fn step_simulation(mut grid: ResMut<WaterGrid>, config: Res<WaterRippleConfig>) {
    let cols = grid.cols;
    let rows = grid.rows;
    let mut next = vec![0.0f32; cols * rows];

    for row in 1..rows - 1 {
        for col in 1..cols - 1 {
            let curr = grid.curr[row * cols + col];
            let prev = grid.prev[row * cols + col];
            let n = grid.curr[(row + 1) * cols + col];
            let s = grid.curr[(row - 1) * cols + col];
            let e = grid.curr[row * cols + col + 1];
            let w = grid.curr[row * cols + col - 1];
            next[row * cols + col] = ripple_step(curr, prev, n, s, e, w, config.damping);
        }
    }

    grid.prev = std::mem::replace(&mut grid.curr, next);
}

fn handle_click(
    buttons: Res<ButtonInput<MouseButton>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    windows: Query<&Window>,
    config: Res<WaterRippleConfig>,
    mut grid: ResMut<WaterGrid>,
) {
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

    let cell_w = 800.0 / grid.cols as f32;
    let cell_h = 600.0 / grid.rows as f32;
    let col = ((world.x + 400.0) / cell_w) as usize;
    let row = ((world.y + 300.0) / cell_h) as usize;
    grid.disturb(col, row, config.disturb_strength);
}

fn sync_colors(grid: Res<WaterGrid>, mut cells: Query<(&Cell, &mut Sprite)>) {
    if !grid.is_changed() {
        return;
    }
    for (cell, mut sprite) in &mut cells {
        let h = grid.curr[grid.idx(cell.0, cell.1)];
        sprite.color = height_to_color(h);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ripple_step_still_water_stays_still() {
        // All zeros, no disturbance → should stay zero
        let h = ripple_step(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.99);
        assert!(h.abs() < 1e-6);
    }

    #[test]
    fn ripple_step_crest_spreads_outward() {
        // A cell at +1.0 surrounded by 0 neighbours should push outward
        let h = ripple_step(1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
        assert!(h < 1.0, "crest should collapse, got {h}");
    }

    #[test]
    fn ripple_step_damping_reduces_amplitude() {
        let undamped = ripple_step(1.0, 0.0, 0.5, 0.5, 0.5, 0.5, 1.0);
        let damped = ripple_step(1.0, 0.0, 0.5, 0.5, 0.5, 0.5, 0.9);
        assert!(damped.abs() < undamped.abs());
    }

    #[test]
    fn height_to_color_zero_is_mid_blue() {
        let c = height_to_color(0.0);
        let lin = c.to_linear();
        assert!(lin.blue > lin.red, "zero height should be blue-dominant");
    }

    #[test]
    fn height_to_color_crest_is_bright() {
        let c = height_to_color(1.0);
        let dark = height_to_color(0.0);
        let lc = c.to_linear();
        let ld = dark.to_linear();
        assert!(lc.red + lc.green + lc.blue > ld.red + ld.green + ld.blue);
    }

    #[test]
    fn height_to_color_trough_is_dark() {
        let trough = height_to_color(-1.0);
        let neutral = height_to_color(0.0);
        let lt = trough.to_linear();
        let ln = neutral.to_linear();
        assert!(lt.red + lt.green + lt.blue < ln.red + ln.green + ln.blue);
    }

    #[test]
    fn water_grid_disturb_sets_height() {
        let mut g = WaterGrid::new(10, 10);
        g.disturb(5, 5, 1.5);
        assert!((g.curr[5 * 10 + 5] - 1.5).abs() < 1e-5);
    }

    #[test]
    fn water_grid_disturb_oob_does_not_panic() {
        let mut g = WaterGrid::new(10, 10);
        g.disturb(99, 99, 1.0); // should silently do nothing
    }

    #[test]
    fn water_grid_idx_is_row_major() {
        let g = WaterGrid::new(10, 10);
        assert_eq!(g.idx(3, 2), 23);
    }

    #[test]
    fn config_default_matches_documented_values() {
        let c = WaterRippleConfig::default();
        assert_eq!(c.cols, COLS);
        assert_eq!(c.rows, ROWS);
        assert!((c.damping - 0.985).abs() < 1e-6);
        assert!((c.disturb_strength - 2.0).abs() < 1e-6);
    }

    #[test]
    fn water_grid_from_world_uses_config_dimensions() {
        let mut app = App::new();
        app.insert_resource(WaterRippleConfig {
            cols: 12,
            rows: 7,
            ..default()
        })
        .init_resource::<WaterGrid>();
        let grid = app.world().resource::<WaterGrid>();
        assert_eq!(grid.cols, 12);
        assert_eq!(grid.rows, 7);
        assert_eq!(grid.curr.len(), 12 * 7);
    }
}
