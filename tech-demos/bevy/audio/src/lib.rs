//! Audio — a reusable Bevy plugin for background music and triggered SFX.
//!
//! This crate is a *building block*: drop [`AudioPlugin`] into any Bevy app with
//! `app.add_plugins(AudioPlugin)` and it starts looping background music and
//! plays a one-shot click each time SPACE is pressed.
//!
//! Required assets (place in `tech-demos/bevy/audio/assets/sounds/`):
//! - `music.ogg`  — loops continuously as background music
//! - `click.ogg`  — plays once each time SPACE is pressed
//!
//! Any royalty-free `.ogg` files work. The app still runs without them; Bevy
//! logs a warning and the `AudioPlayer` entities become silent.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use audio::AudioPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(AudioPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles the music/SFX systems for the audio feature.
///
/// Add it with `app.add_plugins(AudioPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window, rendering, and
/// the underlying `bevy_audio` backend.
pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, play_sfx);
    }
}

/// Caches the click-sound asset handle so [`play_sfx`] can clone it cheaply.
#[derive(Resource)]
pub struct ClickSound(pub Handle<AudioSource>);

/// Spawns the camera, starts looping background music, and stores the SFX handle.
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    commands.spawn((
        AudioPlayer::new(asset_server.load("sounds/music.ogg")),
        PlaybackSettings::LOOP,
    ));

    commands.insert_resource(ClickSound(asset_server.load("sounds/click.ogg")));

    commands.spawn((
        Text::new("SPACE — play click sound effect"),
        TextFont { font_size: 24.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(20.0),
            left: Val::Px(20.0),
            ..default()
        },
    ));
}

/// Spawns a one-shot `AudioPlayer` each time SPACE is pressed.
///
/// `PlaybackSettings::DESPAWN` automatically removes the entity when playback
/// finishes, preventing entity accumulation.
fn play_sfx(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    click: Res<ClickSound>,
) {
    if input.just_pressed(KeyCode::Space) {
        commands.spawn((
            AudioPlayer::new(click.0.clone()),
            PlaybackSettings::DESPAWN,
        ));
    }
}
