//! Runnable entry point for the required-components demo.

use bevy::prelude::*;
use required_components::RequiredComponentsPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Required Components Demo".into(),
                resolution: (800u32, 500u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(RequiredComponentsPlugin)
        .run();
}
