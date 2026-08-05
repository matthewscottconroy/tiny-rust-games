//! Noise map — procedural terrain using seeded value noise and fractal layering,
//! as a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`NoiseMapPlugin`] into any Bevy app
//! with `app.add_plugins(NoiseMapPlugin)` and it spawns a grid of colored tiles
//! forming procedural terrain. Tune it through the [`NoiseMapConfig`] resource
//! without editing the plugin's internals.
//!
//! Key ideas:
//! - [`hash2`] converts a grid cell `(x, y)` and a seed into a deterministic
//!   float in `[0, 1]` using integer arithmetic (no external crate needed).
//! - [`value_noise`] bilinearly interpolates between the four corner hashes of
//!   the cell that contains `(x, y)`, using [`smoothstep`] for C¹ continuity.
//! - [`fbm`] (fractal Brownian motion) stacks multiple octaves at halving
//!   amplitude and doubling frequency to create natural multi-scale features.
//! - [`biome_color`] maps a height value to a colour tier: deep water, shallow
//!   water, sand, grass, rock, snow.
//! - Pressing SPACE re-seeds the map and repaints all tiles in one frame.
//!
//! **Controls:** SPACE — regenerate map with a new seed.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use noise_map::NoiseMapPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(NoiseMapPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system and resource for the noise-map feature.
///
/// Add it with `app.add_plugins(NoiseMapPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct NoiseMapPlugin;

impl Plugin for NoiseMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NoiseMapConfig>()
            .init_resource::<MapSeed>()
            .add_systems(Startup, setup)
            .add_systems(Update, (handle_regen, update_colors));
    }
}

// ── Configuration ───────────────────────────────────────────────────────────

/// Tunable parameters for the noise map. Override before adding the plugin,
/// e.g. `app.insert_resource(NoiseMapConfig { scale: 0.2, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct NoiseMapConfig {
    /// Number of tile columns.
    pub grid_w: usize,
    /// Number of tile rows.
    pub grid_h: usize,
    /// Cell width in world units.
    pub cell_w: f32,
    /// Cell height in world units.
    pub cell_h: f32,
    /// Noise sampling scale (larger = more variation per tile).
    pub scale: f32,
    /// Number of fractal octaves.
    pub octaves: u32,
    /// Seed the map starts with.
    pub initial_seed: u64,
}

impl Default for NoiseMapConfig {
    fn default() -> Self {
        Self {
            grid_w: 62,
            grid_h: 40,
            cell_w: 14.0,
            cell_h: 13.0,
            scale: 0.11,
            octaves: 5,
            initial_seed: 7,
        }
    }
}

// ── Pure noise functions ──────────────────────────────────────────────────────

/// Maps a grid cell `(x, y)` and a `seed` to a value in `[0, 1]`.
pub fn hash2(x: i32, y: i32, seed: u64) -> f32 {
    let h = (x as u64)
        .wrapping_mul(374_761_393)
        .wrapping_add((y as u64).wrapping_mul(668_265_263))
        .wrapping_add(seed)
        .wrapping_mul(2_246_822_519);
    (h >> 33) as f32 / u32::MAX as f32
}

/// Smooth-step (3t²−2t³) — C¹ at the endpoints `0` and `1`.
pub fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Bilinear value noise at fractional position `(x, y)` with `seed`.
pub fn value_noise(x: f32, y: f32, seed: u64) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let tx = smoothstep((x - xi as f32).clamp(0.0, 1.0));
    let ty = smoothstep((y - yi as f32).clamp(0.0, 1.0));

    let v00 = hash2(xi,     yi,     seed);
    let v10 = hash2(xi + 1, yi,     seed);
    let v01 = hash2(xi,     yi + 1, seed);
    let v11 = hash2(xi + 1, yi + 1, seed);

    let top    = v00 + tx * (v10 - v00);
    let bottom = v01 + tx * (v11 - v01);
    top + ty * (bottom - top)
}

/// Fractal Brownian Motion — sums `octaves` octaves of [`value_noise`].
///
/// Each octave halves the amplitude and doubles the frequency.
/// The result is normalized to `[0, 1]`.
pub fn fbm(x: f32, y: f32, seed: u64, octaves: u32) -> f32 {
    let mut value = 0.0f32;
    let mut amplitude = 0.5f32;
    let mut freq = 1.0f32;
    let mut max_val = 0.0f32;
    for o in 0..octaves {
        value += value_noise(x * freq, y * freq, seed.wrapping_add(o as u64)) * amplitude;
        max_val += amplitude;
        amplitude *= 0.5;
        freq *= 2.0;
    }
    value / max_val
}

/// Maps a noise height `v ∈ [0, 1]` to a biome colour.
pub fn biome_color(v: f32) -> Color {
    if v < 0.28 {
        Color::srgb(0.08, 0.22, 0.62) // deep water
    } else if v < 0.40 {
        Color::srgb(0.18, 0.42, 0.78) // shallow water
    } else if v < 0.46 {
        Color::srgb(0.82, 0.78, 0.52) // sand
    } else if v < 0.68 {
        Color::srgb(0.18, 0.62, 0.18) // grass
    } else if v < 0.82 {
        Color::srgb(0.48, 0.48, 0.50) // rock
    } else {
        Color::WHITE // snow
    }
}

// ── ECS ───────────────────────────────────────────────────────────────────────

/// Current noise seed.  When changed, [`update_colors`] repaints all tiles.
#[derive(Resource)]
pub struct MapSeed(pub u64);

impl Default for MapSeed {
    fn default() -> Self {
        MapSeed(NoiseMapConfig::default().initial_seed)
    }
}

/// Identifies a tile by its flat grid index `row * grid_w + col`.
#[derive(Component)]
pub struct MapCell(pub usize);

/// Spawns the camera, all tile sprites, and the key-hint label.
fn setup(mut commands: Commands, config: Res<NoiseMapConfig>, seed: Res<MapSeed>) {
    commands.spawn(Camera2d);

    let origin_x = -(config.grid_w as f32 * config.cell_w) / 2.0 + config.cell_w / 2.0;
    let origin_y =  (config.grid_h as f32 * config.cell_h) / 2.0 - config.cell_h / 2.0;

    for row in 0..config.grid_h {
        for col in 0..config.grid_w {
            let v = fbm(
                col as f32 * config.scale,
                row as f32 * config.scale,
                seed.0,
                config.octaves,
            );
            let pos = Vec3::new(
                origin_x + col as f32 * config.cell_w,
                origin_y - row as f32 * config.cell_h,
                0.0,
            );
            commands.spawn((
                Sprite {
                    color: biome_color(v),
                    custom_size: Some(Vec2::new(config.cell_w - 1.0, config.cell_h - 1.0)),
                    ..default()
                },
                Transform::from_translation(pos),
                MapCell(row * config.grid_w + col),
            ));
        }
    }

    commands.spawn((
        Text::new("SPACE — regenerate with a new seed"),
        TextFont { font_size: 15.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

/// Advances the seed with a simple LCG step when SPACE is pressed.
fn handle_regen(input: Res<ButtonInput<KeyCode>>, mut seed: ResMut<MapSeed>) {
    if input.just_pressed(KeyCode::Space) {
        seed.0 = seed
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
    }
}

/// Repaints every tile when the seed changes.
fn update_colors(
    config: Res<NoiseMapConfig>,
    seed: Res<MapSeed>,
    mut query: Query<(&MapCell, &mut Sprite)>,
) {
    if !seed.is_changed() {
        return;
    }
    for (cell, mut sprite) in &mut query {
        let col = cell.0 % config.grid_w;
        let row = cell.0 / config.grid_w;
        let v = fbm(
            col as f32 * config.scale,
            row as f32 * config.scale,
            seed.0,
            config.octaves,
        );
        sprite.color = biome_color(v);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash2_output_in_unit_interval() {
        for x in -4..=4 {
            for y in -4..=4 {
                let v = hash2(x, y, 99);
                assert!(v >= 0.0 && v <= 1.0, "hash2({x},{y}) = {v} out of [0,1]");
            }
        }
    }

    #[test]
    fn hash2_varies_with_position() {
        let a = hash2(0, 0, 42);
        let b = hash2(1, 0, 42);
        let c = hash2(0, 1, 42);
        assert_ne!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn hash2_varies_with_seed() {
        // Adjacent seeds differ only in bits the `>> 33` shift discards, so use
        // widely separated seeds to observe the seed's effect on the output.
        let a = hash2(0, 0, 1000);
        let b = hash2(0, 0, 999_999_999);
        assert_ne!(a, b);
    }

    #[test]
    fn smoothstep_endpoints() {
        assert_eq!(smoothstep(0.0), 0.0);
        assert_eq!(smoothstep(1.0), 1.0);
    }

    #[test]
    fn smoothstep_midpoint() {
        assert!((smoothstep(0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn value_noise_in_unit_interval() {
        for i in 0..20 {
            let v = value_noise(i as f32 * 0.37, i as f32 * 0.13, 7);
            assert!(v >= 0.0 && v <= 1.0, "value_noise out of [0,1]: {v}");
        }
    }

    #[test]
    fn fbm_in_unit_interval() {
        let octaves = NoiseMapConfig::default().octaves;
        for i in 0..20 {
            let v = fbm(i as f32 * 0.3, i as f32 * 0.7, 13, octaves);
            assert!(v >= 0.0 && v <= 1.0, "fbm out of [0,1]: {v}");
        }
    }

    #[test]
    fn biome_color_deep_water_is_blue_dominant() {
        let Color::Srgba(c) = biome_color(0.1) else { panic!("expected Srgba") };
        assert!(c.blue > c.red && c.blue > c.green);
    }

    #[test]
    fn biome_color_snow_is_near_white() {
        // Snow uses `Color::WHITE` (a LinearRgba), so convert to Srgba first.
        let c = Srgba::from(biome_color(0.95));
        assert!(c.red > 0.9 && c.green > 0.9 && c.blue > 0.9);
    }

    #[test]
    fn biome_color_grass_is_green_dominant() {
        let Color::Srgba(c) = biome_color(0.55) else { panic!("expected Srgba") };
        assert!(c.green > c.red && c.green > c.blue);
    }

    #[test]
    fn config_default_matches_documented_values() {
        let c = NoiseMapConfig::default();
        assert_eq!(c.grid_w, 62);
        assert_eq!(c.grid_h, 40);
        assert_eq!(c.initial_seed, 7);
    }

    #[test]
    fn map_seed_default_uses_config_initial_seed() {
        assert_eq!(MapSeed::default().0, NoiseMapConfig::default().initial_seed);
    }
}
