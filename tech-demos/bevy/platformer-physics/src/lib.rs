//! Platformer-physics — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`PlatformerPhysicsPlugin`] into any
//! Bevy app with `app.add_plugins(PlatformerPhysicsPlugin)` and it manages a
//! player with gravity, AABB collision, coyote time, and a jump buffer. Tune it
//! through the [`PlatformerConfig`] resource without editing the plugin.
//!
//! Key ideas:
//! - Physics ([`apply_gravity`], [`move_and_collide`]) runs in `FixedUpdate` for
//!   deterministic stepping independent of frame rate.
//! - Input is read in `Update` and written into [`Player`] fields; `FixedUpdate`
//!   systems consume those fields, avoiding a per-frame input miss.
//! - **Coyote time** lets the player jump for a brief window after walking off
//!   a ledge (they were recently grounded but haven't jumped yet).
//! - **Jump buffer** remembers a jump press that arrived slightly before the
//!   player landed, consuming it on the next grounded frame.
//! - Collision resolution is factored into [`aabb_penetration`], a pure function
//!   that is unit-testable without a World.
//!
//! **Controls:** A/D or arrow keys to move; SPACE or W/Up to jump.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use platformer_physics::PlatformerPhysicsPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(PlatformerPhysicsPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system and resource for the platformer feature.
///
/// Add it with `app.add_plugins(PlatformerPhysicsPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct PlatformerPhysicsPlugin;

impl Plugin for PlatformerPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlatformerConfig>()
            .add_systems(Startup, setup)
            .add_systems(Update, read_input)
            .add_systems(FixedUpdate, (apply_gravity, move_and_collide).chain());
    }
}

// --- Configuration ---

/// Tunable parameters for the feature. Override before adding the plugin, e.g.
/// `app.insert_resource(PlatformerConfig { move_speed: 300.0, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct PlatformerConfig {
    /// Pixels of downward acceleration per second squared (negative).
    pub gravity: f32,
    /// Initial upward velocity on jump.
    pub jump_velocity: f32,
    /// Horizontal move speed in pixels per second.
    pub move_speed: f32,
    /// Seconds the player can still jump after leaving a platform.
    pub coyote_time: f32,
    /// Seconds a buffered jump press remains active.
    pub jump_buffer_time: f32,
}

impl Default for PlatformerConfig {
    fn default() -> Self {
        Self {
            gravity: -900.0,
            jump_velocity: 420.0,
            move_speed: 200.0,
            coyote_time: 0.12,
            jump_buffer_time: 0.15,
        }
    }
}

// --- Components ---

/// Axis-aligned bounding box for collision.
#[derive(Component, Clone, Copy)]
pub struct Aabb {
    /// Half-extents (half width, half height) in pixels.
    pub half: Vec2,
}

/// Player state including physics and input buffers.
#[derive(Component)]
pub struct Player {
    /// Current velocity in pixels per second.
    pub velocity: Vec2,
    /// True when the player is resting on a platform this frame.
    pub grounded: bool,
    /// Counts down while the player can still jump after leaving a ledge.
    pub coyote_timer: f32,
    /// Counts down from a jump press, consumed on the next grounded frame.
    pub jump_buffer: f32,
    /// Horizontal axis: -1.0, 0.0, or +1.0.
    pub move_axis: f32,
    /// Jump was requested this physics step.
    pub jump_requested: bool,
}

/// Static platform with an [`Aabb`] component.
#[derive(Component)]
pub struct Platform;

/// Spawns the camera, player, and platforms.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Ground platform.
    commands.spawn((
        Sprite {
            color: Color::srgb(0.4, 0.7, 0.3),
            custom_size: Some(Vec2::new(800.0, 30.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, -220.0, 0.0)),
        Aabb {
            half: Vec2::new(400.0, 15.0),
        },
        Platform,
    ));
    // Left ledge.
    commands.spawn((
        Sprite {
            color: Color::srgb(0.4, 0.7, 0.3),
            custom_size: Some(Vec2::new(200.0, 20.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(-250.0, -80.0, 0.0)),
        Aabb {
            half: Vec2::new(100.0, 10.0),
        },
        Platform,
    ));
    // Right ledge.
    commands.spawn((
        Sprite {
            color: Color::srgb(0.4, 0.7, 0.3),
            custom_size: Some(Vec2::new(200.0, 20.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(250.0, 60.0, 0.0)),
        Aabb {
            half: Vec2::new(100.0, 10.0),
        },
        Platform,
    ));

    // Player.
    commands.spawn((
        Sprite {
            color: Color::srgb(0.9, 0.5, 0.2),
            custom_size: Some(Vec2::new(28.0, 40.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(-200.0, 0.0, 1.0)),
        Aabb {
            half: Vec2::new(14.0, 20.0),
        },
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
        Text::new("A/D - move   SPACE - jump"),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

/// Reads keyboard state and writes into the [`Player`] input fields.
fn read_input(
    input: Res<ButtonInput<KeyCode>>,
    config: Res<PlatformerConfig>,
    mut query: Query<&mut Player>,
) {
    let Ok(mut player) = query.single_mut() else {
        return;
    };

    let left = input.pressed(KeyCode::KeyA) || input.pressed(KeyCode::ArrowLeft);
    let right = input.pressed(KeyCode::KeyD) || input.pressed(KeyCode::ArrowRight);
    player.move_axis = right as i8 as f32 - left as i8 as f32;

    let jump_pressed = input.just_pressed(KeyCode::Space)
        || input.just_pressed(KeyCode::KeyW)
        || input.just_pressed(KeyCode::ArrowUp);
    if jump_pressed {
        player.jump_buffer = config.jump_buffer_time;
    }
}

/// Applies gravity and decays the coyote / jump-buffer timers each fixed step.
fn apply_gravity(time: Res<Time>, config: Res<PlatformerConfig>, mut query: Query<&mut Player>) {
    let Ok(mut player) = query.single_mut() else {
        return;
    };
    let dt = time.delta_secs();

    if !player.grounded {
        player.velocity.y += config.gravity * dt;
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
        player.velocity.y = config.jump_velocity;
        player.jump_buffer = 0.0;
        player.coyote_timer = 0.0;
        player.grounded = false;
    }
}

/// Moves the player and resolves AABB overlaps with platforms.
fn move_and_collide(
    time: Res<Time>,
    config: Res<PlatformerConfig>,
    mut player_query: Query<(&mut Transform, &mut Player, &Aabb)>,
    platform_query: Query<(&Transform, &Aabb), (With<Platform>, Without<Player>)>,
) {
    let Ok((mut p_transform, mut player, &p_aabb)) = player_query.single_mut() else {
        return;
    };
    let dt = time.delta_secs();

    player.velocity.x = player.move_axis * config.move_speed;

    // Integrate.
    p_transform.translation.x += player.velocity.x * dt;
    p_transform.translation.y += player.velocity.y * dt;

    // Horizontal screen boundary.
    p_transform.translation.x = p_transform
        .translation
        .x
        .clamp(-400.0 + p_aabb.half.x, 400.0 - p_aabb.half.x);

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
        player.coyote_timer = config.coyote_time;
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
        Aabb {
            half: Vec2::new(half_x, half_y),
        }
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
    fn config_default_values_are_sane() {
        let c = PlatformerConfig::default();
        assert!(c.gravity < 0.0);
        assert!(c.jump_velocity > 0.0);
        assert!(c.move_speed > 0.0);
        assert!(c.coyote_time > 0.0 && c.coyote_time < 1.0);
        assert!(c.jump_buffer_time > 0.0 && c.jump_buffer_time < 1.0);
    }

    #[test]
    fn jump_is_still_rising_before_apex() {
        // Apex is at t = jump_velocity / |gravity| ≈ 0.47 s for the defaults, so
        // the player should still be rising a bit earlier.
        let c = PlatformerConfig::default();
        let apex_t = c.jump_velocity / -c.gravity;
        let vy = c.jump_velocity + c.gravity * (apex_t * 0.8);
        assert!(vy > 0.0, "jump should still be rising before apex");
    }

    #[test]
    fn setup_spawns_one_player() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<PlatformerConfig>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Player>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn setup_spawns_three_platforms() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<PlatformerConfig>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Platform>();
        assert_eq!(q.iter(app.world()).count(), 3);
    }
}
