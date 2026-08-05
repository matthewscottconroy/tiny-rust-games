//! Boids flocking — a reusable Bevy plugin.
//!
//! This crate is a *building block*: add [`BoidsFlockingPlugin`] to any Bevy app
//! and it spawns a flock of boids that steer with the three classic rules —
//! separation, alignment, cohesion — producing emergent flocking with no central
//! controller. Tune it through the [`BoidsFlockingConfig`] resource.
//!
//! Key ideas:
//! - Three steering rules produce emergent flocking with no central controller.
//! - Positions are snapshotted into a `Vec` each frame so we can read all boids
//!   while mutably updating them — Bevy's borrow checker forbids reading and
//!   writing the same query simultaneously.
//! - Screen-wrap prevents boids from flying off into infinity.
//! - Golden-ratio angle spread gives well-distributed initial velocities
//!   without needing a rand crate.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use boids_flocking::BoidsFlockingPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(BoidsFlockingPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles the flocking systems and the [`BoidsFlockingConfig`] tunables.
///
/// Add it with `app.add_plugins(BoidsFlockingPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app owns window and rendering setup.
pub struct BoidsFlockingPlugin;

impl Plugin for BoidsFlockingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BoidsFlockingConfig>()
            .add_systems(Startup, setup)
            .add_systems(Update, (flock, move_boids, wrap_boids));
    }
}

// --- Config ---

/// Tunable parameters for the flock. Override before adding the plugin, e.g.
/// `app.insert_resource(BoidsFlockingConfig { boid_count: 150, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct BoidsFlockingConfig {
    /// Number of boids to spawn.
    pub boid_count: usize,
    /// Radius within which other boids count as neighbours.
    pub neighbor_radius: f32,
    /// Upper speed clamp (pixels/second).
    pub max_speed: f32,
    /// Lower speed clamp (pixels/second).
    pub min_speed: f32,
    /// Weight of the separation (avoid crowding) steering force.
    pub separation_weight: f32,
    /// Weight of the alignment (match heading) steering force.
    pub alignment_weight: f32,
    /// Weight of the cohesion (steer toward center) steering force.
    pub cohesion_weight: f32,
}

impl Default for BoidsFlockingConfig {
    fn default() -> Self {
        Self {
            boid_count: 80,
            neighbor_radius: 60.0,
            max_speed: 120.0,
            min_speed: 40.0,
            separation_weight: 1.8,
            alignment_weight: 1.0,
            cohesion_weight: 0.8,
        }
    }
}

// --- Components ---

/// Tags a boid entity; used to filter queries in every flocking system.
#[derive(Component)]
pub struct Boid;

/// 2D linear velocity for boid movement.
#[derive(Component)]
pub struct Velocity(pub Vec2);

// --- Setup ---

/// Spawns all boids with golden-ratio-distributed initial velocities and hue-cycled colors.
fn setup(mut commands: Commands, config: Res<BoidsFlockingConfig>) {
    commands.spawn(Camera2d);

    let golden = 2.399_963_f32; // 2π/φ² — distributes points uniformly on a circle
    let count = config.boid_count;

    for i in 0..count {
        let angle = i as f32 * golden;
        let spawn_r = 20.0 + (i % 15) as f32 * 16.0;
        let spawn_a = i as f32 * golden * 1.3;
        let x = spawn_a.cos() * spawn_r;
        let y = spawn_a.sin() * spawn_r;

        let speed = config.min_speed + (i % 5) as f32 * 16.0;
        let vel = Vec2::new(angle.cos(), angle.sin()) * speed;

        let hue = (i as f32 / count as f32) * 6.0;
        let color = hue_to_rgb(hue);

        commands.spawn((
            Sprite {
                color,
                custom_size: Some(Vec2::new(5.0, 10.0)),
                ..default()
            },
            Transform::from_xyz(x, y, 0.0),
            Boid,
            Velocity(vel),
        ));
    }

    commands.spawn((
        Text::new("Boids: separation + alignment + cohesion → emergent flocking"),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgb(0.6, 0.6, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

/// Converts a hue value in `[0, 6)` to a desaturated sRGB [`Color`].
///
/// The range maps to the standard six-segment color wheel
/// (red → yellow → green → cyan → blue → magenta → red).
/// A small desaturation (blending toward white) keeps boids legible on dark
/// backgrounds.  Values ≥ 6.0 are wrapped via `% 6`.
pub fn hue_to_rgb(h: f32) -> Color {
    let h = h % 6.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();
    let (r, g, b) = match h as u32 {
        0 => (1.0, x,   0.0),
        1 => (x,   1.0, 0.0),
        2 => (0.0, 1.0, x  ),
        3 => (0.0, x,   1.0),
        4 => (x,   0.0, 1.0),
        _ => (1.0, 0.0, x  ),
    };
    Color::srgb(r * 0.7 + 0.3, g * 0.7 + 0.3, b * 0.7 + 0.3)
}

// --- Systems ---

/// Applies separation, alignment, and cohesion steering, then clamps speed and
/// rotates each boid to face its direction of travel.
///
/// Positions are snapshotted before mutation to avoid aliasing the same query
/// both immutably and mutably in the same frame.
fn flock(
    time: Res<Time>,
    config: Res<BoidsFlockingConfig>,
    mut query: Query<(Entity, &mut Transform, &mut Velocity), With<Boid>>,
) {
    let snapshot: Vec<(Entity, Vec2, Vec2)> = query
        .iter()
        .map(|(e, t, v)| (e, t.translation.truncate(), v.0))
        .collect();

    for (entity, mut transform, mut vel) in &mut query {
        let pos = transform.translation.truncate();

        let mut separation = Vec2::ZERO;
        let mut alignment  = Vec2::ZERO;
        let mut cohesion   = Vec2::ZERO;
        let mut count = 0usize;

        for &(other_e, other_pos, other_vel) in &snapshot {
            if other_e == entity { continue; }
            let dist = pos.distance(other_pos);
            if dist < config.neighbor_radius && dist > 0.0 {
                separation += (pos - other_pos) / dist;
                alignment  += other_vel;
                cohesion   += other_pos;
                count += 1;
            }
        }

        if count > 0 {
            let n = count as f32;
            let sep_force = separation.normalize_or_zero() * config.separation_weight;
            let ali_force = (alignment / n).normalize_or_zero() * config.alignment_weight;
            let coh_force = ((cohesion / n) - pos).normalize_or_zero() * config.cohesion_weight;

            vel.0 += (sep_force + ali_force + coh_force) * time.delta_secs() * 60.0;
        }

        let speed = vel.0.length();
        if speed > config.max_speed {
            vel.0 = vel.0 / speed * config.max_speed;
        } else if speed < config.min_speed && speed > 0.0 {
            vel.0 = vel.0 / speed * config.min_speed;
        }

        if speed > 1.0 {
            let angle = vel.0.y.atan2(vel.0.x) - std::f32::consts::FRAC_PI_2;
            transform.rotation = Quat::from_rotation_z(angle);
        }
    }
}

/// Moves each boid along its current velocity.
fn move_boids(time: Res<Time>, mut query: Query<(&mut Transform, &Velocity), With<Boid>>) {
    for (mut transform, vel) in &mut query {
        transform.translation.x += vel.0.x * time.delta_secs();
        transform.translation.y += vel.0.y * time.delta_secs();
    }
}

/// Wraps boids toroidally when they cross the window boundary.
fn wrap_boids(
    mut query: Query<&mut Transform, With<Boid>>,
    window_query: Query<&Window>,
) {
    let Ok(window) = window_query.single() else { return; };
    let hw = window.width() / 2.0;
    let hh = window.height() / 2.0;

    for mut transform in &mut query {
        let x = &mut transform.translation.x;
        if *x >  hw { *x -= 2.0 * hw; }
        if *x < -hw { *x += 2.0 * hw; }
        let y = &mut transform.translation.y;
        if *y >  hh { *y -= 2.0 * hh; }
        if *y < -hh { *y += 2.0 * hh; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- hue_to_rgb ---

    #[test]
    fn hue_zero_is_red_dominant() {
        let Color::Srgba(s) = hue_to_rgb(0.0) else { panic!("expected Srgba"); };
        assert!(s.red > s.green && s.red > s.blue, "h=0 should be red-dominant");
    }

    #[test]
    fn hue_two_is_green_dominant() {
        let Color::Srgba(s) = hue_to_rgb(2.0) else { panic!("expected Srgba"); };
        assert!(s.green > s.red && s.green > s.blue, "h=2 should be green-dominant");
    }

    #[test]
    fn hue_four_is_blue_dominant() {
        let Color::Srgba(s) = hue_to_rgb(4.0) else { panic!("expected Srgba"); };
        assert!(s.blue > s.red && s.blue > s.green, "h=4 should be blue-dominant");
    }

    #[test]
    fn hue_wraps_at_six() {
        let Color::Srgba(a) = hue_to_rgb(0.0) else { panic!(); };
        let Color::Srgba(b) = hue_to_rgb(6.0) else { panic!(); };
        assert!((a.red   - b.red  ).abs() < 1e-5);
        assert!((a.green - b.green).abs() < 1e-5);
        assert!((a.blue  - b.blue ).abs() < 1e-5);
    }

    #[test]
    fn hue_channels_desaturated_above_floor() {
        for i in 0..60 {
            let h = i as f32 * 0.1;
            let Color::Srgba(s) = hue_to_rgb(h) else { panic!(); };
            assert!(s.red   >= 0.29, "red too low at h={h}");
            assert!(s.green >= 0.29, "green too low at h={h}");
            assert!(s.blue  >= 0.29, "blue too low at h={h}");
        }
    }

    #[test]
    fn hue_channels_in_valid_range() {
        for i in 0..60 {
            let h = i as f32 * 0.1;
            let Color::Srgba(s) = hue_to_rgb(h) else { panic!(); };
            assert!(s.red   >= 0.0 && s.red   <= 1.0, "red out of range at h={h}");
            assert!(s.green >= 0.0 && s.green <= 1.0, "green out of range at h={h}");
            assert!(s.blue  >= 0.0 && s.blue  <= 1.0, "blue out of range at h={h}");
        }
    }

    #[test]
    fn config_default_matches_documented_values() {
        let c = BoidsFlockingConfig::default();
        assert_eq!(c.boid_count, 80);
        assert_eq!(c.separation_weight, 1.8);
        assert_eq!(c.alignment_weight, 1.0);
        assert_eq!(c.cohesion_weight, 0.8);
    }

    // --- ECS ---

    #[test]
    fn setup_spawns_correct_boid_count() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<BoidsFlockingConfig>()
            .add_systems(Startup, setup);
        app.update();

        let expected = BoidsFlockingConfig::default().boid_count;
        let mut q = app.world_mut().query::<&Boid>();
        assert_eq!(q.iter(app.world()).count(), expected);
    }
}
