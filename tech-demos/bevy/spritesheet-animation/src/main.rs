//! Spritesheet animation demo.
//!
//! Key ideas:
//! - A texture atlas is built **programmatically** at startup (four solid-color
//!   32×32 frames) so the demo has no external asset dependency.
//! - `AnimationTimer` drives frame advances at a fixed interval.
//! - `AnimationIndices` stores the first/last frame so the animation can loop.
//! - In a real project, replace the programmatic atlas with
//!   `asset_server.load("spritesheet.png")`.

use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_systems(Startup, setup)
        .add_systems(Update, animate_sprite)
        .run();
}

/// Stores the inclusive index range `[first, last]` for a looping animation.
#[derive(Component)]
struct AnimationIndices {
    first: usize,
    last: usize,
}

/// Per-entity timer that drives frame advances.
///
/// `#[derive(Deref, DerefMut)]` lets callers write `timer.tick(...)` directly
/// instead of `timer.0.tick(...)`.
#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

/// Builds the programmatic atlas and spawns the animated sprite.
fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    commands.spawn(Camera2d);

    const FRAME_PX: u32 = 32;
    const FRAMES: u32 = 4;
    let colors: [[u8; 4]; 4] = [
        [220,  80,  80, 255], // red
        [ 80, 200,  80, 255], // green
        [ 80, 120, 220, 255], // blue
        [220, 200,  60, 255], // yellow
    ];

    let mut data = vec![0u8; (FRAME_PX * FRAMES * FRAME_PX * 4) as usize];
    for frame in 0..FRAMES {
        for y in 0..FRAME_PX {
            for x in 0..FRAME_PX {
                let px = frame * FRAME_PX + x;
                let idx = ((y * FRAME_PX * FRAMES + px) * 4) as usize;
                data[idx..idx + 4].copy_from_slice(&colors[frame as usize]);
            }
        }
    }

    let atlas_image = Image::new(
        Extent3d { width: FRAME_PX * FRAMES, height: FRAME_PX, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    let image_handle = images.add(atlas_image);

    let layout = TextureAtlasLayout::from_grid(UVec2::splat(FRAME_PX), FRAMES, 1, None, None);
    let layout_handle = layouts.add(layout);

    commands.spawn((
        Sprite {
            image: image_handle,
            texture_atlas: Some(TextureAtlas { layout: layout_handle, index: 0 }),
            custom_size: Some(Vec2::splat(128.0)),
            ..default()
        },
        AnimationIndices { first: 0, last: (FRAMES - 1) as usize },
        AnimationTimer(Timer::from_seconds(0.2, TimerMode::Repeating)),
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
        if timer.just_finished() {
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.index = next_frame(atlas.index, indices.first, indices.last);
            }
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
}
