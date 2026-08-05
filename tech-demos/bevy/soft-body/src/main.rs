//! Runnable entry point for the soft-body demo.
//!
//! Left-click drag near a node to grab and pull the elastic net.

use bevy::prelude::*;
use soft_body::SoftBodyPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Soft-Body Simulation".into(),
                resolution: (800u32, 600u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(SoftBodyPlugin)
        .run();
}
