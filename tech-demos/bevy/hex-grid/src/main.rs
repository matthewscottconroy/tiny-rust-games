//! Hexagonal grid demo — Bevy 0.18.
//!
//! Key ideas:
//! - Axial coordinates `(q, r)` represent hex positions without the diagonal
//!   ambiguity of offset grids.
//! - [`axial_to_pixel`] converts axial coords to flat-top pixel centres.
//! - [`pixel_to_axial`] rounds a pixel position back to the nearest hex.
//! - [`hex_neighbors`] enumerates the 6 adjacent cells.
//! - [`axial_distance`] gives the hex-grid Manhattan distance between two cells.
//! - Clicking a hex selects it and highlights its neighbours.
//!
//! **Controls:** left-click — select hex;  R — reset selection.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Hex Grid".into(),
                resolution: (800u32, 600u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(Selected(None))
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_click, update_colors).chain())
        .run();
}

// ── Pure hex math ─────────────────────────────────────────────────────────────

const HEX_SIZE: f32 = 30.0; // flat-top, outer radius

/// Converts axial `(q, r)` to the pixel centre of a flat-top hexagon.
pub fn axial_to_pixel(q: i32, r: i32, size: f32) -> Vec2 {
    Vec2::new(
        size * (3.0_f32 / 2.0) * q as f32,
        size * (3.0_f32.sqrt() / 2.0 * q as f32 + 3.0_f32.sqrt() * r as f32),
    )
}

/// Rounds a pixel position to the nearest axial hex coordinate (flat-top).
pub fn pixel_to_axial(px: Vec2, size: f32) -> (i32, i32) {
    let q_frac = (2.0 / 3.0) * px.x / size;
    let r_frac = (-1.0 / 3.0) * px.x / size + (3.0_f32.sqrt() / 3.0) * px.y / size;
    cube_round(q_frac, -q_frac - r_frac, r_frac)
}

fn cube_round(q: f32, s: f32, r: f32) -> (i32, i32) {
    let (mut rq, mut rs, mut rr) = (q.round(), s.round(), r.round());
    let (dq, ds, dr) = ((rq - q).abs(), (rs - s).abs(), (rr - r).abs());
    if dq > ds && dq > dr {
        rq = -rs - rr;
    } else if ds > dr {
        rs = -rq - rr;
    } else {
        rr = -rq - rs;
    }
    let _ = rs;
    (rq as i32, rr as i32)
}

/// Returns the 6 axial neighbors of `(q, r)`.
pub fn hex_neighbors(q: i32, r: i32) -> [(i32, i32); 6] {
    [
        (q + 1, r),
        (q + 1, r - 1),
        (q,     r - 1),
        (q - 1, r),
        (q - 1, r + 1),
        (q,     r + 1),
    ]
}

/// Hex-grid distance between two axial coordinates.
pub fn axial_distance(q1: i32, r1: i32, q2: i32, r2: i32) -> i32 {
    let dq = (q1 - q2).abs();
    let dr = (r1 - r2).abs();
    let ds = ((q1 + r1) - (q2 + r2)).abs();
    (dq + dr + ds) / 2
}

// ── ECS ───────────────────────────────────────────────────────────────────────

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct HexCoord(i32, i32);

#[derive(Resource)]
struct Selected(Option<(i32, i32)>);

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Text::new("left-click: select   R: reset"),
        TextFont { font_size: 15.0, ..default() },
        TextColor(Color::srgb(0.6, 0.6, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));

    for q in -5..=5i32 {
        for r in -5..=5i32 {
            // Keep a rough hexagonal boundary
            if axial_distance(q, r, 0, 0) > 5 { continue; }
            let pos = axial_to_pixel(q, r, HEX_SIZE);
            commands.spawn((
                Sprite {
                    color: Color::srgb(0.22, 0.22, 0.28),
                    custom_size: Some(Vec2::splat(HEX_SIZE * 1.72)),
                    ..default()
                },
                Transform::from_xyz(pos.x, pos.y, 0.0),
                HexCoord(q, r),
            ));
        }
    }
}

fn handle_click(
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    windows: Query<&Window>,
    hexes: Query<(&HexCoord, &Transform)>,
    mut selected: ResMut<Selected>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        selected.0 = None;
        return;
    }
    if !buttons.just_pressed(MouseButton::Left) { return; }

    let Ok((camera, cam_tf)) = camera_q.single() else { return };
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let Ok(world) = camera.viewport_to_world_2d(cam_tf, cursor) else { return };

    // Find the closest hex within one hex-size radius
    let mut best: Option<(i32, i32)> = None;
    let mut best_dist = f32::MAX;
    for (coord, tf) in &hexes {
        let d = tf.translation.truncate().distance(world);
        if d < HEX_SIZE * 1.1 && d < best_dist {
            best_dist = d;
            best = Some((coord.0, coord.1));
        }
    }
    selected.0 = best;
}

fn update_colors(
    selected: Res<Selected>,
    mut hexes: Query<(&HexCoord, &mut Sprite)>,
) {
    let neighbors: std::collections::HashSet<(i32, i32)> = selected.0
        .map(|(q, r)| hex_neighbors(q, r).into_iter().collect())
        .unwrap_or_default();

    for (coord, mut sprite) in &mut hexes {
        let cell = (coord.0, coord.1);
        sprite.color = if selected.0 == Some(cell) {
            Color::srgb(1.0, 0.75, 0.15)      // selected: gold
        } else if neighbors.contains(&cell) {
            Color::srgb(0.35, 0.55, 0.85)      // neighbor: blue
        } else {
            Color::srgb(0.22, 0.22, 0.28)      // default: dark
        };
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axial_to_pixel_origin_is_zero() {
        let p = axial_to_pixel(0, 0, 30.0);
        assert!(p.x.abs() < 1e-4 && p.y.abs() < 1e-4);
    }

    #[test]
    fn pixel_to_axial_origin_round_trips() {
        let p = axial_to_pixel(2, -1, 30.0);
        let (q, r) = pixel_to_axial(p, 30.0);
        assert_eq!((q, r), (2, -1));
    }

    #[test]
    fn pixel_to_axial_several_cells() {
        for (q, r) in [(0, 0), (3, -2), (-2, 1), (0, 4)] {
            let p = axial_to_pixel(q, r, 30.0);
            assert_eq!(pixel_to_axial(p, 30.0), (q, r), "round-trip failed for ({q},{r})");
        }
    }

    #[test]
    fn hex_neighbors_count_is_six() {
        assert_eq!(hex_neighbors(0, 0).len(), 6);
    }

    #[test]
    fn hex_neighbors_origin_excludes_origin() {
        let ns = hex_neighbors(0, 0);
        assert!(!ns.contains(&(0, 0)));
    }

    #[test]
    fn hex_neighbors_all_distance_one() {
        for (nq, nr) in hex_neighbors(2, -1) {
            assert_eq!(axial_distance(2, -1, nq, nr), 1);
        }
    }

    #[test]
    fn axial_distance_same_cell_is_zero() {
        assert_eq!(axial_distance(3, -2, 3, -2), 0);
    }

    #[test]
    fn axial_distance_adjacent_is_one() {
        let (q, r) = (0, 0);
        for (nq, nr) in hex_neighbors(q, r) {
            assert_eq!(axial_distance(q, r, nq, nr), 1);
        }
    }

    #[test]
    fn axial_distance_known_value() {
        assert_eq!(axial_distance(0, 0, 3, -3), 3);
    }
}
