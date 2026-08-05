//! Runnable entry point for the water-ripple demo.
//!
//! All the reusable logic lives in the crate library ([`water_ripple`]) as
//! [`WaterRipplePlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin.

use bevy::prelude::*;
use water_ripple::WaterRipplePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Water Ripple".into(),
                resolution: (800u32, 600u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(WaterRipplePlugin)
        .run();
}
