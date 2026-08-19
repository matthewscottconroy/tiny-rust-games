//! State Machine demo — Godot 4.3 + gdext 0.5
//!
//! Demonstrates a pure-Rust finite state machine (FSM) driving a Node2D
//! subclass.  Input flags are read every frame and fed into a stateless
//! `transition` function; the resulting state is displayed on a child Label.
//!
//! Teaches: a stateless `transition` function driving a finite state machine, so the
//! transition rules are unit-testable without an engine.
//!
//! Concept: state-machine
//!
//! Counterpart: tech-demos/bevy/state-machine-ai — the same concept in Bevy.

use godot::classes::{INode2D, Input, Label, Node2D};
use godot::prelude::*;

// ---------------------------------------------------------------------------
// Extension entry-point
// ---------------------------------------------------------------------------

struct StateMachineExtension;
#[gdextension]
unsafe impl ExtensionLibrary for StateMachineExtension {}

// ---------------------------------------------------------------------------
// Pure-Rust FSM types
// ---------------------------------------------------------------------------

/// The four states the character can be in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsmState {
    /// Standing still.
    Idle,
    /// Moving under player input.
    Walk,
    /// Airborne.
    Jump,
    /// Playing the attack action.
    Attack,
}

/// Convert a `FsmState` to its u8 discriminant for cross-boundary storage.
pub fn state_to_u8(s: FsmState) -> u8 {
    match s {
        FsmState::Idle => 0,
        FsmState::Walk => 1,
        FsmState::Jump => 2,
        FsmState::Attack => 3,
    }
}

/// Convert a u8 discriminant back to a `FsmState`.
pub fn u8_to_state(v: u8) -> FsmState {
    match v {
        1 => FsmState::Walk,
        2 => FsmState::Jump,
        3 => FsmState::Attack,
        _ => FsmState::Idle,
    }
}

// ---------------------------------------------------------------------------
// Pure helper functions (all testable without Godot runtime)
// ---------------------------------------------------------------------------

/// Compute the next state given the current state and input flags.
pub fn transition(state: u8, moving: bool, jumping: bool, attacking: bool) -> u8 {
    let current = u8_to_state(state);
    let next = match current {
        FsmState::Idle => {
            if attacking {
                FsmState::Attack
            } else if jumping {
                FsmState::Jump
            } else if moving {
                FsmState::Walk
            } else {
                FsmState::Idle
            }
        }
        FsmState::Walk => {
            if attacking {
                FsmState::Attack
            } else if jumping {
                FsmState::Jump
            } else if moving {
                FsmState::Walk
            } else {
                FsmState::Idle
            }
        }
        FsmState::Jump => {
            // Can attack mid-air; land when not jumping
            if attacking {
                FsmState::Attack
            } else if jumping {
                FsmState::Jump
            } else {
                FsmState::Idle
            }
        }
        FsmState::Attack => {
            // Attack finishes when button released
            if attacking {
                FsmState::Attack
            } else if jumping {
                FsmState::Jump
            } else if moving {
                FsmState::Walk
            } else {
                FsmState::Idle
            }
        }
    };
    state_to_u8(next)
}

/// Human-readable name for a state discriminant.
pub fn state_name(s: u8) -> &'static str {
    match s {
        0 => "Idle",
        1 => "Walk",
        2 => "Jump",
        3 => "Attack",
        _ => "Unknown",
    }
}

/// Returns true for states that have no outgoing transitions (none in this
/// simple FSM, but the function is defined to satisfy the spec).
pub fn is_terminal(s: u8) -> bool {
    // No state is truly terminal in this looping FSM
    let _ = s;
    false
}

// ---------------------------------------------------------------------------
// GodotClass
// ---------------------------------------------------------------------------

/// Godot node driving the state enum from input and exposing it to the editor.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct StateMachine {
    current: u8,
    #[export]
    speed: f32,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for StateMachine {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            current: state_to_u8(FsmState::Idle),
            speed: 200.0,
            base,
        }
    }

    fn process(&mut self, _delta: f64) {
        let input = Input::singleton();
        let moving = input.is_action_pressed("ui_right")
            || input.is_action_pressed("ui_left")
            || input.is_action_pressed("ui_down")
            || input.is_action_pressed("ui_up");
        let jumping = input.is_action_pressed("ui_accept");
        let attacking = input.is_action_pressed("ui_select");

        self.current = transition(self.current, moving, jumping, attacking);

        let name = state_name(self.current);
        if let Some(node) = self.base().get_node_or_null("Label")
            && let Ok(mut label) = node.try_cast::<Label>()
        {
            label.set_text(name);
        }
    }
}

#[godot_api]
impl StateMachine {
    /// Expose the current state discriminant to GDScript.
    #[func]
    pub fn get_current_state(&self) -> i32 {
        self.current as i32
    }

    /// Expose the state name to GDScript.
    #[func]
    pub fn get_state_name(&self) -> GString {
        GString::from(state_name(self.current))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_no_input_stays_idle() {
        assert_eq!(transition(0, false, false, false), 0);
    }

    #[test]
    fn idle_move_transitions_to_walk() {
        assert_eq!(transition(0, true, false, false), 1);
    }

    #[test]
    fn idle_jump_transitions_to_jump() {
        assert_eq!(transition(0, false, true, false), 2);
    }

    #[test]
    fn idle_attack_transitions_to_attack() {
        assert_eq!(transition(0, false, false, true), 3);
    }

    #[test]
    fn walk_no_input_returns_to_idle() {
        assert_eq!(transition(1, false, false, false), 0);
    }

    #[test]
    fn jump_landing_returns_to_idle() {
        // jumping flag released while in Jump state
        assert_eq!(transition(2, false, false, false), 0);
    }

    #[test]
    fn attack_released_while_moving_goes_walk() {
        assert_eq!(transition(3, true, false, false), 1);
    }

    #[test]
    fn state_name_covers_all_variants() {
        assert_eq!(state_name(0), "Idle");
        assert_eq!(state_name(1), "Walk");
        assert_eq!(state_name(2), "Jump");
        assert_eq!(state_name(3), "Attack");
        assert_eq!(state_name(99), "Unknown");
    }

    #[test]
    fn is_terminal_always_false() {
        for s in 0..=3 {
            assert!(!is_terminal(s));
        }
    }

    #[test]
    fn roundtrip_u8_state() {
        for v in 0..4u8 {
            assert_eq!(state_to_u8(u8_to_state(v)), v);
        }
    }
}
