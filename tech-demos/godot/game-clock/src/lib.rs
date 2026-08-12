//! In-game day/night clock demo — Godot 4.3 + gdext 0.5.
//!
//! Teaches:
//! - Tracking fractional in-game hours with a configurable speed multiplier.
//! - Pure functions for display formatting, sunrise/sunset detection, and
//!   computing a sky colour from the hour.
//! - Driving a `ColorRect` background and a `Label` HUD from a single clock
//!   resource with no GDScript.
//!
//! The clock runs at 60× real-time by default (one in-game day ≈ 24 minutes).
//!
//! **Controls:** + / – — speed up / slow down;  SPACE — pause.

use godot::classes::{ColorRect, INode2D, Input, Label, Node2D};
use godot::prelude::*;

// ── Extension entry point ─────────────────────────────────────────────────────

struct GameClockExt;
#[gdextension]
unsafe impl ExtensionLibrary for GameClockExt {}

// ── Pure time helpers ─────────────────────────────────────────────────────────

/// `"HH:MM"` string for `hour ∈ [0, 24)`.
pub fn time_string(hour: f32) -> String {
    let total_minutes = (hour * 60.0).round() as u32 % (24 * 60);
    let h = total_minutes / 60;
    let m = total_minutes % 60;
    format!("{:02}:{:02}", h, m)
}

/// `true` between 06:00 and 20:00.
pub fn is_daytime(hour: f32) -> bool {
    let h = hour.rem_euclid(24.0);
    (6.0..20.0).contains(&h)
}

/// Lerps sky colour from deep midnight blue → dawn orange → noon sky → dusk → midnight.
pub fn sky_color(hour: f32) -> Color {
    let h = hour.rem_euclid(24.0);
    // Keyframe colours
    let midnight = Color::from_rgb(0.03, 0.03, 0.12);
    let dawn = Color::from_rgb(0.85, 0.45, 0.15);
    let noon = Color::from_rgb(0.45, 0.72, 0.95);
    let dusk = Color::from_rgb(0.75, 0.35, 0.1);

    let (a, b, t) = if h < 6.0 {
        (midnight, dawn, h / 6.0)
    } else if h < 12.0 {
        (dawn, noon, (h - 6.0) / 6.0)
    } else if h < 20.0 {
        (noon, dusk, (h - 12.0) / 8.0)
    } else {
        (dusk, midnight, (h - 20.0) / 4.0)
    };

    lerp_color(a, b, t)
}

/// Blends two colours, with `t` clamped to `0.0..=1.0`.
pub fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::from_rgb(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
    )
}

/// Name of the current time period.
pub fn time_period(hour: f32) -> &'static str {
    let h = hour.rem_euclid(24.0);
    match h as u32 {
        0..=5 => "Night",
        6..=8 => "Dawn",
        9..=11 => "Morning",
        12..=13 => "Noon",
        14..=17 => "Afternoon",
        18..=19 => "Dusk",
        _ => "Night",
    }
}

// ── GameClockDemo — root Node2D ───────────────────────────────────────────────

/// Godot node advancing an in-game clock and tinting the sky from it.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct GameClockDemo {
    hour: f32,
    speed: f32,
    paused: bool,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for GameClockDemo {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            hour: 6.0,
            speed: 60.0,
            paused: false,
            base,
        }
    }

    fn ready(&mut self) {
        // Sky background
        let mut sky = ColorRect::new_alloc();
        sky.set_name("Sky");
        sky.set_size(Vector2::new(800.0, 600.0));
        sky.set_position(Vector2::new(-400.0, -300.0));
        sky.set_color(sky_color(self.hour));
        self.base_mut().add_child(&sky);

        // Clock label
        let mut clock_lbl = Label::new_alloc();
        clock_lbl.set_name("ClockLabel");
        clock_lbl.set_position(Vector2::new(-60.0, -260.0));
        self.base_mut().add_child(&clock_lbl);

        // Period label
        let mut period_lbl = Label::new_alloc();
        period_lbl.set_name("PeriodLabel");
        period_lbl.set_position(Vector2::new(-60.0, -235.0));
        self.base_mut().add_child(&period_lbl);

        // Speed label
        let mut speed_lbl = Label::new_alloc();
        speed_lbl.set_name("SpeedLabel");
        speed_lbl.set_position(Vector2::new(-390.0, 270.0));
        self.base_mut().add_child(&speed_lbl);

        // Hint
        let mut hint = Label::new_alloc();
        hint.set_text("+/– speed   SPACE pause");
        hint.set_position(Vector2::new(-390.0, -290.0));
        self.base_mut().add_child(&hint);

        self.refresh_ui();
    }

    fn process(&mut self, delta: f64) {
        let input = Input::singleton();

        if input.is_action_just_pressed("ui_accept") {
            self.paused = !self.paused;
        }
        if input.is_action_just_pressed("ui_page_up") {
            self.speed = (self.speed * 2.0).min(3600.0);
        }
        if input.is_action_just_pressed("ui_page_down") {
            self.speed = (self.speed * 0.5).max(1.0);
        }

        if !self.paused {
            self.hour = (self.hour + self.speed * delta as f32 / 3600.0).rem_euclid(24.0);
            self.refresh_ui();
        }
    }
}

#[godot_api]
impl GameClockDemo {
    fn refresh_ui(&mut self) {
        let color = sky_color(self.hour);
        if let Some(mut sky) = self.base().try_get_node_as::<ColorRect>("Sky") {
            sky.set_color(color);
        }
        let time = time_string(self.hour);
        if let Some(mut lbl) = self.base().try_get_node_as::<Label>("ClockLabel") {
            lbl.set_text(&GString::from(time.as_str()));
        }
        let period = time_period(self.hour);
        if let Some(mut lbl) = self.base().try_get_node_as::<Label>("PeriodLabel") {
            lbl.set_text(period);
        }
        let speed_text = format!("Speed: {:.0}×", self.speed);
        if let Some(mut lbl) = self.base().try_get_node_as::<Label>("SpeedLabel") {
            lbl.set_text(&GString::from(speed_text.as_str()));
        }
    }

    /// Current in-game hour, in `0.0..24.0`.
    pub fn get_hour(&self) -> f32 {
        self.hour
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_string_midnight() {
        assert_eq!(time_string(0.0), "00:00");
    }

    #[test]
    fn time_string_noon() {
        assert_eq!(time_string(12.0), "12:00");
    }

    #[test]
    fn time_string_half_past_six() {
        assert_eq!(time_string(6.5), "06:30");
    }

    #[test]
    fn time_string_wraps_at_24() {
        // 24.0 should wrap to 00:00
        assert_eq!(time_string(24.0), "00:00");
    }

    #[test]
    fn is_daytime_at_noon() {
        assert!(is_daytime(12.0));
    }

    #[test]
    fn is_daytime_at_midnight() {
        assert!(!is_daytime(0.0));
    }

    #[test]
    fn is_daytime_at_dawn_boundary() {
        assert!(is_daytime(6.0));
        assert!(!is_daytime(5.99));
    }

    #[test]
    fn sky_color_noon_is_blue_dominant() {
        let c = sky_color(12.0);
        assert!(c.b > c.r, "noon sky should be blue: r={} b={}", c.r, c.b);
    }

    #[test]
    fn sky_color_midnight_is_dark() {
        let c = sky_color(0.0);
        assert!(c.r < 0.1 && c.g < 0.1);
    }

    #[test]
    fn sky_color_varies_across_day() {
        let c_night = sky_color(2.0);
        let c_noon = sky_color(12.0);
        assert!((c_night.b - c_noon.b).abs() > 0.1);
    }

    #[test]
    fn time_period_noon() {
        assert_eq!(time_period(12.5), "Noon");
    }

    #[test]
    fn time_period_night() {
        assert_eq!(time_period(3.0), "Night");
    }
}
