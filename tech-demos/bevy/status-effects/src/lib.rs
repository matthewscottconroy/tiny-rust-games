//! Status Effects — a reusable Bevy plugin for time-decaying debuffs.
//!
//! This crate is a *building block*: drop [`StatusEffectsPlugin`] into any Bevy
//! app with `app.add_plugins(StatusEffectsPlugin)` and it spawns a player that
//! applies time-decaying debuffs (Poison, Burn, Slow, Stun) to an enemy.
//!
//! Key ideas:
//! - Each active [`Effect`] is a plain struct with a remaining duration and
//!   strength.
//! - [`tick_effect`] reduces the timer and returns `None` when the effect
//!   expires.
//! - [`total_dot_dps`] and [`speed_multiplier`] aggregate all active effects
//!   into a scalar damage rate and movement modifier — both pure and testable.
//! - The enemy's behaviour is entirely driven by these aggregated values each
//!   frame.
//! - Tune arena size, speeds, range, and cooldown through the
//!   [`StatusEffectsConfig`] resource.
//!
//! **Controls:** WASD / Arrows — move player   Q — Poison   W — Slow   E — Burn   R — Stun
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use status_effects::StatusEffectsPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(StatusEffectsPlugin)
//!     .run();
//! ```

use bevy::prelude::*;
use std::fmt;

/// Bundles every system and resource for the status-effects feature.
///
/// Add it with `app.add_plugins(StatusEffectsPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct StatusEffectsPlugin;

impl Plugin for StatusEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StatusEffectsConfig>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (move_player, apply_effects, tick_enemy, update_hud).chain(),
            );
    }
}

// --- Configuration ---

/// Tunable parameters for the feature. Override before adding the plugin, e.g.
/// `app.insert_resource(StatusEffectsConfig { apply_range: 200.0, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct StatusEffectsConfig {
    /// Arena width, in pixels (used for movement clamping).
    pub window_w: f32,
    /// Arena height, in pixels (used for movement clamping).
    pub window_h: f32,
    /// Player movement speed in pixels per second.
    pub player_speed: f32,
    /// Enemy base movement speed in pixels per second.
    pub enemy_base_speed: f32,
    /// Maximum distance at which the player can apply an effect.
    pub apply_range: f32,
    /// Cooldown, in seconds, between applying effects.
    pub effect_cooldown: f32,
}

impl Default for StatusEffectsConfig {
    fn default() -> Self {
        Self {
            window_w: 800.0,
            window_h: 500.0,
            player_speed: 160.0,
            enemy_base_speed: 80.0,
            apply_range: 120.0,
            effect_cooldown: 0.25,
        }
    }
}

// ── Pure effect model ─────────────────────────────────────────────────────────

/// The four status effects a target can be under.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EffectKind {
    /// Steady damage over time.
    Poison,
    /// Damage over time at 2.5x the poison rate.
    Burn,
    /// Reduces movement speed by `strength`, in `0.0..=1.0`.
    Slow,
    /// Stops movement entirely while active.
    Stun,
}

impl fmt::Display for EffectKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EffectKind::Poison => write!(f, "Poison"),
            EffectKind::Burn => write!(f, "Burn"),
            EffectKind::Slow => write!(f, "Slow"),
            EffectKind::Stun => write!(f, "Stun"),
        }
    }
}

/// One active status effect on a target.
#[derive(Clone, Debug)]
pub struct Effect {
    /// Which effect this is.
    pub kind: EffectKind,
    /// Seconds remaining.
    pub remaining: f32,
    /// Damage-per-second for DoT kinds; slow fraction for Slow.
    pub strength: f32,
}

/// Advance the effect timer. Returns `None` when the effect expires.
pub fn tick_effect(e: &Effect, dt: f32) -> Option<Effect> {
    let r = e.remaining - dt;
    if r <= 0.0 {
        None
    } else {
        Some(Effect { remaining: r, ..*e })
    }
}

/// Sum DoT damage-per-second from all active effects.
pub fn total_dot_dps(effects: &[Effect]) -> f32 {
    effects
        .iter()
        .map(|e| match e.kind {
            EffectKind::Poison => e.strength,
            EffectKind::Burn => e.strength * 2.5,
            _ => 0.0,
        })
        .sum()
}

/// Movement speed multiplier from 1.0 (full) down to 0.0 (frozen).
pub fn speed_multiplier(effects: &[Effect]) -> f32 {
    if effects.iter().any(|e| e.kind == EffectKind::Stun) {
        return 0.0;
    }
    let slow = effects
        .iter()
        .filter(|e| e.kind == EffectKind::Slow)
        .map(|e| e.strength)
        .fold(0.0f32, f32::max);
    (1.0 - slow).max(0.0)
}

// ── ECS ───────────────────────────────────────────────────────────────────────

/// Marks the player, who periodically applies effects to nearby enemies.
#[derive(Component)]
pub struct Player {
    /// Seconds until the player may apply another effect.
    pub apply_cd: f32,
}

/// An enemy that accumulates status effects and wanders the arena.
#[derive(Component)]
pub struct Enemy {
    /// Current hit points; the enemy despawns at zero.
    pub hp: f32,
    /// Starting hit points, used to scale the health bar.
    pub max_hp: f32,
    /// Every effect currently active on this enemy.
    pub effects: Vec<Effect>,
    /// Current wander velocity, in pixels per second.
    pub vel: Vec2,
    /// Seconds until a new wander direction is chosen.
    pub wander_timer: f32,
}

/// Marks the heads-up display text entity.
#[derive(Component)]
pub struct HudText;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Player { apply_cd: 0.0 },
        Sprite {
            color: Color::srgb(0.3, 0.6, 1.0),
            custom_size: Some(Vec2::splat(20.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(-200.0, 0.0, 1.0)),
    ));

    commands.spawn((
        Enemy {
            hp: 100.0,
            max_hp: 100.0,
            effects: Vec::new(),
            vel: Vec2::new(40.0, 30.0),
            wander_timer: 2.0,
        },
        Sprite {
            color: Color::srgb(0.8, 0.25, 0.25),
            custom_size: Some(Vec2::splat(28.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(150.0, 60.0, 1.0)),
    ));

    commands.spawn((
        HudText,
        Text::new(""),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));

    commands.spawn((
        Text::new("WASD/Arrows - move   Q Poison   W Slow   E Burn   R Stun   (get close first)"),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.5)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

fn move_player(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    config: Res<StatusEffectsConfig>,
    mut q: Query<(&mut Player, &mut Transform)>,
) {
    let Ok((mut player, mut tf)) = q.single_mut() else {
        return;
    };
    player.apply_cd = (player.apply_cd - time.delta_secs()).max(0.0);
    let mut dir = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        dir.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        dir.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        dir.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        dir.x += 1.0;
    }
    if dir == Vec2::ZERO {
        return;
    }
    tf.translation += (dir.normalize() * config.player_speed * time.delta_secs()).extend(0.0);
    tf.translation.x = tf
        .translation
        .x
        .clamp(-config.window_w / 2.0 + 14.0, config.window_w / 2.0 - 14.0);
    tf.translation.y = tf
        .translation
        .y
        .clamp(-config.window_h / 2.0 + 14.0, config.window_h / 2.0 - 14.0);
}

fn apply_effects(
    keys: Res<ButtonInput<KeyCode>>,
    config: Res<StatusEffectsConfig>,
    mut player_q: Query<(&mut Player, &Transform)>,
    mut enemy_q: Query<(&mut Enemy, &Transform), Without<Player>>,
) {
    let Ok((mut player, ptf)) = player_q.single_mut() else {
        return;
    };
    let Ok((mut enemy, etf)) = enemy_q.single_mut() else {
        return;
    };
    if player.apply_cd > 0.0 {
        return;
    }
    let dist = ptf.translation.distance(etf.translation);
    if dist > config.apply_range {
        return;
    }

    let new_effect = if keys.just_pressed(KeyCode::KeyQ) {
        Some(Effect {
            kind: EffectKind::Poison,
            remaining: 5.0,
            strength: 10.0,
        })
    } else if keys.just_pressed(KeyCode::KeyW) {
        Some(Effect {
            kind: EffectKind::Slow,
            remaining: 4.0,
            strength: 0.65,
        })
    } else if keys.just_pressed(KeyCode::KeyE) {
        Some(Effect {
            kind: EffectKind::Burn,
            remaining: 2.5,
            strength: 14.0,
        })
    } else if keys.just_pressed(KeyCode::KeyR) {
        Some(Effect {
            kind: EffectKind::Stun,
            remaining: 1.8,
            strength: 1.0,
        })
    } else {
        None
    };

    if let Some(effect) = new_effect {
        enemy.effects.push(effect);
        player.apply_cd = config.effect_cooldown;
    }
}

fn tick_enemy(
    time: Res<Time>,
    config: Res<StatusEffectsConfig>,
    mut q: Query<(&mut Enemy, &mut Transform, &mut Sprite)>,
) {
    let Ok((mut enemy, mut tf, mut sprite)) = q.single_mut() else {
        return;
    };
    if enemy.hp <= 0.0 {
        return;
    }
    let dt = time.delta_secs();

    // Tick effects and remove expired.
    enemy.effects = enemy
        .effects
        .iter()
        .filter_map(|e| tick_effect(e, dt))
        .collect();

    // Apply DoT damage.
    let dps = total_dot_dps(&enemy.effects);
    enemy.hp = (enemy.hp - dps * dt).max(0.0);

    // Move with wander + speed multiplier.
    let speed_mod = speed_multiplier(&enemy.effects);
    enemy.wander_timer -= dt;
    if enemy.wander_timer <= 0.0 {
        enemy.vel = Vec2::new(
            (tf.translation.x.sin() * 73.1) % 80.0 - 40.0,
            (tf.translation.y.cos() * 51.7) % 80.0 - 40.0,
        )
        .normalize_or_zero()
            * config.enemy_base_speed;
        enemy.wander_timer = 1.5 + (tf.translation.x.abs() % 1.0);
    }
    tf.translation += (enemy.vel * speed_mod * dt).extend(0.0);
    if tf.translation.x.abs() > config.window_w / 2.0 - 20.0 {
        enemy.vel.x *= -1.0;
    }
    if tf.translation.y.abs() > config.window_h / 2.0 - 20.0 {
        enemy.vel.y *= -1.0;
    }
    tf.translation.x = tf
        .translation
        .x
        .clamp(-config.window_w / 2.0 + 20.0, config.window_w / 2.0 - 20.0);
    tf.translation.y = tf
        .translation
        .y
        .clamp(-config.window_h / 2.0 + 20.0, config.window_h / 2.0 - 20.0);

    // Tint by dominant active effect.
    sprite.color = if enemy.effects.iter().any(|e| e.kind == EffectKind::Stun) {
        Color::srgb(0.6, 0.6, 1.0)
    } else if enemy.effects.iter().any(|e| e.kind == EffectKind::Burn) {
        Color::srgb(1.0, 0.45, 0.1)
    } else if enemy.effects.iter().any(|e| e.kind == EffectKind::Poison) {
        Color::srgb(0.35, 0.85, 0.35)
    } else if enemy.effects.iter().any(|e| e.kind == EffectKind::Slow) {
        Color::srgb(0.5, 0.5, 0.9)
    } else {
        Color::srgb(0.8, 0.25, 0.25)
    };
}

fn update_hud(
    config: Res<StatusEffectsConfig>,
    enemy_q: Query<&Enemy>,
    player_q: Query<(&Player, &Transform)>,
    enemy_tf_q: Query<&Transform, (With<Enemy>, Without<Player>)>,
    mut text_q: Query<&mut Text, With<HudText>>,
) {
    let Ok(enemy) = enemy_q.single() else { return };
    let Ok((_player, ptf)) = player_q.single() else {
        return;
    };
    let Ok(etf) = enemy_tf_q.single() else { return };
    let Ok(mut text) = text_q.single_mut() else {
        return;
    };

    let hp_bar = {
        let filled = ((enemy.hp / enemy.max_hp) * 20.0) as usize;
        let empty = 20usize.saturating_sub(filled);
        format!("[{}{}]", "#".repeat(filled), "-".repeat(empty))
    };
    let dps = total_dot_dps(&enemy.effects);
    let spd = speed_multiplier(&enemy.effects);
    let dist = ptf.translation.distance(etf.translation);
    let range_note = if dist <= config.apply_range {
        "(in range)"
    } else {
        "(too far)"
    };

    let effect_lines: String = enemy
        .effects
        .iter()
        .map(|e| format!("  {} {:.1}s\n", e.kind, e.remaining))
        .collect();

    text.0 = format!(
        "Enemy HP: {hp_bar} {:.0}\nDoT DPS: {dps:.1}   Speed: {spd:.0}%\nEffects:\n{effects}Player distance: {dist:.0} {range_note}",
        enemy.hp,
        dps = dps,
        spd = spd * 100.0,
        effects = if effect_lines.is_empty() {
            "  (none)\n".to_string()
        } else {
            effect_lines
        },
        dist = dist,
        range_note = range_note,
    );
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn poison(dur: f32) -> Effect {
        Effect {
            kind: EffectKind::Poison,
            remaining: dur,
            strength: 10.0,
        }
    }
    fn slow(frac: f32) -> Effect {
        Effect {
            kind: EffectKind::Slow,
            remaining: 3.0,
            strength: frac,
        }
    }
    fn stun() -> Effect {
        Effect {
            kind: EffectKind::Stun,
            remaining: 2.0,
            strength: 1.0,
        }
    }
    fn burn(dur: f32) -> Effect {
        Effect {
            kind: EffectKind::Burn,
            remaining: dur,
            strength: 8.0,
        }
    }

    #[test]
    fn effect_expires_when_timer_hits_zero() {
        let e = poison(0.05);
        assert!(tick_effect(&e, 0.05).is_none());
        assert!(tick_effect(&e, 0.06).is_none());
    }

    #[test]
    fn effect_survives_partial_tick() {
        let e = poison(2.0);
        let ticked = tick_effect(&e, 1.0).expect("should survive");
        assert!((ticked.remaining - 1.0).abs() < 1e-5);
    }

    #[test]
    fn burn_deals_more_dps_than_poison() {
        let effects = vec![burn(3.0)];
        assert!(total_dot_dps(&effects) > 10.0); // burn strength 8.0 × 2.5 = 20
    }

    #[test]
    fn stun_overrides_slow_to_zero_speed() {
        let effects = vec![slow(0.3), stun()];
        assert_eq!(speed_multiplier(&effects), 0.0);
    }

    #[test]
    fn slow_reduces_speed_proportionally() {
        let effects = vec![slow(0.4)];
        let spd = speed_multiplier(&effects);
        assert!((spd - 0.6).abs() < 1e-5);
    }

    #[test]
    fn no_effects_gives_full_speed() {
        assert!((speed_multiplier(&[]) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn config_default_matches_documented_values() {
        let c = StatusEffectsConfig::default();
        assert_eq!(c.window_w, 800.0);
        assert_eq!(c.window_h, 500.0);
        assert_eq!(c.apply_range, 120.0);
    }

    // --- ECS ---

    #[test]
    fn plugin_spawns_one_enemy() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatusEffectsPlugin));
        // The plugin's Update systems read input; MinimalPlugins omits it.
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.update();

        let mut q = app.world_mut().query::<&Enemy>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }
}
