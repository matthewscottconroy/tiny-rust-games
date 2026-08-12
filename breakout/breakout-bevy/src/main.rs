//! Runnable entry point for the Bevy Breakout frontend.
//!
//! All rules live in [`breakout_lib`] and all presentation in
//! [`breakout_bevy`]; this binary only boots the engine and adds the plugin.

use bevy::prelude::*;
use breakout_bevy::BreakoutPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Breakout (Bevy)".into(),
                resolution: (820u32, 660u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.04, 0.04, 0.06)))
        .add_plugins(BreakoutPlugin)
        .run();
}
