//! Runnable entry point for the rope-simulation demo.
//!
//! Move the mouse — the rope of point masses hangs from the cursor.

use bevy::prelude::*;
use rope_simulation::RopeSimulationPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Rope Simulation — move the mouse".into(),
                resolution: (800u32, 500u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(RopeSimulationPlugin)
        .run();
}
