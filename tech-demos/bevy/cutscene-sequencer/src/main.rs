//! Runnable entry point for the cutscene-sequencer demo.
//!
//! All the reusable logic lives in the crate library ([`cutscene_sequencer`])
//! as [`CutsceneSequencerPlugin`]. This binary is just the thin harness that
//! boots the engine and adds the plugin — copy `lib.rs` into your own project
//! (or depend on this crate) and add the one plugin to reuse the feature.

use bevy::prelude::*;
use cutscene_sequencer::CutsceneSequencerPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Cutscene Sequencer".into(),
                resolution: (900u32, 550u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(CutsceneSequencerPlugin)
        .run();
}
