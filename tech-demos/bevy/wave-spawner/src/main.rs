//! Runnable entry point for the wave-spawner demo.
//!
//! All the reusable logic lives in the crate library ([`wave_spawner`]) as
//! [`WaveSpawnerPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin.

use bevy::prelude::*;
use wave_spawner::WaveSpawnerPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Wave Spawner".to_string(),
                resolution: (800, 500).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(WaveSpawnerPlugin)
        .run();
}
