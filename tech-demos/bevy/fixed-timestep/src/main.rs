//! Fixed-timestep demo.
//!
//! Key idea: physics runs in `FixedUpdate` at a steady 60 Hz regardless of
//! frame rate.  Rendering (`Update`) runs as fast as the monitor allows.
//! This keeps the simulation deterministic and decoupled from display refresh.

use bevy::prelude::*;

/// Applies gravitational acceleration to a vertical velocity for one timestep.
pub fn apply_gravity(vy: f32, gravity: f32, dt: f32) -> f32 {
    vy + gravity * dt
}

/// Reflects vertical velocity off the floor with restitution when below `floor` and moving down.
pub fn bounce_floor(y: f32, vy: f32, floor: f32, restitution: f32) -> f32 {
    if y < floor && vy < 0.0 { vy * -restitution } else { vy }
}

/// Reflects a velocity component when the position exceeds `limit` on either side.
pub fn reflect_wall(pos: f32, vel: f32, limit: f32) -> f32 {
    if pos.abs() > limit { -vel } else { vel }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .init_resource::<FrameCounters>()
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, (physics_step, bounce_walls, count_fixed))
        .add_systems(Update, (count_update, update_hud))
        .run();
}

// --- Components ---

/// Tags a physics ball.
#[derive(Component)]
struct Ball;

/// 2D linear velocity in world units/second.
#[derive(Component)]
struct Velocity(Vec2);

// --- Resources ---

/// Counts how many times each schedule has run since startup.
#[derive(Resource, Default)]
struct FrameCounters {
    /// Number of `Update` frames rendered.
    update: u64,
    /// Number of `FixedUpdate` physics ticks executed.
    fixed: u64,
}

/// Marks the HUD text that shows the tick counters.
#[derive(Component)]
struct HudText;

// --- Setup ---

/// Spawns two balls with different initial conditions and a tick-counter HUD.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Ball A — falls under simulated gravity
    commands.spawn((
        Sprite { color: Color::srgb(0.9, 0.4, 0.1), custom_size: Some(Vec2::splat(30.0)), ..default() },
        Transform::from_xyz(-100.0, 200.0, 0.0),
        Velocity(Vec2::new(120.0, 0.0)),
        Ball,
    ));

    // Ball B — pure horizontal motion
    commands.spawn((
        Sprite { color: Color::srgb(0.2, 0.7, 0.9), custom_size: Some(Vec2::splat(30.0)), ..default() },
        Transform::from_xyz(100.0, 0.0, 0.0),
        Velocity(Vec2::new(-200.0, 60.0)),
        Ball,
    ));

    commands.spawn((
        Text::new("Fixed: 0  |  Update: 0"),
        TextFont { font_size: 20.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        HudText,
    ));

    commands.spawn((
        Text::new("Physics at 60 Hz fixed step — rendering uncapped"),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgb(0.7, 0.7, 0.7)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

// --- FixedUpdate systems ---

/// Integrates gravity and velocity for all balls.
fn physics_step(time: Res<Time>, mut query: Query<(&mut Transform, &mut Velocity), With<Ball>>) {
    const GRAVITY: f32 = -400.0;
    let dt = time.delta_secs();
    for (mut transform, mut velocity) in &mut query {
        velocity.0.y = apply_gravity(velocity.0.y, GRAVITY, dt);
        transform.translation.x += velocity.0.x * dt;
        transform.translation.y += velocity.0.y * dt;
    }
}

/// Reflects balls when they hit the arena walls or floor/ceiling.
fn bounce_walls(mut query: Query<(&Transform, &mut Velocity), With<Ball>>) {
    for (transform, mut velocity) in &mut query {
        let p = transform.translation;
        velocity.0.y = bounce_floor(p.y, velocity.0.y, -270.0, 0.75);
        velocity.0.x = reflect_wall(p.x, velocity.0.x, 420.0);
        if p.y > 290.0 && velocity.0.y > 0.0 { velocity.0.y *= -1.0; }
    }
}

/// Increments the fixed-update tick counter.
fn count_fixed(mut counters: ResMut<FrameCounters>) {
    counters.fixed += 1;
}

// --- Update systems ---

/// Increments the render-frame counter.
fn count_update(mut counters: ResMut<FrameCounters>) {
    counters.update += 1;
}

/// Rewrites the HUD text with the current tick counts.
fn update_hud(counters: Res<FrameCounters>, mut query: Query<&mut Text, With<HudText>>) {
    for mut text in &mut query {
        *text = Text::new(format!(
            "Fixed ticks: {}  |  Update frames: {}",
            counters.fixed, counters.update
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_counters_default_to_zero() {
        let c = FrameCounters::default();
        assert_eq!(c.update, 0);
        assert_eq!(c.fixed,  0);
    }

    #[test]
    fn setup_spawns_two_balls() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<FrameCounters>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Ball>();
        assert_eq!(q.iter(app.world()).count(), 2);
    }

    #[test]
    fn setup_spawns_one_hud_text() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<FrameCounters>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&HudText>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn apply_gravity_increases_downward_velocity() {
        let vy = apply_gravity(0.0, -400.0, 0.016);
        assert!(vy < 0.0, "gravity should pull velocity negative");
        assert!((vy - (-6.4)).abs() < 1e-3);
    }

    #[test]
    fn bounce_floor_reflects_when_below_floor() {
        let vy = bounce_floor(-280.0, -100.0, -270.0, 0.75);
        assert!(vy > 0.0, "should reflect upward");
        assert!((vy - 75.0).abs() < 1e-4);
    }

    #[test]
    fn bounce_floor_no_reflect_above_floor() {
        let vy = bounce_floor(-100.0, -50.0, -270.0, 0.75);
        assert_eq!(vy, -50.0, "above floor, velocity unchanged");
    }

    #[test]
    fn bounce_floor_no_reflect_moving_upward() {
        // Already moving up through the floor — don't double-reflect
        let vy = bounce_floor(-280.0, 50.0, -270.0, 0.75);
        assert_eq!(vy, 50.0);
    }

    #[test]
    fn reflect_wall_outside_reverses_velocity() {
        let vx = reflect_wall(500.0, 80.0, 420.0);
        assert_eq!(vx, -80.0);
    }

    #[test]
    fn reflect_wall_inside_leaves_velocity_unchanged() {
        let vx = reflect_wall(100.0, 80.0, 420.0);
        assert_eq!(vx, 80.0);
    }
}
