//! Tween animation demo.
//!
//! Key ideas:
//! - A tween component holds `from`, `to`, `duration`, and `elapsed` state.
//!   The tween system advances `elapsed` and writes the interpolated value.
//! - [`smoothstep`] gives ease-in-out; [`ease_out_back`] overshoots for a
//!   "pop" feel.
//! - Three independent tween types (scale, position, alpha) share the same
//!   structure — only the applied field differs.
//!
//! **Bug fixed:** `setup` and `trigger_entrance_tweens` are now chained so
//! `setup` always runs first.  Without `.chain()` Bevy may run them in any
//! order within the same `Startup` set, causing `trigger_entrance_tweens` to
//! find no entities to animate.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Chain ensures setup runs before trigger_entrance_tweens.
        .add_systems(Startup, (setup, trigger_entrance_tweens).chain())
        .add_systems(Update, (handle_keys, tick_scale_tweens, tick_slide_tweens, tick_alpha_tweens))
        .run();
}

// --- Tween components ---

/// Animates an entity's uniform scale between `from` and `to`.
#[derive(Component)]
struct ScaleTween {
    from: f32,
    to: f32,
    duration: f32,
    elapsed: f32,
    looping: bool,
}

/// Animates an entity's X translation between `from_x` and `to_x`.
#[derive(Component)]
struct SlideTween {
    from_x: f32,
    to_x: f32,
    duration: f32,
    elapsed: f32,
}

/// Animates an entity's sprite alpha between `from` and `to`.
#[derive(Component)]
struct AlphaTween {
    from: f32,
    to: f32,
    duration: f32,
    elapsed: f32,
    looping: bool,
    base_color: Color,
}

// --- Markers ---

/// Marker for the scale-pop box (left).
#[derive(Component)]
struct ScaleBox;

/// Marker for the slide-in box (center).
#[derive(Component)]
struct SlideBox;

/// Marker for the alpha-pulse box (right).
#[derive(Component)]
struct AlphaBox;

// --- Setup ---

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Sprite { color: Color::srgb(0.9, 0.4, 0.2), custom_size: Some(Vec2::splat(60.0)), ..default() },
        Transform::from_xyz(-200.0, 0.0, 0.0),
        ScaleBox,
    ));

    // Starts off-screen left; slide tween moves it to center.
    commands.spawn((
        Sprite { color: Color::srgb(0.3, 0.75, 0.9), custom_size: Some(Vec2::splat(60.0)), ..default() },
        Transform::from_xyz(-700.0, 0.0, 0.0),
        SlideBox,
    ));

    commands.spawn((
        Sprite { color: Color::srgba(0.6, 0.9, 0.3, 1.0), custom_size: Some(Vec2::splat(60.0)), ..default() },
        Transform::from_xyz(200.0, 0.0, 0.0),
        AlphaBox,
    ));

    for (label, left_px) in [
        ("1 — scale pop", 120.0_f32),
        ("auto — slide in", 360.0),
        ("2 — alpha pulse", 600.0),
    ] {
        commands.spawn((
            Text::new(label),
            TextFont { font_size: 13.0, ..default() },
            TextColor(Color::srgb(0.65, 0.65, 0.65)),
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(130.0),
                left: Val::Px(left_px),
                ..default()
            },
        ));
    }

    commands.spawn((
        Text::new("1 = scale pop   2 = alpha pulse   (slide plays on startup)"),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgb(0.55, 0.55, 0.55)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

/// Inserts entrance tweens onto the boxes that were just spawned by [`setup`].
///
/// Must run after [`setup`] — guaranteed by `.chain()` in the `Startup` schedule.
fn trigger_entrance_tweens(
    mut commands: Commands,
    slide_query: Query<Entity, With<SlideBox>>,
    alpha_query: Query<Entity, With<AlphaBox>>,
) {
    if let Ok(entity) = slide_query.single() {
        commands.entity(entity).insert(SlideTween {
            from_x: -700.0, to_x: 0.0, duration: 0.7, elapsed: 0.0,
        });
    }

    if let Ok(entity) = alpha_query.single() {
        commands.entity(entity).insert(AlphaTween {
            from: 0.1, to: 1.0, duration: 1.0, elapsed: 0.0,
            looping: true, base_color: Color::srgb(0.6, 0.9, 0.3),
        });
    }
}

// --- Easing helpers ---

/// Smooth ease-in-out curve: slow at both ends, fast in the middle.
///
/// Maps any `t` in `[0, 1]` to a value in `[0, 1]` using the polynomial
/// `3t² − 2t³`.  Inputs outside `[0, 1]` are clamped first.
pub fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Ease-out with an overshoot (the "back" easing family).
///
/// Reaches a value slightly above 1.0 before settling back to exactly 1.0,
/// giving a springy "pop" feel.  Input should be in `[0, 1]`.
pub fn ease_out_back(t: f32) -> f32 {
    let c1 = 1.70158_f32;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

// --- Input ---

fn handle_keys(
    input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    scale_query: Query<Entity, With<ScaleBox>>,
    alpha_query: Query<Entity, With<AlphaBox>>,
) {
    if input.just_pressed(KeyCode::Digit1) {
        if let Ok(entity) = scale_query.single() {
            commands.entity(entity).insert(ScaleTween {
                from: 1.0, to: 1.6, duration: 0.25, elapsed: 0.0, looping: false,
            });
        }
    }

    if input.just_pressed(KeyCode::Digit2) {
        if let Ok(entity) = alpha_query.single() {
            commands.entity(entity).insert(AlphaTween {
                from: 0.0, to: 1.0, duration: 0.5, elapsed: 0.0,
                looping: true, base_color: Color::srgb(0.6, 0.9, 0.3),
            });
        }
    }
}

// --- Tween systems ---

/// Advances scale tweens and removes them when complete.
fn tick_scale_tweens(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut ScaleTween)>,
) {
    for (entity, mut transform, mut tween) in &mut query {
        tween.elapsed += time.delta_secs();
        let t = (tween.elapsed / tween.duration).clamp(0.0, 1.0);

        // First half: grow with overshoot; second half: shrink back to 1.
        let scale = if t < 0.5 {
            tween.from + (tween.to - tween.from) * ease_out_back(t * 2.0)
        } else {
            tween.to + (1.0 - tween.to) * smoothstep((t - 0.5) * 2.0)
        };
        transform.scale = Vec3::splat(scale);

        if tween.elapsed >= tween.duration {
            transform.scale = Vec3::ONE;
            commands.entity(entity).remove::<ScaleTween>();
        }
    }
}

/// Advances slide tweens and removes them when complete.
fn tick_slide_tweens(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut SlideTween)>,
) {
    for (entity, mut transform, mut tween) in &mut query {
        tween.elapsed += time.delta_secs();
        let t = smoothstep((tween.elapsed / tween.duration).clamp(0.0, 1.0));
        transform.translation.x = tween.from_x + (tween.to_x - tween.from_x) * t;

        if tween.elapsed >= tween.duration {
            transform.translation.x = tween.to_x;
            commands.entity(entity).remove::<SlideTween>();
        }
    }
}

/// Advances alpha tweens (with optional ping-pong looping) and removes
/// non-looping tweens when complete.
fn tick_alpha_tweens(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Sprite, &mut AlphaTween)>,
) {
    for (entity, mut sprite, mut tween) in &mut query {
        tween.elapsed += time.delta_secs();
        let raw_t = tween.elapsed / tween.duration;

        let alpha = if tween.looping {
            let phase = raw_t % 2.0;
            let t = if phase < 1.0 { smoothstep(phase) } else { smoothstep(2.0 - phase) };
            tween.from + (tween.to - tween.from) * t
        } else {
            let t = smoothstep(raw_t.clamp(0.0, 1.0));
            tween.from + (tween.to - tween.from) * t
        };

        let Color::Srgba(s) = tween.base_color else { continue };
        sprite.color = Color::srgba(s.red, s.green, s.blue, alpha);

        if !tween.looping && raw_t >= 1.0 {
            commands.entity(entity).remove::<AlphaTween>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- smoothstep ---

    #[test]
    fn smoothstep_zero_returns_zero() {
        assert_eq!(smoothstep(0.0), 0.0);
    }

    #[test]
    fn smoothstep_one_returns_one() {
        assert_eq!(smoothstep(1.0), 1.0);
    }

    #[test]
    fn smoothstep_half_returns_half() {
        assert!((smoothstep(0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn smoothstep_is_monotonically_increasing() {
        let mut prev = 0.0_f32;
        for i in 1..=100 {
            let t = i as f32 / 100.0;
            let v = smoothstep(t);
            assert!(v >= prev, "smoothstep not monotonic at t={t}: {v} < {prev}");
            prev = v;
        }
    }

    #[test]
    fn smoothstep_clamps_below_zero() {
        assert_eq!(smoothstep(-1.0), 0.0);
    }

    #[test]
    fn smoothstep_clamps_above_one() {
        assert_eq!(smoothstep(2.0), 1.0);
    }

    // --- ease_out_back ---

    #[test]
    fn ease_out_back_at_zero_is_zero() {
        assert!(ease_out_back(0.0).abs() < 1e-5);
    }

    #[test]
    fn ease_out_back_at_one_is_one() {
        assert!((ease_out_back(1.0) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn ease_out_back_overshoots_between_endpoints() {
        let max = (0..=100)
            .map(|i| ease_out_back(i as f32 / 100.0))
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(max > 1.0, "ease_out_back should overshoot above 1.0");
    }

    // --- ECS setup ---

    #[test]
    fn setup_spawns_three_boxes() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, (setup, trigger_entrance_tweens).chain());
        app.update();

        let mut scale_q = app.world_mut().query::<&ScaleBox>();
        let mut slide_q = app.world_mut().query::<&SlideBox>();
        let mut alpha_q = app.world_mut().query::<&AlphaBox>();

        assert_eq!(scale_q.iter(app.world()).count(), 1, "should be exactly one ScaleBox");
        assert_eq!(slide_q.iter(app.world()).count(), 1, "should be exactly one SlideBox");
        assert_eq!(alpha_q.iter(app.world()).count(), 1, "should be exactly one AlphaBox");
    }

    #[test]
    fn trigger_entrance_tweens_attaches_slide_tween() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, (setup, trigger_entrance_tweens).chain());
        app.update();

        let mut q = app.world_mut().query::<(&SlideBox, &SlideTween)>();
        assert_eq!(q.iter(app.world()).count(), 1, "SlideBox should have a SlideTween after startup");
    }
}
