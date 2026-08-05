//! Runnable entry point for the destructible-terrain demo.
//!
//! All the reusable logic lives in the crate library ([`destructible_terrain`])
//! as [`DestructibleTerrainPlugin`]. This binary is just the thin harness that
//! boots the engine and adds the plugin — copy `lib.rs` into your own project
//! (or depend on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use destructible_terrain::DestructibleTerrainPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Destructible Terrain".into(),
                resolution: (800u32, 500u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(DestructibleTerrainPlugin)
        .run();
}
