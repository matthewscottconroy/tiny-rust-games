//! Runnable entry point for the fixed-timestep demo.
//!
//! All the reusable logic lives in the crate library ([`fixed_timestep`]) as
//! [`FixedTimestepPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin — copy `lib.rs` into your own project (or depend
//! on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use fixed_timestep::FixedTimestepPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(FixedTimestepPlugin)
        .run();
}
