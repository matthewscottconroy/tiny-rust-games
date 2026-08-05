//! Runnable entry point for the pickup-and-inventory demo.
//!
//! All the reusable logic lives in the crate library ([`pickup_and_inventory`])
//! as [`PickupAndInventoryPlugin`]. This binary is just the thin harness that
//! boots the engine and adds the plugin — copy `lib.rs` into your own project
//! (or depend on this crate) and add the one plugin to reuse the feature.
//!
//! **Controls:** WASD to move, Q to drop an item.

use bevy::prelude::*;
use pickup_and_inventory::PickupAndInventoryPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PickupAndInventoryPlugin)
        .run();
}
