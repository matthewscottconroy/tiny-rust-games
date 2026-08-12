//! Spritesheet animation — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`SpritesheetAnimationPlugin`] into
//! any Bevy app with `app.add_plugins(SpritesheetAnimationPlugin)` and it
//! builds a texture atlas programmatically and animates a looping sprite.
//!
//! Key ideas:
//! - A texture atlas is built **programmatically** at startup (four solid-color
//!   32×32 frames) so the demo has no external asset dependency.
//! - [`AnimationTimer`] drives frame advances at a fixed interval.
//! - [`AnimationIndices`] stores the first/last frame so the animation can loop.
//! - In a real project, replace the programmatic atlas with
//!   `asset_server.load("spritesheet.png")`.
//! - Tune the frame size, count, and speed through the
//!   [`SpritesheetAnimationConfig`] resource.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use spritesheet_animation::SpritesheetAnimationPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(SpritesheetAnimationPlugin)
//!     .run();
//! ```

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Bundles every system and resource for the spritesheet-animation feature.
///
/// Add it with `app.add_plugins(SpritesheetAnimationPlugin)`. It does **not**
/// add `DefaultPlugins`, so the host app stays in control of window and
/// rendering setup.
pub struct SpritesheetAnimationPlugin;

impl Plugin for SpritesheetAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpritesheetAnimationConfig>()
            .add_systems(Startup, setup)
            .add_systems(Update, animate_sprite);
    }
}

// --- Configuration ---

/// Tunable parameters for the feature. Override before adding the plugin, e.g.
/// `app.insert_resource(SpritesheetAnimationConfig { frame_px: 48, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct SpritesheetAnimationConfig {
    /// Width and height of a single frame, in pixels.
    pub frame_px: u32,
    /// Number of frames in the animation.
    pub frames: u32,
    /// Seconds between frame advances.
    pub frame_seconds: f32,
    /// On-screen size of the rendered sprite, in pixels.
    pub sprite_px: f32,
}

impl Default for SpritesheetAnimationConfig {
    fn default() -> Self {
        Self {
            frame_px: 32,
            frames: 4,
            frame_seconds: 0.2,
            sprite_px: 128.0,
        }
    }
}

/// Stores the inclusive index range `[first, last]` for a looping animation.
#[derive(Component)]
pub struct AnimationIndices {
    pub first: usize,
    pub last: usize,
}

/// Per-entity timer that drives frame advances.
///
/// `#[derive(Deref, DerefMut)]` lets callers write `timer.tick(...)` directly
/// instead of `timer.0.tick(...)`.
#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);

/// Builds the programmatic atlas and spawns the animated sprite.
fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    config: Res<SpritesheetAnimationConfig>,
) {
    commands.spawn(Camera2d);

    let frame_px = config.frame_px;
    let frames = config.frames;
    let colors: [[u8; 4]; 4] = [
        [220, 80, 80, 255],  // red
        [80, 200, 80, 255],  // green
        [80, 120, 220, 255], // blue
        [220, 200, 60, 255], // yellow
    ];

    let mut data = vec![0u8; (frame_px * frames * frame_px * 4) as usize];
    for frame in 0..frames {
        for y in 0..frame_px {
            for x in 0..frame_px {
                let px = frame * frame_px + x;
                let idx = ((y * frame_px * frames + px) * 4) as usize;
                data[idx..idx + 4].copy_from_slice(&colors[(frame as usize) % colors.len()]);
            }
        }
    }

    let atlas_image = Image::new(
        Extent3d {
            width: frame_px * frames,
            height: frame_px,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    let image_handle = images.add(atlas_image);

    let layout = TextureAtlasLayout::from_grid(UVec2::splat(frame_px), frames, 1, None, None);
    let layout_handle = layouts.add(layout);

    commands.spawn((
        Sprite {
            image: image_handle,
            texture_atlas: Some(TextureAtlas {
                layout: layout_handle,
                index: 0,
            }),
            custom_size: Some(Vec2::splat(config.sprite_px)),
            ..default()
        },
        AnimationIndices {
            first: 0,
            last: (frames - 1) as usize,
        },
        AnimationTimer(Timer::from_seconds(
            config.frame_seconds,
            TimerMode::Repeating,
        )),
    ));
}

/// Advances the atlas index by one frame when the timer fires, wrapping back
/// to `indices.first` after `indices.last`.
fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(&AnimationIndices, &mut AnimationTimer, &mut Sprite)>,
) {
    for (indices, mut timer, mut sprite) in &mut query {
        timer.tick(time.delta());
        if timer.just_finished()
            && let Some(atlas) = &mut sprite.texture_atlas
        {
            atlas.index = next_frame(atlas.index, indices.first, indices.last);
        }
    }
}

/// Returns the next animation frame index, wrapping from `last` back to `first`.
///
/// # Arguments
/// * `current` — the current atlas frame index.
/// * `first`   — the first frame in the animation range (inclusive).
/// * `last`    — the last frame in the animation range (inclusive).
pub fn next_frame(current: usize, first: usize, last: usize) -> usize {
    if current >= last { first } else { current + 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- next_frame ---

    #[test]
    fn next_frame_advances_within_range() {
        assert_eq!(next_frame(0, 0, 3), 1);
        assert_eq!(next_frame(1, 0, 3), 2);
        assert_eq!(next_frame(2, 0, 3), 3);
    }

    #[test]
    fn next_frame_wraps_at_last() {
        assert_eq!(next_frame(3, 0, 3), 0);
    }

    #[test]
    fn next_frame_single_frame_stays_at_first() {
        assert_eq!(next_frame(0, 0, 0), 0);
    }

    #[test]
    fn next_frame_past_last_also_wraps() {
        // current > last is treated the same as current == last.
        assert_eq!(next_frame(5, 0, 3), 0);
    }

    #[test]
    fn next_frame_non_zero_first() {
        // Animation that starts at frame 2 and ends at frame 5.
        assert_eq!(next_frame(4, 2, 5), 5);
        assert_eq!(next_frame(5, 2, 5), 2); // wraps back to first
    }

    #[test]
    fn config_default_matches_documented_values() {
        let c = SpritesheetAnimationConfig::default();
        assert_eq!(c.frame_px, 32);
        assert_eq!(c.frames, 4);
        assert_eq!(c.frame_seconds, 0.2);
        assert_eq!(c.sprite_px, 128.0);
    }
}
