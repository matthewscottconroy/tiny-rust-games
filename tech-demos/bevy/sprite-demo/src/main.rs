//! Runnable entry point for the sprite-demo demo.

use bevy::prelude::*;
use sprite_demo::SpriteDemoPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SpriteDemoPlugin)
        .run();
}
