//! Platformer-physics demo — gravity, AABB collision, coyote time, jump buffer.
//!
//! Key ideas:
//! - Physics (`apply_gravity`, `move_and_collide`) runs in `FixedUpdate` for
//!   deterministic stepping independent of frame rate.
//! - Input is read in `Update` and written into `Player` fields; `FixedUpdate`
//!   systems consume those fields, avoiding a per-frame input miss.
//! - **Coyote time** lets the player jump for a brief window after walking off
//!   a ledge (they were recently grounded but haven't jumped yet).
//! - **Jump buffer** remembers a jump press that arrived slightly before the
//!   player landed, consuming it on the next grounded frame.
//!
//! **Controls:** A/D or arrow keys to move; SPACE or W/Up to jump.

use bevy::prelude::*;

/// Pixels of downward acceleration per second squared.
const GRAVITY: f32 = -900.0;
/// Initial upward velocity on jump.
const JUMP_VELOCITY: f32 = 420.0;
/// Horizontal move speed in pixels per second.
const MOVE_SPEED: f32 = 200.0;
/// Seconds the player can still jump after leaving a platform.
const COYOTE_TIME: f32 = 0.12;
/// Seconds a buffered jump press remains active.
const JUMP_BUFFER_TIME: f32 = 0.15;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Platformer Physics — A/D move, SPACE jump".to_string(),
                resolution: (800.0, 500.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, read_input)
        .add_systems(FixedUpdate, (apply_gravity, move_and_collide).chain())
        .run();
}

/// Axis-aligned bounding box for collision.
#[derive(Component, Clone, Copy)]
struct Aabb {
    /// Half-extents (half width, half height) in pixels.
    half: Vec2,
}

/// Player state including physics and input buffers.
#[derive(Component)]
struct Player {
    velocity: Vec2,
    /// True when the player is resting on a platform this frame.
    grounded: bool,
    /// Counts down while the player can still jump after leaving a ledge.
    coyote_timer: f32,
    /// Counts down from a jump press, consumed on the next grounded frame.
    jump_buffer: f32,
    /// Horizontal axis: -1.0, 0.0, or +1.0.
    move_axis: f32,
    /// Jump was requested this physics step.
    jump_requested: bool,
}

/// Static platform with an `Aabb` component.
#[derive(Component)]
struct Platform;

/// Spawns the camera, player, and platforms.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Ground platform.
    commands.spawn((
        Sprite { color: Color::srgb(0.4, 0.7, 0.3), custom_size: Some(Vec2::new(800.0, 30.0)), ..default() },
        Transform::from_translation(Vec3::new(0.0, -220.0, 0.0)),
        Aabb { half: Vec2::new(400.0, 15.0) },
        Platform,
    ));
    // Left ledge.
    commands.spawn((
        Sprite { color: Color::srgb(0.4, 0.7, 0.3), custom_size: Some(Vec2::new(200.0, 20.0)), ..default() },
        Transform::from_translation(Vec3::new(-250.0, -80.0, 0.0)),
        Aabb { half: Vec2::new(100.0, 10.0) },
        Platform,
    ));
    // Right ledge.
    commands.spawn((
        Sprite { color: Color::srgb(0.4, 0.7, 0.3), custom_size: Some(Vec2::new(200.0, 20.0)), ..default() },
        Transform::from_translation(Vec3::new(250.0, 60.0, 0.0)),
        Aabb { half: Vec2::new(100.0, 10.0) },
        Platform,
    ));

    // Player.
    commands.spawn((
        Sprite { color: Color::srgb(0.9, 0.5, 0.2), custom_size: Some(Vec2::new(28.0, 40.0)), ..default() },
        Transform::from_translation(Vec3::new(-200.0, 0.0, 1.0)),
        Aabb { half: Vec2::new(14.0, 20.0) },
        Player {
            velocity: Vec2::ZERO,
            grounded: false,
            coyote_timer: 0.0,
            jump_buffer: 0.0,
            move_axis: 0.0,
            jump_requested: false,
        },
    ));

    commands.spawn((
        Text::new("A/D — move   SPACE — jump"),
        TextFont { font_size: 20.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

/// Reads keyboard state and writes into the `Player` input fields.
fn read_input(input: Res<ButtonInput<KeyCode>>, mut query: Query<&mut Player>) {
    let Ok(mut player) = query.get_single_mut() else { return };

    let left = input.pressed(KeyCode::KeyA) || input.pressed(KeyCode::ArrowLeft);
    let right = input.pressed(KeyCode::KeyD) || input.pressed(KeyCode::ArrowRight);
    player.move_axis = right as i8 as f32 - left as i8 as f32;

    let jump_pressed = input.just_pressed(KeyCode::Space)
        || input.just_pressed(KeyCode::KeyW)
        || input.just_pressed(KeyCode::ArrowUp);
    if jump_pressed {
        player.jump_buffer = JUMP_BUFFER_TIME;
    }
}

/// Applies gravity and decays the coyote / jump-buffer timers each fixed step.
fn apply_gravity(time: Res<Time>, mut query: Query<&mut Player>) {
    let Ok(mut player) = query.get_single_mut() else { return };
    let dt = time.delta_secs();

    if !player.grounded {
        player.velocity.y += GRAVITY * dt;
    }

    // Decay timers.
    if player.coyote_timer > 0.0 {
        player.coyote_timer = (player.coyote_timer - dt).max(0.0);
    }
    if player.jump_buffer > 0.0 {
        player.jump_buffer = (player.jump_buffer - dt).max(0.0);
    }

    // Jump: allowed while grounded or within coyote window.
    let can_jump = player.grounded || player.coyote_timer > 0.0;
    if player.jump_buffer > 0.0 && can_jump {
        player.velocity.y = JUMP_VELOCITY;
        player.jump_buffer = 0.0;
        player.coyote_timer = 0.0;
        player.grounded = false;
    }
}

/// Moves the player and resolves AABB overlaps with platforms.
fn move_and_collide(
    time: Res<Time>,
    mut player_query: Query<(&mut Transform, &mut Player, &Aabb)>,
    platform_query: Query<(&Transform, &Aabb), (With<Platform>, Without<Player>)>,
) {
    let Ok((mut p_transform, mut player, &p_aabb)) = player_query.get_single_mut() else { return };
    let dt = time.delta_secs();

    player.velocity.x = player.move_axis * MOVE_SPEED;

    // Integrate.
    p_transform.translation.x += player.velocity.x * dt;
    p_transform.translation.y += player.velocity.y * dt;

    // Horizontal screen boundary.
    p_transform.translation.x = p_transform.translation.x.clamp(-400.0 + p_aabb.half.x, 400.0 - p_aabb.half.x);

    let was_grounded = player.grounded;
    player.grounded = false;

    for (plat_transform, plat_aabb) in &platform_query {
        let pp = p_transform.translation.truncate();
        let qq = plat_transform.translation.truncate();
        if let Some(depth) = aabb_penetration(pp, p_aabb, qq, *plat_aabb) {
            // Only resolve from above (falling onto the platform top).
            if depth.y > 0.0 && player.velocity.y <= 0.0 {
                p_transform.translation.y += depth.y;
                player.velocity.y = 0.0;
                player.grounded = true;
            }
        }
    }

    // Start coyote timer when the player walks off a ledge.
    if was_grounded && !player.grounded {
        player.coyote_timer = COYOTE_TIME;
    }
}

/// Returns the vertical penetration depth (y > 0 means push upward) if two
/// AABBs overlap, or `None` if they are separated.
pub fn aabb_penetration(a_pos: Vec2, a: Aabb, b_pos: Vec2, b: Aabb) -> Option<Vec2> {
    let dx = b_pos.x - a_pos.x;
    let dy = b_pos.y - a_pos.y;
    let overlap_x = (a.half.x + b.half.x) - dx.abs();
    let overlap_y = (a.half.y + b.half.y) - dy.abs();
    if overlap_x <= 0.0 || overlap_y <= 0.0 {
        return None;
    }
    // Resolve on the shallowest axis.
    if overlap_x < overlap_y {
        let sign = if dx > 0.0 { -1.0 } else { 1.0 };
        Some(Vec2::new(sign * overlap_x, 0.0))
    } else {
        let sign = if dy > 0.0 { -1.0 } else { 1.0 };
        Some(Vec2::new(0.0, sign * overlap_y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aabb(half_x: f32, half_y: f32) -> Aabb {
        Aabb { half: Vec2::new(half_x, half_y) }
    }

    #[test]
    fn no_overlap_returns_none() {
        let a = aabb(10.0, 10.0);
        let b = aabb(10.0, 10.0);
        assert!(aabb_penetration(Vec2::ZERO, a, Vec2::new(30.0, 0.0), b).is_none());
    }

    #[test]
    fn vertical_overlap_pushes_up() {
        let player = aabb(14.0, 20.0);
        let platform = aabb(200.0, 10.0);
        // Player bottom at y=-5, platform top at y=0 → 5 px overlap.
        let depth = aabb_penetration(Vec2::new(0.0, 5.0), player, Vec2::ZERO, platform);
        assert!(depth.is_some(), "expected overlap");
        let d = depth.unwrap();
        assert!(d.y.abs() > 0.0, "expected vertical resolution");
    }

    #[test]
    fn exact_touch_returns_none() {
        let a = aabb(10.0, 10.0);
        let b = aabb(10.0, 10.0);
        // Exactly touching edges — no penetration.
        let result = aabb_penetration(Vec2::new(-10.0, 0.0), a, Vec2::new(10.0, 0.0), b);
        assert!(result.is_none());
    }

    #[test]
    fn physics_constants_are_sane() {
        assert!(GRAVITY < 0.0);
        assert!(JUMP_VELOCITY > 0.0);
        assert!(MOVE_SPEED > 0.0);
        assert!(COYOTE_TIME > 0.0 && COYOTE_TIME < 1.0);
        assert!(JUMP_BUFFER_TIME > 0.0 && JUMP_BUFFER_TIME < 1.0);
    }

    #[test]
    fn jump_velocity_overcomes_one_second_gravity() {
        // A reasonable platformer: jump should reach positive height after 0.5 s.
        let vy_at_half = JUMP_VELOCITY + GRAVITY * 0.5;
        assert!(vy_at_half > 0.0, "jump apex too low");
    }
}
