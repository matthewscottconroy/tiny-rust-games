//! Ability Cooldowns — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`AbilityCooldownsPlugin`] into any
//! Bevy app with `app.add_plugins(AbilityCooldownsPlugin)` and it manages
//! multiple player abilities with independent cooldown timers and charge bars.
//!
//! Key ideas:
//! - Each ability is a plain [`Ability`] struct with a `cooldown_max` and
//!   `cooldown_remaining`.
//! - [`is_ready`], [`cooldown_fraction`], [`tick_ability`], and [`use_ability`]
//!   are pure functions that require no Bevy types and can be tested in
//!   isolation.
//! - Three abilities (Dash, Shield, Nova) have different cooldown lengths. Each
//!   has a visual fill-bar that shows charge progress from 0 % to 100 %.
//! - Pressing a key while an ability is on cooldown does nothing; the bar gives
//!   clear feedback without any additional state.
//! - Layout tunables live in [`AbilityCooldownsConfig`] so setup and the UI
//!   system stay in sync.
//!
//! **Controls:** Q — Dash (1 s)   W — Shield (3 s)   E — Nova (8 s)
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use ability_cooldowns::AbilityCooldownsPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(AbilityCooldownsPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system and resource for the ability-cooldown feature.
///
/// Add it with `app.add_plugins(AbilityCooldownsPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct AbilityCooldownsPlugin;

impl Plugin for AbilityCooldownsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AbilityCooldownsConfig>()
            .insert_resource(Abilities(default_abilities()))
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (tick_cooldowns, handle_input, update_ui, tick_flashes).chain(),
            );
    }
}

/// The three abilities the demo starts with, ordered as they appear in the HUD.
///
/// Hand-aligned into columns so the abilities read as a table; `rustfmt` is told
/// to leave it alone.
#[rustfmt::skip]
pub fn default_abilities() -> [Ability; 3] {
    [
        Ability { name: "Dash",   cooldown_max: 1.0, cooldown_remaining: 0.0, color: Color::srgb(0.3, 0.8, 1.0), key_label: "Q" },
        Ability { name: "Shield", cooldown_max: 3.0, cooldown_remaining: 0.0, color: Color::srgb(0.4, 1.0, 0.4), key_label: "W" },
        Ability { name: "Nova",   cooldown_max: 8.0, cooldown_remaining: 0.0, color: Color::srgb(1.0, 0.6, 0.2), key_label: "E" },
    ]
}

// ── Configuration ─────────────────────────────────────────────────────────────

/// Tunable layout parameters for the ability slots and player sprite.
///
/// Override before adding the plugin, e.g.
/// `app.insert_resource(AbilityCooldownsConfig { slot_w: 200.0, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct AbilityCooldownsConfig {
    /// Vertical position of the ability slot row.
    pub slot_y: f32,
    /// Width of each ability slot panel.
    pub slot_w: f32,
    /// Height of each ability slot panel.
    pub slot_h: f32,
    /// Horizontal gap between slots.
    pub gap: f32,
    /// The player sprite's base (idle) color.
    pub player_color: Color,
}

impl Default for AbilityCooldownsConfig {
    fn default() -> Self {
        Self {
            slot_y: -180.0,
            slot_w: 160.0,
            slot_h: 80.0,
            gap: 20.0,
            player_color: Color::srgb(0.5, 0.5, 0.9),
        }
    }
}

impl AbilityCooldownsConfig {
    /// X position of the centre of slot `i` given `count` total slots.
    fn slot_x(&self, i: usize, count: usize) -> f32 {
        let total_w = count as f32 * self.slot_w + (count.saturating_sub(1)) as f32 * self.gap;
        let start_x = -total_w / 2.0 + self.slot_w / 2.0;
        start_x + i as f32 * (self.slot_w + self.gap)
    }
}

// ── Pure ability model ────────────────────────────────────────────────────────

/// A single ability with an independent cooldown timer.
#[derive(Clone, Debug)]
pub struct Ability {
    /// Display name.
    pub name: &'static str,
    /// Full cooldown duration in seconds.
    pub cooldown_max: f32,
    /// Time remaining before the ability is ready again.
    pub cooldown_remaining: f32,
    /// Charge-bar color when ready.
    pub color: Color,
    /// Key label shown to the player.
    pub key_label: &'static str,
}

/// True when the ability has finished charging.
pub fn is_ready(a: &Ability) -> bool {
    a.cooldown_remaining <= 0.0
}

/// Charge fraction: 0.0 = just used, 1.0 = fully charged.
pub fn cooldown_fraction(a: &Ability) -> f32 {
    if a.cooldown_max <= 0.0 {
        return 1.0;
    }
    1.0 - (a.cooldown_remaining / a.cooldown_max).clamp(0.0, 1.0)
}

/// Advance the cooldown timer by `dt` seconds.
pub fn tick_ability(a: &mut Ability, dt: f32) {
    a.cooldown_remaining = (a.cooldown_remaining - dt).max(0.0);
}

/// Attempt to use the ability. Returns `true` and starts the cooldown if ready.
pub fn use_ability(a: &mut Ability) -> bool {
    if !is_ready(a) {
        return false;
    }
    a.cooldown_remaining = a.cooldown_max;
    true
}

// ── ECS ───────────────────────────────────────────────────────────────────────

/// The set of player abilities, indexed by slot.
#[derive(Resource)]
pub struct Abilities(pub [Ability; 3]);

/// Marker on the ability UI slot panel. The slot's index lives on its
/// [`FillBar`] child, which is the part that actually animates.
#[derive(Component)]
struct AbilitySlot;

/// Inner fill bar for each slot.
#[derive(Component)]
struct FillBar(usize);

/// Flash overlay shown when an ability fires.
#[derive(Component)]
struct FlashOverlay {
    timer: f32,
    ability: usize,
}

/// Player sprite used for the ability flash effect.
#[derive(Component)]
pub struct Player;

fn setup(mut commands: Commands, abilities: Res<Abilities>, config: Res<AbilityCooldownsConfig>) {
    commands.spawn(Camera2d);

    // Player sprite centre stage.
    commands.spawn((
        Player,
        Sprite {
            color: config.player_color,
            custom_size: Some(Vec2::splat(36.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, 60.0, 1.0)),
    ));

    // Ability slot panels at the bottom.
    let count = abilities.0.len();
    for (i, ability) in abilities.0.iter().enumerate() {
        let x = config.slot_x(i, count);

        // Background panel.
        commands.spawn((
            AbilitySlot,
            Sprite {
                color: Color::srgb(0.15, 0.15, 0.18),
                custom_size: Some(Vec2::new(config.slot_w, config.slot_h)),
                ..default()
            },
            Transform::from_translation(Vec3::new(x, config.slot_y, 0.0)),
        ));

        // Fill bar (width driven by cooldown_fraction).
        commands.spawn((
            FillBar(i),
            Sprite {
                color: ability.color,
                custom_size: Some(Vec2::new(config.slot_w - 4.0, config.slot_h - 4.0)),
                ..default()
            },
            Transform::from_translation(Vec3::new(x, config.slot_y, 0.5)),
        ));
    }

    commands.spawn((
        Text::new("Q — Dash (1s)   W — Shield (3s)   E — Nova (8s)"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.55)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

fn tick_cooldowns(time: Res<Time>, mut abilities: ResMut<Abilities>) {
    for a in &mut abilities.0 {
        tick_ability(a, time.delta_secs());
    }
}

fn handle_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut abilities: ResMut<Abilities>,
    mut commands: Commands,
    player_q: Query<Entity, With<Player>>,
) {
    let keys_map = [KeyCode::KeyQ, KeyCode::KeyW, KeyCode::KeyE];
    let Ok(player_e) = player_q.single() else {
        return;
    };
    for (i, &key) in keys_map.iter().enumerate() {
        if keys.just_pressed(key) && use_ability(&mut abilities.0[i]) {
            commands.entity(player_e).insert(FlashOverlay {
                timer: 0.18,
                ability: i,
            });
        }
    }
}

fn update_ui(
    abilities: Res<Abilities>,
    config: Res<AbilityCooldownsConfig>,
    mut fill_q: Query<(&FillBar, &mut Sprite, &mut Transform)>,
) {
    let count = abilities.0.len();
    for (bar, mut sprite, mut tf) in &mut fill_q {
        let ability = &abilities.0[bar.0];
        let frac = cooldown_fraction(ability);
        let bar_w = (config.slot_w - 4.0) * frac;
        let x = config.slot_x(bar.0, count);
        // Anchor fill bar to left edge of slot.
        tf.translation.x = x - (config.slot_w - 4.0) / 2.0 + bar_w / 2.0;
        tf.translation.y = config.slot_y;
        sprite.custom_size = Some(Vec2::new(bar_w.max(0.001), config.slot_h - 4.0));
        sprite.color = if is_ready(ability) {
            ability.color
        } else {
            Color::srgb(0.25, 0.25, 0.28)
        };
    }
}

fn tick_flashes(
    time: Res<Time>,
    mut commands: Commands,
    abilities: Res<Abilities>,
    config: Res<AbilityCooldownsConfig>,
    mut q: Query<(Entity, &mut FlashOverlay, &mut Sprite), With<Player>>,
) {
    let Ok((entity, mut flash, mut sprite)) = q.single_mut() else {
        return;
    };
    flash.timer -= time.delta_secs();
    if flash.timer <= 0.0 {
        sprite.color = config.player_color;
        commands.entity(entity).remove::<FlashOverlay>();
    } else {
        sprite.color = abilities.0[flash.ability].color;
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn ability(max: f32, remaining: f32) -> Ability {
        Ability {
            name: "Test",
            cooldown_max: max,
            cooldown_remaining: remaining,
            color: Color::WHITE,
            key_label: "X",
        }
    }

    #[test]
    fn ready_when_remaining_is_zero() {
        assert!(is_ready(&ability(3.0, 0.0)));
    }

    #[test]
    fn not_ready_during_cooldown() {
        assert!(!is_ready(&ability(3.0, 1.5)));
    }

    #[test]
    fn fraction_zero_when_just_used() {
        assert!((cooldown_fraction(&ability(3.0, 3.0)) - 0.0).abs() < 1e-5);
    }

    #[test]
    fn fraction_one_when_ready() {
        assert!((cooldown_fraction(&ability(3.0, 0.0)) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn use_ability_fails_during_cooldown() {
        let mut a = ability(3.0, 1.5);
        assert!(!use_ability(&mut a));
        assert!((a.cooldown_remaining - 1.5).abs() < 1e-5);
    }

    #[test]
    fn use_ability_starts_cooldown_when_ready() {
        let mut a = ability(3.0, 0.0);
        assert!(use_ability(&mut a));
        assert!((a.cooldown_remaining - 3.0).abs() < 1e-5);
    }

    #[test]
    fn tick_ability_reduces_remaining() {
        let mut a = ability(3.0, 2.0);
        tick_ability(&mut a, 0.5);
        assert!((a.cooldown_remaining - 1.5).abs() < 1e-5);
    }

    #[test]
    fn tick_ability_floors_at_zero() {
        let mut a = ability(3.0, 0.2);
        tick_ability(&mut a, 1.0);
        assert_eq!(a.cooldown_remaining, 0.0);
    }

    #[test]
    fn config_default_matches_documented_values() {
        let c = AbilityCooldownsConfig::default();
        assert_eq!(c.slot_w, 160.0);
        assert_eq!(c.slot_h, 80.0);
        assert_eq!(c.gap, 20.0);
    }

    #[test]
    fn slot_x_centres_odd_count() {
        let c = AbilityCooldownsConfig::default();
        // Middle of three slots sits at the origin.
        assert!(c.slot_x(1, 3).abs() < 1e-4);
    }

    #[test]
    fn plugin_spawns_three_fill_bars() {
        // Building-block path: the plugin composes onto a headless app.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AbilityCooldownsPlugin));
        // The plugin's Update systems read key input; MinimalPlugins omits it.
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.update();

        let mut q = app.world_mut().query::<&FillBar>();
        assert_eq!(q.iter(app.world()).count(), 3);
    }
}
