//! Audio demo — background music and triggered sound effects.
//!
//! Required assets (place in `tech-demos/bevy/audio/assets/sounds/`):
//! - `music.ogg`  — loops continuously as background music
//! - `click.ogg`  — plays once each time SPACE is pressed
//!
//! Any royalty-free `.ogg` files work.  The app still runs without them;
//! Bevy logs a warning and the `AudioPlayer` entities become silent.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, play_sfx)
        .run();
}

/// Caches the click-sound asset handle so [`play_sfx`] can clone it cheaply.
#[derive(Resource)]
struct ClickSound(Handle<AudioSource>);

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
