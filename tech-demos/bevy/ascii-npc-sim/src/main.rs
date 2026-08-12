//! Runnable entry point for the ASCII NPC simulation.
//!
//! All the reusable logic lives in the crate library ([`ascii_npc_sim`]) as
//! [`AsciiNpcSimPlugin`]. This binary is the thin headless harness: it prompts
//! for grid config, boots `MinimalPlugins` with a schedule runner, seeds the
//! runtime resources, and adds the plugin.
//!
//! At startup the user is prompted for grid dimensions (X × Y), NPC count N,
//! and the turn duration in seconds.  Defaults: 100 × 100 grid, 1 000 NPCs,
//! 1-second turns.

use ascii_npc_sim::{AsciiNpcSimPlugin, GridConfig, RngState, TurnCount, TurnTimer};
use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use rand::prelude::*;
use std::io::{self, Write as IoWrite};
use std::time::Duration;

fn main() {
    let config = prompt_config();
    let turn_secs = config.turn_secs;

    App::new()
        // Headless: no window, no renderer.  Poll at 50 ms to keep CPU idle
        // between turns while still waking up promptly when the timer fires.
        .add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(50))))
        .insert_resource(config)
        .insert_resource(TurnTimer(Timer::from_seconds(
            turn_secs,
            TimerMode::Repeating,
        )))
        .insert_resource(RngState(StdRng::from_entropy()))
        .insert_resource(TurnCount::default())
        .add_plugins(AsciiNpcSimPlugin)
        .run();
}

/// Reads grid config from stdin, substituting defaults on empty/invalid input.
fn prompt_config() -> GridConfig {
    fn read_line() -> String {
        let mut buf = String::new();
        io::stdin().read_line(&mut buf).ok();
        buf.trim().to_string()
    }

    fn parse_or<T: std::str::FromStr>(s: &str, default: T) -> T {
        s.parse().unwrap_or(default)
    }

    println!("=== ASCII NPC Simulation ===");
    println!("Press Enter to accept each default.");

    print!("Grid width  [100]:  ");
    io::stdout().flush().ok();
    let width = parse_or(&read_line(), 100usize).max(1);

    print!("Grid height [100]:  ");
    io::stdout().flush().ok();
    let height = parse_or(&read_line(), 100usize).max(1);

    print!("NPC count   [1000]: ");
    io::stdout().flush().ok();
    let npc_count = parse_or(&read_line(), 1000usize);

    print!("Turn secs   [1.0]:  ");
    io::stdout().flush().ok();
    let turn_secs = parse_or(&read_line(), 1.0f32).max(0.01);

    GridConfig {
        width,
        height,
        npc_count,
        turn_secs,
    }
}
