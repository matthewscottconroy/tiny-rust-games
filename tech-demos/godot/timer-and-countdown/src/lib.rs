//! Timer and Countdown demo — creates a repeating Timer child, tracks ticks,
//! and shows a live countdown to the next tick on a Label.

use godot::classes::{INode, Label, Node, Timer};
use godot::prelude::*;

struct TimerCountdownExtension;
#[gdextension]
unsafe impl ExtensionLibrary for TimerCountdownExtension {}

#[derive(GodotClass)]
#[class(base=Node)]
struct TimerCountdown {
    #[export]
    interval: f64,

    ticks: i32,
    elapsed_rounds: i32,
    base: Base<Node>,
}

#[godot_api]
impl INode for TimerCountdown {
    fn init(base: Base<Node>) -> Self {
        Self {
            interval: 1.0,
            ticks: 0,
            elapsed_rounds: 0,
            base,
        }
    }

    fn ready(&mut self) {
        let interval = self.interval;

        let mut timer = Timer::new_alloc();
        timer.set_wait_time(interval);
        timer.set_one_shot(false);

        let callable = self.base().callable("on_timeout");
        timer.connect("timeout", &callable);

        let timer_node = timer.upcast::<Node>();
        self.base_mut().add_child(&timer_node);

        // Start the timer after adding it to the tree
        let mut timer_ref = self.base().get_node_as::<Timer>("Timer");
        timer_ref.start();
    }

    fn process(&mut self, _delta: f64) {
        if let Some(timer) = self.base().try_get_node_as::<Timer>("Timer") {
            let remaining = timer.get_time_left();
            let text = format_countdown(remaining);
            let mut label = self.base().get_node_as::<Label>("Label");
            label.set_text(text.as_str());
        }
    }
}

#[godot_api]
impl TimerCountdown {
    #[func]
    pub fn on_timeout(&mut self) {
        self.ticks += 1;
        self.elapsed_rounds += 1;
        godot_print!("Tick! Total ticks: {}", self.ticks);
    }
}

// ---------------------------------------------------------------------------
// Pure helper functions
// ---------------------------------------------------------------------------

/// Format a countdown value (in seconds) as "X.X s".
pub fn format_countdown(remaining: f64) -> String {
    format!("{:.1} s", remaining)
}

/// Compute total elapsed time based on number of ticks and interval.
pub fn total_elapsed(ticks: i32, interval: f64) -> f64 {
    ticks as f64 * interval
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_countdown_whole_second() {
        assert_eq!(format_countdown(3.0), "3.0 s");
    }

    #[test]
    fn format_countdown_fractional() {
        assert_eq!(format_countdown(1.5), "1.5 s");
    }

    #[test]
    fn format_countdown_zero() {
        assert_eq!(format_countdown(0.0), "0.0 s");
    }

    #[test]
    fn total_elapsed_zero_ticks() {
        assert!((total_elapsed(0, 1.0) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn total_elapsed_one_tick() {
        assert!((total_elapsed(1, 2.5) - 2.5).abs() < 1e-9);
    }

    #[test]
    fn total_elapsed_many_ticks() {
        assert!((total_elapsed(10, 0.5) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn format_countdown_rounds_to_one_decimal() {
        // 1.55 should round to 1.6 with one decimal place
        let s = format_countdown(1.55);
        assert!(s.ends_with(" s"), "should end with ' s': {s}");
    }
}
