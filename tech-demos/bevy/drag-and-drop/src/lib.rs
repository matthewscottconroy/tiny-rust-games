//! Drag-and-drop — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`DragAndDropPlugin`] into any Bevy
//! app with `app.add_plugins(DragAndDropPlugin)` and it spawns a set of colored
//! squares you can pick up with the left mouse button, drag around, and drop —
//! snapping to a grid on release. Tune it through the [`DragAndDropConfig`]
//! resource without editing the plugin's internals.
//!
//! Key techniques:
//! - `Camera::viewport_to_world_2d` for screen-to-world conversion.
//! - [`DragState`] resource to track the active [`Draggable`] and grab offset.
//! - Z manipulation to bring the active item to the front while dragging.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use drag_and_drop::DragAndDropPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(DragAndDropPlugin)
//!     .run();
//! ```

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// Bundles every system and resource for the drag-and-drop feature.
///
/// Add it with `app.add_plugins(DragAndDropPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct DragAndDropPlugin;

impl Plugin for DragAndDropPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DragAndDropConfig>()
            .init_resource::<DragState>()
            .add_systems(Startup, setup)
            .add_systems(Update, (start_drag, update_drag, end_drag).chain());
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Tunable parameters for the feature. Override before adding the plugin, e.g.
/// `app.insert_resource(DragAndDropConfig { grid_size: 40.0, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct DragAndDropConfig {
    /// Side length of each square sprite, in world units.
    pub square_size: f32,
    /// Grid spacing squares snap to when dropped. Zero disables snapping.
    pub grid_size: f32,
    /// Z value assigned to the actively dragged square so it draws on top.
    pub drag_z: f32,
}

impl Default for DragAndDropConfig {
    fn default() -> Self {
        Self { square_size: 60.0, grid_size: 20.0, drag_z: 1.0 }
    }
}

// ---------------------------------------------------------------------------
// Components and resources
// ---------------------------------------------------------------------------

/// Marker: this entity can be picked up and dragged.
#[derive(Component)]
pub struct Draggable;

/// Tracks which entity (if any) is currently being dragged and the world-space
/// offset from the entity's origin to the cursor at the moment of picking up.
#[derive(Resource, Default)]
pub struct DragState {
    /// The entity currently being dragged, or `None` when idle.
    pub dragging: Option<Entity>,
    /// World-space vector from the cursor to the entity origin at grab time.
    pub grab_offset: Vec2,
}

// ---------------------------------------------------------------------------
// Pure functions
// ---------------------------------------------------------------------------

/// Returns `true` if `point` lies within an axis-aligned box centered at
/// `center` with the given `half_size` on each axis.
pub fn point_in_box(point: Vec2, center: Vec2, half_size: Vec2) -> bool {
    (point.x - center.x).abs() <= half_size.x && (point.y - center.y).abs() <= half_size.y
}

/// Snaps each component of `pos` to the nearest multiple of `grid_size`.
///
/// Returns `pos` unchanged when `grid_size` is zero.
pub fn snap_to_grid(pos: Vec2, grid_size: f32) -> Vec2 {
    if grid_size == 0.0 {
        return pos;
    }
    Vec2::new(
        (pos.x / grid_size).round() * grid_size,
        (pos.y / grid_size).round() * grid_size,
    )
}

/// Returns the world-space grab offset: the vector from the cursor to the
/// entity origin at the moment of picking up (`entity_pos - cursor_pos`).
pub fn grab_offset(entity_pos: Vec2, cursor_pos: Vec2) -> Vec2 {
    entity_pos - cursor_pos
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// Converts the current cursor screen position to world-space coordinates.
///
/// Returns `None` if the cursor is outside the window or the queries fail.
fn cursor_world_pos(
    windows: &Query<&Window, With<PrimaryWindow>>,
    camera_q: &Query<(&Camera, &GlobalTransform)>,
) -> Option<Vec2> {
    let window = windows.single().ok()?;
    let (camera, cam_transform) = camera_q.single().ok()?;
    let cursor = window.cursor_position()?;
    camera.viewport_to_world_2d(cam_transform, cursor).ok()
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Spawns the camera, eight draggable squares, and the HUD label.
fn setup(mut commands: Commands, config: Res<DragAndDropConfig>) {
    commands.spawn(Camera2d);

    let colors_and_positions: [(Color, Vec2); 8] = [
        (Color::srgb(0.9, 0.2, 0.2), Vec2::new(-320.0, 180.0)),
        (Color::srgb(0.2, 0.8, 0.3), Vec2::new(-160.0, 80.0)),
        (Color::srgb(0.2, 0.4, 0.9), Vec2::new(0.0, 200.0)),
        (Color::srgb(0.9, 0.8, 0.1), Vec2::new(160.0, -100.0)),
        (Color::srgb(0.9, 0.4, 0.1), Vec2::new(-280.0, -150.0)),
        (Color::srgb(0.6, 0.2, 0.9), Vec2::new(280.0, 140.0)),
        (Color::srgb(0.1, 0.8, 0.8), Vec2::new(100.0, -200.0)),
        (Color::srgb(0.9, 0.3, 0.7), Vec2::new(-80.0, -220.0)),
    ];

    for (color, pos) in colors_and_positions {
        commands.spawn((
            Draggable,
            Sprite {
                color,
                custom_size: Some(Vec2::splat(config.square_size)),
                ..default()
            },
            Transform::from_translation(pos.extend(0.0)),
        ));
    }

    // HUD label
    commands.spawn((
        Text::new("Drag squares to rearrange | Snaps to grid on drop"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
    ));
}

/// On left mouse button press: find the topmost `Draggable` under the cursor
/// and begin dragging it.
fn start_drag(
    mouse: Res<ButtonInput<MouseButton>>,
    config: Res<DragAndDropConfig>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut draggables: Query<(Entity, &mut Transform), With<Draggable>>,
    mut drag_state: ResMut<DragState>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(world_pos) = cursor_world_pos(&windows, &camera_q) else {
        return;
    };

    // Find the entity with the highest Z that the cursor overlaps.
    let half = Vec2::splat(config.square_size * 0.5);
    let picked: Option<(Entity, Vec2)> = draggables
        .iter()
        .filter(|(_, t)| point_in_box(world_pos, t.translation.truncate(), half))
        .max_by(|(_, a), (_, b)| a.translation.z.partial_cmp(&b.translation.z).unwrap())
        .map(|(e, t)| (e, t.translation.truncate()));

    if let Some((entity, entity_pos)) = picked {
        drag_state.dragging = Some(entity);
        drag_state.grab_offset = grab_offset(entity_pos, world_pos);

        // Raise Z to bring dragged entity above others.
        if let Ok((_, mut transform)) = draggables.get_mut(entity) {
            transform.translation.z = config.drag_z;
        }
    }
}

/// While dragging: move the active entity to follow the cursor plus the grab offset.
fn update_drag(
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    drag_state: Res<DragState>,
    mut transforms: Query<&mut Transform, With<Draggable>>,
) {
    let Some(entity) = drag_state.dragging else {
        return;
    };
    let Some(world_pos) = cursor_world_pos(&windows, &camera_q) else {
        return;
    };
    if let Ok(mut transform) = transforms.get_mut(entity) {
        let new_pos = world_pos + drag_state.grab_offset;
        transform.translation.x = new_pos.x;
        transform.translation.y = new_pos.y;
    }
}

/// On left mouse button release: snap the entity to the grid and clear `DragState`.
fn end_drag(
    mouse: Res<ButtonInput<MouseButton>>,
    config: Res<DragAndDropConfig>,
    mut drag_state: ResMut<DragState>,
    mut transforms: Query<&mut Transform, With<Draggable>>,
) {
    if !mouse.just_released(MouseButton::Left) {
        return;
    }
    if let Some(entity) = drag_state.dragging.take() {
        if let Ok(mut transform) = transforms.get_mut(entity) {
            let snapped = snap_to_grid(transform.translation.truncate(), config.grid_size);
            transform.translation.x = snapped.x;
            transform.translation.y = snapped.y;
            transform.translation.z = 0.0;
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_in_box_center() {
        assert!(point_in_box(
            Vec2::ZERO,
            Vec2::ZERO,
            Vec2::splat(30.0)
        ));
    }

    #[test]
    fn point_in_box_on_edge() {
        // Exactly on the boundary counts as inside.
        assert!(point_in_box(
            Vec2::new(30.0, 0.0),
            Vec2::ZERO,
            Vec2::splat(30.0)
        ));
    }

    #[test]
    fn point_in_box_outside() {
        assert!(!point_in_box(
            Vec2::new(31.0, 0.0),
            Vec2::ZERO,
            Vec2::splat(30.0)
        ));
    }

    #[test]
    fn point_in_box_displaced_center() {
        assert!(point_in_box(
            Vec2::new(100.0, 200.0),
            Vec2::new(100.0, 200.0),
            Vec2::splat(30.0)
        ));
    }

    #[test]
    fn snap_to_grid_rounds_down() {
        let result = snap_to_grid(Vec2::new(9.0, 9.0), 20.0);
        assert!((result.x).abs() < f32::EPSILON);
        assert!((result.y).abs() < f32::EPSILON);
    }

    #[test]
    fn snap_to_grid_rounds_up() {
        let result = snap_to_grid(Vec2::new(11.0, 11.0), 20.0);
        assert!((result.x - 20.0).abs() < f32::EPSILON);
        assert!((result.y - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn snap_to_grid_already_aligned() {
        let result = snap_to_grid(Vec2::new(40.0, -60.0), 20.0);
        assert!((result.x - 40.0).abs() < f32::EPSILON);
        assert!((result.y + 60.0).abs() < f32::EPSILON);
    }

    #[test]
    fn snap_to_grid_zero_size_returns_pos() {
        let pos = Vec2::new(13.7, -42.1);
        assert_eq!(snap_to_grid(pos, 0.0), pos);
    }

    #[test]
    fn grab_offset_calculation() {
        let entity = Vec2::new(100.0, 200.0);
        let cursor = Vec2::new(90.0, 185.0);
        let offset = grab_offset(entity, cursor);
        assert!((offset.x - 10.0).abs() < f32::EPSILON);
        assert!((offset.y - 15.0).abs() < f32::EPSILON);
    }

    #[test]
    fn grab_offset_zero_when_same() {
        let pos = Vec2::new(50.0, 75.0);
        assert_eq!(grab_offset(pos, pos), Vec2::ZERO);
    }

    #[test]
    fn drag_state_default_has_no_entity() {
        let state = DragState::default();
        assert!(state.dragging.is_none());
        assert_eq!(state.grab_offset, Vec2::ZERO);
    }

    #[test]
    fn config_default_matches_documented_values() {
        let c = DragAndDropConfig::default();
        assert_eq!(c.square_size, 60.0);
        assert_eq!(c.grid_size, 20.0);
        assert_eq!(c.drag_z, 1.0);
    }

    // --- ECS setup test ---

    #[test]
    fn setup_spawns_eight_draggables() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<DragAndDropConfig>()
            .init_resource::<DragState>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Draggable>();
        assert_eq!(q.iter(app.world()).count(), 8);
    }
}
