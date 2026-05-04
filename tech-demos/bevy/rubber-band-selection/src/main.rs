//! # Rubber-Band Selection Demo
//!
//! Demonstrates RTS-style marquee (rubber-band) selection in Bevy.
//!
//! Twenty selectable units are scattered across the screen at hardcoded
//! positions (no external RNG crate needed). Left-click and drag to draw a
//! visible selection rectangle; any unit whose center falls inside the rect
//! becomes highlighted in yellow when the mouse is released. Clicking on empty
//! space without dragging deselects all units.
//!
//! Key techniques:
//! - A single reused `SelectionRect` sprite entity that is resized and
//!   repositioned each frame during a drag
//! - `Commands::entity` to add/remove the `Selected` component dynamically
//! - `Camera::viewport_to_world_2d` for screen-to-world conversion
//! - Pure geometry functions for rect hit-testing

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Marker: this entity is a selectable unit.
#[derive(Component)]
pub struct Unit;

/// Marker: this unit is currently selected.
#[derive(Component)]
pub struct Selected;

/// Marker: this entity renders the drag selection rectangle.
#[derive(Component)]
pub struct SelectionRect;

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Tracks the state of an ongoing rubber-band drag.
#[derive(Resource, Default)]
pub struct SelectionDrag {
    /// World-space start corner of the drag, set on mouse press.
    pub start: Option<Vec2>,
    /// Whether a drag is currently in progress.
    pub active: bool,
}

// ---------------------------------------------------------------------------
// Pure functions
// ---------------------------------------------------------------------------

/// Returns `true` if `point` is inside or on the boundary of the axis-aligned
/// rect defined by any two corner points.
pub fn point_in_rect(point: Vec2, corner_a: Vec2, corner_b: Vec2) -> bool {
    let min_x = corner_a.x.min(corner_b.x);
    let max_x = corner_a.x.max(corner_b.x);
    let min_y = corner_a.y.min(corner_b.y);
    let max_y = corner_a.y.max(corner_b.y);
    point.x >= min_x && point.x <= max_x && point.y >= min_y && point.y <= max_y
}

/// Returns the center of the rect defined by two corner points.
pub fn rect_center(a: Vec2, b: Vec2) -> Vec2 {
    (a + b) * 0.5
}

/// Returns the size (always positive on each axis) of the rect defined by two corners.
pub fn rect_size(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new((b.x - a.x).abs(), (b.y - a.y).abs())
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
// Hardcoded unit positions (20 units, no RNG)
// ---------------------------------------------------------------------------

const UNIT_POSITIONS: [Vec2; 20] = [
    Vec2::new(-380.0, 230.0),
    Vec2::new(-290.0, 180.0),
    Vec2::new(-200.0, 250.0),
    Vec2::new(-140.0, 100.0),
    Vec2::new(-60.0, 200.0),
    Vec2::new(30.0, 240.0),
    Vec2::new(120.0, 170.0),
    Vec2::new(220.0, 210.0),
    Vec2::new(310.0, 140.0),
    Vec2::new(380.0, 230.0),
    Vec2::new(-350.0, -50.0),
    Vec2::new(-240.0, -120.0),
    Vec2::new(-130.0, -80.0),
    Vec2::new(-20.0, -160.0),
    Vec2::new(90.0, -50.0),
    Vec2::new(190.0, -130.0),
    Vec2::new(280.0, -60.0),
    Vec2::new(-300.0, -220.0),
    Vec2::new(0.0, -240.0),
    Vec2::new(340.0, -200.0),
];

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Spawns the camera, all unit entities, the selection rect sprite, and the HUD.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Spawn all units.
    for pos in UNIT_POSITIONS {
        commands.spawn((
            Unit,
            Sprite {
                color: Color::srgb(0.3, 0.7, 1.0),
                custom_size: Some(Vec2::splat(20.0)),
                ..default()
            },
            Transform::from_translation(pos.extend(0.0)),
        ));
    }

    // Spawn the selection rectangle (starts invisible).
    commands.spawn((
        SelectionRect,
        Sprite {
            color: Color::srgba(1.0, 1.0, 1.0, 0.15),
            custom_size: Some(Vec2::ZERO),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.5)),
    ));

    // HUD
    commands.spawn((
        Text::new("Selected: 0 / 20"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
        HudText,
    ));
}

/// Marker for the HUD text entity.
#[derive(Component)]
struct HudText;

/// On left press: record the drag start position and mark the drag as active.
fn begin_selection(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut drag: ResMut<SelectionDrag>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    if let Some(world_pos) = cursor_world_pos(&windows, &camera_q) {
        drag.start = Some(world_pos);
        drag.active = true;
    }
}

/// While dragging: update the selection rect sprite to cover the area between
/// the drag start and the current cursor position.
fn update_selection_rect(
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    drag: Res<SelectionDrag>,
    mut rect_q: Query<(&mut Transform, &mut Sprite), With<SelectionRect>>,
) {
    let Ok((mut transform, mut sprite)) = rect_q.single_mut() else {
        return;
    };

    if !drag.active {
        sprite.custom_size = Some(Vec2::ZERO);
        return;
    }

    let Some(start) = drag.start else {
        return;
    };
    let Some(current) = cursor_world_pos(&windows, &camera_q) else {
        return;
    };

    let center = rect_center(start, current);
    let size = rect_size(start, current);

    transform.translation.x = center.x;
    transform.translation.y = center.y;
    sprite.custom_size = Some(size);
}

/// Minimum drag distance (in world units) before treating the gesture as a
/// rect-select rather than a plain click.
const MIN_DRAG_DISTANCE: f32 = 5.0;

/// On left release: apply selection or deselect all, then reset drag state.
fn end_selection(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut drag: ResMut<SelectionDrag>,
    units: Query<(Entity, &Transform), With<Unit>>,
    selected: Query<Entity, With<Selected>>,
    mut commands: Commands,
    mut rect_q: Query<&mut Sprite, With<SelectionRect>>,
) {
    if !mouse.just_released(MouseButton::Left) {
        return;
    }

    let start = drag.start.take();
    drag.active = false;

    // Hide the selection rect.
    if let Ok(mut sprite) = rect_q.single_mut() {
        sprite.custom_size = Some(Vec2::ZERO);
    }

    let Some(start_pos) = start else { return };
    let Some(end_pos) = cursor_world_pos(&windows, &camera_q) else {
        // No cursor — deselect all.
        for e in &selected {
            commands.entity(e).remove::<Selected>();
        }
        return;
    };

    let drag_distance = (end_pos - start_pos).length();

    if drag_distance < MIN_DRAG_DISTANCE {
        // Plain click: deselect all.
        for e in &selected {
            commands.entity(e).remove::<Selected>();
        }
        return;
    }

    // Rect-select: deselect all first, then select those in rect.
    for e in &selected {
        commands.entity(e).remove::<Selected>();
    }
    for (entity, transform) in &units {
        let pos = transform.translation.truncate();
        if point_in_rect(pos, start_pos, end_pos) {
            commands.entity(entity).insert(Selected);
        }
    }
}

/// Updates the HUD text with the current selection count.
fn update_hud(
    selected: Query<(), With<Selected>>,
    units: Query<(), With<Unit>>,
    mut hud: Query<&mut Text, With<HudText>>,
) {
    let Ok(mut text) = hud.single_mut() else {
        return;
    };
    text.0 = format!(
        "Selected: {} / {}\nDrag to select | Click empty space to deselect",
        selected.iter().count(),
        units.iter().count(),
    );
}

/// Tints selected units yellow and unselected units blue each frame.
fn update_unit_colors(
    mut units: Query<(&mut Sprite, Option<&Selected>), With<Unit>>,
) {
    for (mut sprite, is_selected) in &mut units {
        sprite.color = if is_selected.is_some() {
            Color::srgb(1.0, 0.9, 0.1)
        } else {
            Color::srgb(0.3, 0.7, 1.0)
        };
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Rubber-Band Selection Demo".into(),
                resolution: (900u32, 600u32).into(),
                ..default()
            }),
            ..default()
        }))
        .init_resource::<SelectionDrag>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                begin_selection,
                update_selection_rect,
                end_selection,
                update_unit_colors,
                update_hud,
            )
                .chain(),
        )
        .run();
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_in_rect_center() {
        assert!(point_in_rect(Vec2::ZERO, Vec2::new(-50.0, -50.0), Vec2::new(50.0, 50.0)));
    }

    #[test]
    fn point_in_rect_corner_a_equals_b() {
        // Degenerate rect: both corners at same position — only that exact point is inside.
        let corner = Vec2::new(10.0, 20.0);
        assert!(point_in_rect(corner, corner, corner));
        assert!(!point_in_rect(Vec2::new(11.0, 20.0), corner, corner));
    }

    #[test]
    fn point_in_rect_on_border() {
        // Points exactly on the boundary should be inside.
        let a = Vec2::new(-100.0, -100.0);
        let b = Vec2::new(100.0, 100.0);
        assert!(point_in_rect(Vec2::new(-100.0, 0.0), a, b)); // left edge
        assert!(point_in_rect(Vec2::new(100.0, 0.0), a, b));  // right edge
        assert!(point_in_rect(Vec2::new(0.0, -100.0), a, b)); // bottom edge
        assert!(point_in_rect(Vec2::new(0.0, 100.0), a, b));  // top edge
    }

    #[test]
    fn point_in_rect_outside() {
        let a = Vec2::new(-50.0, -50.0);
        let b = Vec2::new(50.0, 50.0);
        assert!(!point_in_rect(Vec2::new(51.0, 0.0), a, b));
        assert!(!point_in_rect(Vec2::new(0.0, -51.0), a, b));
    }

    #[test]
    fn point_in_rect_reversed_corners() {
        // Works regardless of which corner is min/max.
        let a = Vec2::new(50.0, 50.0);
        let b = Vec2::new(-50.0, -50.0);
        assert!(point_in_rect(Vec2::ZERO, a, b));
        assert!(point_in_rect(Vec2::new(49.0, 49.0), a, b));
    }

    #[test]
    fn rect_center_midpoint() {
        let center = rect_center(Vec2::new(-100.0, -100.0), Vec2::new(100.0, 100.0));
        assert!((center.x).abs() < f32::EPSILON);
        assert!((center.y).abs() < f32::EPSILON);
    }

    #[test]
    fn rect_center_asymmetric() {
        let center = rect_center(Vec2::new(0.0, 0.0), Vec2::new(10.0, 20.0));
        assert!((center.x - 5.0).abs() < f32::EPSILON);
        assert!((center.y - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn rect_size_always_positive() {
        let size = rect_size(Vec2::new(50.0, 30.0), Vec2::new(-50.0, -30.0));
        assert!((size.x - 100.0).abs() < f32::EPSILON);
        assert!((size.y - 60.0).abs() < f32::EPSILON);
    }

    #[test]
    fn rect_size_degenerate_zero() {
        let size = rect_size(Vec2::new(5.0, 5.0), Vec2::new(5.0, 5.0));
        assert_eq!(size, Vec2::ZERO);
    }

    #[test]
    fn selection_drag_default_is_inactive() {
        let drag = SelectionDrag::default();
        assert!(drag.start.is_none());
        assert!(!drag.active);
    }

    #[test]
    fn unit_positions_count() {
        assert_eq!(UNIT_POSITIONS.len(), 20);
    }

    #[test]
    fn ecs_units_spawn_without_selected() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        for pos in UNIT_POSITIONS {
            app.world_mut().spawn((Unit, Transform::from_translation(pos.extend(0.0))));
        }
        let selected_count = app
            .world_mut()
            .query::<&Selected>()
            .iter(app.world())
            .count();
        assert_eq!(selected_count, 0);
        let unit_count = app
            .world_mut()
            .query::<&Unit>()
            .iter(app.world())
            .count();
        assert_eq!(unit_count, 20);
    }
}
