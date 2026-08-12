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
//! - The player is clamped to the visible grid with `IVec2::clamp`.
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
        Text::new("Arrow keys or WASD — move one cell per press"),
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

    let mut delta = IVec2::ZERO;
    if input.just_pressed(KeyCode::ArrowUp) || input.just_pressed(KeyCode::KeyW) {
        delta.y += 1;
    }
    if input.just_pressed(KeyCode::ArrowDown) || input.just_pressed(KeyCode::KeyS) {
        delta.y -= 1;
    }
    if input.just_pressed(KeyCode::ArrowLeft) || input.just_pressed(KeyCode::KeyA) {
        delta.x -= 1;
    }
    if input.just_pressed(KeyCode::ArrowRight) || input.just_pressed(KeyCode::KeyD) {
        delta.x += 1;
    }

    if delta != IVec2::ZERO {
        let new_pos = grid.0 + delta;
        grid.0 = new_pos.clamp(IVec2::splat(-config.half), IVec2::splat(config.half));
    }
}

/// Writes the player's world-space `Transform` from its [`GridPos`] each frame.
fn sync_transform(
    config: Res<GridConfig>,
    mut query: Query<(&GridPos, &mut Transform), With<Player>>,
) {
    for (grid, mut transform) in &mut query {
        transform.translation.x = grid.0.x as f32 * config.cell;
        transform.translation.y = grid.0.y as f32 * config.cell;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Grid clamping logic ---

    #[test]
    fn clamp_keeps_in_bounds_position_unchanged() {
        let half = GridConfig::default().half;
        let pos = IVec2::new(3, -2);
        let clamped = pos.clamp(IVec2::splat(-half), IVec2::splat(half));
        assert_eq!(clamped, pos);
    }

    #[test]
    fn clamp_past_right_edge_snaps_to_boundary() {
        let half = GridConfig::default().half;
        let pos = IVec2::new(half + 5, 0);
        let clamped = pos.clamp(IVec2::splat(-half), IVec2::splat(half));
        assert_eq!(clamped.x, half);
    }

    #[test]
    fn clamp_past_bottom_edge_snaps_to_boundary() {
        let half = GridConfig::default().half;
        let pos = IVec2::new(0, -half - 3);
        let clamped = pos.clamp(IVec2::splat(-half), IVec2::splat(half));
        assert_eq!(clamped.y, -half);
    }

    #[test]
    fn cell_size_converts_grid_to_world() {
        let cell = GridConfig::default().cell;
        let grid = IVec2::new(2, -3);
        let world_x = grid.x as f32 * cell;
        let world_y = grid.y as f32 * cell;
        assert!((world_x - 96.0).abs() < 1e-5);
        assert!((world_y + 144.0).abs() < 1e-5);
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
