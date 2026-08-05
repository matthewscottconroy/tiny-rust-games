//! Runnable entry point for the pixel-perfect-camera demo.
//!
//! All the reusable logic lives in the crate library ([`pixel_perfect_camera`])
//! as [`PixelPerfectCameraPlugin`]. This binary is just the thin harness that
//! boots the engine, sizes the real window (1280×720 = exactly 4× the virtual
//! 320×180 canvas), and adds the plugin.
//!
//! **Controls:** WASD (one grid step per press).

use bevy::{prelude::*, window::WindowResolution};
use pixel_perfect_camera::PixelPerfectCameraPlugin;

/// Real window width in physical pixels.
const WIN_W: u32 = 1280;
/// Real window height in physical pixels.
const WIN_H: u32 = 720;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Pixel-Perfect Camera — WASD to move (grid-snapped)".to_string(),
                resolution: WindowResolution::new(WIN_W, WIN_H),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PixelPerfectCameraPlugin)
        .run();
}
