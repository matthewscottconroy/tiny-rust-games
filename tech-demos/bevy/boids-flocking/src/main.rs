//! Runnable entry point for the boids-flocking demo.
//!
//! All the reusable logic lives in the crate library ([`boids_flocking`]) as
//! [`BoidsFlockingPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin — copy `lib.rs` into your own project (or depend
//! on this crate) and add the one plugin to reuse the flocking behavior.

use bevy::prelude::*;
use boids_flocking::BoidsFlockingPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BoidsFlockingPlugin)
        .run();
}
