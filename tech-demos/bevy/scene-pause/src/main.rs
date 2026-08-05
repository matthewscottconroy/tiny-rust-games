//! Runnable entry point for the scene-pause demo.
use bevy::prelude::*;
use scene_pause::ScenePausePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ScenePausePlugin)
        .run();
}
