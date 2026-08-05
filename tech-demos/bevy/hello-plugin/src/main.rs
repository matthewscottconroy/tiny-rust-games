//! Hello plugin demo — wrapping systems in a reusable Bevy [`Plugin`].
//!
//! Key idea: a `Plugin` groups related systems and resources into a single
//! `build` call.  The same `App` can combine many plugins without the `main`
//! function growing large.

use bevy::prelude::*;

/// Tags a person entity.
#[derive(Component)]
struct Person;

/// Stores the display name of a person.
#[derive(Component)]
struct Name(String);

/// Registers the hello-world systems as a self-contained plugin.
pub struct HelloPlugin;

impl Plugin for HelloPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, add_people)
            .add_systems(Update, (hello_world, greet_people));
    }
}

fn main() {
    App::new().add_plugins(HelloPlugin).run();
}

/// Prints "Hello, World!" every frame.
fn hello_world() {
    println!("Hello, World!");
}

/// Spawns three named person entities.
fn add_people(mut commands: Commands) {
    commands.spawn((Person, Name("Sam".to_string())));
    commands.spawn((Person, Name("Charlie".to_string())));
    commands.spawn((Person, Name("David".to_string())));
}

/// Prints a greeting for each person in the ECS world.
fn greet_people(query: Query<&Name, With<Person>>) {
    for name in &query {
        println!("Hello {}!", name.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_registers_three_people() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, HelloPlugin));
        app.update();

        let mut q = app.world_mut().query::<&Person>();
        assert_eq!(q.iter(app.world()).count(), 3);
    }

    #[test]
    fn plugin_attaches_name_to_each_person() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, HelloPlugin));
        app.update();

        let mut q = app.world_mut().query::<(&Person, &Name)>();
        assert_eq!(q.iter(app.world()).count(), 3, "each person should have a Name");
    }

    #[test]
    fn plugin_names_are_sam_charlie_david() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, HelloPlugin));
        app.update();

        let mut q = app.world_mut().query::<(&Person, &Name)>();
        let names: Vec<String> = q.iter(app.world()).map(|(_, n)| n.0.clone()).collect();
        assert!(names.contains(&"Sam".to_string()));
        assert!(names.contains(&"Charlie".to_string()));
        assert!(names.contains(&"David".to_string()));
    }

    #[test]
    fn plugin_no_duplicate_names() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, HelloPlugin));
        app.update();

        let mut q = app.world_mut().query::<(&Person, &Name)>();
        let names: Vec<String> = q.iter(app.world()).map(|(_, n)| n.0.clone()).collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(unique.len(), names.len(), "all names should be unique");
    }

    #[test]
    fn plugin_spawns_exactly_one_camera() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, HelloPlugin));
        app.update();

        // HelloPlugin has no camera — verifies no stray camera is spawned
        let mut q = app.world_mut().query::<&Camera>();
        assert_eq!(q.iter(app.world()).count(), 0);
    }
}
