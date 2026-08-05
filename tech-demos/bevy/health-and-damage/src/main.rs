//! Runnable entry point for the health-and-damage demo.
//!
//! All the reusable logic lives in the crate library ([`health_and_damage`]) as
//! [`HealthAndDamagePlugin`]. This binary is just the thin harness that boots
//! the engine and adds the plugin — copy `lib.rs` into your own project (or
//! depend on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use health_and_damage::HealthAndDamagePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(HealthAndDamagePlugin)
        .run();
}
