//! Weather System — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`WeatherSystemPlugin`] into any Bevy
//! app with `app.add_plugins(WeatherSystemPlugin)` and it manages dynamic
//! weather states with sky colour, rain particles, and wind. Tune it through
//! the [`WeatherConfig`] resource without editing the plugin's internals.
//!
//! Key ideas:
//! - Four weather states (Clear, Cloudy, Rainy, Stormy) cycle on a timer.
//! - [`sky_color`], [`rain_intensity`], and [`wind_force`] are pure functions
//!   of the current weather state — easy to test and tune without touching Bevy.
//! - Sky colour is applied to Bevy's `ClearColor` resource each frame.
//! - Rain particles are pooled: all exist at startup, toggled via `Visibility`.
//! - Wind is a Vec2 applied as a force to the player each frame.
//!
//! **Controls:** WASD / Arrows — move through the storm   R — advance weather
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use weather_system::WeatherSystemPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(WeatherSystemPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system and resource for the weather feature.
///
/// Add it with `app.add_plugins(WeatherSystemPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct WeatherSystemPlugin;

impl Plugin for WeatherSystemPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WeatherConfig>()
            .init_resource::<WeatherState>()
            .init_resource::<RainPool>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    advance_weather,
                    update_sky,
                    move_rain,
                    move_player,
                    update_hud,
                )
                    .chain(),
            );
    }
}

// ── Configuration ─────────────────────────────────────────────────────────────

/// Tunable parameters for the feature. Override before adding the plugin, e.g.
/// `app.insert_resource(WeatherConfig { pool_size: 240, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct WeatherConfig {
    /// Arena width in pixels (used for wrapping rain and clamping the player).
    pub window_w: f32,
    /// Arena height in pixels.
    pub window_h: f32,
    /// Player movement speed in pixels per second.
    pub player_speed: f32,
    /// Number of pooled rain drops.
    pub pool_size: usize,
    /// Seconds between automatic weather transitions.
    pub transition_secs: f32,
}

impl Default for WeatherConfig {
    fn default() -> Self {
        Self {
            window_w: 800.0,
            window_h: 500.0,
            player_speed: 140.0,
            pool_size: 120,
            transition_secs: 8.0,
        }
    }
}

// ── Pure weather model ────────────────────────────────────────────────────────

/// The four cyclic weather states.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Weather {
    /// Bright sky, no precipitation.
    Clear,
    /// Dimmed sky, no precipitation.
    Cloudy,
    /// Steady rain and a darker sky.
    Rainy,
    /// Heavy rain, strong wind, and the darkest sky.
    Stormy,
}

impl Weather {
    /// Human-readable label for the HUD.
    pub fn label(self) -> &'static str {
        match self {
            Weather::Clear => "Clear",
            Weather::Cloudy => "Cloudy",
            Weather::Rainy => "Rainy",
            Weather::Stormy => "Stormy",
        }
    }
}

/// Next weather in the cycle.
pub fn next_weather(w: Weather) -> Weather {
    match w {
        Weather::Clear => Weather::Cloudy,
        Weather::Cloudy => Weather::Rainy,
        Weather::Rainy => Weather::Stormy,
        Weather::Stormy => Weather::Clear,
    }
}

/// Background sky colour for each weather state.
pub fn sky_color(w: Weather) -> (f32, f32, f32) {
    match w {
        Weather::Clear => (0.42, 0.68, 1.00),
        Weather::Cloudy => (0.52, 0.55, 0.62),
        Weather::Rainy => (0.24, 0.28, 0.36),
        Weather::Stormy => (0.10, 0.10, 0.14),
    }
}

/// Fraction of the rain pool that should be active (0.0 – 1.0).
pub fn rain_intensity(w: Weather) -> f32 {
    match w {
        Weather::Clear => 0.00,
        Weather::Cloudy => 0.05,
        Weather::Rainy => 0.55,
        Weather::Stormy => 1.00,
    }
}

/// Horizontal wind force applied to the player each second (positive = rightward).
pub fn wind_force(w: Weather) -> f32 {
    match w {
        Weather::Clear => 0.0,
        Weather::Cloudy => 20.0,
        Weather::Rainy => 80.0,
        Weather::Stormy => 180.0,
    }
}

/// Rain fall speed (pixels per second, downward).
pub fn rain_speed(w: Weather) -> f32 {
    match w {
        Weather::Clear => 200.0,
        Weather::Cloudy => 220.0,
        Weather::Rainy => 320.0,
        Weather::Stormy => 480.0,
    }
}

// ── ECS ───────────────────────────────────────────────────────────────────────

/// Tracks the active weather and the countdown to the next transition.
#[derive(Resource)]
pub struct WeatherState {
    /// The weather in effect right now.
    pub current: Weather,
    /// Seconds until the weather changes again.
    pub timer: f32,
}

impl Default for WeatherState {
    fn default() -> Self {
        Self {
            current: Weather::Clear,
            timer: WeatherConfig::default().transition_secs,
        }
    }
}

/// Holds the pool of rain-drop entities spawned at startup.
#[derive(Resource, Default)]
pub struct RainPool(pub Vec<Entity>);

/// A single pooled rain drop with its logical position and per-drop speed scale.
#[derive(Component)]
pub struct RainDrop {
    /// Horizontal position of the raindrop, in pixels.
    pub x: f32,
    /// Vertical position of the raindrop, in pixels.
    pub y: f32,
    /// Per-drop speed multiplier, so rain does not fall in lockstep.
    pub speed_scale: f32,
}

/// Marks the player entity.
#[derive(Component)]
pub struct Player;

/// Marks the dynamic HUD text.
#[derive(Component)]
pub struct HudText;

fn setup(mut commands: Commands, config: Res<WeatherConfig>, mut pool: ResMut<RainPool>) {
    commands.spawn(Camera2d);

    // Ground.
    commands.spawn((
        Sprite {
            color: Color::srgb(0.18, 0.22, 0.16),
            custom_size: Some(Vec2::new(config.window_w, 80.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, -config.window_h / 2.0 + 40.0, 0.1)),
    ));

    // Player.
    commands.spawn((
        Player,
        Sprite {
            color: Color::srgb(0.7, 0.6, 0.4),
            custom_size: Some(Vec2::splat(22.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, -config.window_h / 2.0 + 90.0, 1.0)),
    ));

    // Rain pool.
    let mut rng = 0xABCD_1234u64;
    let mut lcg = move || -> f32 {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        (rng & 0xFFFF) as f32 / 65535.0
    };
    let mut entities = Vec::with_capacity(config.pool_size);
    for _ in 0..config.pool_size {
        let x = lcg() * config.window_w - config.window_w / 2.0;
        let y = lcg() * config.window_h - config.window_h / 2.0;
        let e = commands
            .spawn((
                RainDrop {
                    x,
                    y,
                    speed_scale: 0.7 + lcg() * 0.6,
                },
                Sprite {
                    color: Color::srgba(0.7, 0.8, 1.0, 0.55),
                    custom_size: Some(Vec2::new(1.5, 12.0)),
                    ..default()
                },
                Transform::from_translation(Vec3::new(x, y, 2.0)),
                Visibility::Hidden,
            ))
            .id();
        entities.push(e);
    }
    pool.0 = entities;

    commands.spawn((
        HudText,
        Text::new(""),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));

    commands.spawn((
        Text::new("WASD / Arrows — move   R — advance weather"),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.55)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

fn advance_weather(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    config: Res<WeatherConfig>,
    mut state: ResMut<WeatherState>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        state.current = next_weather(state.current);
        state.timer = config.transition_secs;
        return;
    }
    state.timer -= time.delta_secs();
    if state.timer <= 0.0 {
        state.current = next_weather(state.current);
        state.timer = config.transition_secs;
    }
}

fn update_sky(state: Res<WeatherState>, mut clear: ResMut<ClearColor>) {
    let (r, g, b) = sky_color(state.current);
    clear.0 = Color::srgb(r, g, b);
}

fn move_rain(
    time: Res<Time>,
    state: Res<WeatherState>,
    config: Res<WeatherConfig>,
    pool: Res<RainPool>,
    mut drop_q: Query<(&mut RainDrop, &mut Transform, &mut Visibility)>,
) {
    let intensity = rain_intensity(state.current);
    let active_count = (intensity * config.pool_size as f32) as usize;
    let speed = rain_speed(state.current);
    let wind = wind_force(state.current);
    let dt = time.delta_secs();

    for (idx, &entity) in pool.0.iter().enumerate() {
        let Ok((mut drop, mut tf, mut vis)) = drop_q.get_mut(entity) else {
            continue;
        };
        if idx >= active_count {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Visible;
        drop.y -= speed * drop.speed_scale * dt;
        drop.x += wind * dt;
        if drop.y < -config.window_h / 2.0 {
            drop.y = config.window_h / 2.0;
            drop.x = drop.x.rem_euclid(config.window_w) - config.window_w / 2.0;
        }
        if drop.x > config.window_w / 2.0 {
            drop.x -= config.window_w;
        }
        if drop.x < -config.window_w / 2.0 {
            drop.x += config.window_w;
        }
        tf.translation.x = drop.x;
        tf.translation.y = drop.y;
    }
}

fn move_player(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    state: Res<WeatherState>,
    config: Res<WeatherConfig>,
    mut q: Query<&mut Transform, With<Player>>,
) {
    let Ok(mut tf) = q.single_mut() else { return };
    let dt = time.delta_secs();
    let mut dir = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        dir.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        dir.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        dir.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        dir.x += 1.0;
    }
    let input_vel = if dir != Vec2::ZERO {
        dir.normalize() * config.player_speed
    } else {
        Vec2::ZERO
    };
    let wind = Vec2::new(wind_force(state.current), 0.0);
    tf.translation += ((input_vel + wind) * dt).extend(0.0);
    tf.translation.x = tf
        .translation
        .x
        .clamp(-config.window_w / 2.0 + 14.0, config.window_w / 2.0 - 14.0);
    tf.translation.y = tf
        .translation
        .y
        .clamp(-config.window_h / 2.0 + 50.0, config.window_h / 2.0 - 14.0);
}

fn update_hud(state: Res<WeatherState>, mut q: Query<&mut Text, With<HudText>>) {
    let Ok(mut text) = q.single_mut() else { return };
    let wind = wind_force(state.current);
    let next = next_weather(state.current);
    text.0 = format!(
        "Weather: {}  |  Wind: {:.0} px/s  |  Next: {} in {:.0}s",
        state.current.label(),
        wind,
        next.label(),
        state.timer.max(0.0)
    );
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_cycles_back_to_clear() {
        let mut w = Weather::Clear;
        for _ in 0..4 {
            w = next_weather(w);
        }
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
        for w in [
            Weather::Clear,
            Weather::Cloudy,
            Weather::Rainy,
            Weather::Stormy,
        ] {
            let i = rain_intensity(w);
            assert!((0.0..=1.0).contains(&i));
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

    #[test]
    fn config_default_matches_documented_values() {
        let c = WeatherConfig::default();
        assert_eq!(c.pool_size, 120);
        assert_eq!(c.transition_secs, 8.0);
    }

    #[test]
    fn setup_spawns_full_rain_pool() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<WeatherConfig>()
            .init_resource::<WeatherState>()
            .init_resource::<RainPool>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&RainDrop>();
        assert_eq!(q.iter(app.world()).count(), 120);
    }

    #[test]
    fn setup_spawns_one_player() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<WeatherConfig>()
            .init_resource::<WeatherState>()
            .init_resource::<RainPool>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Player>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }
}
