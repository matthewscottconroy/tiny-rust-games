//! Observer events — a reusable Bevy plugin comparing two event delivery styles.
//!
//! This crate is a *building block*: drop [`ObserverEventsPlugin`] into any
//! Bevy app with `app.add_plugins(ObserverEventsPlugin)` for a side-by-side
//! comparison of Bevy 0.18's two event delivery systems.
//!
//! **Left target — Message system** (queued delivery via `MessageWriter`/`MessageReader`):
//! - Press `Q` to write a [`TraditionalHit`] message via `MessageWriter`.
//! - A separate reader system picks it up on the same frame and records the hit.
//! - This pattern (`Message` + `MessageWriter`/`MessageReader`) is the Bevy 0.18
//!   equivalent of the old `EventWriter`/`EventReader` pattern. It is useful when
//!   many systems need to poll for the same kind of notification each frame.
//!
//! **Right target — Observer system** (immediate reactive delivery via `On<E>`):
//! - Press `E` to trigger an [`ObserverHit`] event via `commands.trigger(...)`.
//! - A global observer reacts immediately, without a separate polling system.
//! - Observers (registered with `app.add_observer`) give reactive, callback-style
//!   semantics and are the recommended default for targeted one-off reactions.
//!
//! A hit counter and flash effect (red for [`ObserverConfig::flash_duration`]
//! seconds) illustrate both paths in action.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use observer_events::ObserverEventsPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(ObserverEventsPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles the systems, message, observer, and resources for the demo.
///
/// Add it with `app.add_plugins(ObserverEventsPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct ObserverEventsPlugin;

impl Plugin for ObserverEventsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ObserverConfig>()
            .add_message::<TraditionalHit>()
            .add_observer(react_observer)
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    fire_traditional,
                    react_traditional,
                    fire_observer,
                    update_visuals,
                )
                    .chain(),
            );
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const BASE_COLOR: [f32; 3] = [0.2, 0.6, 0.2];
const FLASH_COLOR: [f32; 3] = [0.9, 0.1, 0.1];

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Tunable parameters for the feature. Override before adding the plugin, e.g.
/// `app.insert_resource(ObserverConfig { flash_duration: 0.5 })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct ObserverConfig {
    /// How long a target stays red after being hit, in seconds.
    pub flash_duration: f32,
}

impl Default for ObserverConfig {
    fn default() -> Self {
        Self {
            flash_duration: 0.3,
        }
    }
}

// ---------------------------------------------------------------------------
// Pure helper functions
// ---------------------------------------------------------------------------

/// Returns the interpolated colour between `base` and `flash` based on how
/// much time has elapsed since the hit.
///
/// Returns `flash` at `elapsed = 0` and `base` once `elapsed >= duration`.
pub fn flash_color(elapsed: f32, duration: f32, base: [f32; 3], flash: [f32; 3]) -> [f32; 3] {
    if duration <= 0.0 || elapsed >= duration {
        return base;
    }
    let t = elapsed / duration; // 0 → 1
    [
        flash[0] + (base[0] - flash[0]) * t,
        flash[1] + (base[1] - flash[1]) * t,
        flash[2] + (base[2] - flash[2]) * t,
    ]
}

/// Returns `true` if an event triggered at `event_time` is still active at
/// `current_time` given a `duration`.
pub fn is_active(event_time: f32, current_time: f32, duration: f32) -> bool {
    current_time - event_time < duration
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Marks a target entity and stores its hit count plus last-hit timestamp.
#[derive(Component)]
pub struct HitTarget {
    /// Number of times this target has been hit.
    pub hits: u32,
    /// Elapsed-seconds timestamp of the most recent hit, or `f32::NEG_INFINITY`.
    pub last_hit: f32,
}

/// Distinguishes which input key / event system drives a target.
#[derive(Component, PartialEq, Clone, Copy)]
pub enum TargetKind {
    /// Driven by the message (queued) path — press `Q`.
    Traditional,
    /// Driven by the observer (immediate) path — press `E`.
    Observer,
}

// ---------------------------------------------------------------------------
// Message (traditional queued-event equivalent)
// ---------------------------------------------------------------------------

/// Fired via [`MessageWriter`] when the player presses `Q`.
///
/// This is the Bevy 0.18 equivalent of the old `EventWriter`-based pattern.
/// Messages are queued and read by polling systems using [`MessageReader`].
#[derive(Message, Clone)]
pub struct TraditionalHit {
    /// The entity to flash.
    pub target: Entity,
}

// ---------------------------------------------------------------------------
// Observer event (immediate trigger)
// ---------------------------------------------------------------------------

/// Triggered via `commands.trigger(...)` when the player presses `E`.
///
/// Global observers registered with `app.add_observer` react immediately
/// using the `On<ObserverHit>` system parameter.
#[derive(Event, Clone)]
pub struct ObserverHit {
    /// The entity to flash.
    pub target: Entity,
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Caches the two target entities so input systems can reference them easily.
#[derive(Resource)]
pub struct Targets {
    /// Entity driven by the message path.
    pub traditional: Entity,
    /// Entity driven by the observer path.
    pub observer: Entity,
}

// ---------------------------------------------------------------------------
// HUD marker
// ---------------------------------------------------------------------------

/// Marker component for the HUD text entity.
#[derive(Component)]
pub struct HudText;

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let trad_entity = commands
        .spawn((
            Sprite {
                color: Color::srgb(BASE_COLOR[0], BASE_COLOR[1], BASE_COLOR[2]),
                custom_size: Some(Vec2::new(120.0, 120.0)),
                ..default()
            },
            Transform::from_xyz(-180.0, 0.0, 0.0),
            HitTarget {
                hits: 0,
                last_hit: f32::NEG_INFINITY,
            },
            TargetKind::Traditional,
        ))
        .id();

    let obs_entity = commands
        .spawn((
            Sprite {
                color: Color::srgb(BASE_COLOR[0], BASE_COLOR[1], BASE_COLOR[2]),
                custom_size: Some(Vec2::new(120.0, 120.0)),
                ..default()
            },
            Transform::from_xyz(180.0, 0.0, 0.0),
            HitTarget {
                hits: 0,
                last_hit: f32::NEG_INFINITY,
            },
            TargetKind::Observer,
        ))
        .id();

    commands.insert_resource(Targets {
        traditional: trad_entity,
        observer: obs_entity,
    });

    // Labels above each target
    for (label, left_px) in [
        ("Message system\n(MessageWriter - press Q)", 80.0_f32),
        ("Observer system\n(Trigger - press E)", 470.0_f32),
    ] {
        commands.spawn((
            Text::new(label),
            TextFont {
                font_size: 15.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(140.0),
                left: Val::Px(left_px),
                ..default()
            },
        ));
    }

    // HUD
    commands.spawn((
        Text::new("Message hits: 0  |  Observer hits: 0"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        HudText,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
    ));
}

// ---------------------------------------------------------------------------
// Input systems
// ---------------------------------------------------------------------------

/// Writes a [`TraditionalHit`] message when `Q` is pressed.
fn fire_traditional(
    keys: Res<ButtonInput<KeyCode>>,
    targets: Res<Targets>,
    mut writer: MessageWriter<TraditionalHit>,
) {
    if keys.just_pressed(KeyCode::KeyQ) {
        writer.write(TraditionalHit {
            target: targets.traditional,
        });
    }
}

/// Triggers an [`ObserverHit`] event when `E` is pressed.
fn fire_observer(keys: Res<ButtonInput<KeyCode>>, targets: Res<Targets>, mut commands: Commands) {
    if keys.just_pressed(KeyCode::KeyE) {
        commands.trigger(ObserverHit {
            target: targets.observer,
        });
    }
}

// ---------------------------------------------------------------------------
// Reaction systems
// ---------------------------------------------------------------------------

/// Reads [`TraditionalHit`] messages and records the hit on the target entity.
fn react_traditional(
    mut reader: MessageReader<TraditionalHit>,
    mut query: Query<&mut HitTarget>,
    time: Res<Time>,
) {
    for ev in reader.read() {
        if let Ok(mut ht) = query.get_mut(ev.target) {
            ht.hits += 1;
            ht.last_hit = time.elapsed_secs();
        }
    }
}

/// Global observer that reacts to [`ObserverHit`] triggers.
fn react_observer(trigger: On<ObserverHit>, mut query: Query<&mut HitTarget>, time: Res<Time>) {
    let ev = trigger.event();
    if let Ok(mut ht) = query.get_mut(ev.target) {
        ht.hits += 1;
        ht.last_hit = time.elapsed_secs();
    }
}

// ---------------------------------------------------------------------------
// Visual update system
// ---------------------------------------------------------------------------

/// Updates sprite colours based on flash timers and refreshes the HUD text.
fn update_visuals(
    time: Res<Time>,
    config: Res<ObserverConfig>,
    mut targets: Query<(&HitTarget, &mut Sprite, &TargetKind)>,
    mut hud: Query<&mut Text, With<HudText>>,
) {
    let now = time.elapsed_secs();
    let mut trad_hits = 0u32;
    let mut obs_hits = 0u32;

    for (ht, mut sprite, kind) in &mut targets {
        let elapsed = now - ht.last_hit;
        let [r, g, b] = flash_color(elapsed, config.flash_duration, BASE_COLOR, FLASH_COLOR);
        sprite.color = Color::srgb(r, g, b);

        match kind {
            TargetKind::Traditional => trad_hits = ht.hits,
            TargetKind::Observer => obs_hits = ht.hits,
        }
    }

    if let Ok(mut text) = hud.single_mut() {
        **text = format!("Message hits: {trad_hits}  |  Observer hits: {obs_hits}");
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-5;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPS
    }

    /// At elapsed=0 the colour should be pure flash.
    #[test]
    fn flash_color_at_zero_elapsed() {
        let result = flash_color(0.0, 0.3, BASE_COLOR, FLASH_COLOR);
        assert!(approx_eq(result[0], FLASH_COLOR[0]));
        assert!(approx_eq(result[1], FLASH_COLOR[1]));
        assert!(approx_eq(result[2], FLASH_COLOR[2]));
    }

    /// At elapsed >= duration the colour should be the base colour.
    #[test]
    fn flash_color_at_full_duration() {
        let result = flash_color(0.3, 0.3, BASE_COLOR, FLASH_COLOR);
        assert!(approx_eq(result[0], BASE_COLOR[0]));
        assert!(approx_eq(result[1], BASE_COLOR[1]));
        assert!(approx_eq(result[2], BASE_COLOR[2]));
    }

    /// Past the duration the base colour is returned.
    #[test]
    fn flash_color_past_duration() {
        let result = flash_color(1.0, 0.3, BASE_COLOR, FLASH_COLOR);
        assert!(approx_eq(result[0], BASE_COLOR[0]));
    }

    /// At the midpoint the colour is the average of flash and base.
    #[test]
    fn flash_color_midpoint() {
        let result = flash_color(0.15, 0.3, BASE_COLOR, FLASH_COLOR);
        let expected_r = FLASH_COLOR[0] + (BASE_COLOR[0] - FLASH_COLOR[0]) * 0.5;
        assert!(approx_eq(result[0], expected_r));
    }

    /// Zero duration returns the base colour immediately.
    #[test]
    fn flash_color_zero_duration_returns_base() {
        let result = flash_color(0.0, 0.0, BASE_COLOR, FLASH_COLOR);
        assert!(approx_eq(result[0], BASE_COLOR[0]));
    }

    /// `is_active` returns true when within duration.
    #[test]
    fn is_active_within_duration() {
        assert!(is_active(1.0, 1.2, 0.3));
    }

    /// `is_active` returns false when outside duration.
    #[test]
    fn is_active_outside_duration() {
        assert!(!is_active(1.0, 1.4, 0.3));
    }

    /// `is_active` returns false slightly past the duration boundary.
    #[test]
    fn is_active_at_boundary() {
        // Use a value clearly beyond the boundary to avoid f32 rounding surprises.
        assert!(!is_active(1.0, 1.301, 0.3));
    }

    /// The default config exposes a positive flash duration.
    #[test]
    fn config_default_is_valid() {
        assert!(ObserverConfig::default().flash_duration > 0.0);
    }

    /// The plugin spawns both hit targets on a headless app.
    #[test]
    fn setup_spawns_two_targets() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, ObserverEventsPlugin));
        // Input systems read the keyboard, absent under MinimalPlugins.
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.update();

        let mut q = app.world_mut().query::<&HitTarget>();
        assert_eq!(q.iter(app.world()).count(), 2);
    }
}
