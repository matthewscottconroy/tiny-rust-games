//! Runnable entry point for the grid-movement demo.
//!
//! All the reusable logic lives in the crate library ([`grid_movement`]) as
//! [`GridMovementPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin — copy `lib.rs` into your own project (or depend
//! on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use grid_movement::GridMovementPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(GridMovementPlugin)
        .run();
}
