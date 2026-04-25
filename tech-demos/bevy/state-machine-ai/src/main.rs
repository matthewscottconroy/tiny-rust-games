//! Per-entity state machine AI demo.
//!
//! Key ideas:
//! - [`EnemyState`] is a **per-entity** component enum — distinct from Bevy's
//!   global app `States`.  Per-entity FSMs live in components, not
//!   `NextState<T>`.
//! - Transitions are driven by distance thresholds checked every frame.
//! - Each state runs different behavior in the same system, branching on the
//!   enum variant.
//!
//! Enemy behavior:
//! - **Patrol** — bounces between two waypoints
//! - **Chase**  — steers toward the player when within `DETECT_RADIUS`
//! - **Attack** — pulses red when within `ATTACK_RADIUS`; stops moving

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (move_player, update_enemy_state, run_enemy_behavior, update_hud))
        .run();
}

// --- Components ---

/// Marks the player entity.
#[derive(Component)]
struct Player;

/// Per-entity enemy data: patrol waypoints and attack pulse state.
#[derive(Component)]
struct Enemy {
    patrol_a: Vec2,
    patrol_b: Vec2,
    /// Index into `[patrol_a, patrol_b]` of the current destination.
    patrol_target: usize,
    /// Accumulated time used to animate the attack pulse.
    pulse_timer: f32,
}

/// Per-entity FSM state.  Transitions are computed from player distance.
#[derive(Component, Debug, PartialEq, Clone, Copy)]
enum EnemyState {
    Patrol,
    Chase,
    Attack,
}

/// Marks the state label HUD text.
#[derive(Component)]
struct HudText;

// --- Constants ---

const PLAYER_SPEED: f32 = 160.0;
const ENEMY_PATROL_SPEED: f32 = 70.0;
const ENEMY_CHASE_SPEED: f32 = 110.0;

/// Distance at which the enemy enters [`EnemyState::Chase`].
const DETECT_RADIUS: f32 = 160.0;

/// Distance at which the enemy enters [`EnemyState::Attack`].
const ATTACK_RADIUS: f32 = 50.0;

// --- Setup ---

/// Spawns radius indicators, the enemy, the player, and HUD labels.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Detection radius indicator
    commands.spawn((
        Sprite {
            color: Color::srgba(0.9, 0.3, 0.1, 0.08),
            custom_size: Some(Vec2::splat(DETECT_RADIUS * 2.0)),
            ..default()
        },
        Transform::from_xyz(120.0, 40.0, -1.0),
    ));

    // Attack radius indicator
    commands.spawn((
        Sprite {
            color: Color::srgba(1.0, 0.1, 0.1, 0.12),
            custom_size: Some(Vec2::splat(ATTACK_RADIUS * 2.0)),
            ..default()
        },
        Transform::from_xyz(120.0, 40.0, -1.0),
    ));

    commands.spawn((
        Sprite {
            color: Color::srgb(0.5, 0.5, 0.5),
            custom_size: Some(Vec2::splat(28.0)),
            ..default()
        },
        Transform::from_xyz(120.0, 40.0, 0.0),
        Enemy {
            patrol_a: Vec2::new(-80.0, 40.0),
            patrol_b: Vec2::new(200.0, 40.0),
            patrol_target: 1,
            pulse_timer: 0.0,
        },
        EnemyState::Patrol,
    ));

    commands.spawn((
        Sprite {
            color: Color::srgb(0.3, 0.8, 0.4),
            custom_size: Some(Vec2::splat(24.0)),
            ..default()
        },
        Transform::default(),
        Player,
    ));

    commands.spawn((
        Text::new("State: Patrol"),
        TextFont { font_size: 22.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        HudText,
    ));

    commands.spawn((
        Text::new("WASD — move player   approach the grey enemy"),
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

// --- Pure state selection ---

/// Selects the correct [`EnemyState`] for a given enemy–player distance.
///
/// * `dist < ATTACK_RADIUS` → [`EnemyState::Attack`]
/// * `dist < DETECT_RADIUS` → [`EnemyState::Chase`]
/// * otherwise               → [`EnemyState::Patrol`]
pub fn select_state(dist: f32) -> EnemyState {
    if dist < ATTACK_RADIUS {
        EnemyState::Attack
    } else if dist < DETECT_RADIUS {
        EnemyState::Chase
    } else {
        EnemyState::Patrol
    }
}

// --- Systems ---

/// Reads WASD input and moves the player.
fn move_player(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    let Ok(mut t) = query.single_mut() else { return; };
    let mut dir = Vec2::ZERO;
    if input.pressed(KeyCode::KeyW) { dir.y += 1.0; }
    if input.pressed(KeyCode::KeyS) { dir.y -= 1.0; }
    if input.pressed(KeyCode::KeyA) { dir.x -= 1.0; }
    if input.pressed(KeyCode::KeyD) { dir.x += 1.0; }
    if dir != Vec2::ZERO {
        let d = dir.normalize() * PLAYER_SPEED * time.delta_secs();
        t.translation += d.extend(0.0);
    }
}

/// Recomputes each enemy's [`EnemyState`] from current player distance.
fn update_enemy_state(
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemy_query: Query<(&Transform, &mut EnemyState), With<Enemy>>,
) {
    let Ok(player) = player_query.single() else { return; };
    let player_pos = player.translation.truncate();

    for (transform, mut state) in &mut enemy_query {
        let dist = transform.translation.truncate().distance(player_pos);
        *state = select_state(dist);
    }
}

/// Runs per-state behavior: patrol movement, chase steering, or attack pulse.
fn run_enemy_behavior(
    time: Res<Time>,
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemy_query: Query<(&mut Transform, &mut Sprite, &mut Enemy, &EnemyState)>,
) {
    let Ok(player) = player_query.single() else { return; };
    let player_pos = player.translation.truncate();

    for (mut transform, mut sprite, mut enemy, state) in &mut enemy_query {
        let pos = transform.translation.truncate();

        match state {
            EnemyState::Patrol => {
                let target = if enemy.patrol_target == 0 { enemy.patrol_a } else { enemy.patrol_b };
                let dir = (target - pos).normalize_or_zero();
                let move_delta = dir * ENEMY_PATROL_SPEED * time.delta_secs();
                transform.translation.x += move_delta.x;
                transform.translation.y += move_delta.y;

                if pos.distance(target) < 8.0 {
                    enemy.patrol_target = 1 - enemy.patrol_target;
                }
                sprite.color = Color::srgb(0.5, 0.5, 0.5);
            }

            EnemyState::Chase => {
                let dir = (player_pos - pos).normalize_or_zero();
                let move_delta = dir * ENEMY_CHASE_SPEED * time.delta_secs();
                transform.translation.x += move_delta.x;
                transform.translation.y += move_delta.y;
                sprite.color = Color::srgb(0.9, 0.6, 0.1);
            }

            EnemyState::Attack => {
                enemy.pulse_timer += time.delta_secs() * 6.0;
                let pulse = (enemy.pulse_timer.sin() * 0.5 + 0.5).clamp(0.0, 1.0);
                sprite.color = Color::srgb(0.7 + pulse * 0.3, 0.05, 0.05);
            }
        }
    }
}

/// Rewrites the HUD state label each frame.
fn update_hud(
    enemy_query: Query<&EnemyState, With<Enemy>>,
    mut hud_query: Query<&mut Text, With<HudText>>,
) {
    let Ok(state) = enemy_query.single() else { return; };
    for mut text in &mut hud_query {
        let (label, color_desc) = match state {
            EnemyState::Patrol => ("Patrol", "grey — wandering waypoints"),
            EnemyState::Chase  => ("Chase",  "orange — chasing player"),
            EnemyState::Attack => ("Attack", "red pulse — in range"),
        };
        *text = Text::new(format!("State: {}  ({})", label, color_desc));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- select_state ---

    #[test]
    fn inside_attack_radius_is_attack() {
        assert_eq!(select_state(ATTACK_RADIUS - 1.0), EnemyState::Attack);
    }

    #[test]
    fn between_radii_is_chase() {
        let mid = (ATTACK_RADIUS + DETECT_RADIUS) / 2.0;
        assert_eq!(select_state(mid), EnemyState::Chase);
    }

    #[test]
    fn outside_detect_radius_is_patrol() {
        assert_eq!(select_state(DETECT_RADIUS + 1.0), EnemyState::Patrol);
    }

    #[test]
    fn exactly_at_attack_boundary_is_chase() {
        // dist == ATTACK_RADIUS is NOT < ATTACK_RADIUS → Chase.
        assert_eq!(select_state(ATTACK_RADIUS), EnemyState::Chase);
    }

    #[test]
    fn exactly_at_detect_boundary_is_patrol() {
        // dist == DETECT_RADIUS is NOT < DETECT_RADIUS → Patrol.
        assert_eq!(select_state(DETECT_RADIUS), EnemyState::Patrol);
    }

    #[test]
    fn zero_distance_is_attack() {
        assert_eq!(select_state(0.0), EnemyState::Attack);
    }

    // --- ECS ---

    #[test]
    fn setup_spawns_one_player() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Player>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn setup_spawns_one_enemy_in_patrol_state() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<(&Enemy, &EnemyState)>();
        for (_, state) in q.iter(app.world()) {
            assert_eq!(*state, EnemyState::Patrol);
        }
    }
}
