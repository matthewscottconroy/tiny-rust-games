use bevy::prelude::*;
use bevy_egui::{egui, EguiContext, EguiPlugin};
use rand::Rng;

// --- Application States ---
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum GameState {
    MainMenu,
    InGame,
}

// --- Resources ---
#[derive(Default)]
struct SimulationConfig {
    initial_rabbits: usize,
    initial_wolves: usize,
    carrot_max: usize,
    carrot_refill_interval: f32, // seconds; default set equal to rabbit lifespan (25 sec)
}

// Resource for carrot refill timer.
struct CarrotRefillTimer(Timer);

// --- Components ---
struct Rabbit {
    age: f32,
    lifespan: f32, // seconds
}
struct Wolf {
    age: f32,
    lifespan: f32, // seconds
}
struct Carrot;

// Velocity (in units per second)
struct Velocity(Vec2);

// For collision checking, we use a radius (half the sprite size)
struct CollisionRadius(f32);

// --- Main ---
fn main() {
    App::build()
        .insert_resource(WindowDescriptor {
            title: "Ecosystem Simulation".to_string(),
            width: 1100.0, // simulation area + UI
            height: 600.0,
            ..Default::default()
        })
        .insert_resource(SimulationConfig {
            initial_rabbits: 5,
            initial_wolves: 3,
            carrot_max: 20,
            carrot_refill_interval: 25.0, // default equal to rabbit lifespan
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(EguiPlugin)
        .add_state(GameState::MainMenu)
        .add_startup_system(setup_camera.system())
        .add_system_set(SystemSet::on_update(GameState::MainMenu)
            .with_system(main_menu_ui.system()))
        .add_system_set(SystemSet::on_enter(GameState::InGame)
            .with_system(setup_simulation.system()))
        .add_system_set(SystemSet::on_update(GameState::InGame)
            .with_system(movement_system.system())
            .with_system(random_direction_system.system())
            .with_system(age_system.system())
            .with_system(rabbit_eating_system.system())
            .with_system(wolf_eating_system.system())
            .with_system(carrot_refill_system.system())
            .with_system(ui_population_system.system()))
        .run();
}

// --- Setup camera ---
fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

// --- Main Menu UI using bevy_egui ---
fn main_menu_ui(
    mut egui_context: ResMut<EguiContext>,
    mut state: ResMut<State<GameState>>,
    mut config: ResMut<SimulationConfig>,
) {
    egui::Window::new("Simulation Setup").show(egui_context.ctx_mut(), |ui| {
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
            if ui.add(egui::DragValue::new(&mut interval).speed(0.5)).changed() {
                config.carrot_refill_interval = interval;
            }
        });
        if ui.button("Play").clicked() {
            state.set(GameState::InGame).unwrap();
        }
    });
}

// --- Setup Simulation on entering InGame ---
fn setup_simulation(
    mut commands: Commands,
    config: Res<SimulationConfig>,
) {
    // Insert carrot refill timer resource.
    commands.insert_resource(CarrotRefillTimer(Timer::from_seconds(
        config.carrot_refill_interval,
        true,
    )));
    
    let mut rng = rand::thread_rng();
    // Spawn initial Rabbits.
    for _ in 0..config.initial_rabbits {
        let pos = Vec3::new(rng.gen_range(-400.0..400.0), rng.gen_range(-300.0..300.0), 0.0);
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        commands.spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::WHITE,
                custom_size: Some(Vec2::splat(15.0)),
                ..Default::default()
            },
            transform: Transform {
                translation: pos,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Rabbit { age: 0.0, lifespan: 25.0 })
        .insert(Velocity(Vec2::new(2.0 * angle.cos(), 2.0 * angle.sin())))
        .insert(CollisionRadius(7.5));
    }
    // Spawn initial Wolves.
    for _ in 0..config.initial_wolves {
        let pos = Vec3::new(rng.gen_range(-400.0..400.0), rng.gen_range(-300.0..300.0), 0.0);
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        commands.spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::DARK_GRAY,
                custom_size: Some(Vec2::splat(20.0)),
                ..Default::default()
            },
            transform: Transform {
                translation: pos,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Wolf { age: 0.0, lifespan: 30.0 })
        .insert(Velocity(Vec2::new(3.0 * angle.cos(), 3.0 * angle.sin())))
        .insert(CollisionRadius(10.0));
    }
    // Spawn initial Carrots (fill up to carrot_max).
    for _ in 0..config.carrot_max {
        let pos = Vec3::new(rng.gen_range(-400.0..400.0), rng.gen_range(-300.0..300.0), 0.0);
        commands.spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::ORANGE,
                custom_size: Some(Vec2::splat(10.0)),
                ..Default::default()
            },
            transform: Transform {
                translation: pos,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Carrot);
    }
}

// --- Movement System ---
fn movement_system(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Velocity)>
) {
    for (mut transform, velocity) in query.iter_mut() {
        transform.translation += Vec3::new(
            velocity.0.x * time.delta_seconds(),
            velocity.0.y * time.delta_seconds(),
            0.0,
        );
        // Keep entities within bounds (assume simulation area: -400..400, -300..300).
        transform.translation.x = transform.translation.x.clamp(-400.0, 400.0);
        transform.translation.y = transform.translation.y.clamp(-300.0, 300.0);
    }
}

// --- Random Direction System ---
fn random_direction_system(
    mut query: Query<&mut Velocity>,
) {
    let mut rng = rand::thread_rng();
    for mut velocity in query.iter_mut() {
        if rng.gen_bool(0.05) {
            let angle_perturb = (rng.gen_range(0.0..1.0) - 0.5) * std::f32::consts::FRAC_PI_4;
            let speed = velocity.0.length();
            let current_angle = velocity.0.y.atan2(velocity.0.x);
            let new_angle = current_angle + angle_perturb;
            velocity.0 = Vec2::new(speed * new_angle.cos(), speed * new_angle.sin());
        }
    }
}

// --- Age System ---
fn age_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query_rabbit: Query<(Entity, &mut Rabbit)>,
    mut query_wolf: Query<(Entity, &mut Wolf)>
) {
    for (entity, mut rabbit) in query_rabbit.iter_mut() {
        rabbit.age += time.delta_seconds();
        if rabbit.age > rabbit.lifespan {
            commands.entity(entity).despawn();
        }
    }
    for (entity, mut wolf) in query_wolf.iter_mut() {
        wolf.age += time.delta_seconds();
        if wolf.age > wolf.lifespan {
            commands.entity(entity).despawn();
        }
    }
}

// --- Rabbit Eating System ---
fn rabbit_eating_system(
    mut commands: Commands,
    mut query_rabbit: Query<(&Transform, &CollisionRadius, &Rabbit)>,
    query_carrot: Query<(Entity, &Transform, &Sprite), With<Carrot>>,
) {
    for (rabbit_transform, rabbit_radius, _rabbit) in query_rabbit.iter() {
        for (carrot_entity, carrot_transform, sprite) in query_carrot.iter() {
            let distance = rabbit_transform
                .translation
                .distance(carrot_transform.translation);
            let carrot_radius = sprite.custom_size.unwrap().x / 2.0;
            if distance < rabbit_radius.0 + carrot_radius {
                // Rabbit eats the carrot.
                commands.entity(carrot_entity).despawn();
                // Spawn one offspring at the same position.
                let pos = rabbit_transform.translation;
                let mut rng = rand::thread_rng();
                let angle = rng.gen_range(0.0..std::f32::consts::TAU);
                commands.spawn_bundle(SpriteBundle {
                    sprite: Sprite {
                        color: Color::WHITE,
                        custom_size: Some(Vec2::splat(15.0)),
                        ..Default::default()
                    },
                    transform: Transform {
                        translation: pos,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(Rabbit { age: 0.0, lifespan: 25.0 })
                .insert(Velocity(Vec2::new(2.0 * angle.cos(), 2.0 * angle.sin())))
                .insert(CollisionRadius(7.5));
                break; // one carrot per rabbit per update
            }
        }
    }
}

// --- Wolf Eating System ---
fn wolf_eating_system(
    mut commands: Commands,
    mut query_wolf: Query<(&Transform, &CollisionRadius, &Wolf)>,
    query_rabbit: Query<(Entity, &Transform, &CollisionRadius), With<Rabbit>>,
) {
    for (wolf_transform, wolf_radius, _wolf) in query_wolf.iter() {
        for (rabbit_entity, rabbit_transform, rabbit_radius) in query_rabbit.iter() {
            let distance = wolf_transform.translation.distance(rabbit_transform.translation);
            if distance < wolf_radius.0 + rabbit_radius.0 {
                // Wolf eats the rabbit.
                commands.entity(rabbit_entity).despawn();
                // Spawn one new wolf.
                let pos = wolf_transform.translation;
                let mut rng = rand::thread_rng();
                let angle = rng.gen_range(0.0..std::f32::consts::TAU);
                commands.spawn_bundle(SpriteBundle {
                    sprite: Sprite {
                        color: Color::DARK_GRAY,
                        custom_size: Some(Vec2::splat(20.0)),
                        ..Default::default()
                    },
                    transform: Transform {
                        translation: pos,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(Wolf { age: 0.0, lifespan: 30.0 })
                .insert(Velocity(Vec2::new(3.0 * angle.cos(), 3.0 * angle.sin())))
                .insert(CollisionRadius(10.0));
                break;
            }
        }
    }
}

// --- Carrot Refill System ---
fn carrot_refill_system(
    mut commands: Commands,
    time: Res<Time>,
    config: Res<SimulationConfig>,
    mut timer: ResMut<CarrotRefillTimer>,
    query: Query<Entity, With<Carrot>>,
) {
    timer.0.tick(time.delta());
    if timer.0.finished() {
        // Despawn all carrots.
        for entity in query.iter() {
            commands.entity(entity).despawn();
        }
        // Refill carrots to carrot_max.
        let mut rng = rand::thread_rng();
        for _ in 0..config.carrot_max {
            let pos = Vec3::new(rng.gen_range(-400.0..400.0), rng.gen_range(-300.0..300.0), 0.0);
            commands.spawn_bundle(SpriteBundle {
                sprite: Sprite {
                    color: Color::ORANGE,
                    custom_size: Some(Vec2::splat(10.0)),
                    ..Default::default()
                },
                transform: Transform {
                    translation: pos,
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(Carrot);
        }
    }
}

// --- UI Population System ---
fn ui_population_system(
    mut egui_context: ResMut<EguiContext>,
    query_rabbit: Query<&Rabbit>,
    query_wolf: Query<&Wolf>,
    query_carrot: Query<&Carrot>,
) {
    let rabbit_count = query_rabbit.iter().count();
    let wolf_count = query_wolf.iter().count();
    let carrot_count = query_carrot.iter().count();

    egui::Window::new("Population").show(egui_context.ctx(), |ui| {
        ui.label(format!("Rabbits: {}", rabbit_count));
        ui.label(format!("Wolves: {}", wolf_count));
        ui.label(format!("Carrots: {}", carrot_count));
    });
}
