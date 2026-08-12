//! Audio — a reusable Bevy plugin for background music and triggered SFX.
//!
//! This crate is a *building block*: drop [`AudioPlugin`] into any Bevy app with
//! `app.add_plugins(AudioPlugin)` and it starts looping a background tone and
//! plays a one-shot blip each time SPACE is pressed.
//!
//! Key ideas:
//! - Sounds are **generated in code**, not loaded from disk. [`Tone`] describes
//!   a waveform and [`render_tone`] turns it into PCM samples, which are wrapped
//!   in an [`AudioSource`] and added to `Assets<AudioSource>` directly. That
//!   keeps the demo runnable with no asset files to fetch, and makes the sound
//!   itself unit-testable.
//! - `PlaybackSettings::LOOP` keeps the pad playing; `PlaybackSettings::DESPAWN`
//!   cleans up one-shot entities when they finish, preventing accumulation.
//! - Tune pitch, duration, and volume through [`AudioConfig`].
//!
//! To use real files instead, replace the [`render_tone`] calls in `setup` with
//! `asset_server.load("sounds/music.ogg")` and drop an `assets/sounds/`
//! directory next to `Cargo.toml`.
//!
//! **Controls:** SPACE — play the blip.
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

use bevy::audio::AddAudioSource;
use bevy::audio::Source;
use bevy::prelude::*;
use std::time::Duration;

/// Sample rate used for every generated sound, in Hz.
pub const SAMPLE_RATE: u32 = 44_100;

/// Bundles the music/SFX systems for the audio feature.
///
/// Add it with `app.add_plugins(AudioPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window, rendering, and
/// the underlying `bevy_audio` backend.
pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioConfig>()
            .add_audio_source::<PcmAudio>()
            .add_systems(Startup, setup)
            .add_systems(Update, play_sfx);
    }
}

/// Tunables for the generated sounds. Override before adding the plugin, e.g.
/// `app.insert_resource(AudioConfig { blip_hz: 880.0, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct AudioConfig {
    /// Pitch of the looping background pad, in Hz.
    pub pad_hz: f32,
    /// Length of one loop of the background pad, in seconds.
    pub pad_seconds: f32,
    /// Amplitude of the background pad, in `0.0..=1.0`.
    pub pad_amplitude: f32,
    /// Pitch of the one-shot blip, in Hz.
    pub blip_hz: f32,
    /// Length of the blip, in seconds.
    pub blip_seconds: f32,
    /// Amplitude of the blip, in `0.0..=1.0`.
    pub blip_amplitude: f32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            pad_hz: 110.0,
            pad_seconds: 2.0,
            pad_amplitude: 0.15,
            blip_hz: 660.0,
            blip_seconds: 0.12,
            blip_amplitude: 0.35,
        }
    }
}

/// A sine tone with a linear fade-out, described independently of Bevy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tone {
    /// Pitch in Hz.
    pub frequency: f32,
    /// Duration in seconds.
    pub seconds: f32,
    /// Peak amplitude, in `0.0..=1.0`.
    pub amplitude: f32,
}

/// Renders a [`Tone`] to mono PCM samples in `-1.0..=1.0`.
///
/// Applies a linear fade to silence across the tone so looping the pad and
/// re-triggering the blip do not click. Pure, so the waveform can be tested
/// without an audio device.
pub fn render_tone(tone: &Tone, sample_rate: u32) -> Vec<f32> {
    let count = (tone.seconds.max(0.0) * sample_rate as f32) as usize;
    (0..count)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            // Linear ramp from full amplitude to silence.
            let fade = 1.0 - (i as f32 / count as f32);
            let phase = std::f32::consts::TAU * tone.frequency * t;
            phase.sin() * tone.amplitude * fade
        })
        .collect()
}

/// An [`AudioSource`]-compatible asset backed by in-memory PCM samples.
#[derive(Asset, TypePath, Clone)]
pub struct PcmAudio {
    /// Mono samples in `-1.0..=1.0`.
    pub samples: Vec<f32>,
    /// Samples per second.
    pub sample_rate: u32,
}

/// Streams a [`PcmAudio`] asset to the audio backend.
pub struct PcmDecoder {
    samples: std::vec::IntoIter<f32>,
    sample_rate: u32,
}

impl Iterator for PcmDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        self.samples.next()
    }
}

impl Source for PcmDecoder {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl Decodable for PcmAudio {
    type Decoder = PcmDecoder;
    type DecoderItem = f32;

    fn decoder(&self) -> Self::Decoder {
        PcmDecoder {
            samples: self.samples.clone().into_iter(),
            sample_rate: self.sample_rate,
        }
    }
}

/// Caches the blip asset handle so [`play_sfx`] can clone it cheaply.
#[derive(Resource)]
pub struct BlipSound(pub Handle<PcmAudio>);

/// Spawns the camera, starts the looping pad, and stores the blip handle.
fn setup(mut commands: Commands, mut sounds: ResMut<Assets<PcmAudio>>, config: Res<AudioConfig>) {
    commands.spawn(Camera2d);

    let pad = sounds.add(PcmAudio {
        samples: render_tone(
            &Tone {
                frequency: config.pad_hz,
                seconds: config.pad_seconds,
                amplitude: config.pad_amplitude,
            },
            SAMPLE_RATE,
        ),
        sample_rate: SAMPLE_RATE,
    });
    commands.spawn((AudioPlayer(pad), PlaybackSettings::LOOP));

    let blip = sounds.add(PcmAudio {
        samples: render_tone(
            &Tone {
                frequency: config.blip_hz,
                seconds: config.blip_seconds,
                amplitude: config.blip_amplitude,
            },
            SAMPLE_RATE,
        ),
        sample_rate: SAMPLE_RATE,
    });
    commands.insert_resource(BlipSound(blip));

    commands.spawn((
        Text::new("SPACE — play blip (all sound generated in code)"),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(20.0),
            left: Val::Px(20.0),
            ..default()
        },
    ));
}

/// Spawns a one-shot player each time SPACE is pressed.
///
/// `PlaybackSettings::DESPAWN` removes the entity when playback finishes.
fn play_sfx(mut commands: Commands, input: Res<ButtonInput<KeyCode>>, blip: Res<BlipSound>) {
    if input.just_pressed(KeyCode::Space) {
        commands.spawn((AudioPlayer(blip.0.clone()), PlaybackSettings::DESPAWN));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone() -> Tone {
        Tone {
            frequency: 440.0,
            seconds: 0.5,
            amplitude: 0.5,
        }
    }

    #[test]
    fn render_tone_produces_one_sample_per_tick() {
        let samples = render_tone(&tone(), 1000);
        assert_eq!(samples.len(), 500);
    }

    #[test]
    fn samples_stay_within_the_amplitude() {
        for s in render_tone(&tone(), SAMPLE_RATE) {
            assert!(s.abs() <= 0.5, "sample {s} exceeds amplitude");
        }
    }

    #[test]
    fn tone_fades_to_silence_so_loops_do_not_click() {
        let samples = render_tone(&tone(), SAMPLE_RATE);
        let last = samples.last().copied().unwrap();
        assert!(
            last.abs() < 1e-3,
            "tone should end near silence, got {last}"
        );
    }

    #[test]
    fn a_zero_length_tone_renders_nothing() {
        let silent = Tone {
            seconds: 0.0,
            ..tone()
        };
        assert!(render_tone(&silent, SAMPLE_RATE).is_empty());
        // A negative duration is clamped rather than panicking on the cast.
        let negative = Tone {
            seconds: -1.0,
            ..tone()
        };
        assert!(render_tone(&negative, SAMPLE_RATE).is_empty());
    }

    #[test]
    fn higher_frequency_crosses_zero_more_often() {
        let count_crossings = |hz: f32| {
            let s = render_tone(
                &Tone {
                    frequency: hz,
                    seconds: 1.0,
                    amplitude: 1.0,
                },
                SAMPLE_RATE,
            );
            s.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count()
        };
        assert!(count_crossings(880.0) > count_crossings(440.0));
    }

    #[test]
    fn default_config_is_audible_and_bounded() {
        let c = AudioConfig::default();
        assert!(c.pad_hz > 0.0 && c.blip_hz > 0.0);
        assert!(c.pad_seconds > 0.0 && c.blip_seconds > 0.0);
        assert!(c.pad_amplitude > 0.0 && c.pad_amplitude <= 1.0);
        assert!(c.blip_amplitude > 0.0 && c.blip_amplitude <= 1.0);
    }

    #[test]
    fn plugin_generates_both_sounds_on_startup() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_resource::<AudioConfig>()
            .init_asset::<PcmAudio>()
            .add_systems(Startup, setup);
        app.update();

        assert!(
            app.world().get_resource::<BlipSound>().is_some(),
            "setup should cache the blip handle"
        );
        let sounds = app.world().resource::<Assets<PcmAudio>>();
        assert_eq!(sounds.len(), 2, "expected a pad and a blip");
    }
}
