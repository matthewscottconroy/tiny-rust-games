//! Grid movement demo.
//!
//! Key ideas:
//! - [`GridPos`]`(IVec2)` is the authoritative position in grid space.
//! - `just_pressed()` fires once per keypress, giving clean one-cell-per-tap movement.
//! - `Transform` is derived from [`GridPos`] each frame via [`sync_transform`].
//! - The player is clamped to the visible grid with `IVec2::clamp`.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (grid_move, sync_transform))
        .run();
}

// --- Components ---

/// Marks the player entity.
#[derive(Component)]
struct Player;

/// Authoritative grid-space position.  World-space `Transform` is derived from this.
#[derive(Component)]
struct GridPos(IVec2);

// --- Constants ---

/// World-space size of one grid cell in pixels.
const CELL: f32 = 48.0;

/// The grid extends from `-GRID_HALF` to `+GRID_HALF` on both axes.
const GRID_HALF: i32 = 7;

const GRID_COLOR_A: Color = Color::srgb(0.14, 0.14, 0.14);
const GRID_COLOR_B: Color = Color::srgb(0.20, 0.20, 0.20);

// --- Setup ---

/// Spawns a checkerboard grid and the player at the grid origin.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    for row in -GRID_HALF..=GRID_HALF {
        for col in -GRID_HALF..=GRID_HALF {
            let color = if (row + col) % 2 == 0 { GRID_COLOR_A } else { GRID_COLOR_B };
            commands.spawn((
                Sprite { color, custom_size: Some(Vec2::splat(CELL - 2.0)), ..default() },
                Transform::from_xyz(col as f32 * CELL, row as f32 * CELL, -1.0),
            ));
        }
    }

    commands.spawn((
        Sprite {
            color: Color::srgb(0.3, 0.75, 0.95),
            custom_size: Some(Vec2::splat(CELL - 8.0)),
            ..default()
        },
        Transform::default(),
        Player,
        GridPos(IVec2::ZERO),
    ));

    commands.spawn((
        Text::new("Arrow keys or WASD — move one cell per press"),
        TextFont { font_size: 15.0, ..default() },
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
    mut query: Query<&mut GridPos, With<Player>>,
) {
    let Ok(mut grid) = query.single_mut() else { return; };

    let mut delta = IVec2::ZERO;
    if input.just_pressed(KeyCode::ArrowUp)    || input.just_pressed(KeyCode::KeyW) { delta.y += 1; }
    if input.just_pressed(KeyCode::ArrowDown)  || input.just_pressed(KeyCode::KeyS) { delta.y -= 1; }
    if input.just_pressed(KeyCode::ArrowLeft)  || input.just_pressed(KeyCode::KeyA) { delta.x -= 1; }
    if input.just_pressed(KeyCode::ArrowRight) || input.just_pressed(KeyCode::KeyD) { delta.x += 1; }

    if delta != IVec2::ZERO {
        let new_pos = grid.0 + delta;
        grid.0 = new_pos.clamp(IVec2::splat(-GRID_HALF), IVec2::splat(GRID_HALF));
    }
}

/// Writes the player's world-space `Transform` from its [`GridPos`] each frame.
fn sync_transform(mut query: Query<(&GridPos, &mut Transform), With<Player>>) {
    for (grid, mut transform) in &mut query {
        transform.translation.x = grid.0.x as f32 * CELL;
        transform.translation.y = grid.0.y as f32 * CELL;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Grid clamping logic ---

    #[test]
    fn clamp_keeps_in_bounds_position_unchanged() {
        let pos = IVec2::new(3, -2);
        let clamped = pos.clamp(IVec2::splat(-GRID_HALF), IVec2::splat(GRID_HALF));
        assert_eq!(clamped, pos);
    }

    #[test]
    fn clamp_past_right_edge_snaps_to_boundary() {
        let pos = IVec2::new(GRID_HALF + 5, 0);
        let clamped = pos.clamp(IVec2::splat(-GRID_HALF), IVec2::splat(GRID_HALF));
        assert_eq!(clamped.x, GRID_HALF);
    }

    #[test]
    fn clamp_past_bottom_edge_snaps_to_boundary() {
        let pos = IVec2::new(0, -GRID_HALF - 3);
        let clamped = pos.clamp(IVec2::splat(-GRID_HALF), IVec2::splat(GRID_HALF));
        assert_eq!(clamped.y, -GRID_HALF);
    }

    #[test]
    fn cell_size_converts_grid_to_world() {
        let grid = IVec2::new(2, -3);
        let world_x = grid.x as f32 * CELL;
        let world_y = grid.y as f32 * CELL;
        assert!((world_x - 96.0).abs() < 1e-5);
        assert!((world_y + 144.0).abs() < 1e-5);
    }

    // --- ECS ---

    #[test]
    fn setup_spawns_one_player() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Player>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn player_starts_at_grid_origin() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<(&Player, &GridPos)>();
        for (_, grid) in q.iter(app.world()) {
            assert_eq!(grid.0, IVec2::ZERO);
        }
    }
}
