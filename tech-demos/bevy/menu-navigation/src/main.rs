//! Runnable entry point for the menu-navigation demo.
//!
//! All the reusable logic lives in the crate library ([`menu_navigation`]) as
//! [`MenuNavigationPlugin`]. This binary is just the thin harness that boots
//! the engine and adds the plugin — copy `lib.rs` into your own project (or
//! depend on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use bevy::window::WindowResolution;
use menu_navigation::MenuNavigationPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Menu Navigation".to_string(),
                resolution: WindowResolution::from((800_u32, 500_u32)),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MenuNavigationPlugin)
        .run();
}
