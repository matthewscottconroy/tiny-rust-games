//! Runnable entry point for the ECS ASCII NPC simulation.
//!
//! All the reusable logic lives in the crate library ([`ecs_ascii_sim`]) as
//! [`EcsAsciiSimPlugin`]. This binary is just the thin harness that reads the
//! grid dimensions and turn rate from stdin, boots the **headless** engine
//! (`MinimalPlugins` + `ScheduleRunnerPlugin`), and adds the plugin.
//!
//! The bootstrap — the headless runner and the turn timer — stays here because
//! it is a host concern; the plugin itself is host-agnostic. Copy `lib.rs` into
//! your own project (or depend on this crate) and add the one plugin to reuse
//! the simulation.
//!
//! ```bash
//! cargo run
//! ```
//!
//! At startup you will be asked for the grid width, grid height, NPC count, and
//! turn duration in seconds (each with a default).

use bevy::{app::ScheduleRunnerPlugin, prelude::*};
use ecs_ascii_sim::{EcsAsciiSimConfig, EcsAsciiSimPlugin};
use std::{
    io::{self, Write},
    time::Duration,
};

fn prompt(label: &str, default: &str) -> String {
    print!("  {label} [default {default}]: ");
    let _ = io::stdout().flush();
    let mut buf = String::new();
    let _ = io::stdin().read_line(&mut buf);
    let s = buf.trim().to_string();
    if s.is_empty() { default.to_string() } else { s }
}

/// Reads the tunable [`EcsAsciiSimConfig`] and the turn duration from stdin.
///
/// The turn duration is not part of the config resource: it configures the
/// host's [`ScheduleRunnerPlugin`], which is bootstrap the plugin never owns.
fn read_sim_config() -> (EcsAsciiSimConfig, Duration) {
    println!("\n╔══════════════════════════════════════╗");
    println!("║    ECS ASCII NPC Simulation (Bevy)   ║");
    println!("╚══════════════════════════════════════╝\n");
    println!("Press Enter to accept defaults.\n");
    let defaults = EcsAsciiSimConfig::default();
    let width = prompt("Grid width       (X)", "100")
        .parse()
        .unwrap_or(defaults.width);
    let height = prompt("Grid height      (Y)", "100")
        .parse()
        .unwrap_or(defaults.height);
    let npc_count = prompt("Number of NPCs   (N)", "1000")
        .parse()
        .unwrap_or(defaults.npc_count);
    let turn_secs = prompt("Turn duration (secs)", "1.0")
        .parse::<f64>()
        .unwrap_or(1.0);
    println!();

    // Clear terminal before the simulation starts.
    print!("\x1b[2J\x1b[H");
    let _ = io::stdout().flush();

    let config = EcsAsciiSimConfig {
        width,
        height,
        npc_count,
        ..defaults
    };
    (config, Duration::from_secs_f64(turn_secs))
}

fn main() {
    // Read configuration before building the App so we know the turn duration
    // (needed to configure ScheduleRunnerPlugin) and grid dimensions (which the
    // plugin reads from EcsAsciiSimConfig to size OccupancyGrid).
    let (config, turn_dur) = read_sim_config();

    App::new()
        // MinimalPlugins: no window, no audio, no renderer. Just the ECS
        // scheduler. ScheduleRunnerPlugin drives the Update schedule on a
        // fixed timer instead of vsync.
        .add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(turn_dur)))
        // Insert the tunable config before adding the plugin so the plugin
        // sizes its resources from it.
        .insert_resource(config)
        .add_plugins(EcsAsciiSimPlugin)
        .run();
}
