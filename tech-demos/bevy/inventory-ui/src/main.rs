//! Runnable entry point for the inventory-ui demo.
//!
//! All the reusable logic lives in the crate library ([`inventory_ui`]) as
//! [`InventoryUiPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin — copy `lib.rs` into your own project (or depend
//! on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use inventory_ui::InventoryUiPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Inventory UI".into(),
                resolution: (800u32, 600u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(InventoryUiPlugin)
        .run();
}
