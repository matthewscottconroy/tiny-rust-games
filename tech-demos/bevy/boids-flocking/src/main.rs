//! Boids flocking demo.
//!
//! Key ideas:
//! - Three steering rules (separation, alignment, cohesion) produce emergent
//!   flocking behavior with no central controller.
//! - Positions are snapshotted into a `Vec` each frame so we can read all boids
//!   while mutably updating them — Bevy's borrow checker forbids reading and
//!   writing the same query simultaneously.
//! - Screen-wrap prevents boids from flying off into infinity.
//! - Golden-ratio angle spread gives well-distributed initial velocities
//!   without needing a rand crate.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (flock, move_boids, wrap_boids))
        .run();
}

// --- Components ---

/// Tags a boid entity; used to filter queries in every flocking system.
#[derive(Component)]
struct Boid;

/// 2D linear velocity for boid movement.
#[derive(Component)]
struct Velocity(Vec2);

// --- Constants ---

const BOID_COUNT: usize = 80;
const NEIGHBOR_RADIUS: f32 = 60.0;
const MAX_SPEED: f32 = 120.0;
const MIN_SPEED: f32 = 40.0;

const SEPARATION_WEIGHT: f32 = 1.8;
const ALIGNMENT_WEIGHT: f32  = 1.0;
const COHESION_WEIGHT: f32   = 0.8;

// --- Setup ---

/// Spawns all boids with golden-ratio-distributed initial velocities and hue-cycled colors.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let golden = 2.399_963_f32; // 2π/φ² — distributes points uniformly on a circle

    for i in 0..BOID_COUNT {
        let angle = i as f32 * golden;
        let spawn_r = 20.0 + (i % 15) as f32 * 16.0;
        let spawn_a = i as f32 * golden * 1.3;
        let x = spawn_a.cos() * spawn_r;
        let y = spawn_a.sin() * spawn_r;

        let speed = MIN_SPEED + (i % 5) as f32 * 16.0;
        let vel = Vec2::new(angle.cos(), angle.sin()) * speed;

        let hue = (i as f32 / BOID_COUNT as f32) * 6.0;
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
fn flock(time: Res<Time>, mut query: Query<(Entity, &mut Transform, &mut Velocity), With<Boid>>) {
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
            if dist < NEIGHBOR_RADIUS && dist > 0.0 {
                separation += (pos - other_pos) / dist;
                alignment  += other_vel;
                cohesion   += other_pos;
                count += 1;
            }
        }

        if count > 0 {
            let n = count as f32;
            let sep_force = separation.normalize_or_zero() * SEPARATION_WEIGHT;
            let ali_force = (alignment / n).normalize_or_zero() * ALIGNMENT_WEIGHT;
            let coh_force = ((cohesion / n) - pos).normalize_or_zero() * COHESION_WEIGHT;

            vel.0 += (sep_force + ali_force + coh_force) * time.delta_secs() * 60.0;
        }

        let speed = vel.0.length();
        if speed > MAX_SPEED {
            vel.0 = vel.0 / speed * MAX_SPEED;
        } else if speed < MIN_SPEED && speed > 0.0 {
            vel.0 = vel.0 / speed * MIN_SPEED;
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

    // --- ECS ---

    #[test]
    fn setup_spawns_correct_boid_count() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Boid>();
        assert_eq!(q.iter(app.world()).count(), BOID_COUNT);
    }
}
