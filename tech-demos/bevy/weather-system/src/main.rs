//! Runnable entry point for the weather-system demo.
//!
//! All the reusable logic lives in the crate library ([`weather_system`]) as
//! [`WeatherSystemPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin — copy `lib.rs` into your own project (or depend
//! on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use weather_system::WeatherSystemPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Weather System".into(),
                resolution: (800u32, 500u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.42, 0.68, 1.0)))
        .add_plugins(WeatherSystemPlugin)
        .run();
}
