//! Runnable entry point for the time-scale demo.
//!
//! All the reusable logic lives in the crate library ([`time_scale`]) as
//! [`TimeScalePlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin — copy `lib.rs` into your own project (or depend
//! on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use bevy::window::WindowResolution;
use time_scale::TimeScalePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Time Scale — 1/2/3/4 speed, SPACE pause".to_string(),
                resolution: WindowResolution::from((800u32, 500u32)),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(TimeScalePlugin)
        .run();
}
