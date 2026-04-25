//! Hello world demo — the minimal Bevy ECS starting point.
//!
//! Demonstrates:
//! - Spawning entities with multiple components in `Startup`.
//! - Running two systems every `Update` frame.
//! - Querying components with `With<T>` filter.

use bevy::prelude::*;

/// Tags a person entity.
#[derive(Component)]
struct Person;

/// Stores the name of a person.
#[derive(Component)]
struct Name(String);

fn main() {
    App::new()
        .add_systems(Startup, add_people)
        .add_systems(Update, (hello_world, greet_people))
        .run();
}

/// Prints "Hello, World!" every frame.
fn hello_world() {
    println!("Hello, World!");
}

/// Spawns three named people into the ECS world.
fn add_people(mut commands: Commands) {
    commands.spawn((Person, Name("Sam".to_string())));
    commands.spawn((Person, Name("Charlie".to_string())));
    commands.spawn((Person, Name("David".to_string())));
}

/// Greets every person by printing their name.
fn greet_people(query: Query<&Name, With<Person>>) {
    for name in &query {
        println!("Hello {}!", name.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_people_spawns_three_entities() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, add_people);
        app.update();

        let mut q = app.world_mut().query::<&Person>();
        assert_eq!(q.iter(app.world()).count(), 3);
    }

    #[test]
    fn people_have_name_components() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, add_people);
        app.update();

        let mut q = app.world_mut().query::<(&Person, &Name)>();
        let names: Vec<&str> = q.iter(app.world()).map(|(_, n)| n.0.as_str()).collect();
        assert!(names.contains(&"Sam"));
        assert!(names.contains(&"Charlie"));
        assert!(names.contains(&"David"));
    }
}
