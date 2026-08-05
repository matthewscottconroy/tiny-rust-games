//! Runnable entry point for the rabbit_carrot_wolf demo.
use bevy::prelude::*;
use rabbit_carrot_wolf::RabbitCarrotWolfPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Ecosystem Simulation".to_string(),
                resolution: (1100, 600).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(RabbitCarrotWolfPlugin)
        .run();
}
