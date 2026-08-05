//! Runnable entry point for the rubber-band-selection demo.
use bevy::prelude::*;
use rubber_band_selection::RubberBandSelectionPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Rubber-Band Selection Demo".into(),
                resolution: (900u32, 600u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(RubberBandSelectionPlugin)
        .run();
}
