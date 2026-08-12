//! # Flow-Field Pathfinding — a reusable Bevy plugin.
//!
//! Instead of running A* per agent, this *building block* pre-computes a "flow
//! field" — a direction vector at every grid cell pointing toward a goal — so
//! thousands of agents can look up their cell's direction in O(1) per frame.
//! The field itself is recomputed with a single BFS from the goal outward,
//! costing O(grid size) once per goal/wall change.
//!
//! Drop [`FlowFieldPathfindingPlugin`] into any Bevy app with
//! `app.add_plugins(FlowFieldPathfindingPlugin)` and it manages a grid, a goal,
//! walls, and agents that follow the field. The plugin never adds
//! `DefaultPlugins`, so the host app stays in control of window/rendering setup.
//!
//! ## Controls
//! - **Left-click** a cell to move the goal there.
//! - **Right-click** a cell to toggle it as a wall.
//!
//! ## Layout
//! - 30 × 20 tile grid, each tile 30 × 30 px (window 900 × 600).
//! - 50 orange agent entities that follow the flow field toward the goal.
//! - Cells are tinted by flow direction angle for visual debugging.
//!
//! Grid dimensions are compile-time [`COLS`]/[`ROWS`]/[`CELL_PX`] constants so
//! the pure [`compute_flow_field`]/[`world_to_cell`]/[`cell_to_world`] helpers
//! stay unit-testable without a `World`.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use flow_field_pathfinding::FlowFieldPathfindingPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(FlowFieldPathfindingPlugin)
//!     .run();
//! ```

use bevy::prelude::*;
use std::collections::VecDeque;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Number of grid columns.
pub const COLS: usize = 30;
/// Number of grid rows.
pub const ROWS: usize = 20;
/// Side length of a single square cell, in world pixels.
pub const CELL_PX: f32 = 30.0;
/// Agent movement speed, in world pixels per second.
pub const AGENT_SPEED: f32 = 60.0;
/// Number of agents spawned on the grid.
pub const AGENT_COUNT: usize = 50;
/// Side length of an agent sprite, in world pixels.
pub const AGENT_SIZE: f32 = 10.0;

/// World-space origin of the grid (top-left corner of cell (0,0)).
pub const GRID_ORIGIN: Vec2 = Vec2::new(
    -(COLS as f32 * CELL_PX / 2.0),
    -(ROWS as f32 * CELL_PX / 2.0),
);

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// Bundles every system and resource for the flow-field pathfinding feature.
///
/// Add it with `app.add_plugins(FlowFieldPathfindingPlugin)`. It does **not**
/// add `DefaultPlugins`, so the host app stays in control of window and
/// rendering setup.
pub struct FlowFieldPathfindingPlugin;

impl Plugin for FlowFieldPathfindingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GridState>()
            .add_systems(Startup, (setup_camera, setup_grid, setup_agents))
            .add_systems(
                Update,
                (
                    handle_mouse_input,
                    recompute_field_if_dirty,
                    update_tile_colors,
                    move_agents,
                )
                    .chain(),
            );
    }
}

// ---------------------------------------------------------------------------
// Pure functions (no Bevy types in parameters except Vec2 which is math)
// ---------------------------------------------------------------------------

/// Computes a flow field on a `width × height` grid using reverse BFS from the goal.
///
/// `walls`: flat array of bools, index = `y * width + x`; `true` means impassable.
/// `goal`: `(col, row)` destination cell.
///
/// Returns a flat `Vec<Option<(i32, i32)>>` where each entry is the `(dx, dy)`
/// step toward the goal, or `None` for unreachable / wall cells.
/// The goal cell itself is `None` (agents are already there).
pub fn compute_flow_field(
    walls: &[bool],
    width: usize,
    height: usize,
    goal: (usize, usize),
) -> Vec<Option<(i32, i32)>> {
    let total = width * height;
    let mut field: Vec<Option<(i32, i32)>> = vec![None; total];
    // dist[i] = Some(steps) once visited
    let mut dist: Vec<Option<u32>> = vec![None; total];

    let (gx, gy) = goal;
    if gx >= width || gy >= height {
        return field;
    }
    let goal_idx = gy * width + gx;
    if walls[goal_idx] {
        return field;
    }

    // BFS from goal outward
    let mut queue: VecDeque<(usize, usize)> = VecDeque::new();
    dist[goal_idx] = Some(0);
    queue.push_back((gx, gy));

    let neighbors: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

    while let Some((cx, cy)) = queue.pop_front() {
        let cd = dist[cy * width + cx].unwrap();
        for (dx, dy) in neighbors {
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                continue;
            }
            let nx = nx as usize;
            let ny = ny as usize;
            let ni = ny * width + nx;
            if walls[ni] || dist[ni].is_some() {
                continue;
            }
            dist[ni] = Some(cd + 1);
            // direction FROM (nx,ny) TOWARD goal is (-dx, -dy)
            field[ni] = Some((-dx, -dy));
            queue.push_back((nx, ny));
        }
    }

    field
}

/// Returns the `(col, row)` cell that contains world position `pos`.
/// Returns `None` if `pos` is outside the grid.
pub fn world_to_cell(pos: Vec2, cell_px: f32, grid_origin: Vec2) -> Option<(usize, usize)> {
    let local = pos - grid_origin;
    if local.x < 0.0 || local.y < 0.0 {
        return None;
    }
    let col = (local.x / cell_px) as usize;
    let row = (local.y / cell_px) as usize;
    Some((col, row))
}

/// Returns the world-space center of cell `(col, row)`.
pub fn cell_to_world(col: usize, row: usize, cell_px: f32, grid_origin: Vec2) -> Vec2 {
    Vec2::new(
        grid_origin.x + col as f32 * cell_px + cell_px * 0.5,
        grid_origin.y + row as f32 * cell_px + cell_px * 0.5,
    )
}

/// Maps a flow direction `(dx, dy)` to a debug color for rendering.
pub fn direction_to_color(dir: Option<(i32, i32)>) -> Color {
    match dir {
        None => Color::srgb(0.15, 0.15, 0.15),
        Some((1, 0)) => Color::srgb(0.8, 0.2, 0.2),
        Some((-1, 0)) => Color::srgb(0.2, 0.5, 0.8),
        Some((0, 1)) => Color::srgb(0.2, 0.8, 0.2),
        Some((0, -1)) => Color::srgb(0.8, 0.8, 0.2),
        Some(_) => Color::srgb(0.5, 0.5, 0.5),
    }
}

// ---------------------------------------------------------------------------
// ECS Components & Resources
// ---------------------------------------------------------------------------

/// Marks an entity as a grid cell tile; stores its `(col, row)` address.
#[derive(Component)]
struct Tile {
    col: usize,
    row: usize,
}

/// Marks an entity as an agent navigating the flow field.
#[derive(Component)]
pub struct Agent;

/// Current target world-position the agent is moving toward (the next cell center).
#[derive(Component)]
pub struct AgentTarget(pub Vec2);

/// Marker for the green goal sprite so we can reposition it.
#[derive(Component)]
struct GoalMarker;

/// Stores the current state of the grid.
#[derive(Resource)]
pub struct GridState {
    /// Flat wall array, index = `row * COLS + col`; `true` means impassable.
    pub walls: Vec<bool>,
    /// The current goal cell as `(col, row)`.
    pub goal: (usize, usize),
    /// Flat flow field: one entry per cell.
    pub field: Vec<Option<(i32, i32)>>,
    /// Set to true when the field needs recomputation.
    pub dirty: bool,
}

impl Default for GridState {
    fn default() -> Self {
        let total = COLS * ROWS;
        let walls = vec![false; total];
        let goal = (COLS / 2, ROWS / 2);
        let field = compute_flow_field(&walls, COLS, ROWS, goal);
        Self {
            walls,
            goal,
            field,
            dirty: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Startup systems
// ---------------------------------------------------------------------------

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn setup_grid(mut commands: Commands, state: Res<GridState>) {
    for row in 0..ROWS {
        for col in 0..COLS {
            let center = cell_to_world(col, row, CELL_PX, GRID_ORIGIN);
            let field_idx = row * COLS + col;
            let color = direction_to_color(state.field[field_idx]);

            commands.spawn((
                Tile { col, row },
                Sprite {
                    color,
                    custom_size: Some(Vec2::splat(CELL_PX - 1.0)),
                    ..default()
                },
                Transform::from_translation(center.extend(-1.0)),
            ));
        }
    }

    // Goal marker (green overlay)
    let (gc, gr) = state.goal;
    let goal_center = cell_to_world(gc, gr, CELL_PX, GRID_ORIGIN);
    commands.spawn((
        GoalMarker,
        Sprite {
            color: Color::srgb(0.1, 0.9, 0.1),
            custom_size: Some(Vec2::splat(CELL_PX - 4.0)),
            ..default()
        },
        Transform::from_translation(goal_center.extend(0.5)),
    ));

    // HUD text
    commands.spawn((
        Text::new("Left-click: move goal  |  Right-click: toggle wall"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(6.0),
            left: Val::Px(8.0),
            ..default()
        },
    ));
}

fn setup_agents(mut commands: Commands, state: Res<GridState>) {
    use std::collections::HashSet;
    let mut rng_seed: u64 = 12345;
    let mut occupied: HashSet<(usize, usize)> = HashSet::new();
    occupied.insert(state.goal);

    let mut spawned = 0;
    while spawned < AGENT_COUNT {
        // Simple LCG random
        rng_seed = rng_seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let col = (rng_seed >> 33) as usize % COLS;
        rng_seed = rng_seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let row = (rng_seed >> 33) as usize % ROWS;

        if state.walls[row * COLS + col] || occupied.contains(&(col, row)) {
            continue;
        }
        occupied.insert((col, row));

        let pos = cell_to_world(col, row, CELL_PX, GRID_ORIGIN);
        commands.spawn((
            Agent,
            AgentTarget(pos),
            Sprite {
                color: Color::srgb(1.0, 0.5, 0.1),
                custom_size: Some(Vec2::splat(AGENT_SIZE)),
                ..default()
            },
            Transform::from_translation(pos.extend(1.0)),
        ));
        spawned += 1;
    }
}

// ---------------------------------------------------------------------------
// Update systems
// ---------------------------------------------------------------------------

fn handle_mouse_input(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut state: ResMut<GridState>,
    mut goal_marker: Query<&mut Transform, With<GoalMarker>>,
) {
    let Ok(window) = windows.single() else { return };
    let Ok((camera, cam_transform)) = camera_q.single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos) else {
        return;
    };

    let Some((col, row)) = world_to_cell(world_pos, CELL_PX, GRID_ORIGIN) else {
        return;
    };
    if col >= COLS || row >= ROWS {
        return;
    }

    if buttons.just_pressed(MouseButton::Left) {
        // Move goal
        let idx = row * COLS + col;
        if !state.walls[idx] {
            state.goal = (col, row);
            state.dirty = true;
            if let Ok(mut t) = goal_marker.single_mut() {
                t.translation = cell_to_world(col, row, CELL_PX, GRID_ORIGIN).extend(0.5);
            }
        }
    }

    if buttons.just_pressed(MouseButton::Right) {
        // Toggle wall (cannot wall the goal cell)
        if (col, row) != state.goal {
            let idx = row * COLS + col;
            state.walls[idx] = !state.walls[idx];
            state.dirty = true;
        }
    }
}

fn recompute_field_if_dirty(mut state: ResMut<GridState>) {
    if state.dirty {
        state.field = compute_flow_field(&state.walls, COLS, ROWS, state.goal);
        state.dirty = false;
    }
}

fn update_tile_colors(state: Res<GridState>, mut tiles: Query<(&Tile, &mut Sprite)>) {
    for (tile, mut sprite) in &mut tiles {
        let idx = tile.row * COLS + tile.col;
        if state.walls[idx] {
            sprite.color = Color::srgb(0.6, 0.1, 0.1);
        } else if (tile.col, tile.row) == state.goal {
            sprite.color = Color::srgb(0.1, 0.5, 0.1);
        } else {
            sprite.color = direction_to_color(state.field[idx]);
        }
    }
}

fn move_agents(
    state: Res<GridState>,
    time: Res<Time>,
    mut agents: Query<(&mut Transform, &mut AgentTarget), With<Agent>>,
) {
    let dt = time.delta_secs();
    for (mut transform, mut target) in &mut agents {
        let pos2 = transform.translation.truncate();
        let to_target = target.0 - pos2;
        let dist = to_target.length();

        if dist < 1.5 {
            // Snap to cell center and pick next cell from field
            transform.translation = target.0.extend(1.0);
            let cell = world_to_cell(target.0, CELL_PX, GRID_ORIGIN);
            if let Some((col, row)) = cell
                && col < COLS
                && row < ROWS
            {
                let idx = row * COLS + col;
                if let Some((dx, dy)) = state.field[idx] {
                    let ncol = col as i32 + dx;
                    let nrow = row as i32 + dy;
                    if ncol >= 0 && nrow >= 0 && ncol < COLS as i32 && nrow < ROWS as i32 {
                        target.0 =
                            cell_to_world(ncol as usize, nrow as usize, CELL_PX, GRID_ORIGIN);
                    }
                }
            }
        } else {
            let dir = to_target / dist;
            transform.translation += (dir * AGENT_SPEED * dt).extend(0.0);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn no_walls() -> Vec<bool> {
        vec![false; COLS * ROWS]
    }

    #[test]
    fn flow_field_open_grid_all_reachable() {
        let walls = no_walls();
        let goal = (5, 5);
        let field = compute_flow_field(&walls, COLS, ROWS, goal);
        // Every non-goal cell must have a direction
        for row in 0..ROWS {
            for col in 0..COLS {
                if (col, row) == goal {
                    continue;
                }
                let idx = row * COLS + col;
                assert!(
                    field[idx].is_some(),
                    "cell ({col},{row}) unreachable in open grid"
                );
            }
        }
    }

    #[test]
    fn flow_field_goal_cell_is_none() {
        let walls = no_walls();
        let goal = (3, 7);
        let field = compute_flow_field(&walls, COLS, ROWS, goal);
        let idx = 7 * COLS + 3;
        assert_eq!(
            field[idx], None,
            "goal cell should have no outgoing direction"
        );
    }

    #[test]
    fn flow_field_wall_blocks_propagation() {
        // Build a vertical wall cutting the grid in half; left side cannot reach right goal
        let mut walls = vec![false; 10 * 5]; // 10 wide, 5 tall small grid
        for row in 0..5 {
            walls[row * 10 + 5] = true; // col 5 is solid wall
        }
        let field = compute_flow_field(&walls, 10, 5, (8, 2));
        // Cell on the left side should be unreachable
        let idx = 2 * 10 + 2; // col 2, row 2
        assert_eq!(field[idx], None, "left-of-wall cell should be unreachable");
    }

    #[test]
    fn flow_field_unreachable_island_returns_none() {
        // Surround a cell so it is completely enclosed by walls
        let width = 5;
        let height = 5;
        let mut walls = vec![false; width * height];
        // Enclose cell (2,2)
        walls[width + 2] = true; // above
        walls[3 * width + 2] = true; // below
        walls[2 * width + 1] = true; // left
        walls[2 * width + 3] = true; // right
        let field = compute_flow_field(&walls, width, height, (0, 0));
        let idx = 2 * width + 2;
        assert_eq!(field[idx], None, "isolated island should be unreachable");
    }

    #[test]
    fn flow_field_adjacent_to_goal_points_at_goal() {
        let walls = no_walls();
        let goal = (10, 10);
        let field = compute_flow_field(&walls, COLS, ROWS, goal);
        // Cell to the right of goal: (11, 10) should point left (-1, 0)
        let right_idx = 10 * COLS + 11;
        assert_eq!(field[right_idx], Some((-1, 0)));
        // Cell above goal: (10, 11) should point down (0, -1)
        let above_idx = 11 * COLS + 10;
        assert_eq!(field[above_idx], Some((0, -1)));
    }

    #[test]
    fn world_to_cell_round_trip() {
        let col = 7usize;
        let row = 12usize;
        let world = cell_to_world(col, row, CELL_PX, GRID_ORIGIN);
        let result = world_to_cell(world, CELL_PX, GRID_ORIGIN);
        assert_eq!(result, Some((col, row)));
    }

    #[test]
    fn world_to_cell_out_of_bounds_returns_none() {
        // Position far to the left of the grid
        let pos = Vec2::new(-9999.0, 0.0);
        assert_eq!(world_to_cell(pos, CELL_PX, GRID_ORIGIN), None);
    }

    #[test]
    fn cell_to_world_origin_cell() {
        let world = cell_to_world(0, 0, CELL_PX, GRID_ORIGIN);
        // Center of cell (0,0) = GRID_ORIGIN + half cell size
        assert!((world.x - (GRID_ORIGIN.x + CELL_PX * 0.5)).abs() < 0.001);
        assert!((world.y - (GRID_ORIGIN.y + CELL_PX * 0.5)).abs() < 0.001);
    }

    #[test]
    fn flow_field_goal_on_wall_returns_all_none() {
        let mut walls = vec![false; 4 * 4];
        walls[4 + 1] = true; // wall the goal
        let field = compute_flow_field(&walls, 4, 4, (1, 1));
        assert!(field.iter().all(|f| f.is_none()));
    }

    // --- ECS wiring tests ---

    #[test]
    fn grid_state_default_field_matches_center_goal() {
        let state = GridState::default();
        assert_eq!(state.goal, (COLS / 2, ROWS / 2));
        assert!(!state.dirty);
        // Goal cell has no outgoing direction.
        let idx = state.goal.1 * COLS + state.goal.0;
        assert_eq!(state.field[idx], None);
    }

    #[test]
    fn plugin_spawns_agents() {
        // Building-block path: the plugin composes onto a headless app.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, FlowFieldPathfindingPlugin));
        // The plugin's Update systems read the mouse; MinimalPlugins omits it.
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.insert_resource(ButtonInput::<MouseButton>::default());
        app.update();

        let mut q = app.world_mut().query::<&Agent>();
        assert_eq!(q.iter(app.world()).count(), AGENT_COUNT);
    }
}
