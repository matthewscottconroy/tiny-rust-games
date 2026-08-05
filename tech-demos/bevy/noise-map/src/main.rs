//! Runnable entry point for the noise-map demo.
//!
//! All the reusable logic lives in the crate library ([`noise_map`]) as
//! [`NoiseMapPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin — copy `lib.rs` into your own project (or depend
//! on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use noise_map::NoiseMapPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Noise Map — SPACE to regenerate".into(),
                resolution: (900u32, 550u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(NoiseMapPlugin)
        .run();
}
