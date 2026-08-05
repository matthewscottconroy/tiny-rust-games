//! Turn-based combat — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`TurnBasedPlugin`] into any Bevy app
//! with `app.add_plugins(TurnBasedPlugin)` and it runs a small tactical combat
//! demo — initiative order, action points, and grid movement. Tune the grid and
//! combat numbers through the [`TurnBasedConfig`] resource without editing the
//! plugin's internals.
//!
//! Key ideas:
//! - Each actor has `hp`, `action_points` (AP), and `initiative`.
//!   [`sort_by_initiative`] returns a sorted turn order (pure, testable).
//! - The turn loop alternates between `PlayerTurn` and `EnemyTurn` phases.
//!   During the player's turn, WASD moves (costs 1 AP) or bump-attacks an
//!   adjacent enemy (costs 2 AP).  SPACE ends the turn early.
//! - Enemies use a simple AI: step one cell toward the player, then end turn.
//! - [`step_toward`] and [`manhattan`] are pure helper functions.
//! - Dead actors are removed from the turn order; the game ends when all
//!   enemies (or the player) are defeated.
//!
//! **Controls:** WASD / Arrow keys — move or attack   SPACE — end turn.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use turn_based::TurnBasedPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(TurnBasedPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

/// Bundles every system and resource for the turn-based combat demo.
///
/// Add it with `app.add_plugins(TurnBasedPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct TurnBasedPlugin;

impl Plugin for TurnBasedPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TurnBasedConfig>()
            .add_systems(Startup, setup)
            .add_systems(Update, (handle_player_turn, run_enemy_turn, update_hud));
    }
}

// --- Configuration ---

/// Tunable parameters for the demo. Override before adding the plugin, e.g.
/// `app.insert_resource(TurnBasedConfig { player_hp: 20, ..default() })`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct TurnBasedConfig {
    /// Number of grid columns.
    pub cols: usize,
    /// Number of grid rows.
    pub rows: usize,
    /// Pixel size of one grid tile.
    pub tile_px: f32,
    /// Player action points per turn.
    pub player_max_ap: i32,
    /// Enemy action points per turn.
    pub enemy_max_ap: i32,
    /// Damage dealt by a player attack.
    pub player_atk: i32,
    /// Damage dealt by an enemy attack.
    pub enemy_atk: i32,
    /// Player starting (and maximum) hit points.
    pub player_hp: i32,
    /// Enemy starting (and maximum) hit points.
    pub enemy_hp: i32,
}

impl Default for TurnBasedConfig {
    fn default() -> Self {
        Self {
            cols: 14,
            rows: 10,
            tile_px: 46.0,
            player_max_ap: 4,
            enemy_max_ap: 2,
            player_atk: 3,
            enemy_atk: 2,
            player_hp: 12,
            enemy_hp: 6,
        }
    }
}

// ─── Pure helpers ────────────────────────────────────────────────────────────

/// Sorts `(index, initiative)` pairs in place, highest initiative first.
pub fn sort_by_initiative(actors: &mut Vec<(usize, i32)>) {
    actors.sort_by(|a, b| b.1.cmp(&a.1));
}

/// Returns the Manhattan distance between two grid cells.
pub fn manhattan(a: IVec2, b: IVec2) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

/// Returns `from` moved one step (horizontally or vertically) toward `to`.
/// Prefers reducing the larger axis distance first; returns `from` unchanged
/// if `from == to`.
pub fn step_toward(from: IVec2, to: IVec2) -> IVec2 {
    let d = to - from;
    if d == IVec2::ZERO {
        return from;
    }
    if d.x.abs() >= d.y.abs() {
        from + IVec2::new(d.x.signum(), 0)
    } else {
        from + IVec2::new(0, d.y.signum())
    }
}

// ─── Actor ───────────────────────────────────────────────────────────────────

/// A combatant on the grid: player or enemy.
#[derive(Component, Clone)]
pub struct Actor {
    /// Current hit points.
    pub hp: i32,
    /// Maximum hit points.
    pub max_hp: i32,
    /// Remaining action points this turn.
    pub ap: i32,
    /// Action points restored at the start of a turn.
    pub max_ap: i32,
    /// Turn-order priority; higher acts first.
    pub initiative: i32,
    /// Grid cell the actor occupies.
    pub cell: IVec2,
    /// Whether this actor is the player.
    pub is_player: bool,
}

// ─── Turn state ──────────────────────────────────────────────────────────────

/// Tracks the turn order and current phase.
#[derive(Resource)]
pub struct TurnState {
    /// Entities sorted by initiative.
    pub order: Vec<Entity>,
    /// Index into `order` of the actor whose turn it is.
    pub current_idx: usize,
    /// Current turn phase.
    pub phase: TurnPhase,
}

/// Which side is currently acting (or whether the game is over).
#[derive(PartialEq, Clone, Copy)]
pub enum TurnPhase {
    /// The player may act.
    PlayerTurn,
    /// An enemy is acting.
    EnemyTurn,
    /// The game has ended.
    GameOver,
}

// ─── HUD components ──────────────────────────────────────────────────────────

/// Marks a HUD text label with its role.
#[derive(Component)]
pub enum HudLabel {
    /// The main status line (turn/HP/AP).
    Status,
    /// The controls hint line.
    Log,
}

// ─── Systems ─────────────────────────────────────────────────────────────────

fn cell_to_world(cell: IVec2, config: &TurnBasedConfig) -> Vec3 {
    let ox = -(config.cols as f32 * config.tile_px) / 2.0 + config.tile_px / 2.0;
    let oy = (config.rows as f32 * config.tile_px) / 2.0 - config.tile_px / 2.0;
    Vec3::new(
        ox + cell.x as f32 * config.tile_px,
        oy - cell.y as f32 * config.tile_px,
        0.0,
    )
}

fn setup(mut commands: Commands, config: Res<TurnBasedConfig>) {
    commands.spawn(Camera2d);

    // Floor tiles
    for r in 0..config.rows {
        for c in 0..config.cols {
            let color = if (r + c) % 2 == 0 {
                Color::srgb(0.22, 0.22, 0.28)
            } else {
                Color::srgb(0.18, 0.18, 0.24)
            };
            commands.spawn((
                Sprite { color, custom_size: Some(Vec2::splat(config.tile_px - 1.0)), ..default() },
                Transform::from_translation(cell_to_world(IVec2::new(c as i32, r as i32), &config)),
            ));
        }
    }

    // Spawn player
    let player_cell = IVec2::new(1, 5);
    let player = commands.spawn((
        Sprite { color: Color::srgb(0.3, 0.85, 1.0), custom_size: Some(Vec2::splat(config.tile_px * 0.7)), ..default() },
        Transform::from_translation(cell_to_world(player_cell, &config).with_z(1.0)),
        Actor {
            hp: config.player_hp, max_hp: config.player_hp,
            ap: config.player_max_ap, max_ap: config.player_max_ap,
            initiative: 10, cell: player_cell, is_player: true,
        },
    )).id();

    // Spawn enemies
    let enemy_starts = [
        (IVec2::new(12, 2), 7i32),
        (IVec2::new(12, 7), 5),
        (IVec2::new(7, 1), 6),
    ];
    let mut entities = vec![(player, 10i32)];
    for (cell, init) in enemy_starts {
        let e = commands.spawn((
            Sprite { color: Color::srgb(0.9, 0.25, 0.2), custom_size: Some(Vec2::splat(config.tile_px * 0.65)), ..default() },
            Transform::from_translation(cell_to_world(cell, &config).with_z(1.0)),
            Actor {
                hp: config.enemy_hp, max_hp: config.enemy_hp,
                ap: config.enemy_max_ap, max_ap: config.enemy_max_ap,
                initiative: init, cell, is_player: false,
            },
        )).id();
        entities.push((e, init));
    }

    // Build sorted turn order
    let mut indexed: Vec<(usize, i32)> = entities.iter().enumerate()
        .map(|(i, (_, init))| (i, *init)).collect();
    sort_by_initiative(&mut indexed);
    let order: Vec<Entity> = indexed.iter().map(|(i, _)| entities[*i].0).collect();

    commands.insert_resource(TurnState { order, current_idx: 0, phase: TurnPhase::PlayerTurn });

    // HUD
    commands.spawn((
        Text::new("Player turn  AP: 4/4"),
        TextFont { font_size: 18.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(32.0),
            left: Val::Px(8.0),
            ..default()
        },
        HudLabel::Status,
    ));
    commands.spawn((
        Text::new("WASD — move/attack   SPACE — end turn"),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgb(0.6, 0.6, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
        HudLabel::Log,
    ));
}

fn handle_player_turn(
    input: Res<ButtonInput<KeyCode>>,
    config: Res<TurnBasedConfig>,
    mut ts: ResMut<TurnState>,
    mut actors: Query<(Entity, &mut Actor, &mut Transform, &mut Sprite)>,
) {
    if ts.phase != TurnPhase::PlayerTurn {
        return;
    }

    // Collect cells occupied by actors for collision.
    let occupied: Vec<(Entity, IVec2)> = actors.iter()
        .filter(|(_, a, _, _)| a.hp > 0)
        .map(|(e, a, _, _)| (e, a.cell))
        .collect();

    let player_entity = ts.order.iter().find(|&&e| {
        actors.get(e).map(|(_, a, _, _)| a.is_player).unwrap_or(false)
    }).copied();
    let Some(pe) = player_entity else { return };
    let Ok((_, player, _, _)) = actors.get(pe) else { return };
    if player.hp <= 0 {
        drop(player);
        advance_turn(&mut ts, &actors);
        return;
    }
    // AP is 0 at the start of a fresh turn — restore it before accepting input.
    // (Mid-turn depletion is handled below and correctly ends the turn there.)
    if player.ap <= 0 {
        drop(player);
        if let Ok((_, mut p, _, _)) = actors.get_mut(pe) {
            p.ap = p.max_ap;
        }
        return;
    }
    drop(player);

    let dirs = [
        (KeyCode::KeyW, IVec2::NEG_Y), (KeyCode::ArrowUp, IVec2::NEG_Y),
        (KeyCode::KeyS, IVec2::Y),     (KeyCode::ArrowDown, IVec2::Y),
        (KeyCode::KeyA, IVec2::NEG_X), (KeyCode::ArrowLeft, IVec2::NEG_X),
        (KeyCode::KeyD, IVec2::X),     (KeyCode::ArrowRight, IVec2::X),
    ];
    if input.just_pressed(KeyCode::Space) {
        advance_turn(&mut ts, &actors);
        return;
    }

    for (key, delta) in dirs {
        if !input.just_pressed(key) {
            continue;
        }
        let Ok((_, player, _, _)) = actors.get(pe) else { break };
        let target_cell = player.cell + delta;
        if target_cell.x < 0 || target_cell.y < 0
            || target_cell.x >= config.cols as i32 || target_cell.y >= config.rows as i32
        {
            break;
        }
        // Check for enemy at target cell.
        let enemy_there = occupied.iter()
            .find(|(e, c)| *e != pe && *c == target_cell)
            .map(|(e, _)| *e);
        drop(player);

        if let Some(enemy_e) = enemy_there {
            // Bump attack
            if let Ok((_, mut player, _, _)) = actors.get_mut(pe) {
                if player.ap >= 2 {
                    player.ap -= 2;
                }
            }
            if let Ok((_, mut enemy, _, mut sprite)) = actors.get_mut(enemy_e) {
                enemy.hp -= config.player_atk;
                if enemy.hp <= 0 {
                    sprite.color = Color::srgb(0.3, 0.3, 0.3);
                }
            }
        } else {
            // Move
            if let Ok((_, mut player, mut t, _)) = actors.get_mut(pe) {
                if player.ap >= 1 {
                    player.cell = target_cell;
                    player.ap -= 1;
                    t.translation = cell_to_world(target_cell, &config).with_z(1.0);
                }
            }
        }
        break;
    }

    // Auto-end if AP runs out.
    if let Ok((_, player, _, _)) = actors.get(pe) {
        if player.ap <= 0 {
            advance_turn(&mut ts, &actors);
        }
    }
}

fn advance_turn(ts: &mut TurnState, actors: &Query<(Entity, &mut Actor, &mut Transform, &mut Sprite)>) {
    // Remove dead actors from order.
    ts.order.retain(|&e| actors.get(e).map(|(_, a, _, _)| a.hp > 0).unwrap_or(false));
    if ts.order.is_empty() {
        ts.phase = TurnPhase::GameOver;
        return;
    }
    ts.current_idx = (ts.current_idx + 1) % ts.order.len();
    let current = ts.order[ts.current_idx];
    let is_player = actors.get(current).map(|(_, a, _, _)| a.is_player).unwrap_or(false);
    ts.phase = if is_player { TurnPhase::PlayerTurn } else { TurnPhase::EnemyTurn };
}

fn run_enemy_turn(
    config: Res<TurnBasedConfig>,
    mut ts: ResMut<TurnState>,
    mut actors: Query<(Entity, &mut Actor, &mut Transform, &mut Sprite)>,
) {
    if ts.phase != TurnPhase::EnemyTurn {
        return;
    }
    let Some(enemy_e) = ts.order.get(ts.current_idx).copied() else {
        advance_turn(&mut ts, &actors);
        return;
    };

    // --- data-collection phase (immutable) -----------------------------------
    let enemy_data = actors.get(enemy_e)
        .ok()
        .filter(|(_, a, _, _)| !a.is_player && a.hp > 0)
        .map(|(_, a, _, _)| a.cell);
    let Some(enemy_cell) = enemy_data else {
        advance_turn(&mut ts, &actors);
        return;
    };

    let player_info = actors.iter()
        .find(|(_, a, _, _)| a.is_player && a.hp > 0)
        .map(|(e, a, _, _)| (e, a.cell));
    let Some((player_e, pc)) = player_info else {
        ts.phase = TurnPhase::GameOver;
        return;
    };

    let occupied: Vec<IVec2> = actors.iter()
        .filter(|(e, a, _, _)| *e != enemy_e && a.hp > 0)
        .map(|(_, a, _, _)| a.cell)
        .collect();

    // --- mutation phase (one entity at a time) --------------------------------
    if manhattan(enemy_cell, pc) == 1 {
        // Bump-attack the player.
        if let Ok((_, mut player, _, _)) = actors.get_mut(player_e) {
            player.hp -= config.enemy_atk;
        }
    } else {
        let next = step_toward(enemy_cell, pc);
        if !occupied.contains(&next) {
            if let Ok((_, mut enemy, mut t, _)) = actors.get_mut(enemy_e) {
                enemy.cell = next;
                t.translation = cell_to_world(next, &config).with_z(1.0);
            }
        }
    }
    if let Ok((_, mut enemy, _, _)) = actors.get_mut(enemy_e) {
        enemy.ap = 0;
    }

    advance_turn(&mut ts, &actors);
}

fn update_hud(
    ts: Res<TurnState>,
    config: Res<TurnBasedConfig>,
    actors: Query<&Actor>,
    mut hud: Query<(&mut Text, &mut TextColor, &HudLabel)>,
) {
    let player = actors.iter().find(|a| a.is_player);
    let enemies_alive = actors.iter().filter(|a| !a.is_player && a.hp > 0).count();

    for (mut text, mut color, label) in &mut hud {
        match label {
            HudLabel::Status => {
                text.0 = match ts.phase {
                    TurnPhase::GameOver => {
                        if player.map(|p| p.hp <= 0).unwrap_or(true) {
                            "You were defeated!".to_string()
                        } else {
                            "Victory! All enemies defeated.".to_string()
                        }
                    }
                    TurnPhase::PlayerTurn => {
                        let ap = player.map(|p| p.ap).unwrap_or(0);
                        let hp = player.map(|p| p.hp).unwrap_or(0);
                        format!("Player turn  HP: {}  AP: {}/{}  Enemies: {}", hp, ap, config.player_max_ap, enemies_alive)
                    }
                    TurnPhase::EnemyTurn => {
                        let hp = player.map(|p| p.hp).unwrap_or(0);
                        format!("Enemy turn...  Player HP: {}  Enemies: {}", hp, enemies_alive)
                    }
                };
                color.0 = match ts.phase {
                    TurnPhase::PlayerTurn => Color::srgb(0.3, 0.9, 1.0),
                    TurnPhase::EnemyTurn  => Color::srgb(1.0, 0.4, 0.3),
                    TurnPhase::GameOver   => Color::srgb(1.0, 0.9, 0.2),
                };
            }
            HudLabel::Log => {}
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_by_initiative_highest_first() {
        let mut actors = vec![(0usize, 5i32), (1, 10), (2, 3)];
        sort_by_initiative(&mut actors);
        assert_eq!(actors[0].1, 10);
        assert_eq!(actors[1].1, 5);
        assert_eq!(actors[2].1, 3);
    }

    #[test]
    fn sort_by_initiative_empty_ok() {
        let mut actors: Vec<(usize, i32)> = vec![];
        sort_by_initiative(&mut actors);
        assert!(actors.is_empty());
    }

    #[test]
    fn manhattan_correct() {
        assert_eq!(manhattan(IVec2::new(0, 0), IVec2::new(3, 4)), 7);
        assert_eq!(manhattan(IVec2::new(1, 1), IVec2::new(1, 1)), 0);
    }

    #[test]
    fn step_toward_moves_horizontally() {
        let result = step_toward(IVec2::new(0, 0), IVec2::new(5, 0));
        assert_eq!(result, IVec2::new(1, 0));
    }

    #[test]
    fn step_toward_moves_vertically() {
        let result = step_toward(IVec2::new(0, 0), IVec2::new(0, 3));
        assert_eq!(result, IVec2::new(0, 1));
    }

    #[test]
    fn step_toward_same_position_unchanged() {
        let pos = IVec2::new(4, 4);
        assert_eq!(step_toward(pos, pos), pos);
    }

    #[test]
    fn config_default_matches_documented_values() {
        let c = TurnBasedConfig::default();
        assert_eq!(c.cols, 14);
        assert_eq!(c.rows, 10);
        assert_eq!(c.player_hp, 12);
        assert_eq!(c.enemy_hp, 6);
    }

    #[test]
    fn setup_spawns_player_and_enemies() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<TurnBasedConfig>()
            .add_systems(Startup, setup);
        app.update();

        let mut q = app.world_mut().query::<&Actor>();
        let actors: Vec<bool> = q.iter(app.world()).map(|a| a.is_player).collect();
        assert_eq!(actors.iter().filter(|&&p| p).count(), 1, "one player");
        assert_eq!(actors.iter().filter(|&&p| !p).count(), 3, "three enemies");
    }
}
