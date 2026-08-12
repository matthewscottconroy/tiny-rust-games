//! Runnable entry point for the Bevy tic-tac-toe frontend.
//!
//! All game logic lives in [`tic_tac_toe_lib`] and all presentation in
//! [`tic_tac_toe_bevy`]; this binary only boots the engine and adds the plugin.

use bevy::prelude::*;
use tic_tac_toe_bevy::TicTacToePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Tic-Tac-Toe (Bevy)".into(),
                resolution: (560u32, 680u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.08, 0.08, 0.10)))
        .add_plugins(TicTacToePlugin)
        .run();
}
