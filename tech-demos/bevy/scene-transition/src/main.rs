//! Runnable entry point for the scene-transition demo.
use bevy::prelude::*;
use scene_transition::SceneTransitionPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SceneTransitionPlugin)
        .run();
}
