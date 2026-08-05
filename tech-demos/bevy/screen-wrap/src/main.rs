//! Runnable entry point for the screen-wrap demo.
//! The reusable logic lives in the crate library as [`ScreenWrapPlugin`].

use bevy::prelude::*;
use screen_wrap::ScreenWrapPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ScreenWrapPlugin)
        .run();
}
