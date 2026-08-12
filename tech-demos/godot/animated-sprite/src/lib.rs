//! Animated Sprite demo — switches between Idle, Walk, and Attack states
//! based on user input, driving an AnimatedSprite2D child node.

use godot::classes::{AnimatedSprite2D, INode2D, Input, Node2D};
use godot::prelude::*;

struct AnimatedSpriteExtension;
#[gdextension]
unsafe impl ExtensionLibrary for AnimatedSpriteExtension {}

/// Animation state for the character.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AnimState {
    Idle,
    Walk,
    Attack,
}

#[derive(GodotClass)]
#[class(base=Node2D)]
struct AnimatedSpritePlayer {
    #[export]
    walk_speed: f32,

    state: AnimState,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for AnimatedSpritePlayer {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            walk_speed: 100.0,
            state: AnimState::Idle,
            base,
        }
    }

    fn ready(&mut self) {
        let mut sprite = self
            .base()
            .get_node_as::<AnimatedSprite2D>("AnimatedSprite2D");
        sprite.play_ex().name("idle").done();
    }

    fn process(&mut self, _delta: f64) {
        let input = Input::singleton();
        let moving = input.is_action_pressed("ui_right") || input.is_action_pressed("ui_left");
        let attacking = input.is_action_just_pressed("ui_accept");

        let state_byte = anim_state_to_u8(self.state);
        let new_state_byte = next_anim_state(state_byte, moving, attacking);
        let new_state = u8_to_anim_state(new_state_byte);

        if new_state != self.state {
            self.state = new_state;
            let name = anim_name(new_state_byte);
            let mut sprite = self
                .base()
                .get_node_as::<AnimatedSprite2D>("AnimatedSprite2D");
            sprite.play_ex().name(name).done();
        }

        if moving {
            let speed = self.walk_speed;
            let dir = if input.is_action_pressed("ui_right") {
                1.0_f32
            } else {
                -1.0_f32
            };
            let pos = self.base().get_position();
            self.base_mut()
                .set_position(Vector2::new(pos.x + dir * speed * _delta as f32, pos.y));
        }
    }
}

#[godot_api]
impl AnimatedSpritePlayer {
    #[func]
    pub fn force_idle(&mut self) {
        self.state = AnimState::Idle;
        let mut sprite = self
            .base()
            .get_node_as::<AnimatedSprite2D>("AnimatedSprite2D");
        sprite.play_ex().name("idle").done();
    }
}

// ---------------------------------------------------------------------------
// Pure helper functions
// ---------------------------------------------------------------------------

/// Convert an AnimState to its u8 discriminant.
pub fn anim_state_to_u8(state: AnimState) -> u8 {
    match state {
        AnimState::Idle => 0,
        AnimState::Walk => 1,
        AnimState::Attack => 2,
    }
}

/// Convert a u8 discriminant back to AnimState.
pub fn u8_to_anim_state(v: u8) -> AnimState {
    match v {
        1 => AnimState::Walk,
        2 => AnimState::Attack,
        _ => AnimState::Idle,
    }
}

/// Determine the next animation state.
/// - attacking takes highest priority
/// - moving comes next
/// - otherwise idle
///
/// Returns 0=Idle, 1=Walk, 2=Attack
pub fn next_anim_state(current: u8, moving: bool, attacking: bool) -> u8 {
    if attacking {
        return 2;
    }
    // If we were attacking and no longer attacking, go back to idle/walk
    if current == 2 {
        return if moving { 1 } else { 0 };
    }
    if moving { 1 } else { 0 }
}

/// Return the animation clip name for a given state byte.
pub fn anim_name(state: u8) -> &'static str {
    match state {
        1 => "walk",
        2 => "attack",
        _ => "idle",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_when_not_moving_not_attacking() {
        assert_eq!(next_anim_state(0, false, false), 0);
    }

    #[test]
    fn walk_when_moving() {
        assert_eq!(next_anim_state(0, true, false), 1);
    }

    #[test]
    fn attack_takes_priority_over_walk() {
        assert_eq!(next_anim_state(1, true, true), 2);
    }

    #[test]
    fn attack_takes_priority_over_idle() {
        assert_eq!(next_anim_state(0, false, true), 2);
    }

    #[test]
    fn after_attack_returns_to_walk_if_moving() {
        assert_eq!(next_anim_state(2, true, false), 1);
    }

    #[test]
    fn after_attack_returns_to_idle_if_not_moving() {
        assert_eq!(next_anim_state(2, false, false), 0);
    }

    #[test]
    fn anim_name_idle() {
        assert_eq!(anim_name(0), "idle");
    }

    #[test]
    fn anim_name_walk() {
        assert_eq!(anim_name(1), "walk");
    }

    #[test]
    fn anim_name_attack() {
        assert_eq!(anim_name(2), "attack");
    }

    #[test]
    fn anim_name_unknown_defaults_to_idle() {
        assert_eq!(anim_name(99), "idle");
    }

    #[test]
    fn roundtrip_state_conversion() {
        for v in 0u8..=2 {
            assert_eq!(anim_state_to_u8(u8_to_anim_state(v)), v);
        }
    }

    #[test]
    fn u8_out_of_range_defaults_idle() {
        assert_eq!(anim_state_to_u8(u8_to_anim_state(42)), 0);
    }
}
