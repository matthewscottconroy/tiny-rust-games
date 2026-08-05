//! Runnable entry point for the save-load demo.
//! The reusable logic lives in the crate library as [`SaveLoadPlugin`].

use bevy::prelude::*;
use save_load::SaveLoadPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SaveLoadPlugin)
        .run();
}
