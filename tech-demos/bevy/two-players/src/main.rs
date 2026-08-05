//! Runnable entry point for the two-player movement demo.
//!
//! All the reusable logic lives in the crate library ([`two_players`]) as
//! [`TwoPlayersPlugin`]. This binary is just the thin harness that boots the
//! engine and adds the plugin.
//!
//! **Controls:** Player 1 uses WASD; Player 2 uses IJKL.

use bevy::prelude::*;
use two_players::TwoPlayersPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TwoPlayersPlugin)
        .run();
}
