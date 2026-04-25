//! Day-night cycle demo — ambient colour lerping over a virtual 24-hour clock.
//!
//! Key ideas:
//! - `DayClock` resource tracks the virtual hour in `[0.0, 24.0)` and a
//!   configurable speed multiplier.
//! - `time_of_day_to_rgb` is a pure function that blends between four key
//!   colours (night → dawn → day → dusk) using linear interpolation, making
//!   it easy to unit-test without touching Bevy's `Color` type.
//! - `lerp_f32` is extracted as its own testable helper.
//! - The `ClearColor` resource is updated every frame so the background
//!   transitions smoothly; foreground objects (stars, a sun/moon disc, and
//!   silhouette hills) help sell the time of day visually.
//!
//! **Controls:** + / = — speed up time   -  — slow down   R — reset to midnight.

use bevy::prelude::*;
use bevy::window::WindowResolution;

// ─── Pure helpers ────────────────────────────────────────────────────────────

/// Linearly interpolates between `a` and `b` by factor `t` clamped to [0, 1].
pub fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    a + (b - a) * t
}

/// Key sky colours as (r, g, b) tuples (no Bevy dependency).
const NIGHT: (f32, f32, f32) = (0.02, 0.02, 0.08);
const DAWN:  (f32, f32, f32) = (0.72, 0.38, 0.18);
const DAY:   (f32, f32, f32) = (0.42, 0.68, 1.00);
const DUSK:  (f32, f32, f32) = (0.58, 0.18, 0.32);

/// Returns the sky colour (r, g, b) for a given hour in [0.0, 24.0).
///
/// Blends across four key times: midnight (0), dawn (6), noon (12), dusk (18).
pub fn time_of_day_to_rgb(hours: f32) -> (f32, f32, f32) {
    let t = hours.rem_euclid(24.0);
    let (from, to, frac) = if t < 6.0 {
        (NIGHT, DAWN, t / 6.0)
    } else if t < 12.0 {
        (DAWN, DAY, (t - 6.0) / 6.0)
    } else if t < 18.0 {
        (DAY, DUSK, (t - 12.0) / 6.0)
    } else {
        (DUSK, NIGHT, (t - 18.0) / 6.0)
    };
    (
        lerp_f32(from.0, to.0, frac),
        lerp_f32(from.1, to.1, frac),
        lerp_f32(from.2, to.2, frac),
    )
}

/// Returns a human-readable time label for `hours`.
pub fn hours_to_label(hours: f32) -> &'static str {
    let h = hours.rem_euclid(24.0) as u32;
    match h {
        0..=5   => "Night",
        6..=8   => "Dawn",
        9..=16  => "Day",
        17..=19 => "Dusk",
        _       => "Night",
    }
}

// ─── Resources & components ──────────────────────────────────────────────────

#[derive(Resource)]
struct DayClock {
    hours: f32,
    speed: f32, // virtual hours per real second
}

impl Default for DayClock {
    fn default() -> Self {
        Self { hours: 0.0, speed: 1.0 }
    }
}

#[derive(Component)]
struct SunMoon;

#[derive(Component)]
struct StarSprite(f32); // base alpha

#[derive(Component)]
struct HudLabel;

// ─── Bevy app ────────────────────────────────────────────────────────────────

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Day-Night Cycle — +/- to change speed, R to reset".to_string(),
                resolution: (800u32, 500u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.02, 0.02, 0.08)))
        .init_resource::<DayClock>()
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_input, advance_clock, update_sky, update_hud))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Sun / moon disc
    commands.spawn((
        Sprite { color: Color::srgb(1.0, 1.0, 0.8), custom_size: Some(Vec2::splat(60.0)), ..default() },
        Transform::from_translation(Vec3::new(0.0, 100.0, 0.5)),
        SunMoon,
    ));

    // Silhouette hills
    for (x, w, h) in [(-300.0, 260.0, 120.0), (0.0, 320.0, 90.0), (300.0, 220.0, 140.0)] {
        commands.spawn((
            Sprite { color: Color::srgb(0.05, 0.05, 0.08), custom_size: Some(Vec2::new(w, h)), ..default() },
            Transform::from_translation(Vec3::new(x, -250.0 + h / 2.0, 1.0)),
        ));
    }

    // Stars (random-ish scatter)
    let positions: &[(f32, f32)] = &[
        (-350.0, 200.0), (100.0, 180.0), (280.0, 220.0), (-200.0, 230.0),
        (50.0, 210.0), (-100.0, 190.0), (320.0, 200.0), (-280.0, 170.0),
        (150.0, 240.0), (-50.0, 160.0), (240.0, 150.0), (-180.0, 250.0),
    ];
    for &(x, y) in positions {
        commands.spawn((
            Sprite { color: Color::srgba(1.0, 1.0, 1.0, 0.0), custom_size: Some(Vec2::splat(4.0)), ..default() },
            Transform::from_translation(Vec3::new(x, y, 0.3)),
            StarSprite(1.0),
        ));
    }

    // HUD
    commands.spawn((
        Text::new("00:00  Night  ×1.0"),
        TextFont { font_size: 22.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(16.0),
            ..default()
        },
        HudLabel,
    ));
    commands.spawn((
        Text::new("+/= — faster   - — slower   R — reset"),
        TextFont { font_size: 16.0, ..default() },
        TextColor(Color::srgb(0.7, 0.7, 0.7)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(8.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

fn handle_input(input: Res<ButtonInput<KeyCode>>, mut clock: ResMut<DayClock>) {
    if input.just_pressed(KeyCode::Equal) || input.just_pressed(KeyCode::NumpadAdd) {
        clock.speed = (clock.speed * 1.5).min(120.0);
    }
    if input.just_pressed(KeyCode::Minus) || input.just_pressed(KeyCode::NumpadSubtract) {
        clock.speed = (clock.speed / 1.5).max(0.1);
    }
    if input.just_pressed(KeyCode::KeyR) {
        clock.hours = 0.0;
        clock.speed = 1.0;
    }
}

fn advance_clock(time: Res<Time>, mut clock: ResMut<DayClock>) {
    clock.hours = (clock.hours + clock.speed * time.delta_secs()).rem_euclid(24.0);
}

/// Updates the clear colour, sun/moon position and colour, and star alpha.
fn update_sky(
    clock: Res<DayClock>,
    mut clear: ResMut<ClearColor>,
    mut sun_q: Query<(&mut Transform, &mut Sprite), With<SunMoon>>,
    mut stars: Query<(&StarSprite, &mut Sprite), Without<SunMoon>>,
) {
    let (r, g, b) = time_of_day_to_rgb(clock.hours);
    clear.0 = Color::srgb(r, g, b);

    // Sun/moon arcs across the sky: angle goes 0 (left) → π (right) over 12h.
    // Offset so noon has the body at top.
    let angle = std::f32::consts::PI * (clock.hours / 12.0 - 0.5);
    let radius = 220.0_f32;
    if let Ok((mut t, mut sprite)) = sun_q.single_mut() {
        t.translation.x = angle.cos() * radius;
        t.translation.y = angle.sin() * radius * 0.5;
        // Daytime = yellow sun, night = grey moon.
        let day_frac = ((clock.hours - 6.0) / 12.0).clamp(0.0, 1.0)
            * (1.0 - ((clock.hours - 18.0) / 6.0).clamp(0.0, 1.0));
        sprite.color = Color::srgb(
            lerp_f32(0.85, 1.0, day_frac),
            lerp_f32(0.85, 0.95, day_frac),
            lerp_f32(0.9, 0.6, day_frac),
        );
    }

    // Stars fade in at night, out during day.
    let star_alpha = if clock.hours < 6.0 {
        1.0 - clock.hours / 6.0
    } else if clock.hours < 18.0 {
        0.0
    } else {
        (clock.hours - 18.0) / 6.0
    };
    for (StarSprite(base), mut sprite) in &mut stars {
        sprite.color = Color::srgba(1.0, 1.0, 1.0, base * star_alpha);
    }
}

fn update_hud(clock: Res<DayClock>, mut q: Query<&mut Text, With<HudLabel>>) {
    let h = clock.hours as u32;
    let m = ((clock.hours - h as f32) * 60.0) as u32;
    let label = hours_to_label(clock.hours);
    for mut text in &mut q {
        text.0 = format!("{:02}:{:02}  {}  ×{:.1}", h, m, label, clock.speed);
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midnight_is_dark() {
        let (r, g, b) = time_of_day_to_rgb(0.0);
        assert!(r < 0.1 && g < 0.1 && b < 0.2, "midnight should be dark");
    }

    #[test]
    fn noon_is_bright() {
        let (r, g, b) = time_of_day_to_rgb(12.0);
        assert!(r + g + b > 1.0, "noon should be bright");
    }

    #[test]
    fn lerp_at_zero_returns_a() {
        assert!((lerp_f32(3.0, 9.0, 0.0) - 3.0).abs() < 1e-5);
    }

    #[test]
    fn lerp_at_one_returns_b() {
        assert!((lerp_f32(3.0, 9.0, 1.0) - 9.0).abs() < 1e-5);
    }

    #[test]
    fn lerp_clamps_t() {
        assert!((lerp_f32(0.0, 10.0, 2.0) - 10.0).abs() < 1e-5);
        assert!((lerp_f32(0.0, 10.0, -1.0)).abs() < 1e-5);
    }

    #[test]
    fn hours_to_label_correct() {
        assert_eq!(hours_to_label(0.0), "Night");
        assert_eq!(hours_to_label(7.0), "Dawn");
        assert_eq!(hours_to_label(12.0), "Day");
        assert_eq!(hours_to_label(18.0), "Dusk");
        assert_eq!(hours_to_label(22.0), "Night");
    }

    #[test]
    fn hours_wrap_past_24() {
        let a = time_of_day_to_rgb(0.0);
        let b = time_of_day_to_rgb(24.0);
        assert!((a.0 - b.0).abs() < 1e-5);
    }
}
