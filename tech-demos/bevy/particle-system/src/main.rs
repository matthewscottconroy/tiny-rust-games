//! Runnable entry point for the particle-system demo.
//!
//! All the reusable logic lives in the crate library ([`particle_system`]) as
//! [`ParticleSystemPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin — copy `lib.rs` into your own project (or depend
//! on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use particle_system::ParticleSystemPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ParticleSystemPlugin)
        .run();
}
