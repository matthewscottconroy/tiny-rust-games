//! Simulated 2D lighting demo — distance-based light intensity on a tile grid.
//!
//! Key ideas:
//! - Each tile sprite's colour is recomputed every frame from `light_intensity`,
//!   a pure function that accumulates contributions from all active lights using
//!   a smooth quadratic falloff.
//! - `apply_light_intensity` scales a base RGB tuple by the computed intensity,
//!   keeping the colour arithmetic free of Bevy types and trivially testable.
//! - Three floating point lights orbit the arena; the player carries a fourth
//!   light that moves with WASD.  An adjustable ambient floor prevents total
//!   darkness.
//! - The approach uses no custom shaders or Bevy lighting components — just
//!   per-frame sprite colour updates — making it fully cross-platform.
//!
//! **Controls:** WASD / Arrow keys — move player light   + / - — raise / lower ambient.

use bevy::prelude::*;
use bevy::window::WindowResolution;

const COLS: usize = 32;
const ROWS: usize = 22;
const TILE_PX: f32 = 22.0;
const PLAYER_SPEED: f32 = 140.0;

// ─── Pure helpers ────────────────────────────────────────────────────────────

/// Accumulates light from every `(position, radius)` source at `pos`.
///
/// Each source contributes `(1 − (dist/radius)²)` when inside its radius;
/// contributions are summed and clamped to `[0.0, 1.0]`.
pub fn light_intensity(pos: Vec2, lights: &[(Vec2, f32)]) -> f32 {
    let mut total = 0.0_f32;
    for &(lpos, radius) in lights {
        let dist = pos.distance(lpos);
        if dist < radius {
            let t = 1.0 - dist / radius;
            total += t * t;
        }
    }
    total.min(1.0)
}

/// Scales an RGB base colour by `intensity`, adding `ambient` as a floor.
///
/// Both `intensity` and `ambient` are clamped to `[0.0, 1.0]`.
pub fn apply_light_intensity(
    base: (f32, f32, f32),
    intensity: f32,
    ambient: f32,
) -> (f32, f32, f32) {
    let level = (intensity + ambient).min(1.0);
    (base.0 * level, base.1 * level, base.2 * level)
}

// ─── Tile setup ──────────────────────────────────────────────────────────────

fn make_grid() -> Vec<Vec<bool>> {
    let rows = [
        "################################",
        "#..............................#",
        "#...####....####....####.......#",
        "#...#..#....#..#....#..#.......#",
        "#...####....####....####.......#",
        "#..............................#",
        "#..............................#",
        "#.......####....####...........#",
        "#.......#..#....#..#...........#",
        "#.......####....####...........#",
        "#..............................#",
        "#..............................#",
        "#...####....####....####.......#",
        "#...#..#....#..#....#..#.......#",
        "#...####....####....####.......#",
        "#..............................#",
        "#..............................#",
        "#.......####....####...........#",
        "#.......#..#....#..#...........#",
        "#.......####....####...........#",
        "#..............................#",
        "################################",
    ];
    rows.iter()
        .map(|row| row.chars().map(|c| c == '.').collect())
        .collect()
}

// ─── Components & resources ──────────────────────────────────────────────────

/// World-space position stored on each tile for fast lighting lookup.
#[derive(Component)]
struct LitTile {
    world_pos: Vec2,
    base_color: (f32, f32, f32),
}

/// Three orbiting light sources stored as (angle, radius, orbit_radius, speed, light_color).
#[derive(Resource)]
struct OrbitLights(Vec<OrbitLight>);

struct OrbitLight {
    angle: f32,
    orbit_radius: f32,
    speed: f32,
    color: (f32, f32, f32),
    reach: f32,
}

#[derive(Resource)]
struct Ambient(f32);

#[derive(Component)]
struct PlayerLight;

#[derive(Component)]
struct HudLabel;

// ─── Bevy app ────────────────────────────────────────────────────────────────

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Simulated 2D Lighting — WASD to move light, +/- ambient".to_string(),
                resolution: (704u32, 514u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Ambient(0.04))
        .insert_resource(OrbitLights(vec![
            OrbitLight { angle: 0.0,    orbit_radius: 180.0, speed: 0.7, color: (1.0, 0.4, 0.2), reach: 120.0 },
            OrbitLight { angle: 2.094,  orbit_radius: 220.0, speed: -0.5, color: (0.2, 0.5, 1.0), reach: 100.0 },
            OrbitLight { angle: 4.189,  orbit_radius: 150.0, speed: 1.1, color: (0.3, 1.0, 0.4), reach: 90.0  },
        ]))
        .add_systems(Startup, setup)
        .add_systems(Update, (move_player, advance_orbits, apply_lighting, update_hud))
        .run();
}

fn cell_to_world(cell: IVec2) -> Vec2 {
    let ox = -(COLS as f32 * TILE_PX) / 2.0 + TILE_PX / 2.0;
    let oy = (ROWS as f32 * TILE_PX) / 2.0 - TILE_PX / 2.0;
    Vec2::new(ox + cell.x as f32 * TILE_PX, oy - cell.y as f32 * TILE_PX)
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let grid = make_grid();
    for (r, row) in grid.iter().enumerate() {
        for (c, &walkable) in row.iter().enumerate() {
            let world_pos = cell_to_world(IVec2::new(c as i32, r as i32));
            let base = if walkable {
                (0.55, 0.5, 0.42)
            } else {
                (0.3, 0.3, 0.38)
            };
            commands.spawn((
                Sprite { color: Color::BLACK, custom_size: Some(Vec2::splat(TILE_PX - 1.0)), ..default() },
                Transform::from_translation(world_pos.extend(0.0)),
                LitTile { world_pos, base_color: base },
            ));
        }
    }

    // Player (carries a light)
    commands.spawn((
        Sprite { color: Color::srgb(1.0, 0.95, 0.7), custom_size: Some(Vec2::splat(12.0)), ..default() },
        Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
        PlayerLight,
    ));

    // Orbit light visual dots
    for _ in 0..3 {
        commands.spawn((
            Sprite { color: Color::srgb(1.0, 1.0, 0.9), custom_size: Some(Vec2::splat(8.0)), ..default() },
            Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
        ));
    }

    commands.spawn((
        Text::new("Ambient: 0.04"),
        TextFont { font_size: 16.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(4.0),
            left: Val::Px(8.0),
            ..default()
        },
        HudLabel,
    ));
    commands.spawn((
        Text::new("WASD — move   + — brighter   - — darker"),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgb(0.6, 0.6, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(4.0),
            right: Val::Px(8.0),
            ..default()
        },
    ));
}

fn move_player(
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut ambient: ResMut<Ambient>,
    mut q: Query<&mut Transform, With<PlayerLight>>,
) {
    let Ok(mut t) = q.single_mut() else { return };
    let mut dir = Vec2::ZERO;
    if input.pressed(KeyCode::KeyW) || input.pressed(KeyCode::ArrowUp)    { dir.y += 1.0; }
    if input.pressed(KeyCode::KeyS) || input.pressed(KeyCode::ArrowDown)   { dir.y -= 1.0; }
    if input.pressed(KeyCode::KeyA) || input.pressed(KeyCode::ArrowLeft)   { dir.x -= 1.0; }
    if input.pressed(KeyCode::KeyD) || input.pressed(KeyCode::ArrowRight)  { dir.x += 1.0; }
    if dir != Vec2::ZERO {
        t.translation += (dir.normalize() * PLAYER_SPEED * time.delta_secs()).extend(0.0);
    }
    if input.just_pressed(KeyCode::Equal) || input.just_pressed(KeyCode::NumpadAdd) {
        ambient.0 = (ambient.0 + 0.04).min(1.0);
    }
    if input.just_pressed(KeyCode::Minus) || input.just_pressed(KeyCode::NumpadSubtract) {
        ambient.0 = (ambient.0 - 0.04).max(0.0);
    }
}

fn advance_orbits(time: Res<Time>, mut lights: ResMut<OrbitLights>) {
    for light in &mut lights.0 {
        light.angle += light.speed * time.delta_secs();
    }
}

/// Recolours every tile based on accumulated light from all sources.
fn apply_lighting(
    orbits: Res<OrbitLights>,
    ambient: Res<Ambient>,
    player_q: Query<&Transform, With<PlayerLight>>,
    mut tiles: Query<(&LitTile, &mut Sprite)>,
) {
    let player_pos = player_q.single()
        .map(|t| t.translation.truncate())
        .unwrap_or(Vec2::ZERO);

    // Collect all light source positions with reach (tinted by colour).
    let mut light_sources: Vec<(Vec2, f32)> = orbits.0.iter().map(|o| {
        let pos = Vec2::new(o.orbit_radius * o.angle.cos(), o.orbit_radius * o.angle.sin());
        (pos, o.reach)
    }).collect();
    light_sources.push((player_pos, 130.0));

    for (tile, mut sprite) in &mut tiles {
        // Compute per-channel intensity for coloured lights.
        let mut r_total = 0.0_f32;
        let mut g_total = 0.0_f32;
        let mut b_total = 0.0_f32;

        for (idx, o) in orbits.0.iter().enumerate() {
            let lpos = Vec2::new(o.orbit_radius * o.angle.cos(), o.orbit_radius * o.angle.sin());
            let dist = tile.world_pos.distance(lpos);
            if dist < o.reach {
                let t = 1.0 - dist / o.reach;
                let contrib = t * t;
                r_total += contrib * o.color.0;
                g_total += contrib * o.color.1;
                b_total += contrib * o.color.2;
            }
            let _ = idx; // suppress unused warning when orbit count changes
        }
        // Player light is warm white.
        let pd = tile.world_pos.distance(player_pos);
        let player_reach = 130.0_f32;
        if pd < player_reach {
            let t = 1.0 - pd / player_reach;
            let c = t * t;
            r_total += c;
            g_total += c * 0.95;
            b_total += c * 0.7;
        }

        let amb = ambient.0;
        let (br, bg, bb) = tile.base_color;
        sprite.color = Color::srgb(
            (br * (r_total.min(1.0) + amb)).min(1.0),
            (bg * (g_total.min(1.0) + amb)).min(1.0),
            (bb * (b_total.min(1.0) + amb)).min(1.0),
        );
    }
}

fn update_hud(ambient: Res<Ambient>, mut q: Query<&mut Text, With<HudLabel>>) {
    for mut text in &mut q {
        text.0 = format!("Ambient: {:.2}", ambient.0);
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_intensity_at_light_center() {
        let lights = vec![(Vec2::ZERO, 100.0)];
        assert!((light_intensity(Vec2::ZERO, &lights) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn zero_intensity_beyond_radius() {
        let lights = vec![(Vec2::ZERO, 100.0)];
        assert_eq!(light_intensity(Vec2::new(200.0, 0.0), &lights), 0.0);
    }

    #[test]
    fn multiple_lights_combine_and_clamp() {
        let lights = vec![(Vec2::ZERO, 100.0), (Vec2::ZERO, 100.0)];
        assert_eq!(light_intensity(Vec2::ZERO, &lights), 1.0);
    }

    #[test]
    fn apply_light_at_zero_intensity_is_ambient() {
        let base = (1.0, 0.5, 0.2);
        let (r, g, b) = apply_light_intensity(base, 0.0, 0.1);
        assert!((r - 0.1).abs() < 1e-5);
        assert!((g - 0.05).abs() < 1e-5);
    }

    #[test]
    fn apply_light_at_full_intensity_equals_base() {
        let base = (0.8, 0.6, 0.4);
        let (r, g, b) = apply_light_intensity(base, 1.0, 0.0);
        assert!((r - 0.8).abs() < 1e-5);
        assert!((g - 0.6).abs() < 1e-5);
        assert!((b - 0.4).abs() < 1e-5);
    }
}
