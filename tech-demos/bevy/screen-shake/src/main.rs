//! Runnable entry point for the screen-shake demo.
//! The reusable logic lives in the crate library as [`ScreenShakePlugin`].

use bevy::prelude::*;
use screen_shake::ScreenShakePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Screen Shake — SPACE to shake, R to reset".to_string(),
                resolution: (800, 500).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(ScreenShakePlugin)
        .run();
}
