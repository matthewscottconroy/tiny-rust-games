//! Runnable entry point for the projectiles demo.
//! The reusable logic lives in the crate library as [`ProjectilesPlugin`].

use bevy::prelude::*;
use projectiles::ProjectilesPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ProjectilesPlugin)
        .run();
}
