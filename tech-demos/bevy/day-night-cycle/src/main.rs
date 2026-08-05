//! Runnable entry point for the day-night-cycle demo.
//!
//! All the reusable logic lives in the crate library ([`day_night_cycle`]) as
//! [`DayNightCyclePlugin`]. This binary is just the thin harness that boots the
//! engine, configures the window and initial clear colour, and adds the plugin
//! — copy `lib.rs` into your own project (or depend on this crate) and add the
//! one plugin to reuse the feature.

use bevy::prelude::*;
use bevy::window::WindowResolution;
use day_night_cycle::DayNightCyclePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Day-Night Cycle — +/- to change speed, R to reset".to_string(),
                resolution: WindowResolution::from((800u32, 500u32)),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.02, 0.02, 0.08)))
        .add_plugins(DayNightCyclePlugin)
        .run();
}
