//! Runnable entry point for the Bevy Snake frontend.
//!
//! All rules live in [`snake_lib`] and all presentation in [`snake_bevy`]; this
//! binary only boots the engine and adds the plugin.

use bevy::prelude::*;
use snake_bevy::SnakePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Snake (Bevy)".into(),
                resolution: (720u32, 560u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.05, 0.05, 0.07)))
        .add_plugins(SnakePlugin)
        .run();
}
