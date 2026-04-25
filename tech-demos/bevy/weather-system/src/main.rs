//! Weather System — dynamic weather states with sky colour, rain particles, and wind.
//!
//! Key ideas:
//! - Four weather states (Clear, Cloudy, Rainy, Stormy) cycle on a timer.
//! - `sky_color`, `rain_intensity`, and `wind_force` are pure functions of the
//!   current weather state — easy to test and tune without touching Bevy.
//! - Sky colour is applied to Bevy's `ClearColor` resource each frame.
//! - Rain particles are pooled: all exist at startup, toggled via `Visibility`.
//! - Wind is a Vec2 applied as a force to the player each frame.
//!
//! **Controls:** WASD / Arrows — move through the storm   R — advance weather

use bevy::prelude::*;

const WINDOW_W: f32 = 800.0;
const WINDOW_H: f32 = 500.0;
const PLAYER_SPEED: f32 = 140.0;
const POOL_SIZE: usize = 120;
const TRANSITION_SECS: f32 = 8.0;

// ── Pure weather model ────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Weather { Clear, Cloudy, Rainy, Stormy }

impl Weather {
    pub fn label(self) -> &'static str {
        match self { Weather::Clear => "Clear", Weather::Cloudy => "Cloudy", Weather::Rainy => "Rainy", Weather::Stormy => "Stormy" }
    }
}

/// Next weather in the cycle.
pub fn next_weather(w: Weather) -> Weather {
    match w {
        Weather::Clear  => Weather::Cloudy,
        Weather::Cloudy => Weather::Rainy,
        Weather::Rainy  => Weather::Stormy,
        Weather::Stormy => Weather::Clear,
    }
}

/// Background sky colour for each weather state.
pub fn sky_color(w: Weather) -> (f32, f32, f32) {
    match w {
        Weather::Clear  => (0.42, 0.68, 1.00),
        Weather::Cloudy => (0.52, 0.55, 0.62),
        Weather::Rainy  => (0.24, 0.28, 0.36),
        Weather::Stormy => (0.10, 0.10, 0.14),
    }
}

/// Fraction of the rain pool that should be active (0.0 – 1.0).
pub fn rain_intensity(w: Weather) -> f32 {
    match w {
        Weather::Clear  => 0.00,
        Weather::Cloudy => 0.05,
        Weather::Rainy  => 0.55,
        Weather::Stormy => 1.00,
    }
}

/// Horizontal wind force applied to the player each second (positive = rightward).
pub fn wind_force(w: Weather) -> f32 {
    match w {
        Weather::Clear  =>   0.0,
        Weather::Cloudy =>  20.0,
        Weather::Rainy  =>  80.0,
        Weather::Stormy => 180.0,
    }
}

/// Rain fall speed (pixels per second, downward).
pub fn rain_speed(w: Weather) -> f32 {
    match w {
        Weather::Clear  => 200.0,
        Weather::Cloudy => 220.0,
        Weather::Rainy  => 320.0,
        Weather::Stormy => 480.0,
    }
}

// ── ECS ───────────────────────────────────────────────────────────────────────

#[derive(Resource)]
struct WeatherState { current: Weather, timer: f32 }

#[derive(Resource)]
struct RainPool(Vec<Entity>);

#[derive(Component)]
struct RainDrop { x: f32, y: f32, speed_scale: f32 }

#[derive(Component)]
struct Player;

#[derive(Component)]
struct HudText;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Weather System".into(),
                resolution: (800u32, 500u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(WeatherState { current: Weather::Clear, timer: TRANSITION_SECS })
        .insert_resource(ClearColor(Color::srgb(0.42, 0.68, 1.0)))
        .insert_resource(RainPool(Vec::new()))
        .add_systems(Startup, setup)
        .add_systems(Update, (advance_weather, update_sky, move_rain, move_player, update_hud).chain())
        .run();
}

fn setup(mut commands: Commands, mut pool: ResMut<RainPool>) {
    commands.spawn(Camera2d);

    // Ground.
    commands.spawn((
        Sprite { color: Color::srgb(0.18, 0.22, 0.16), custom_size: Some(Vec2::new(WINDOW_W, 80.0)), ..default() },
        Transform::from_translation(Vec3::new(0.0, -WINDOW_H / 2.0 + 40.0, 0.1)),
    ));

    // Player.
    commands.spawn((
        Player,
        Sprite { color: Color::srgb(0.7, 0.6, 0.4), custom_size: Some(Vec2::splat(22.0)), ..default() },
        Transform::from_translation(Vec3::new(0.0, -WINDOW_H / 2.0 + 90.0, 1.0)),
    ));

    // Rain pool.
    let mut rng = 0xABCD_1234u64;
    let mut lcg = move || -> f32 {
        rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
        (rng & 0xFFFF) as f32 / 65535.0
    };
    let mut entities = Vec::with_capacity(POOL_SIZE);
    for _ in 0..POOL_SIZE {
        let x = lcg() * WINDOW_W - WINDOW_W / 2.0;
        let y = lcg() * WINDOW_H - WINDOW_H / 2.0;
        let e = commands.spawn((
            RainDrop { x, y, speed_scale: 0.7 + lcg() * 0.6 },
            Sprite { color: Color::srgba(0.7, 0.8, 1.0, 0.55), custom_size: Some(Vec2::new(1.5, 12.0)), ..default() },
            Transform::from_translation(Vec3::new(x, y, 2.0)),
            Visibility::Hidden,
        )).id();
        entities.push(e);
    }
    pool.0 = entities;

    commands.spawn((
        HudText,
        Text::new(""),
        TextFont { font_size: 18.0, ..default() },
        TextColor(Color::WHITE),
        Node { position_type: PositionType::Absolute, top: Val::Px(10.0), left: Val::Px(10.0), ..default() },
    ));

    commands.spawn((
        Text::new("WASD / Arrows — move   R — advance weather"),
        TextFont { font_size: 13.0, ..default() },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.55)),
        Node { position_type: PositionType::Absolute, bottom: Val::Px(10.0), left: Val::Px(10.0), ..default() },
    ));
}

fn advance_weather(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut state: ResMut<WeatherState>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        state.current = next_weather(state.current);
        state.timer = TRANSITION_SECS;
        return;
    }
    state.timer -= time.delta_secs();
    if state.timer <= 0.0 {
        state.current = next_weather(state.current);
        state.timer = TRANSITION_SECS;
    }
}

fn update_sky(state: Res<WeatherState>, mut clear: ResMut<ClearColor>) {
    let (r, g, b) = sky_color(state.current);
    clear.0 = Color::srgb(r, g, b);
}

fn move_rain(
    time: Res<Time>,
    state: Res<WeatherState>,
    pool: Res<RainPool>,
    mut drop_q: Query<(&mut RainDrop, &mut Transform, &mut Visibility)>,
) {
    let intensity = rain_intensity(state.current);
    let active_count = (intensity * POOL_SIZE as f32) as usize;
    let speed = rain_speed(state.current);
    let wind = wind_force(state.current);
    let dt = time.delta_secs();

    for (idx, &entity) in pool.0.iter().enumerate() {
        let Ok((mut drop, mut tf, mut vis)) = drop_q.get_mut(entity) else { continue };
        if idx >= active_count {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Visible;
        drop.y -= speed * drop.speed_scale * dt;
        drop.x += wind * dt;
        if drop.y < -WINDOW_H / 2.0 {
            drop.y = WINDOW_H / 2.0;
            drop.x = drop.x.rem_euclid(WINDOW_W) - WINDOW_W / 2.0;
        }
        if drop.x > WINDOW_W / 2.0 { drop.x -= WINDOW_W; }
        if drop.x < -WINDOW_W / 2.0 { drop.x += WINDOW_W; }
        tf.translation.x = drop.x;
        tf.translation.y = drop.y;
    }
}

fn move_player(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    state: Res<WeatherState>,
    mut q: Query<&mut Transform, With<Player>>,
) {
    let Ok(mut tf) = q.single_mut() else { return };
    let dt = time.delta_secs();
    let mut dir = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp)    { dir.y += 1.0; }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown)  { dir.y -= 1.0; }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft)  { dir.x -= 1.0; }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) { dir.x += 1.0; }
    let input_vel = if dir != Vec2::ZERO { dir.normalize() * PLAYER_SPEED } else { Vec2::ZERO };
    let wind = Vec2::new(wind_force(state.current), 0.0);
    tf.translation += ((input_vel + wind) * dt).extend(0.0);
    tf.translation.x = tf.translation.x.clamp(-WINDOW_W / 2.0 + 14.0, WINDOW_W / 2.0 - 14.0);
    tf.translation.y = tf.translation.y.clamp(-WINDOW_H / 2.0 + 50.0, WINDOW_H / 2.0 - 14.0);
}

fn update_hud(state: Res<WeatherState>, mut q: Query<&mut Text, With<HudText>>) {
    let Ok(mut text) = q.single_mut() else { return };
    let wind = wind_force(state.current);
    let next = next_weather(state.current);
    text.0 = format!(
        "Weather: {}  |  Wind: {:.0} px/s  |  Next: {} in {:.0}s",
        state.current.label(), wind, next.label(), state.timer.max(0.0)
    );
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_cycles_back_to_clear() {
        let mut w = Weather::Clear;
        for _ in 0..4 { w = next_weather(w); }
        assert_eq!(w, Weather::Clear);
    }

    #[test]
    fn stormy_has_highest_wind() {
        assert!(wind_force(Weather::Stormy) > wind_force(Weather::Rainy));
        assert!(wind_force(Weather::Rainy) > wind_force(Weather::Cloudy));
        assert!(wind_force(Weather::Cloudy) > wind_force(Weather::Clear));
    }

    #[test]
    fn rain_intensity_ranges_zero_to_one() {
        for w in [Weather::Clear, Weather::Cloudy, Weather::Rainy, Weather::Stormy] {
            let i = rain_intensity(w);
            assert!(i >= 0.0 && i <= 1.0);
        }
    }

    #[test]
    fn clear_has_brightest_sky() {
        let (r, _, _) = sky_color(Weather::Clear);
        let (rs, _, _) = sky_color(Weather::Stormy);
        assert!(r > rs);
    }

    #[test]
    fn stormy_has_fastest_rain() {
        assert!(rain_speed(Weather::Stormy) > rain_speed(Weather::Rainy));
    }

    #[test]
    fn next_weather_from_each_state() {
        assert_eq!(next_weather(Weather::Clear), Weather::Cloudy);
        assert_eq!(next_weather(Weather::Stormy), Weather::Clear);
    }
}
