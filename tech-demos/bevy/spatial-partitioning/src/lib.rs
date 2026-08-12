//! Spatial partitioning — grid-cell bucketing for O(1) neighbour queries.
//!
//! This crate is a *building block*: drop [`SpatialPartitioningPlugin`] into any
//! Bevy app with `app.add_plugins(SpatialPartitioningPlugin)`.
//!
//! Key ideas:
//! - The world is divided into a fixed-size grid. Each frame every entity is
//!   assigned to the cell that contains its position.
//! - Proximity checks only examine entities in the same cell and its 8 neighbours —
//!   a 3×3 patch — instead of checking every pair.
//! - [`cell_of`] and [`neighbour_cells`] are pure functions with no ECS dependency.
//! - The HUD compares the brute-force pair count (N*(N-1)/2) to the spatial
//!   check count, showing the savings as entities cluster or spread out.
//!
//! Balls that are within proximity of another ball turn red. Tune the grid,
//! ball count, and speeds through [`SpatialPartitioningConfig`].
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use spatial_partitioning::SpatialPartitioningPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(SpatialPartitioningPlugin)
//!     .run();
//! ```

use bevy::prelude::*;
use std::collections::HashMap;

/// Bundles every system and resource for the spatial-partitioning demo.
///
/// Add it with `app.add_plugins(SpatialPartitioningPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct SpatialPartitioningPlugin;

impl Plugin for SpatialPartitioningPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpatialPartitioningConfig>()
            .init_resource::<SpatialGrid>()
            .init_resource::<Stats>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    move_balls,
                    rebuild_grid,
                    check_proximity,
                    paint_balls,
                    update_hud,
                )
                    .chain(),
            );
    }
}

// ── Configuration ──────────────────────────────────────────────────────────────

/// Tunable parameters for the demo. Override before adding the plugin, e.g.
/// `app.insert_resource(SpatialPartitioningConfig { ball_count: 120, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct SpatialPartitioningConfig {
    /// Half-extents area width used for spawning and bouncing balls.
    pub world_width: f32,
    /// Half-extents area height used for spawning and bouncing balls.
    pub world_height: f32,
    /// Edge length of each square grid cell.
    pub cell_size: f32,
    /// Number of balls to spawn.
    pub ball_count: usize,
    /// Visual radius of each ball.
    pub ball_radius: f32,
    /// Distance under which two balls count as "near" (turn red).
    pub proximity_radius: f32,
    /// Maximum per-axis speed magnitude of a spawned ball.
    pub max_speed: f32,
}

impl Default for SpatialPartitioningConfig {
    fn default() -> Self {
        Self {
            world_width: 800.0,
            world_height: 500.0,
            cell_size: 60.0,
            ball_count: 60,
            ball_radius: 8.0,
            proximity_radius: 60.0,
            max_speed: 140.0,
        }
    }
}

// ── Pure helpers ────────────────────────────────────────────────────────────────

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
        cell + IVec2::new(-1, -1),
        cell + IVec2::new(0, -1),
        cell + IVec2::new(1, -1),
        cell + IVec2::new(-1, 0),
        cell,
        cell + IVec2::new(1, 0),
        cell + IVec2::new(-1, 1),
        cell + IVec2::new(0, 1),
        cell + IVec2::new(1, 1),
    ]
}

/// Number of unique pairs in N entities (brute-force cost).
pub fn brute_pairs(n: usize) -> usize {
    n.saturating_sub(1) * n / 2
}

// ── ECS ─────────────────────────────────────────────────────────────────────────

/// A moving ball; `near` is set when another ball is within proximity.
#[derive(Component)]
pub struct Ball {
    /// Velocity in world units per second.
    pub vel: Vec2,
    /// Whether this ball is within proximity of another ball this frame.
    pub near: bool,
}

/// Buckets ball indices by the grid cell they occupy, rebuilt each frame.
#[derive(Resource, Default)]
pub struct SpatialGrid(pub HashMap<IVec2, Vec<usize>>);

/// Per-frame count of the proximity checks the spatial grid performed.
#[derive(Resource, Default)]
pub struct Stats {
    /// Number of pair checks the spatial approach performed this frame.
    pub spatial_checks: usize,
}

/// Marker for the HUD text entity.
#[derive(Component)]
pub struct HudText;

fn setup(mut commands: Commands, config: Res<SpatialPartitioningConfig>) {
    let mut rng_seed: u64 = 0xDEAD_BEEF_C0DE_1234;
    let mut rng = move || -> f32 {
        rng_seed ^= rng_seed << 13;
        rng_seed ^= rng_seed >> 7;
        rng_seed ^= rng_seed << 17;
        (rng_seed & 0xFFFF) as f32 / 65535.0
    };

    commands.spawn(Camera2d);

    // Spawn balls with random positions and velocities seeded above.
    for _ in 0..config.ball_count {
        let x = rng() * config.world_width - config.world_width / 2.0;
        let y = rng() * config.world_height - config.world_height / 2.0;
        let vx = (rng() - 0.5) * config.max_speed;
        let vy = (rng() - 0.5) * config.max_speed;
        commands.spawn((
            Ball {
                vel: Vec2::new(vx, vy),
                near: false,
            },
            Sprite {
                color: Color::srgb(0.3, 0.55, 1.0),
                custom_size: Some(Vec2::splat(config.ball_radius * 2.0)),
                ..default()
            },
            Transform::from_translation(Vec3::new(x, y, 0.0)),
        ));
    }

    commands.spawn((
        HudText,
        Text::new(""),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

fn move_balls(
    time: Res<Time>,
    config: Res<SpatialPartitioningConfig>,
    mut q: Query<(&mut Ball, &mut Transform)>,
) {
    let dt = time.delta_secs();
    let hw = config.world_width / 2.0 - config.ball_radius;
    let hh = config.world_height / 2.0 - config.ball_radius;
    for (mut ball, mut tf) in &mut q {
        tf.translation += (ball.vel * dt).extend(0.0);
        if tf.translation.x > hw {
            tf.translation.x = hw;
            ball.vel.x *= -1.0;
        }
        if tf.translation.x < -hw {
            tf.translation.x = -hw;
            ball.vel.x *= -1.0;
        }
        if tf.translation.y > hh {
            tf.translation.y = hh;
            ball.vel.y *= -1.0;
        }
        if tf.translation.y < -hh {
            tf.translation.y = -hh;
            ball.vel.y *= -1.0;
        }
    }
}

fn rebuild_grid(
    q: Query<(Entity, &Transform), With<Ball>>,
    config: Res<SpatialPartitioningConfig>,
    mut grid: ResMut<SpatialGrid>,
) {
    grid.0.clear();
    for (i, (_, tf)) in q.iter().enumerate() {
        let cell = cell_of(tf.translation.truncate(), config.cell_size);
        grid.0.entry(cell).or_default().push(i);
    }
}

fn check_proximity(
    q: Query<(Entity, &Transform), With<Ball>>,
    config: Res<SpatialPartitioningConfig>,
    grid: Res<SpatialGrid>,
    mut stats: ResMut<Stats>,
    mut ball_q: Query<&mut Ball>,
) {
    // Collect positions into a Vec for indexed access.
    let positions: Vec<(Entity, Vec2)> = q
        .iter()
        .map(|(e, tf)| (e, tf.translation.truncate()))
        .collect();

    // Reset near flag.
    for mut b in &mut ball_q {
        b.near = false;
    }

    let mut checks = 0usize;
    for (i, (ei, pos_i)) in positions.iter().enumerate() {
        let cell = cell_of(*pos_i, config.cell_size);
        for ncell in neighbour_cells(cell) {
            if let Some(neighbours) = grid.0.get(&ncell) {
                for &j in neighbours {
                    if j <= i {
                        continue;
                    }
                    if let Some((ej, pos_j)) = positions.get(j) {
                        checks += 1;
                        if pos_i.distance(*pos_j) < config.proximity_radius {
                            if let Ok(mut b) = ball_q.get_mut(*ei) {
                                b.near = true;
                            }
                            if let Ok(mut b) = ball_q.get_mut(*ej) {
                                b.near = true;
                            }
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

fn update_hud(
    config: Res<SpatialPartitioningConfig>,
    stats: Res<Stats>,
    mut q: Query<&mut Text, With<HudText>>,
) {
    let Ok(mut text) = q.single_mut() else { return };
    let brute = brute_pairs(config.ball_count);
    let saved = brute.saturating_sub(stats.spatial_checks);
    text.0 = format!(
        "Balls: {}  |  Brute-force pairs: {brute}  |  Spatial checks: {}  |  Saved: {saved}  |  Red = within proximity",
        config.ball_count, stats.spatial_checks,
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

    #[test]
    fn config_default_matches_documented_values() {
        let c = SpatialPartitioningConfig::default();
        assert_eq!(c.cell_size, 60.0);
        assert_eq!(c.ball_count, 60);
        assert_eq!(c.proximity_radius, 60.0);
    }

    #[test]
    fn plugin_spawns_expected_ball_count() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, SpatialPartitioningPlugin));
        app.update();

        let mut q = app.world_mut().query::<&Ball>();
        assert_eq!(q.iter(app.world()).count(), 60);
    }
}
