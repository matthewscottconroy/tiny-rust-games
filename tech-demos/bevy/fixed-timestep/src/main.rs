//! Fixed-timestep demo.
//!
//! Key idea: physics runs in `FixedUpdate` at a steady 60 Hz regardless of
//! frame rate.  Rendering (`Update`) runs as fast as the monitor allows.
//! This keeps the simulation deterministic and decoupled from display refresh.

use bevy::prelude::*;

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
    for (mut transform, mut velocity) in &mut query {
        velocity.0.y += GRAVITY * time.delta_secs();
        transform.translation.x += velocity.0.x * time.delta_secs();
        transform.translation.y += velocity.0.y * time.delta_secs();
    }
}

/// Reflects balls when they hit the arena walls or floor/ceiling.
fn bounce_walls(mut query: Query<(&Transform, &mut Velocity), With<Ball>>) {
    for (transform, mut velocity) in &mut query {
        let p = transform.translation;
        if p.y < -270.0 && velocity.0.y < 0.0 { velocity.0.y *= -0.75; }
        if p.x.abs() > 420.0               { velocity.0.x *= -1.0; }
        if p.y > 290.0 && velocity.0.y > 0.0  { velocity.0.y *= -1.0; }
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
}
