//! Water-ripple simulation demo — Bevy 0.18.
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

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Water Ripple".into(),
                resolution: (800u32, 600u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(WaterGrid::new(COLS, ROWS))
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, step_simulation)
        .add_systems(Update, (handle_click, sync_colors).chain())
        .run();
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

const COLS: usize = 80;
const ROWS: usize = 60;

#[derive(Resource)]
pub struct WaterGrid {
    pub cols: usize,
    pub rows: usize,
    pub curr: Vec<f32>,
    pub prev: Vec<f32>,
}

impl WaterGrid {
    pub fn new(cols: usize, rows: usize) -> Self {
        let n = cols * rows;
        Self { cols, rows, curr: vec![0.0; n], prev: vec![0.0; n] }
    }

    pub fn idx(&self, col: usize, row: usize) -> usize {
        row * self.cols + col
    }

    pub fn disturb(&mut self, col: usize, row: usize, strength: f32) {
        if col < self.cols && row < self.rows {
            let i = self.idx(col, row);
            self.curr[i] += strength;
        }
    }
}

// ── ECS ───────────────────────────────────────────────────────────────────────

#[derive(Component)]
struct Cell(usize, usize); // (col, row)

fn setup(mut commands: Commands, grid: Res<WaterGrid>) {
    commands.spawn(Camera2d);

    commands.spawn((
        Text::new("left-click — add ripple"),
        TextFont { font_size: 15.0, ..default() },
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

fn step_simulation(mut grid: ResMut<WaterGrid>) {
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
            next[row * cols + col] = ripple_step(curr, prev, n, s, e, w, 0.985);
        }
    }

    grid.prev = std::mem::replace(&mut grid.curr, next);
}

fn handle_click(
    buttons: Res<ButtonInput<MouseButton>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    windows: Query<&Window>,
    mut grid: ResMut<WaterGrid>,
) {
    if !buttons.just_pressed(MouseButton::Left) { return; }
    let Ok((camera, cam_tf)) = camera_q.single() else { return };
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let Ok(world) = camera.viewport_to_world_2d(cam_tf, cursor) else { return };

    let cell_w = 800.0 / grid.cols as f32;
    let cell_h = 600.0 / grid.rows as f32;
    let col = ((world.x + 400.0) / cell_w) as usize;
    let row = ((world.y + 300.0) / cell_h) as usize;
    grid.disturb(col, row, 2.0);
}

fn sync_colors(
    grid: Res<WaterGrid>,
    mut cells: Query<(&Cell, &mut Sprite)>,
) {
    if !grid.is_changed() { return; }
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
        let damped   = ripple_step(1.0, 0.0, 0.5, 0.5, 0.5, 0.5, 0.9);
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
}
