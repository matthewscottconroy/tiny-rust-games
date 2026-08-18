//! Grid movement — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`GridMovementPlugin`] into any Bevy
//! app with `app.add_plugins(GridMovementPlugin)` and it spawns a checkerboard
//! grid plus a player that moves one cell per key press. Tune it through the
//! [`GridConfig`] resource without editing the plugin's internals.
//!
//! Key ideas:
//! - [`GridPos`]`(IVec2)` is the authoritative position in grid space.
//! - `just_pressed()` fires once per keypress, giving clean one-cell-per-tap movement.
//! - `Transform` is derived from [`GridPos`] each frame via [`sync_transform`].
//! - Movement rules live in pure functions ([`input_delta`], [`step`],
//!   [`grid_to_world`]), so they are testable without a World and liftable.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use grid_movement::GridMovementPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(GridMovementPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system and resource for the grid-movement feature.
///
/// Add it with `app.add_plugins(GridMovementPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct GridMovementPlugin;

impl Plugin for GridMovementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GridConfig>()
            .add_systems(Startup, setup)
            .add_systems(Update, (grid_move, sync_transform));
    }
}

// --- Configuration ---

/// Tunable parameters for the feature. Override before adding the plugin, e.g.
/// `app.insert_resource(GridConfig { cell: 64.0, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct GridConfig {
    /// World-space size of one grid cell in pixels.
    pub cell: f32,
    /// The grid extends from `-half` to `+half` on both axes.
    pub half: i32,
    /// Color of the "even" checkerboard cells.
    pub color_a: Color,
    /// Color of the "odd" checkerboard cells.
    pub color_b: Color,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            cell: 48.0,
            half: 7,
            color_a: Color::srgb(0.14, 0.14, 0.14),
            color_b: Color::srgb(0.20, 0.20, 0.20),
        }
    }
}

// --- Pure grid math ---
//
// The rules of grid movement are kept here as free functions taking plain
// values, so they can be tested without a World and lifted into another
// project. The systems below are thin wrappers over them.

/// Converts a grid cell to its world-space centre.
pub fn grid_to_world(grid: IVec2, cell: f32) -> Vec2 {
    Vec2::new(grid.x as f32 * cell, grid.y as f32 * cell)
}

/// Converts a world-space point to the grid cell containing it.
///
/// Rounds to the nearest cell, so a point anywhere inside a cell maps to it.
pub fn world_to_grid(world: Vec2, cell: f32) -> IVec2 {
    IVec2::new(
        (world.x / cell).round() as i32,
        (world.y / cell).round() as i32,
    )
}

/// Clamps a cell to the square grid spanning `-half..=half` on both axes.
pub fn clamp_to_grid(pos: IVec2, half: i32) -> IVec2 {
    pos.clamp(IVec2::splat(-half), IVec2::splat(half))
}

/// Turns four directional presses into a one-cell step.
///
/// Opposite presses in the same frame cancel out, so a player mashing left and
/// right does not drift.
pub fn input_delta(up: bool, down: bool, left: bool, right: bool) -> IVec2 {
    IVec2::new(
        i32::from(right) - i32::from(left),
        i32::from(up) - i32::from(down),
    )
}

/// Applies a step to a position and clamps the result to the grid.
pub fn step(pos: IVec2, delta: IVec2, half: i32) -> IVec2 {
    clamp_to_grid(pos + delta, half)
}

// --- Components ---

/// Marks the player entity.
#[derive(Component)]
pub struct Player;

/// Authoritative grid-space position.  World-space `Transform` is derived from this.
#[derive(Component)]
pub struct GridPos(pub IVec2);

// --- Setup ---

/// Spawns a checkerboard grid and the player at the grid origin.
fn setup(mut commands: Commands, config: Res<GridConfig>) {
    commands.spawn(Camera2d);

    for row in -config.half..=config.half {
        for col in -config.half..=config.half {
            let color = if (row + col) % 2 == 0 {
                config.color_a
            } else {
                config.color_b
            };
            commands.spawn((
                Sprite {
                    color,
                    custom_size: Some(Vec2::splat(config.cell - 2.0)),
                    ..default()
                },
                Transform::from_xyz(col as f32 * config.cell, row as f32 * config.cell, -1.0),
            ));
        }
    }

    commands.spawn((
        Sprite {
            color: Color::srgb(0.3, 0.75, 0.95),
            custom_size: Some(Vec2::splat(config.cell - 8.0)),
            ..default()
        },
        Transform::default(),
        Player,
        GridPos(IVec2::ZERO),
    ));

    commands.spawn((
        Text::new("Arrow keys or WASD - move one cell per press"),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.8, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

// --- Systems ---

/// Reads directional input and moves the player one cell per key press,
/// clamping to the visible grid boundary.
fn grid_move(
    input: Res<ButtonInput<KeyCode>>,
    config: Res<GridConfig>,
    mut query: Query<&mut GridPos, With<Player>>,
) {
    let Ok(mut grid) = query.single_mut() else {
        return;
    };

    // `just_pressed` fires once per keypress, giving one cell per tap.
    let delta = input_delta(
        input.just_pressed(KeyCode::ArrowUp) || input.just_pressed(KeyCode::KeyW),
        input.just_pressed(KeyCode::ArrowDown) || input.just_pressed(KeyCode::KeyS),
        input.just_pressed(KeyCode::ArrowLeft) || input.just_pressed(KeyCode::KeyA),
        input.just_pressed(KeyCode::ArrowRight) || input.just_pressed(KeyCode::KeyD),
    );

    if delta != IVec2::ZERO {
        grid.0 = step(grid.0, delta, config.half);
    }
}

/// Writes the player's world-space `Transform` from its [`GridPos`] each frame.
fn sync_transform(
    config: Res<GridConfig>,
    mut query: Query<(&GridPos, &mut Transform), With<Player>>,
) {
    for (grid, mut transform) in &mut query {
        let world = grid_to_world(grid.0, config.cell);
        transform.translation.x = world.x;
        transform.translation.y = world.y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Pure grid math ---

    #[test]
    fn clamp_keeps_in_bounds_position_unchanged() {
        assert_eq!(clamp_to_grid(IVec2::new(3, -2), 7), IVec2::new(3, -2));
    }

    #[test]
    fn clamp_snaps_every_edge_to_the_boundary() {
        assert_eq!(clamp_to_grid(IVec2::new(12, 0), 7).x, 7);
        assert_eq!(clamp_to_grid(IVec2::new(-12, 0), 7).x, -7);
        assert_eq!(clamp_to_grid(IVec2::new(0, 12), 7).y, 7);
        assert_eq!(clamp_to_grid(IVec2::new(0, -12), 7).y, -7);
    }

    #[test]
    fn grid_and_world_coordinates_round_trip() {
        let cell = GridConfig::default().cell;
        for grid in [IVec2::ZERO, IVec2::new(2, -3), IVec2::new(-7, 7)] {
            assert_eq!(world_to_grid(grid_to_world(grid, cell), cell), grid);
        }
    }

    #[test]
    fn grid_to_world_scales_by_cell_size() {
        let world = grid_to_world(IVec2::new(2, -3), 48.0);
        assert!((world.x - 96.0).abs() < 1e-5);
        assert!((world.y + 144.0).abs() < 1e-5);
    }

    #[test]
    fn world_to_grid_rounds_to_the_nearest_cell() {
        // A point just past a cell centre still belongs to that cell.
        assert_eq!(world_to_grid(Vec2::new(50.0, -2.0), 48.0), IVec2::new(1, 0));
        assert_eq!(world_to_grid(Vec2::new(70.0, 0.0), 48.0), IVec2::new(1, 0));
    }

    #[test]
    fn input_delta_maps_each_direction() {
        assert_eq!(input_delta(true, false, false, false), IVec2::new(0, 1));
        assert_eq!(input_delta(false, true, false, false), IVec2::new(0, -1));
        assert_eq!(input_delta(false, false, true, false), IVec2::new(-1, 0));
        assert_eq!(input_delta(false, false, false, true), IVec2::new(1, 0));
    }

    #[test]
    fn opposite_presses_cancel_out() {
        assert_eq!(input_delta(true, true, true, true), IVec2::ZERO);
        assert_eq!(input_delta(false, false, true, true), IVec2::ZERO);
    }

    #[test]
    fn diagonal_presses_combine() {
        assert_eq!(input_delta(true, false, false, true), IVec2::new(1, 1));
    }

    #[test]
    fn step_moves_one_cell_and_respects_the_boundary() {
        assert_eq!(step(IVec2::ZERO, IVec2::new(1, 0), 7), IVec2::new(1, 0));
        // Already at the edge: stepping further is a no-op.
        assert_eq!(
            step(IVec2::new(7, 0), IVec2::new(1, 0), 7),
            IVec2::new(7, 0)
        );
    }

    #[test]
    fn config_default_matches_documented_values() {
        let c = GridConfig::default();
        assert_eq!(c.cell, 48.0);
        assert_eq!(c.half, 7);
    }

    // --- ECS ---

    #[test]
    fn setup_spawns_one_player() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<GridConfig>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Player>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn player_starts_at_grid_origin() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<GridConfig>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<(&Player, &GridPos)>();
        for (_, grid) in q.iter(app.world()) {
            assert_eq!(grid.0, IVec2::ZERO);
        }
    }
}
