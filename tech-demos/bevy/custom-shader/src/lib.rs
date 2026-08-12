//! Custom WGSL shader — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`CustomShaderPlugin`] into any Bevy
//! app with `app.add_plugins(CustomShaderPlugin)` and it spawns a full-window
//! quad rendered with a hand-written WGSL fragment shader that animates a
//! ripple pattern driven by a `time` uniform updated each frame.
//!
//! Key concepts:
//! - Defining a [`Material2d`] with `AsBindGroup` + `#[uniform]` for GPU data.
//! - Loading a WGSL shader from `assets/shaders/`.
//! - Spawning a full-window `Mesh2d` quad and assigning the custom material.
//! - Updating a material uniform each frame via `Assets<RippleMaterial>`.
//!
//! The pure math ([`ripple_intensity`], [`ripple_color`]) mirrors the WGSL so
//! it can be unit-tested without a GPU.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use custom_shader::CustomShaderPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(CustomShaderPlugin)
//!     .run();
//! ```

use bevy::{
    mesh::Mesh2d,
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite_render::{Material2d, Material2dPlugin, MeshMaterial2d},
};

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// Bundles the ripple material, its render sub-plugin, and update systems.
///
/// Add it with `app.add_plugins(CustomShaderPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup. It *does* register [`Material2dPlugin<RippleMaterial>`], which the
/// feature needs to render the custom material.
pub struct CustomShaderPlugin;

impl Plugin for CustomShaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<RippleMaterial>::default())
            .add_systems(Startup, setup)
            .add_systems(Update, update_time);
    }
}

// ---------------------------------------------------------------------------
// Material definition
// ---------------------------------------------------------------------------

/// A custom 2D material that drives the ripple WGSL shader.
///
/// The single `time` uniform is updated every frame so the shader can animate.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct RippleMaterial {
    /// Elapsed seconds since app start; passed to the WGSL shader as a uniform.
    #[uniform(0)]
    pub time: f32,
}

impl Material2d for RippleMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/ripple.wgsl".into()
    }
}

// ---------------------------------------------------------------------------
// Resource to hold the material handle
// ---------------------------------------------------------------------------

/// Stores the handle of the active [`RippleMaterial`] so the update system
/// can mutate it without querying the mesh entity.
#[derive(Resource)]
pub struct RippleHandle(pub Handle<RippleMaterial>);

// ---------------------------------------------------------------------------
// Pure math functions (testable without Bevy)
// ---------------------------------------------------------------------------

/// Returns the ripple wave intensity in `[0, 1]` at `dist` from center given `time`.
///
/// Uses the same formula as the WGSL shader so Rust tests can validate the math.
pub fn ripple_intensity(dist: f32, time: f32, frequency: f32, speed: f32) -> f32 {
    (dist * frequency - time * speed).sin() * 0.5 + 0.5
}

/// Maps a ripple intensity value in `[0, 1]` to an RGB triple.
///
/// Matches the colour channels used by the WGSL shader:
/// - red:   `intensity * 0.15`
/// - green: `intensity * 0.50`
/// - blue:  `intensity * 0.90`
pub fn ripple_color(intensity: f32) -> [f32; 3] {
    [intensity * 0.15, intensity * 0.5, intensity * 0.9]
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<RippleMaterial>>,
) {
    commands.spawn(Camera2d);

    let handle = materials.add(RippleMaterial { time: 0.0 });
    commands.insert_resource(RippleHandle(handle.clone()));

    // Full-window quad
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(600.0, 600.0))),
        MeshMaterial2d(handle),
        Transform::default(),
    ));

    // HUD label
    commands.spawn((
        Text::new("Custom WGSL shader — animated ripple"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
    ));
}

/// Advances the `time` uniform on the [`RippleMaterial`] each frame.
fn update_time(
    time: Res<Time>,
    handle: Res<RippleHandle>,
    mut materials: ResMut<Assets<RippleMaterial>>,
) {
    if let Some(mat) = materials.get_mut(&handle.0) {
        mat.time = time.elapsed_secs();
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPSILON
    }

    /// At dist=0, time=0, with frequency/speed defaults the wave is sin(0)*0.5+0.5 = 0.5.
    #[test]
    fn ripple_intensity_center_at_zero_time() {
        let v = ripple_intensity(0.0, 0.0, 30.0, 3.0);
        assert!(approx_eq(v, 0.5), "expected 0.5, got {v}");
    }

    /// Output is always in [0, 1] because sin is in [-1, 1].
    #[test]
    fn ripple_intensity_bounds() {
        for i in 0..=20 {
            for j in 0..=20 {
                let dist = i as f32 * 0.05;
                let time = j as f32 * 0.1;
                let v = ripple_intensity(dist, time, 30.0, 3.0);
                assert!(
                    (0.0..=1.0).contains(&v),
                    "out of [0,1]: dist={dist} time={time} v={v}"
                );
            }
        }
    }

    /// ripple_color channels are proportional to intensity.
    #[test]
    fn ripple_color_proportions() {
        let intensity = 0.8;
        let [r, g, b] = ripple_color(intensity);
        assert!(approx_eq(r, intensity * 0.15));
        assert!(approx_eq(g, intensity * 0.5));
        assert!(approx_eq(b, intensity * 0.9));
    }

    /// ripple_color at intensity=0 returns black.
    #[test]
    fn ripple_color_zero_is_black() {
        let [r, g, b] = ripple_color(0.0);
        assert!(approx_eq(r, 0.0));
        assert!(approx_eq(g, 0.0));
        assert!(approx_eq(b, 0.0));
    }

    /// ripple_color at intensity=1 matches maximum channel values.
    #[test]
    fn ripple_color_full_intensity() {
        let [r, g, b] = ripple_color(1.0);
        assert!(approx_eq(r, 0.15));
        assert!(approx_eq(g, 0.5));
        assert!(approx_eq(b, 0.9));
    }

    /// Higher frequency means faster spatial oscillation: at a non-zero dist,
    /// changing frequency changes the wave value.
    #[test]
    fn ripple_intensity_frequency_effect() {
        let v_low = ripple_intensity(0.1, 0.0, 10.0, 3.0);
        let v_high = ripple_intensity(0.1, 0.0, 50.0, 3.0);
        assert!(
            (v_low - v_high).abs() > 1e-3,
            "frequency should change the wave"
        );
    }

    /// Time advances produce different intensities at a fixed dist (animation works).
    #[test]
    fn ripple_intensity_animates_over_time() {
        let v0 = ripple_intensity(0.3, 0.0, 30.0, 3.0);
        let v1 = ripple_intensity(0.3, 1.0, 30.0, 3.0);
        assert!(
            (v0 - v1).abs() > 1e-3,
            "animation should produce different values over time"
        );
    }
}
