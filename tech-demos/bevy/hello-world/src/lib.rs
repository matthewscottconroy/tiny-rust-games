//! Hello world — the minimal Bevy ECS building block.
//!
//! This crate is a *building block*: drop [`HelloWorldPlugin`] into any Bevy
//! app with `app.add_plugins(HelloWorldPlugin)` and it spawns a few named
//! people and greets them every frame.
//!
//! Demonstrates:
//! - Spawning entities with multiple components in `Startup`.
//! - Running two systems every `Update` frame.
//! - Querying components with a `With<T>` filter.
//!
//! # Example
//! ```no_run
//! use bevy::prelude::*;
//! use hello_world::HelloWorldPlugin;
//!
//! App::new()
//!     .add_plugins(MinimalPlugins)
//!     .add_plugins(HelloWorldPlugin)
//!     .run();
//! ```
//!
//! Counterpart: tech-demos/godot/hello-world — the same concept in Godot.

use bevy::prelude::*;

/// Registers the hello-world systems as a self-contained plugin.
///
/// Add it with `app.add_plugins(HelloWorldPlugin)`. It does **not** add
/// `DefaultPlugins`, so the host app stays in control of setup.
pub struct HelloWorldPlugin;

impl Plugin for HelloWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, add_people)
            .add_systems(Update, (hello_world, greet_people));
    }
}

/// Tags a person entity.
#[derive(Component)]
pub struct Person;

/// Stores the name of a person.
#[derive(Component)]
pub struct Name(pub String);

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
        app.add_plugins((MinimalPlugins, HelloWorldPlugin));
        app.update();

        let mut q = app.world_mut().query::<&Person>();
        assert_eq!(q.iter(app.world()).count(), 3);
    }

    #[test]
    fn people_have_name_components() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, HelloWorldPlugin));
        app.update();

        let mut q = app.world_mut().query::<(&Person, &Name)>();
        let names: Vec<String> = q.iter(app.world()).map(|(_, n)| n.0.clone()).collect();
        assert!(names.contains(&"Sam".to_string()));
        assert!(names.contains(&"Charlie".to_string()));
        assert!(names.contains(&"David".to_string()));
    }
}
