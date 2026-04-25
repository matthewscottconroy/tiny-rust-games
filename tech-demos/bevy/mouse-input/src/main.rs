//! Mouse input demo.
//!
//! Key ideas:
//! - [`cursor_to_world`] converts window pixel coordinates to Bevy world space
//!   via [`Camera::viewport_to_world_2d`] — this conversion is required any time
//!   you want to act on where the user clicked in the game world.
//! - Drag state is tracked in a [`Resource`]; on left-press the closest entity
//!   within its radius is picked, on left-release it is dropped.
//! - Hover is detected every frame by comparing cursor distance to each entity.
//!
//! **Bug fixed:** the original `let Color::Srgba(s) = color else { color }` had
//! a non-diverging `else` branch, which is a compile error.  Replaced with
//! `if let` so the non-Srgba case can fall through gracefully.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<DragState>()
        .add_systems(Startup, setup)
        .add_systems(Update, (update_hover, handle_drag, update_labels))
        .run();
}

// --- Components ---

/// Marks a sprite as draggable and stores its base color and hit radius.
#[derive(Component)]
struct Draggable {
    base_color: Color,
    radius: f32,
}

/// Marker for the top-left label that shows hover / drag status.
#[derive(Component)]
struct HoverLabel;

// --- Resources ---

/// Tracks which entity (if any) is currently being dragged and the
/// pick-point offset so the sprite doesn't jump on first drag.
#[derive(Resource, Default)]
struct DragState {
    entity: Option<Entity>,
    offset: Vec2,
}

// --- Setup ---

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let items: &[(Vec3, Color, f32)] = &[
        (Vec3::new(-200.0,  80.0, 0.0), Color::srgb(0.9, 0.35, 0.2), 38.0),
        (Vec3::new(   0.0,  80.0, 0.0), Color::srgb(0.2, 0.65, 0.9), 28.0),
        (Vec3::new( 200.0,  80.0, 0.0), Color::srgb(0.7, 0.9,  0.2), 48.0),
        (Vec3::new(-100.0, -80.0, 0.0), Color::srgb(0.8, 0.3,  0.8), 34.0),
        (Vec3::new( 100.0, -80.0, 0.0), Color::srgb(0.95, 0.75, 0.1), 42.0),
    ];

    for &(pos, color, radius) in items {
        commands.spawn((
            Sprite { color, custom_size: Some(Vec2::splat(radius * 2.0)), ..default() },
            Transform::from_translation(pos),
            Draggable { base_color: color, radius },
        ));
    }

    commands.spawn((
        Text::new(""),
        TextFont { font_size: 15.0, ..default() },
        TextColor(Color::srgb(0.8, 0.8, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        HoverLabel,
    ));

    commands.spawn((
        Text::new("Left-drag — move shapes"),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgb(0.55, 0.55, 0.55)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

// --- Helpers ---

/// Converts a window-space cursor position to 2D world coordinates.
///
/// Returns `None` when the cursor is outside the window or the conversion
/// fails (e.g. no primary window exists).
pub fn cursor_to_world(
    window: &Window,
    cam: &Camera,
    cam_transform: &GlobalTransform,
) -> Option<Vec2> {
    let cursor = window.cursor_position()?;
    cam.viewport_to_world_2d(cam_transform, cursor).ok()
}

/// Lightens an sRGB color by adding `amount` to each channel, clamped to 1.
///
/// Falls back to the original color if it isn't an `Srgba` variant.
pub fn lighten(color: Color, amount: f32) -> Color {
    if let Color::Srgba(s) = color {
        Color::srgb(
            (s.red   + amount).min(1.0),
            (s.green + amount).min(1.0),
            (s.blue  + amount).min(1.0),
        )
    } else {
        color
    }
}

// --- Systems ---

/// Tints hovered shapes lighter and updates the status label.
fn update_hover(
    window_query: Query<&Window>,
    cam_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut sprite_query: Query<(&mut Sprite, &Transform, &Draggable)>,
    drag: Res<DragState>,
    mut label_query: Query<&mut Text, With<HoverLabel>>,
) {
    let Ok(window) = window_query.single() else { return; };
    let Ok((cam, cam_transform)) = cam_query.single() else { return; };
    let cursor = cursor_to_world(window, cam, cam_transform);

    let mut hovered_name: Option<String> = None;

    for (mut sprite, transform, draggable) in &mut sprite_query {
        let pos = transform.translation.truncate();
        let hovered = cursor
            .map(|c| c.distance(pos) < draggable.radius)
            .unwrap_or(false);

        // While something is being dragged, reset all colors.
        if drag.entity.is_some() {
            sprite.color = draggable.base_color;
            continue;
        }

        sprite.color = if hovered {
            hovered_name = Some(format!("Hovering shape at ({:.0}, {:.0})", pos.x, pos.y));
            lighten(draggable.base_color, 0.25)
        } else {
            draggable.base_color
        };
    }

    for mut text in &mut label_query {
        *text = Text::new(hovered_name.unwrap_or_default());
    }
}

/// Picks, moves, and drops draggable entities on left mouse button events.
fn handle_drag(
    mouse: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window>,
    cam_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut drag: ResMut<DragState>,
    mut query: Query<(Entity, &mut Transform, &Draggable)>,
) {
    let Ok(window) = window_query.single() else { return; };
    let Ok((cam, cam_transform)) = cam_query.single() else { return; };
    let Some(cursor) = cursor_to_world(window, cam, cam_transform) else { return; };

    if mouse.just_pressed(MouseButton::Left) {
        let mut best: Option<(Entity, f32, Vec2)> = None;
        for (entity, transform, draggable) in &query {
            let pos = transform.translation.truncate();
            let dist = cursor.distance(pos);
            if dist < draggable.radius && best.map_or(true, |(_, d, _)| dist < d) {
                best = Some((entity, dist, pos));
            }
        }
        if let Some((entity, _, pos)) = best {
            drag.entity = Some(entity);
            drag.offset = pos - cursor;
        }
    }

    if mouse.pressed(MouseButton::Left) {
        if let Some(dragged) = drag.entity {
            if let Ok((_, mut transform, _)) = query.get_mut(dragged) {
                let target = cursor + drag.offset;
                transform.translation.x = target.x;
                transform.translation.y = target.y;
            }
        }
    }

    if mouse.just_released(MouseButton::Left) {
        drag.entity = None;
    }
}

/// Shows "Dragging…" in the label while a drag is in progress.
fn update_labels(drag: Res<DragState>, mut label_query: Query<&mut Text, With<HoverLabel>>) {
    if drag.entity.is_some() {
        for mut text in &mut label_query {
            *text = Text::new("Dragging…");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lighten_increases_all_channels() {
        let base = Color::srgb(0.5, 0.3, 0.1);
        let lit = lighten(base, 0.2);
        let Color::Srgba(s) = lit else { panic!("expected Srgba") };
        assert!((s.red   - 0.7).abs() < 1e-5);
        assert!((s.green - 0.5).abs() < 1e-5);
        assert!((s.blue  - 0.3).abs() < 1e-5);
    }

    #[test]
    fn lighten_clamps_at_one() {
        let base = Color::srgb(0.9, 0.9, 0.9);
        let lit = lighten(base, 0.5);
        let Color::Srgba(s) = lit else { panic!("expected Srgba") };
        assert_eq!(s.red,   1.0);
        assert_eq!(s.green, 1.0);
        assert_eq!(s.blue,  1.0);
    }

    #[test]
    fn drag_state_default_has_no_entity() {
        let drag = DragState::default();
        assert!(drag.entity.is_none());
        assert_eq!(drag.offset, Vec2::ZERO);
    }

    #[test]
    fn setup_spawns_five_draggables() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Draggable>();
        assert_eq!(q.iter(app.world()).count(), 5);
    }
}
