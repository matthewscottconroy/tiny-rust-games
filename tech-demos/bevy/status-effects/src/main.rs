//! Status Effects — time-decaying debuffs (Poison, Burn, Slow, Stun) applied to an enemy.
//!
//! Key ideas:
//! - Each active effect is a plain struct with a remaining duration and strength.
//! - `tick_effect` reduces the timer and returns `None` when the effect expires.
//! - `total_dot_dps` and `speed_multiplier` aggregate all active effects into
//!   a scalar damage rate and movement modifier — both pure and testable.
//! - The enemy's behaviour is entirely driven by these aggregated values each frame.
//!
//! **Controls:** WASD / Arrows — move player   Q — Poison   W — Slow   E — Burn   R — Stun

use bevy::prelude::*;
use std::fmt;

const WINDOW_W: f32 = 800.0;
const WINDOW_H: f32 = 500.0;
const PLAYER_SPEED: f32 = 160.0;
const ENEMY_BASE_SPEED: f32 = 80.0;
const APPLY_RANGE: f32 = 120.0;
const EFFECT_COOLDOWN: f32 = 0.25;

// ── Pure effect model ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EffectKind { Poison, Burn, Slow, Stun }

impl fmt::Display for EffectKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EffectKind::Poison => write!(f, "Poison"),
            EffectKind::Burn   => write!(f, "Burn"),
            EffectKind::Slow   => write!(f, "Slow"),
            EffectKind::Stun   => write!(f, "Stun"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Effect {
    pub kind: EffectKind,
    /// Seconds remaining.
    pub remaining: f32,
    /// Damage-per-second for DoT kinds; slow fraction for Slow.
    pub strength: f32,
}

/// Advance the effect timer. Returns `None` when the effect expires.
pub fn tick_effect(e: &Effect, dt: f32) -> Option<Effect> {
    let r = e.remaining - dt;
    if r <= 0.0 { None } else { Some(Effect { remaining: r, ..*e }) }
}

/// Sum DoT damage-per-second from all active effects.
pub fn total_dot_dps(effects: &[Effect]) -> f32 {
    effects.iter().map(|e| match e.kind {
        EffectKind::Poison => e.strength,
        EffectKind::Burn   => e.strength * 2.5,
        _                  => 0.0,
    }).sum()
}

/// Movement speed multiplier from 1.0 (full) down to 0.0 (frozen).
pub fn speed_multiplier(effects: &[Effect]) -> f32 {
    if effects.iter().any(|e| e.kind == EffectKind::Stun) { return 0.0; }
    let slow = effects.iter()
        .filter(|e| e.kind == EffectKind::Slow)
        .map(|e| e.strength)
        .fold(0.0f32, f32::max);
    (1.0 - slow).max(0.0)
}

// ── ECS ───────────────────────────────────────────────────────────────────────

#[derive(Component)]
struct Player { apply_cd: f32 }

#[derive(Component)]
struct Enemy {
    hp: f32,
    max_hp: f32,
    effects: Vec<Effect>,
    vel: Vec2,
    wander_timer: f32,
}

#[derive(Component)]
struct HudText;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Status Effects".into(),
                resolution: (800u32, 500u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (move_player, apply_effects, tick_enemy, update_hud).chain())
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Player { apply_cd: 0.0 },
        Sprite { color: Color::srgb(0.3, 0.6, 1.0), custom_size: Some(Vec2::splat(20.0)), ..default() },
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
        Sprite { color: Color::srgb(0.8, 0.25, 0.25), custom_size: Some(Vec2::splat(28.0)), ..default() },
        Transform::from_translation(Vec3::new(150.0, 60.0, 1.0)),
    ));

    commands.spawn((
        HudText,
        Text::new(""),
        TextFont { font_size: 15.0, ..default() },
        TextColor(Color::WHITE),
        Node { position_type: PositionType::Absolute, top: Val::Px(10.0), left: Val::Px(10.0), ..default() },
    ));

    commands.spawn((
        Text::new("WASD/Arrows — move   Q Poison   W Slow   E Burn   R Stun   (get close first)"),
        TextFont { font_size: 13.0, ..default() },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.5)),
        Node { position_type: PositionType::Absolute, bottom: Val::Px(10.0), left: Val::Px(10.0), ..default() },
    ));
}

fn move_player(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut q: Query<(&mut Player, &mut Transform)>,
) {
    let Ok((mut player, mut tf)) = q.single_mut() else { return };
    player.apply_cd = (player.apply_cd - time.delta_secs()).max(0.0);
    let mut dir = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp)    { dir.y += 1.0; }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown)  { dir.y -= 1.0; }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft)  { dir.x -= 1.0; }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) { dir.x += 1.0; }
    if dir == Vec2::ZERO { return; }
    tf.translation += (dir.normalize() * PLAYER_SPEED * time.delta_secs()).extend(0.0);
    tf.translation.x = tf.translation.x.clamp(-WINDOW_W / 2.0 + 14.0, WINDOW_W / 2.0 - 14.0);
    tf.translation.y = tf.translation.y.clamp(-WINDOW_H / 2.0 + 14.0, WINDOW_H / 2.0 - 14.0);
}

fn apply_effects(
    keys: Res<ButtonInput<KeyCode>>,
    mut player_q: Query<(&mut Player, &Transform)>,
    mut enemy_q: Query<(&mut Enemy, &Transform), Without<Player>>,
) {
    let Ok((mut player, ptf)) = player_q.single_mut() else { return };
    let Ok((mut enemy, etf)) = enemy_q.single_mut() else { return };
    if player.apply_cd > 0.0 { return; }
    let dist = ptf.translation.distance(etf.translation);
    if dist > APPLY_RANGE { return; }

    let new_effect = if keys.just_pressed(KeyCode::KeyQ) {
        Some(Effect { kind: EffectKind::Poison, remaining: 5.0, strength: 10.0 })
    } else if keys.just_pressed(KeyCode::KeyW) {
        Some(Effect { kind: EffectKind::Slow, remaining: 4.0, strength: 0.65 })
    } else if keys.just_pressed(KeyCode::KeyE) {
        Some(Effect { kind: EffectKind::Burn, remaining: 2.5, strength: 14.0 })
    } else if keys.just_pressed(KeyCode::KeyR) {
        Some(Effect { kind: EffectKind::Stun, remaining: 1.8, strength: 1.0 })
    } else {
        None
    };

    if let Some(effect) = new_effect {
        enemy.effects.push(effect);
        player.apply_cd = EFFECT_COOLDOWN;
    }
}

fn tick_enemy(
    time: Res<Time>,
    mut q: Query<(&mut Enemy, &mut Transform, &mut Sprite)>,
) {
    let Ok((mut enemy, mut tf, mut sprite)) = q.single_mut() else { return };
    if enemy.hp <= 0.0 { return; }
    let dt = time.delta_secs();

    // Tick effects and remove expired.
    enemy.effects = enemy.effects.iter()
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
            (tf.translation.x.sin() * 73.1) as f32 % 80.0 - 40.0,
            (tf.translation.y.cos() * 51.7) as f32 % 80.0 - 40.0,
        ).normalize_or_zero() * ENEMY_BASE_SPEED;
        enemy.wander_timer = 1.5 + (tf.translation.x.abs() % 1.0);
    }
    tf.translation += (enemy.vel * speed_mod * dt).extend(0.0);
    if tf.translation.x.abs() > WINDOW_W / 2.0 - 20.0 { enemy.vel.x *= -1.0; }
    if tf.translation.y.abs() > WINDOW_H / 2.0 - 20.0 { enemy.vel.y *= -1.0; }
    tf.translation.x = tf.translation.x.clamp(-WINDOW_W / 2.0 + 20.0, WINDOW_W / 2.0 - 20.0);
    tf.translation.y = tf.translation.y.clamp(-WINDOW_H / 2.0 + 20.0, WINDOW_H / 2.0 - 20.0);

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
    enemy_q: Query<&Enemy>,
    player_q: Query<(&Player, &Transform)>,
    enemy_tf_q: Query<&Transform, (With<Enemy>, Without<Player>)>,
    mut text_q: Query<&mut Text, With<HudText>>,
) {
    let Ok(enemy) = enemy_q.single() else { return };
    let Ok((player, ptf)) = player_q.single() else { return };
    let Ok(etf) = enemy_tf_q.single() else { return };
    let Ok(mut text) = text_q.single_mut() else { return };

    let hp_bar = {
        let filled = ((enemy.hp / enemy.max_hp) * 20.0) as usize;
        let empty = 20usize.saturating_sub(filled);
        format!("[{}{}]", "#".repeat(filled), "-".repeat(empty))
    };
    let dps = total_dot_dps(&enemy.effects);
    let spd = speed_multiplier(&enemy.effects);
    let dist = ptf.translation.distance(etf.translation);
    let range_note = if dist <= APPLY_RANGE { "(in range)" } else { "(too far)" };

    let effect_lines: String = enemy.effects.iter()
        .map(|e| format!("  {} {:.1}s\n", e.kind, e.remaining))
        .collect();

    text.0 = format!(
        "Enemy HP: {hp_bar} {:.0}\nDoT DPS: {dps:.1}   Speed: {spd:.0}%\nEffects:\n{effects}Player distance: {dist:.0} {range_note}",
        enemy.hp,
        dps = dps,
        spd = spd * 100.0,
        effects = if effect_lines.is_empty() { "  (none)\n".to_string() } else { effect_lines },
        dist = dist,
        range_note = range_note,
    );
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn poison(dur: f32) -> Effect { Effect { kind: EffectKind::Poison, remaining: dur, strength: 10.0 } }
    fn slow(frac: f32) -> Effect { Effect { kind: EffectKind::Slow, remaining: 3.0, strength: frac } }
    fn stun() -> Effect { Effect { kind: EffectKind::Stun, remaining: 2.0, strength: 1.0 } }
    fn burn(dur: f32) -> Effect { Effect { kind: EffectKind::Burn, remaining: dur, strength: 8.0 } }

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
}
