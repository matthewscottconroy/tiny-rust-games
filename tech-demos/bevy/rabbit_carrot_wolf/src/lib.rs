//! Rabbit / carrot / wolf ecosystem simulation — a reusable Bevy plugin.
//!
//! This crate is a *building block*: drop [`RabbitCarrotWolfPlugin`] into any
//! Bevy app and it runs a small predator/prey ecosystem.
//!
//! Key ideas:
//! - Three species (rabbits, wolves, carrots) interact through proximity checks.
//! - Rabbits eat carrots and reproduce; wolves eat rabbits and reproduce.
//! - Each animal has an `age`/`lifespan` field — entities despawn when age exceeds lifespan.
//! - `bevy_egui` drives the config menu and the live population HUD.
//! - Tune it through the [`SimulationConfig`] resource.
//!
//! **Controls:** configure species counts in the egui menu, then click Play.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use rabbit_carrot_wolf::RabbitCarrotWolfPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(RabbitCarrotWolfPlugin)
//!     .run();
//! ```

use bevy::color::palettes::css;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, egui};
use rand::Rng;

/// Bundles every system, state, and resource for the ecosystem simulation.
///
/// Add it with `app.add_plugins(RabbitCarrotWolfPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of window and rendering
/// setup.
pub struct RabbitCarrotWolfPlugin;

impl Plugin for RabbitCarrotWolfPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimulationConfig>()
            .add_plugins(EguiPlugin::default())
            .init_state::<GameState>()
            .add_systems(Startup, setup_camera)
            .add_systems(Update, main_menu_ui.run_if(in_state(GameState::MainMenu)))
            .add_systems(OnEnter(GameState::InGame), setup_simulation)
            .add_systems(
                Update,
                (
                    movement_system,
                    random_direction_system,
                    age_system,
                    rabbit_eating_system,
                    wolf_eating_system,
                    carrot_refill_system,
                    ui_population_system,
                )
                    .run_if(in_state(GameState::InGame)),
            );
    }
}

// --- Pure helpers ---

/// Returns `true` when two circles (defined by center and radius) overlap.
pub fn circles_overlap(pos_a: Vec2, r_a: f32, pos_b: Vec2, r_b: f32) -> bool {
    pos_a.distance(pos_b) < r_a + r_b
}

/// Clamps a position into the rectangular arena `[-half_w, half_w] × [-half_h, half_h]`.
pub fn clamp_to_arena(pos: Vec2, half_w: f32, half_h: f32) -> Vec2 {
    Vec2::new(pos.x.clamp(-half_w, half_w), pos.y.clamp(-half_h, half_h))
}

// --- Application States ---

/// Top-level state: configure then play.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, States)]
pub enum GameState {
    /// Title screen, waiting for the player to start.
    #[default]
    MainMenu,
    /// The simulation is running.
    InGame,
}

// --- Resources ---

/// User-configurable parameters set in the main-menu egui window.
#[derive(Resource)]
pub struct SimulationConfig {
    /// How many rabbits the simulation starts with.
    pub initial_rabbits: usize,
    /// How many wolves the simulation starts with.
    pub initial_wolves: usize,
    /// Maximum carrots that can exist at once.
    pub carrot_max: usize,
    /// Seconds between carrot spawns.
    pub carrot_refill_interval: f32,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            initial_rabbits: 5,
            initial_wolves: 3,
            carrot_max: 20,
            carrot_refill_interval: 25.0,
        }
    }
}

/// Drives the periodic carrot respawn.
#[derive(Resource)]
pub struct CarrotRefillTimer(Timer);

// --- Components ---

/// A rabbit entity — eats carrots, eaten by wolves.
#[derive(Component)]
pub struct Rabbit {
    age: f32,
    lifespan: f32,
}

/// A wolf entity — eats rabbits, reproduces on eating.
#[derive(Component)]
pub struct Wolf {
    age: f32,
    lifespan: f32,
}

/// A carrot entity — eaten by rabbits.
#[derive(Component)]
pub struct Carrot;

/// 2D linear velocity.
#[derive(Component)]
pub struct Velocity(Vec2);

/// Radius used for simple circle–circle collision detection.
#[derive(Component)]
pub struct CollisionRadius(f32);

// --- Systems ---

/// Spawns the persistent camera.
fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// Renders the egui configuration window; transitions to InGame on Play.
fn main_menu_ui(
    mut contexts: EguiContexts,
    mut next_state: ResMut<NextState<GameState>>,
    mut config: ResMut<SimulationConfig>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    egui::Window::new("Simulation Setup").show(ctx, |ui| {
        ui.label("Enter initial values:");
        ui.horizontal(|ui| {
            ui.label("Initial Rabbits:");
            let mut rabbits = config.initial_rabbits as i32;
            if ui.add(egui::DragValue::new(&mut rabbits)).changed() {
                config.initial_rabbits = rabbits as usize;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Initial Wolves:");
            let mut wolves = config.initial_wolves as i32;
            if ui.add(egui::DragValue::new(&mut wolves)).changed() {
                config.initial_wolves = wolves as usize;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Carrot Maximum:");
            let mut carrot_max = config.carrot_max as i32;
            if ui.add(egui::DragValue::new(&mut carrot_max)).changed() {
                config.carrot_max = carrot_max as usize;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Carrot Refill Interval (s):");
            let mut interval = config.carrot_refill_interval;
            if ui
                .add(egui::DragValue::new(&mut interval).speed(0.5))
                .changed()
            {
                config.carrot_refill_interval = interval;
            }
        });
        if ui.button("Play").clicked() {
            next_state.set(GameState::InGame);
        }
    });
}

/// Spawns rabbits, wolves, and carrots using the configured counts.
fn setup_simulation(mut commands: Commands, config: Res<SimulationConfig>) {
    commands.insert_resource(CarrotRefillTimer(Timer::from_seconds(
        config.carrot_refill_interval,
        TimerMode::Repeating,
    )));

    let mut rng = rand::thread_rng();

    for _ in 0..config.initial_rabbits {
        let pos = Vec3::new(
            rng.gen_range(-400.0..400.0),
            rng.gen_range(-300.0..300.0),
            0.0,
        );
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        commands.spawn((
            Sprite {
                color: Color::WHITE,
                custom_size: Some(Vec2::splat(15.0)),
                ..default()
            },
            Transform::from_translation(pos),
            Rabbit {
                age: 0.0,
                lifespan: 25.0,
            },
            Velocity(Vec2::new(2.0 * angle.cos(), 2.0 * angle.sin())),
            CollisionRadius(7.5),
        ));
    }

    for _ in 0..config.initial_wolves {
        let pos = Vec3::new(
            rng.gen_range(-400.0..400.0),
            rng.gen_range(-300.0..300.0),
            0.0,
        );
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        commands.spawn((
            Sprite {
                color: css::DARK_GRAY.into(),
                custom_size: Some(Vec2::splat(20.0)),
                ..default()
            },
            Transform::from_translation(pos),
            Wolf {
                age: 0.0,
                lifespan: 30.0,
            },
            Velocity(Vec2::new(3.0 * angle.cos(), 3.0 * angle.sin())),
            CollisionRadius(10.0),
        ));
    }

    for _ in 0..config.carrot_max {
        let pos = Vec3::new(
            rng.gen_range(-400.0..400.0),
            rng.gen_range(-300.0..300.0),
            0.0,
        );
        commands.spawn((
            Sprite {
                color: css::ORANGE.into(),
                custom_size: Some(Vec2::splat(10.0)),
                ..default()
            },
            Transform::from_translation(pos),
            Carrot,
        ));
    }
}

/// Moves all entities and clamps them to the arena boundary.
fn movement_system(time: Res<Time>, mut query: Query<(&mut Transform, &Velocity)>) {
    for (mut transform, velocity) in query.iter_mut() {
        transform.translation.x += velocity.0.x * time.delta_secs();
        transform.translation.y += velocity.0.y * time.delta_secs();
        let clamped = clamp_to_arena(transform.translation.truncate(), 400.0, 300.0);
        transform.translation.x = clamped.x;
        transform.translation.y = clamped.y;
    }
}

/// Randomly perturbs each entity's heading to produce wandering movement.
fn random_direction_system(mut query: Query<&mut Velocity>) {
    let mut rng = rand::thread_rng();
    for mut velocity in query.iter_mut() {
        if rng.gen_bool(0.05) {
            let perturb = (rng.gen_range(0.0..1.0_f32) - 0.5) * std::f32::consts::FRAC_PI_4;
            let speed = velocity.0.length();
            let angle = velocity.0.y.atan2(velocity.0.x) + perturb;
            velocity.0 = Vec2::new(speed * angle.cos(), speed * angle.sin());
        }
    }
}

/// Despawns rabbits and wolves whose age exceeds their lifespan.
fn age_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query_rabbit: Query<(Entity, &mut Rabbit)>,
    mut query_wolf: Query<(Entity, &mut Wolf)>,
) {
    for (entity, mut rabbit) in query_rabbit.iter_mut() {
        rabbit.age += time.delta_secs();
        if rabbit.age > rabbit.lifespan {
            commands.entity(entity).despawn();
        }
    }
    for (entity, mut wolf) in query_wolf.iter_mut() {
        wolf.age += time.delta_secs();
        if wolf.age > wolf.lifespan {
            commands.entity(entity).despawn();
        }
    }
}

/// Rabbits eat nearby carrots and spawn an offspring at the same position.
fn rabbit_eating_system(
    mut commands: Commands,
    query_rabbit: Query<(&Transform, &CollisionRadius), With<Rabbit>>,
    query_carrot: Query<(Entity, &Transform, &Sprite), With<Carrot>>,
) {
    let mut rng = rand::thread_rng();
    for (rabbit_transform, rabbit_radius) in query_rabbit.iter() {
        for (carrot_entity, carrot_transform, sprite) in query_carrot.iter() {
            let carrot_radius = sprite.custom_size.unwrap_or(Vec2::splat(10.0)).x / 2.0;
            if circles_overlap(
                rabbit_transform.translation.truncate(),
                rabbit_radius.0,
                carrot_transform.translation.truncate(),
                carrot_radius,
            ) {
                commands.entity(carrot_entity).despawn();
                let pos = rabbit_transform.translation;
                let angle = rng.gen_range(0.0..std::f32::consts::TAU);
                commands.spawn((
                    Sprite {
                        color: Color::WHITE,
                        custom_size: Some(Vec2::splat(15.0)),
                        ..default()
                    },
                    Transform::from_translation(pos),
                    Rabbit {
                        age: 0.0,
                        lifespan: 25.0,
                    },
                    Velocity(Vec2::new(2.0 * angle.cos(), 2.0 * angle.sin())),
                    CollisionRadius(7.5),
                ));
                break;
            }
        }
    }
}

/// Wolves eat nearby rabbits and spawn an offspring at the same position.
fn wolf_eating_system(
    mut commands: Commands,
    query_wolf: Query<(&Transform, &CollisionRadius), With<Wolf>>,
    query_rabbit: Query<(Entity, &Transform, &CollisionRadius), With<Rabbit>>,
) {
    let mut rng = rand::thread_rng();
    for (wolf_transform, wolf_radius) in query_wolf.iter() {
        for (rabbit_entity, rabbit_transform, rabbit_radius) in query_rabbit.iter() {
            if circles_overlap(
                wolf_transform.translation.truncate(),
                wolf_radius.0,
                rabbit_transform.translation.truncate(),
                rabbit_radius.0,
            ) {
                commands.entity(rabbit_entity).despawn();
                let pos = wolf_transform.translation;
                let angle = rng.gen_range(0.0..std::f32::consts::TAU);
                commands.spawn((
                    Sprite {
                        color: css::DARK_GRAY.into(),
                        custom_size: Some(Vec2::splat(20.0)),
                        ..default()
                    },
                    Transform::from_translation(pos),
                    Wolf {
                        age: 0.0,
                        lifespan: 30.0,
                    },
                    Velocity(Vec2::new(3.0 * angle.cos(), 3.0 * angle.sin())),
                    CollisionRadius(10.0),
                ));
                break;
            }
        }
    }
}

/// Despawns all carrots and respawns a full batch when the refill timer fires.
fn carrot_refill_system(
    mut commands: Commands,
    time: Res<Time>,
    config: Res<SimulationConfig>,
    mut timer: ResMut<CarrotRefillTimer>,
    query: Query<Entity, With<Carrot>>,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        for entity in query.iter() {
            commands.entity(entity).despawn();
        }
        let mut rng = rand::thread_rng();
        for _ in 0..config.carrot_max {
            let pos = Vec3::new(
                rng.gen_range(-400.0..400.0),
                rng.gen_range(-300.0..300.0),
                0.0,
            );
            commands.spawn((
                Sprite {
                    color: css::ORANGE.into(),
                    custom_size: Some(Vec2::splat(10.0)),
                    ..default()
                },
                Transform::from_translation(pos),
                Carrot,
            ));
        }
    }
}

/// Renders the live population counts in an egui window.
fn ui_population_system(
    mut contexts: EguiContexts,
    query_rabbit: Query<&Rabbit>,
    query_wolf: Query<&Wolf>,
    query_carrot: Query<&Carrot>,
) {
    let rabbit_count = query_rabbit.iter().count();
    let wolf_count = query_wolf.iter().count();
    let carrot_count = query_carrot.iter().count();

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    egui::Window::new("Population").show(ctx, |ui| {
        ui.label(format!("Rabbits: {}", rabbit_count));
        ui.label(format!("Wolves:  {}", wolf_count));
        ui.label(format!("Carrots: {}", carrot_count));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulation_config_default_values_are_nonzero() {
        let cfg = SimulationConfig::default();
        assert!(cfg.initial_rabbits > 0);
        assert!(cfg.initial_wolves > 0);
        assert!(cfg.carrot_max > 0);
        assert!(cfg.carrot_refill_interval > 0.0);
    }

    #[test]
    fn game_state_default_is_main_menu() {
        assert_eq!(GameState::default(), GameState::MainMenu);
    }

    #[test]
    fn circles_overlap_touching_returns_true() {
        // Centers 9 units apart, radii 5 + 5 = 10 → overlap
        assert!(circles_overlap(Vec2::ZERO, 5.0, Vec2::new(9.0, 0.0), 5.0));
    }

    #[test]
    fn circles_overlap_far_apart_returns_false() {
        assert!(!circles_overlap(Vec2::ZERO, 5.0, Vec2::new(20.0, 0.0), 5.0));
    }

    #[test]
    fn circles_overlap_exact_boundary_returns_false() {
        // Distance == sum of radii: not strictly less than, so no overlap
        assert!(!circles_overlap(Vec2::ZERO, 5.0, Vec2::new(10.0, 0.0), 5.0));
    }

    #[test]
    fn clamp_to_arena_inside_is_unchanged() {
        let pos = Vec2::new(100.0, -50.0);
        assert_eq!(clamp_to_arena(pos, 400.0, 300.0), pos);
    }

    #[test]
    fn clamp_to_arena_outside_is_clamped() {
        let pos = Vec2::new(600.0, -400.0);
        let clamped = clamp_to_arena(pos, 400.0, 300.0);
        assert_eq!(clamped.x, 400.0);
        assert_eq!(clamped.y, -300.0);
    }
}
