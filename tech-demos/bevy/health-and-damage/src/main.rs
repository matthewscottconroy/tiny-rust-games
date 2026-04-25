//! Health and damage demo.
//!
//! Key ideas:
//! - [`Enemy`] holds HP as a plain component field; damage flows through a
//!   `DamageMessage` so the input system is decoupled from the HP system.
//! - Sprite color lerps from green (full HP) to red (near death) via
//!   [`Enemy::fraction`].
//! - Entity despawns on death; a [`RespawnTimer`] triggers a fresh spawn.
//!
//! **Bug fixed:** on respawn the old health-bar UI was not removed, causing
//! stacked bars to accumulate.  `HealthBarRoot` now tags the bar container so
//! `tick_respawn` can despawn it before calling `spawn_enemy`.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_message::<DamageMessage>()
        .init_resource::<RespawnTimer>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (handle_input, apply_damage, update_health_color,
             update_health_bar, update_hud, tick_respawn),
        )
        .run();
}

// --- Message ---

/// Carries a damage request targeting a specific entity.
#[derive(Message)]
struct DamageMessage {
    target: Entity,
    amount: f32,
}

// --- Components ---

/// Holds the current and maximum hit points of an enemy.
#[derive(Component)]
struct Enemy {
    hp: f32,
    max_hp: f32,
}

impl Enemy {
    /// Creates a new enemy at full health.
    pub fn new(max_hp: f32) -> Self {
        Self { hp: max_hp, max_hp }
    }

    /// Returns HP as a fraction in `[0.0, 1.0]`.
    pub fn fraction(&self) -> f32 {
        (self.hp / self.max_hp).clamp(0.0, 1.0)
    }
}

/// Marker for the health bar fill node (the colored inner bar).
#[derive(Component)]
struct HealthFill;

/// Marker for the outer health bar container so it can be despawned on respawn.
#[derive(Component)]
struct HealthBarRoot;

/// Marker for the HP text label.
#[derive(Component)]
struct HudText;

// --- Resource ---

/// Optional countdown before a new enemy is spawned after death.
#[derive(Resource, Default)]
struct RespawnTimer(Option<Timer>);

// --- Setup ---

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    spawn_enemy(&mut commands);
    spawn_hud(&mut commands);
}

/// Spawns the enemy sprite and its health-bar UI.
fn spawn_enemy(commands: &mut Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                left: Val::Px(12.0),
                width: Val::Px(200.0),
                height: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
            HealthBarRoot,
        ))
        .with_children(|p| {
            p.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.8, 0.2)),
                HealthFill,
            ));
        });

    commands.spawn((
        Sprite {
            color: Color::srgb(0.2, 0.8, 0.2),
            custom_size: Some(Vec2::splat(64.0)),
            ..default()
        },
        Transform::default(),
        Enemy::new(100.0),
    ));
}

/// Spawns the static HUD elements (HP label and instructions).
fn spawn_hud(commands: &mut Commands) {
    commands.spawn((
        Text::new("HP: 100 / 100"),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(34.0),
            left: Val::Px(12.0),
            ..default()
        },
        HudText,
    ));

    commands.spawn((
        Text::new("SPACE = deal 10 damage   R = reset health"),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgb(0.6, 0.6, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

// --- Systems ---

/// Sends [`DamageMessage`] on SPACE; resets HP directly on R.
fn handle_input(
    input: Res<ButtonInput<KeyCode>>,
    mut writer: MessageWriter<DamageMessage>,
    enemy_query: Query<Entity, With<Enemy>>,
    mut enemy_hp_query: Query<&mut Enemy>,
) {
    if input.just_pressed(KeyCode::KeyR) {
        for mut enemy in &mut enemy_hp_query {
            enemy.hp = enemy.max_hp;
        }
        return;
    }

    if input.just_pressed(KeyCode::Space) {
        for entity in &enemy_query {
            writer.write(DamageMessage { target: entity, amount: 10.0 });
        }
    }
}

/// Reads [`DamageMessage`] and decrements HP; schedules respawn on death.
fn apply_damage(
    mut reader: MessageReader<DamageMessage>,
    mut commands: Commands,
    mut query: Query<&mut Enemy>,
    mut respawn: ResMut<RespawnTimer>,
) {
    for msg in reader.read() {
        let Ok(mut enemy) = query.get_mut(msg.target) else { continue; };
        enemy.hp = (enemy.hp - msg.amount).max(0.0);
        if enemy.hp <= 0.0 {
            commands.entity(msg.target).despawn();
            respawn.0 = Some(Timer::from_seconds(1.2, TimerMode::Once));
        }
    }
}

/// Tints the enemy sprite from green (full HP) to red (empty).
fn update_health_color(mut query: Query<(&Enemy, &mut Sprite)>) {
    for (enemy, mut sprite) in &mut query {
        let f = enemy.fraction();
        sprite.color = Color::srgb(1.0 - f, f * 0.8, 0.05);
    }
}

/// Shrinks and recolors the health bar fill to match current HP.
fn update_health_bar(
    enemy_query: Query<&Enemy>,
    mut fill_query: Query<(&mut Node, &mut BackgroundColor), With<HealthFill>>,
) {
    let pct = enemy_query
        .iter()
        .next()
        .map(|e| e.fraction() * 100.0)
        .unwrap_or(0.0);

    for (mut node, mut bg) in &mut fill_query {
        node.width = Val::Percent(pct);
        let f = pct / 100.0;
        *bg = BackgroundColor(Color::srgb(1.0 - f, f * 0.8, 0.05));
    }
}

/// Updates the HP text label.
fn update_hud(
    enemy_query: Query<&Enemy>,
    mut hud_query: Query<&mut Text, With<HudText>>,
) {
    let (hp, max) = enemy_query
        .iter()
        .next()
        .map(|e| (e.hp as u32, e.max_hp as u32))
        .unwrap_or((0, 100));

    for mut text in &mut hud_query {
        *text = Text::new(format!("HP: {} / {}", hp, max));
    }
}

/// Ticks the respawn countdown; despawns the old health bar and spawns a
/// fresh enemy when the timer fires.
fn tick_respawn(
    mut commands: Commands,
    time: Res<Time>,
    mut respawn: ResMut<RespawnTimer>,
    bar_query: Query<Entity, With<HealthBarRoot>>,
) {
    let Some(timer) = respawn.0.as_mut() else { return; };
    if timer.tick(time.delta()).just_finished() {
        respawn.0 = None;
        // Despawn the old health bar before spawning a new one.
        for entity in &bar_query {
            commands.entity(entity).despawn();
        }
        spawn_enemy(&mut commands);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Enemy unit tests ---

    #[test]
    fn enemy_fraction_at_full_health() {
        let e = Enemy::new(100.0);
        assert_eq!(e.fraction(), 1.0);
    }

    #[test]
    fn enemy_fraction_at_zero_health() {
        let mut e = Enemy::new(100.0);
        e.hp = 0.0;
        assert_eq!(e.fraction(), 0.0);
    }

    #[test]
    fn enemy_fraction_at_half_health() {
        let mut e = Enemy::new(100.0);
        e.hp = 50.0;
        assert!((e.fraction() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn enemy_fraction_clamped_above_max() {
        let mut e = Enemy::new(100.0);
        e.hp = 200.0; // over-healed
        assert_eq!(e.fraction(), 1.0);
    }

    #[test]
    fn enemy_fraction_clamped_below_zero() {
        let mut e = Enemy::new(100.0);
        e.hp = -50.0;
        assert_eq!(e.fraction(), 0.0);
    }

    #[test]
    fn enemy_new_starts_at_full_hp() {
        let e = Enemy::new(75.0);
        assert_eq!(e.hp, 75.0);
        assert_eq!(e.max_hp, 75.0);
    }

    // --- ECS setup test ---

    #[test]
    fn setup_spawns_one_enemy() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<DamageMessage>()
            .init_resource::<RespawnTimer>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Enemy>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn setup_spawns_one_health_bar_root() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<DamageMessage>()
            .init_resource::<RespawnTimer>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&HealthBarRoot>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }
}
