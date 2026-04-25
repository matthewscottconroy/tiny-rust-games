//! Screen wrap demo.
//!
//! Key idea: query the primary [`Window`] for its dimensions and use them to
//! implement toroidal (Pac-Man style) position wrapping.  Works regardless of
//! window size because the wrap boundaries are derived at runtime.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (move_player, move_objects, wrap_positions))
        .run();
}

// --- Components ---

/// Marks the player entity.
#[derive(Component)]
struct Player;

/// Tags any entity that should wrap when it crosses the screen boundary.
#[derive(Component)]
struct Wrappable;

/// 2D linear velocity for autonomous objects.
#[derive(Component)]
struct Velocity(Vec2);

// --- Setup ---

/// Spawns the player, a set of autonomously drifting objects, and a HUD label.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Sprite {
            color: Color::srgb(0.3, 0.85, 0.4),
            custom_size: Some(Vec2::splat(28.0)),
            ..default()
        },
        Transform::default(),
        Player,
        Wrappable,
        Velocity(Vec2::ZERO),
    ));

    let objects: &[(Vec3, Vec2, Color)] = &[
        (Vec3::new(-180.0,  80.0, 0.0), Vec2::new(110.0,   60.0), Color::srgb(0.8, 0.35, 0.2)),
        (Vec3::new( 200.0, -90.0, 0.0), Vec2::new(-80.0,  130.0), Color::srgb(0.2, 0.55, 0.85)),
        (Vec3::new(  60.0, 140.0, 0.0), Vec2::new(150.0,  -90.0), Color::srgb(0.8, 0.75, 0.1)),
        (Vec3::new(-220.0,-130.0, 0.0), Vec2::new(-120.0, -70.0), Color::srgb(0.6, 0.2,  0.8)),
        (Vec3::new( 140.0,  30.0, 0.0), Vec2::new(-60.0,  170.0), Color::srgb(0.85, 0.5, 0.15)),
    ];

    for &(pos, vel, color) in objects {
        commands.spawn((
            Sprite { color, custom_size: Some(Vec2::splat(22.0)), ..default() },
            Transform::from_translation(pos),
            Wrappable,
            Velocity(vel),
        ));
    }

    commands.spawn((
        Text::new("WASD — move   objects and player wrap at screen edges"),
        TextFont { font_size: 15.0, ..default() },
        TextColor(Color::srgb(0.75, 0.75, 0.75)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

// --- Pure helper ---

/// Wraps coordinate `v` into `(-half, half]`.
///
/// When `v` exceeds `half` it jumps to the opposite edge; when it is below
/// `-half` it jumps to the corresponding positive edge.  Values already in
/// range are returned unchanged.
pub fn wrap_coord(v: f32, half: f32) -> f32 {
    if v > half       { v - 2.0 * half }
    else if v < -half { v + 2.0 * half }
    else              { v }
}

// --- Systems ---

/// Reads WASD input and sets the player velocity accordingly.
fn move_player(
    input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Velocity, With<Player>>,
) {
    let Ok(mut vel) = query.single_mut() else { return; };
    let mut dir = Vec2::ZERO;
    if input.pressed(KeyCode::KeyW) { dir.y += 1.0; }
    if input.pressed(KeyCode::KeyS) { dir.y -= 1.0; }
    if input.pressed(KeyCode::KeyA) { dir.x -= 1.0; }
    if input.pressed(KeyCode::KeyD) { dir.x += 1.0; }
    vel.0 = if dir != Vec2::ZERO { dir.normalize() * 220.0 } else { Vec2::ZERO };
}

/// Advances all entities with a [`Velocity`] component by `velocity * dt`.
fn move_objects(time: Res<Time>, mut query: Query<(&mut Transform, &Velocity)>) {
    for (mut transform, vel) in &mut query {
        transform.translation.x += vel.0.x * time.delta_secs();
        transform.translation.y += vel.0.y * time.delta_secs();
    }
}

/// Applies toroidal wrapping to every [`Wrappable`] entity.
fn wrap_positions(
    mut query: Query<&mut Transform, With<Wrappable>>,
    window_query: Query<&Window>,
) {
    let Ok(window) = window_query.single() else { return; };
    let half_w = window.width()  / 2.0;
    let half_h = window.height() / 2.0;

    for mut transform in &mut query {
        transform.translation.x = wrap_coord(transform.translation.x, half_w);
        transform.translation.y = wrap_coord(transform.translation.y, half_h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- wrap_coord ---

    #[test]
    fn wrap_coord_inside_range_unchanged() {
        assert_eq!(wrap_coord(0.0,  100.0),  0.0);
        assert_eq!(wrap_coord(50.0, 100.0), 50.0);
    }

    #[test]
    fn wrap_coord_past_right_edge_wraps() {
        assert!((wrap_coord(110.0, 100.0) - (-90.0)).abs() < 1e-5);
    }

    #[test]
    fn wrap_coord_past_left_edge_wraps() {
        assert!((wrap_coord(-110.0, 100.0) - 90.0).abs() < 1e-5);
    }

    #[test]
    fn wrap_coord_exactly_at_right_boundary_unchanged() {
        assert_eq!(wrap_coord(100.0, 100.0), 100.0);
    }

    #[test]
    fn wrap_coord_exactly_at_left_boundary_unchanged() {
        assert_eq!(wrap_coord(-100.0, 100.0), -100.0);
    }

    #[test]
    fn wrap_coord_large_overshoot() {
        // 250 with half=100 → 250 > 100 → 250 - 200 = 50
        assert!((wrap_coord(250.0, 100.0) - 50.0).abs() < 1e-5);
    }

    // --- ECS ---

    #[test]
    fn setup_spawns_one_player() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Player>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn setup_spawns_six_wrappable_entities() {
        // 1 player + 5 autonomous objects
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Wrappable>();
        assert_eq!(q.iter(app.world()).count(), 6);
    }
}
