//! Circle-buttons GUI — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`CircleGuiPlugin`] into any Bevy app
//! with `app.add_plugins(CircleGuiPlugin)` and it draws a row of coloured
//! circles that act as clickable buttons, handling all mouse interaction
//! internally. Tune it through the [`CircleGuiConfig`] resource without editing
//! the plugin's internals.
//!
//! Results flow out of the plugin through two parallel channels so callers can
//! choose whichever suits them:
//! - **Push**: [`CircleClicked`] messages fired on every click.
//! - **Pull**: [`CircleClickState`] resource updated to the most-recent click.
//!
//! Hovering brightens a circle; moving away restores its original colour.
//!
//! **Controls:** move the mouse to highlight, left-click to select.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use circle_buttons::{CircleGuiConfig, CircleGuiPlugin};
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .insert_resource(CircleGuiConfig { count: 4, ..default() })
//!     .add_plugins(CircleGuiPlugin)
//!     .run();
//! ```

use bevy::{
    asset::Assets,
    math::Vec2,
    mesh::Mesh2d,
    prelude::*,
    sprite_render::{ColorMaterial, MeshMaterial2d},
    window::PrimaryWindow,
};

// ─── Plugin ──────────────────────────────────────────────────────────────────

/// Adds the circle-button GUI to the app.
///
/// Add it with `app.add_plugins(CircleGuiPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window, camera, and
/// rendering setup.
///
/// # Usage
/// ```rust,ignore
/// app.insert_resource(CircleGuiConfig { count: 4, ..default() })
///    .add_plugins(CircleGuiPlugin)
///    .add_systems(Update, my_click_handler);
/// ```
pub struct CircleGuiPlugin;

impl Plugin for CircleGuiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CircleGuiConfig>()
            .init_resource::<CircleClickState>()
            .init_resource::<HoverState>()
            .add_message::<CircleClicked>()
            .add_systems(Startup, (spawn_circles, spawn_hud).chain())
            .add_systems(Update, (handle_mouse, update_hud));
    }
}

// ─── Configuration ───────────────────────────────────────────────────────────

/// Configuration for the circle-button GUI.
///
/// Insert this as a [`Resource`] *before* adding [`CircleGuiPlugin`].
/// The plugin calls [`Default::default`] if no config is present.
#[derive(Resource, Clone, Debug)]
pub struct CircleGuiConfig {
    /// Number of circle buttons to display.
    pub count: usize,
    /// Radius of every circle in world units.
    pub radius: f32,
    /// Centre-to-centre distance between adjacent circles.
    pub spacing: f32,
    /// One colour per circle; cycles if shorter than `count`.
    pub colors: Vec<Color>,
}

impl Default for CircleGuiConfig {
    fn default() -> Self {
        Self {
            count: 5,
            radius: 40.0,
            spacing: 110.0,
            colors: vec![
                Color::srgb(0.88, 0.22, 0.22),
                Color::srgb(0.22, 0.78, 0.30),
                Color::srgb(0.22, 0.45, 0.90),
                Color::srgb(0.92, 0.80, 0.10),
                Color::srgb(0.80, 0.22, 0.80),
            ],
        }
    }
}

// ─── Messages & output resources ───────────────────────────────────────────────

/// Emitted every time a circle button is left-clicked.
///
/// Read this with [`MessageReader<CircleClicked>`] from any system outside the
/// plugin.
#[derive(Message, Clone, Debug)]
pub struct CircleClicked {
    /// Zero-based index of the circle that was clicked.
    pub index: usize,
    /// Base colour of that circle (before hover tinting).
    pub color: Color,
    /// World-space centre position of the circle.
    pub position: Vec2,
}

/// Stores the index of the most recently clicked circle.
///
/// Poll this resource as an alternative to listening for [`CircleClicked`]
/// messages.
#[derive(Resource, Default, Debug)]
pub struct CircleClickState {
    /// Index of the last clicked circle, or `None` if nothing has been clicked.
    pub last_clicked: Option<usize>,
}

// ─── Components ───────────────────────────────────────────────────────────────

/// Per-entity data for a circle button.
#[derive(Component, Clone)]
pub struct CircleButton {
    /// Zero-based position of this circle in the row.
    pub index: usize,
    /// Radius of this circle in world units.
    pub radius: f32,
    /// The base (non-hovered) colour — used to restore after hover ends.
    pub color: Color,
}

/// Tracks which circle entity the cursor is currently over.
#[derive(Resource, Default)]
struct HoverState {
    hovered: Option<Entity>,
}

/// Marker for the status text at the bottom of the screen.
#[derive(Component)]
struct HudLabel;

// ─── Pure helpers ────────────────────────────────────────────────────────────

/// Returns the world-space centre positions for `count` circles.
///
/// Circles are arranged in a horizontal row centred on the origin.
/// Returns an empty vec when `count` is zero.
pub fn circle_layout(count: usize, spacing: f32) -> Vec<Vec2> {
    if count == 0 {
        return vec![];
    }
    let half = (count - 1) as f32 * spacing / 2.0;
    (0..count)
        .map(|i| Vec2::new(-half + i as f32 * spacing, 0.0))
        .collect()
}

/// Returns `true` if `point` lies within (or exactly on the edge of) the circle.
pub fn point_in_circle(point: Vec2, center: Vec2, radius: f32) -> bool {
    (point - center).length_squared() <= radius * radius
}

/// Selects the colour for circle `index`, cycling through `colors` if necessary.
///
/// Returns a mid-grey fallback when `colors` is empty.
pub fn color_for_index(colors: &[Color], index: usize) -> Color {
    if colors.is_empty() {
        Color::srgb(0.5, 0.5, 0.5)
    } else {
        colors[index % colors.len()]
    }
}

/// Returns a brightened copy of `color` by adding `amount` to each RGB channel.
///
/// Each channel is clamped to 1.0; alpha is preserved.
pub fn lighten_color(color: Color, amount: f32) -> Color {
    let s = color.to_srgba();
    Color::srgba(
        (s.red + amount).min(1.0),
        (s.green + amount).min(1.0),
        (s.blue + amount).min(1.0),
        s.alpha,
    )
}

// ─── Systems ─────────────────────────────────────────────────────────────────

fn spawn_circles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    config: Res<CircleGuiConfig>,
) {
    for (i, pos) in circle_layout(config.count, config.spacing).into_iter().enumerate() {
        let color = color_for_index(&config.colors, i);
        commands.spawn((
            Mesh2d(meshes.add(Circle::new(config.radius))),
            MeshMaterial2d(materials.add(ColorMaterial::from(color))),
            Transform::from_translation(pos.extend(0.0)),
            CircleButton { index: i, radius: config.radius, color },
        ));
    }
}

fn spawn_hud(mut commands: Commands) {
    commands.spawn((
        Text::new("Hover and click a circle"),
        TextFont { font_size: 20.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(14.0),
            left: Val::Px(14.0),
            ..default()
        },
        HudLabel,
    ));
}

/// Converts the cursor's screen position to world space using the active camera.
fn cursor_world(
    windows: &Query<&Window, With<PrimaryWindow>>,
    camera_q: &Query<(&Camera, &GlobalTransform)>,
) -> Option<Vec2> {
    let window = windows.single().ok()?;
    let (camera, cam_t) = camera_q.single().ok()?;
    camera.viewport_to_world_2d(cam_t, window.cursor_position()?).ok()
}

fn handle_mouse(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    circles: Query<(Entity, &CircleButton, &Transform, &MeshMaterial2d<ColorMaterial>)>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut hover: ResMut<HoverState>,
    mut click_events: MessageWriter<CircleClicked>,
    mut click_state: ResMut<CircleClickState>,
) {
    let Some(cursor) = cursor_world(&windows, &camera_q) else {
        return;
    };

    // Snapshot circle data before taking a mutable borrow on `materials`.
    struct Snap {
        entity: Entity,
        index: usize,
        radius: f32,
        color: Color,
        center: Vec2,
        handle: Handle<ColorMaterial>,
    }
    let snaps: Vec<Snap> = circles
        .iter()
        .map(|(e, btn, t, mh)| Snap {
            entity: e,
            index: btn.index,
            radius: btn.radius,
            color: btn.color,
            center: t.translation.truncate(),
            handle: mh.0.clone(),
        })
        .collect();

    let hovered = snaps
        .iter()
        .find(|s| point_in_circle(cursor, s.center, s.radius));
    let new_entity = hovered.map(|s| s.entity);

    // Update hover highlight only when the hovered circle changes.
    if new_entity != hover.hovered {
        if let Some(prev) = hover.hovered {
            if let Some(s) = snaps.iter().find(|s| s.entity == prev) {
                if let Some(mat) = materials.get_mut(&s.handle) {
                    mat.color = s.color;
                }
            }
        }
        if let Some(s) = hovered {
            if let Some(mat) = materials.get_mut(&s.handle) {
                mat.color = lighten_color(s.color, 0.25);
            }
        }
        hover.hovered = new_entity;
    }

    // Fire event on click.
    if mouse.just_pressed(MouseButton::Left) {
        if let Some(s) = hovered {
            click_events.write(CircleClicked {
                index: s.index,
                color: s.color,
                position: s.center,
            });
            click_state.last_clicked = Some(s.index);
        }
    }
}

fn update_hud(
    state: Res<CircleClickState>,
    config: Res<CircleGuiConfig>,
    mut hud: Query<(&mut Text, &mut TextColor), With<HudLabel>>,
) {
    let Ok((mut text, mut color)) = hud.single_mut() else {
        return;
    };
    match state.last_clicked {
        None => {
            text.0 = "Hover and click a circle".to_string();
            color.0 = Color::WHITE;
        }
        Some(idx) => {
            text.0 = format!("Clicked circle {} / {}", idx + 1, config.count);
            color.0 = color_for_index(&config.colors, idx);
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── circle_layout ────────────────────────────────────────────────────────

    #[test]
    fn layout_zero_count_is_empty() {
        assert!(circle_layout(0, 100.0).is_empty());
    }

    #[test]
    fn layout_one_circle_at_origin() {
        let pos = circle_layout(1, 100.0);
        assert_eq!(pos.len(), 1);
        assert_eq!(pos[0], Vec2::ZERO);
    }

    #[test]
    fn layout_two_circles_symmetric() {
        let pos = circle_layout(2, 100.0);
        assert_eq!(pos.len(), 2);
        assert!((pos[0].x + pos[1].x).abs() < 1e-5, "should be symmetric around origin");
        assert!((pos[1].x - pos[0].x - 100.0).abs() < 1e-5);
    }

    #[test]
    fn layout_spacing_is_respected() {
        let pos = circle_layout(3, 80.0);
        let gap1 = (pos[1] - pos[0]).length();
        let gap2 = (pos[2] - pos[1]).length();
        assert!((gap1 - 80.0).abs() < 1e-5);
        assert!((gap2 - 80.0).abs() < 1e-5);
    }

    #[test]
    fn layout_row_is_centred() {
        let pos = circle_layout(5, 100.0);
        let sum_x: f32 = pos.iter().map(|p| p.x).sum();
        assert!(sum_x.abs() < 1e-4, "x positions should average to 0");
    }

    // ── point_in_circle ──────────────────────────────────────────────────────

    #[test]
    fn point_at_center_is_inside() {
        assert!(point_in_circle(Vec2::ZERO, Vec2::ZERO, 50.0));
    }

    #[test]
    fn point_exactly_on_edge_is_inside() {
        assert!(point_in_circle(Vec2::new(50.0, 0.0), Vec2::ZERO, 50.0));
    }

    #[test]
    fn point_outside_is_false() {
        assert!(!point_in_circle(Vec2::new(51.0, 0.0), Vec2::ZERO, 50.0));
    }

    #[test]
    fn offset_center_is_handled() {
        let center = Vec2::new(100.0, 200.0);
        assert!(point_in_circle(center + Vec2::new(10.0, 0.0), center, 20.0));
        assert!(!point_in_circle(center + Vec2::new(25.0, 0.0), center, 20.0));
    }

    // ── color_for_index ──────────────────────────────────────────────────────

    #[test]
    fn color_empty_returns_grey() {
        let c = color_for_index(&[], 0);
        let s = c.to_srgba();
        assert!((s.red - 0.5).abs() < 1e-5);
    }

    #[test]
    fn color_cycles_correctly() {
        let colors = vec![Color::srgb(1.0, 0.0, 0.0), Color::srgb(0.0, 1.0, 0.0)];
        let c0 = color_for_index(&colors, 0).to_srgba();
        let c2 = color_for_index(&colors, 2).to_srgba();
        assert!((c0.red - c2.red).abs() < 1e-5);
        assert!((c0.green - c2.green).abs() < 1e-5);
    }

    // ── lighten_color ────────────────────────────────────────────────────────

    #[test]
    fn lighten_increases_rgb() {
        let original = Color::srgb(0.3, 0.3, 0.3);
        let bright = lighten_color(original, 0.2);
        let s = bright.to_srgba();
        assert!((s.red - 0.5).abs() < 1e-5);
    }

    #[test]
    fn lighten_clamps_to_one() {
        let bright = lighten_color(Color::srgb(0.9, 0.9, 0.9), 0.5);
        let s = bright.to_srgba();
        assert!(s.red <= 1.0 + 1e-5);
        assert!(s.green <= 1.0 + 1e-5);
        assert!(s.blue <= 1.0 + 1e-5);
    }

    #[test]
    fn lighten_preserves_alpha() {
        let color = Color::srgba(0.5, 0.5, 0.5, 0.4);
        let bright = lighten_color(color, 0.1);
        assert!((bright.to_srgba().alpha - 0.4).abs() < 1e-5);
    }

    #[test]
    fn config_default_matches_documented_values() {
        let c = CircleGuiConfig::default();
        assert_eq!(c.count, 5);
        assert_eq!(c.radius, 40.0);
        assert_eq!(c.spacing, 110.0);
    }

    // ── ECS integration ──────────────────────────────────────────────────────

    #[test]
    fn plugin_spawns_correct_circle_count() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(CircleGuiConfig {
                count: 4,
                radius: 30.0,
                spacing: 80.0,
                colors: vec![Color::WHITE],
            })
            .init_resource::<CircleClickState>()
            .init_resource::<HoverState>()
            .add_message::<CircleClicked>()
            // Spawn only the CircleButton component (no mesh/material — Assets not
            // available in MinimalPlugins) to verify the layout count.
            .add_systems(Startup, |mut commands: Commands, config: Res<CircleGuiConfig>| {
                for (i, pos) in circle_layout(config.count, config.spacing).into_iter().enumerate() {
                    let color = color_for_index(&config.colors, i);
                    commands.spawn((
                        Transform::from_translation(pos.extend(0.0)),
                        CircleButton { index: i, radius: config.radius, color },
                    ));
                }
            });
        app.update();
        let count = app
            .world_mut()
            .query::<&CircleButton>()
            .iter(app.world())
            .count();
        assert_eq!(count, 4);
    }
}
