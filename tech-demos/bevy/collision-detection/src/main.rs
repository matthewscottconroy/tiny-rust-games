//! Runnable entry point for the collision-detection demo.
//!
//! All the reusable logic lives in the crate library ([`collision_detection`]) as
//! [`CollisionDetectionPlugin`]. This binary is just the thin harness that boots
//! the engine and adds the plugin.

use bevy::prelude::*;
use collision_detection::CollisionDetectionPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(CollisionDetectionPlugin)
        .run();
}
