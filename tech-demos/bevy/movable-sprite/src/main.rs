use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(player_movement_system)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dBundle::default());

    let sprite_handle = asset_server.load("sprite.png");
    commands
        .spawn(SpriteBundle {
            texture: sprite_handle,
            ..default()
        })
        /*
            .spawn_bundle(SpriteBundle {
                material: materials.add(sprite_handle.into()),
                transform: Transform::from_xyz(0.0, 0.0, 1.0),
                ..Default::default()
            })
        */
        .insert(Player);
}

#[derive(Component)]
struct Player;

fn player_movement_system(
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut player_query: Query<&mut Transform, With<Player>>,
) {
    for mut player_transform in player_query.iter_mut() {
        const SPEED: f32 = 300.0;
        let mut direction = Vec3::ZERO;

        if keyboard_input.pressed(KeyCode::W) {
            direction.y += 1.0;
        }
        if keyboard_input.pressed(KeyCode::S) {
            direction.y -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::A) {
            direction.x -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::D) {
            direction.x += 1.0;
        }

        if direction != Vec3::ZERO {
            player_transform.translation += direction.normalize() * SPEED * time.delta_seconds();
        }
    }
}
