//! Spatial Partitioning — grid-cell bucketing for O(1) neighbour queries.
//!
//! Key ideas:
//! - The world is divided into a fixed-size grid. Each frame every entity is
//!   assigned to the cell that contains its position.
//! - Proximity checks only examine entities in the same cell and its 8 neighbours —
//!   a 3×3 patch — instead of checking every pair.
//! - `cell_of` and `neighbour_cells` are pure functions with no Bevy dependency.
//! - The HUD compares the brute-force pair count (N*(N-1)/2) to the spatial
//!   check count, showing the savings as entities cluster or spread out.
//!
//! Balls that are within proximity of another ball turn red.

use bevy::prelude::*;
use std::collections::HashMap;

const WINDOW_W: f32 = 800.0;
const WINDOW_H: f32 = 500.0;
const CELL_SIZE: f32 = 60.0;
const BALL_COUNT: usize = 60;
const BALL_RADIUS: f32 = 8.0;
const PROXIMITY_RADIUS: f32 = CELL_SIZE;

// ── Pure helpers ──────────────────────────────────────────────────────────────

/// Grid cell that contains `pos`.
pub fn cell_of(pos: Vec2, cell_size: f32) -> IVec2 {
    IVec2::new(
        (pos.x / cell_size).floor() as i32,
        (pos.y / cell_size).floor() as i32,
    )
}

/// The 3×3 neighbourhood (8 neighbours + self) of a cell.
pub fn neighbour_cells(cell: IVec2) -> [IVec2; 9] {
    [
        cell + IVec2::new(-1, -1), cell + IVec2::new(0, -1), cell + IVec2::new(1, -1),
        cell + IVec2::new(-1,  0), cell,                      cell + IVec2::new(1,  0),
        cell + IVec2::new(-1,  1), cell + IVec2::new(0,  1),  cell + IVec2::new(1,  1),
    ]
}

/// Number of unique pairs in N entities (brute-force cost).
pub fn brute_pairs(n: usize) -> usize { n.saturating_sub(1) * n / 2 }

// ── ECS ───────────────────────────────────────────────────────────────────────

#[derive(Component)]
struct Ball { vel: Vec2, near: bool }

#[derive(Resource, Default)]
struct SpatialGrid(HashMap<IVec2, Vec<usize>>);

#[derive(Resource, Default)]
struct Stats { spatial_checks: usize }

#[derive(Component)]
struct HudText;

fn main() {
    let mut rng_seed: u64 = 0xDEAD_BEEF_C0DE_1234;
    let mut rng = move || -> f32 {
        rng_seed ^= rng_seed << 13;
        rng_seed ^= rng_seed >> 7;
        rng_seed ^= rng_seed << 17;
        (rng_seed & 0xFFFF) as f32 / 65535.0
    };

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Spatial Partitioning".into(),
                resolution: (800u32, 500u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(SpatialGrid::default())
        .insert_resource(Stats::default())
        .add_systems(Startup, move |mut commands: Commands| {
            commands.spawn(Camera2d);

            // Spawn balls with random positions and velocities seeded above.
            for _ in 0..BALL_COUNT {
                let x = rng() * WINDOW_W - WINDOW_W / 2.0;
                let y = rng() * WINDOW_H - WINDOW_H / 2.0;
                let vx = (rng() - 0.5) * 140.0;
                let vy = (rng() - 0.5) * 140.0;
                commands.spawn((
                    Ball { vel: Vec2::new(vx, vy), near: false },
                    Sprite { color: Color::srgb(0.3, 0.55, 1.0), custom_size: Some(Vec2::splat(BALL_RADIUS * 2.0)), ..default() },
                    Transform::from_translation(Vec3::new(x, y, 0.0)),
                ));
            }

            commands.spawn((
                HudText,
                Text::new(""),
                TextFont { font_size: 15.0, ..default() },
                TextColor(Color::WHITE),
                Node { position_type: PositionType::Absolute, top: Val::Px(10.0), left: Val::Px(10.0), ..default() },
            ));
        })
        .add_systems(Update, (move_balls, rebuild_grid, check_proximity, paint_balls, update_hud).chain())
        .run();
}

fn move_balls(time: Res<Time>, mut q: Query<(&mut Ball, &mut Transform)>) {
    let dt = time.delta_secs();
    let hw = WINDOW_W / 2.0 - BALL_RADIUS;
    let hh = WINDOW_H / 2.0 - BALL_RADIUS;
    for (mut ball, mut tf) in &mut q {
        tf.translation += (ball.vel * dt).extend(0.0);
        if tf.translation.x >  hw { tf.translation.x =  hw; ball.vel.x *= -1.0; }
        if tf.translation.x < -hw { tf.translation.x = -hw; ball.vel.x *= -1.0; }
        if tf.translation.y >  hh { tf.translation.y =  hh; ball.vel.y *= -1.0; }
        if tf.translation.y < -hh { tf.translation.y = -hh; ball.vel.y *= -1.0; }
    }
}

fn rebuild_grid(
    q: Query<(Entity, &Transform), With<Ball>>,
    mut grid: ResMut<SpatialGrid>,
) {
    grid.0.clear();
    for (i, (_, tf)) in q.iter().enumerate() {
        let cell = cell_of(tf.translation.truncate(), CELL_SIZE);
        grid.0.entry(cell).or_default().push(i);
    }
}

fn check_proximity(
    q: Query<(Entity, &Transform), With<Ball>>,
    grid: Res<SpatialGrid>,
    mut stats: ResMut<Stats>,
    mut ball_q: Query<&mut Ball>,
) {
    // Collect positions into a Vec for indexed access.
    let positions: Vec<(Entity, Vec2)> = q.iter()
        .map(|(e, tf)| (e, tf.translation.truncate()))
        .collect();

    // Reset near flag.
    for mut b in &mut ball_q { b.near = false; }

    let mut checks = 0usize;
    for (i, (ei, pos_i)) in positions.iter().enumerate() {
        let cell = cell_of(*pos_i, CELL_SIZE);
        for ncell in neighbour_cells(cell) {
            if let Some(neighbours) = grid.0.get(&ncell) {
                for &j in neighbours {
                    if j <= i { continue; }
                    if let Some((ej, pos_j)) = positions.get(j) {
                        checks += 1;
                        if pos_i.distance(*pos_j) < PROXIMITY_RADIUS {
                            if let Ok(mut b) = ball_q.get_mut(*ei) { b.near = true; }
                            if let Ok(mut b) = ball_q.get_mut(*ej) { b.near = true; }
                        }
                    }
                }
            }
        }
    }
    stats.spatial_checks = checks;
}

fn paint_balls(mut q: Query<(&Ball, &mut Sprite)>) {
    for (ball, mut sprite) in &mut q {
        sprite.color = if ball.near {
            Color::srgb(1.0, 0.3, 0.3)
        } else {
            Color::srgb(0.3, 0.55, 1.0)
        };
    }
}

fn update_hud(stats: Res<Stats>, mut q: Query<&mut Text, With<HudText>>) {
    let Ok(mut text) = q.single_mut() else { return };
    let brute = brute_pairs(BALL_COUNT);
    let saved = brute.saturating_sub(stats.spatial_checks);
    text.0 = format!(
        "Balls: {BALL_COUNT}  |  Brute-force pairs: {brute}  |  Spatial checks: {}  |  Saved: {saved}  |  Red = within proximity",
        stats.spatial_checks,
    );
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_of_origin() {
        assert_eq!(cell_of(Vec2::ZERO, 60.0), IVec2::ZERO);
    }

    #[test]
    fn cell_of_positive_quadrant() {
        assert_eq!(cell_of(Vec2::new(65.0, 125.0), 60.0), IVec2::new(1, 2));
    }

    #[test]
    fn cell_of_negative_quadrant() {
        assert_eq!(cell_of(Vec2::new(-10.0, -70.0), 60.0), IVec2::new(-1, -2));
    }

    #[test]
    fn neighbour_cells_count() {
        assert_eq!(neighbour_cells(IVec2::ZERO).len(), 9);
    }

    #[test]
    fn neighbour_cells_contains_self() {
        let cell = IVec2::new(3, 5);
        assert!(neighbour_cells(cell).contains(&cell));
    }

    #[test]
    fn brute_pairs_zero_and_one() {
        assert_eq!(brute_pairs(0), 0);
        assert_eq!(brute_pairs(1), 0);
        assert_eq!(brute_pairs(4), 6);
    }
}
