//! Runnable entry point for the lighting-2d demo.
//!
//! All the reusable logic lives in the crate library ([`lighting_2d`]) as
//! [`Lighting2dPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin — copy `lib.rs` into your own project (or depend
//! on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use lighting_2d::Lighting2dPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Simulated 2D Lighting — WASD to move light, +/- ambient".to_string(),
                resolution: (704u32, 514u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(Lighting2dPlugin)
        .run();
}
