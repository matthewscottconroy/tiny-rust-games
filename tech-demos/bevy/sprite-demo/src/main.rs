//! Sprite loading demo — the simplest way to display an image in Bevy.
//!
//! `AssetServer::load` returns a handle immediately; Bevy loads the file
//! asynchronously in the background and the sprite appears once ready.
//! Place `assets/sprite.png` relative to this crate's directory.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

/// Spawns a camera and a single sprite loaded from disk.
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    commands.spawn(Sprite {
        image: asset_server.load("sprite.png"),
        ..default()
    });
}
