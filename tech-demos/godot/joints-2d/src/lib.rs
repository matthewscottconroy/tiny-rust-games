//! Joints 2D GDExtension demo — connect physics bodies with PinJoint2D and
//! DampedSpringJoint2D entirely from Rust.
//!
//! Demonstrates:
//!
//! - Spawning `RigidBody2D` nodes with `CollisionShape2D` children at runtime.
//! - Creating a `PinJoint2D` and connecting two bodies via `set_node_a`/`set_node_b`.
//! - Creating a `DampedSpringJoint2D` with configurable stiffness and damping.
//! - Getting node paths after `add_child` via `body.get_path()`.
//! - Displaying joint count on a `Label` child.

use godot::classes::{
    CircleShape2D, CollisionShape2D, DampedSpringJoint2D, INode2D, Label, Node2D, PinJoint2D,
    RigidBody2D,
};
use godot::prelude::*;

// ─── Extension entry point ───────────────────────────────────────────────────

struct Joints2DExt;

#[gdextension]
unsafe impl ExtensionLibrary for Joints2DExt {}

// ─── Pure functions ───────────────────────────────────────────────────────────

/// Computes the restoring force of a spring given displacement and stiffness.
///
/// `force = displacement * stiffness` (Hooke's law, unsigned).
///
/// # Examples
/// ```
/// assert!((joints_2d::spring_force(10.0, 20.0) - 200.0).abs() < 1e-4);
/// assert!((joints_2d::spring_force(0.0, 20.0)).abs() < 1e-4);
/// ```
pub fn spring_force(displacement: f32, stiffness: f32) -> f32 {
    displacement * stiffness
}

/// Computes the damping force opposing motion.
///
/// `force = velocity * damping` (unsigned magnitude).
///
/// # Examples
/// ```
/// assert!((joints_2d::damping_force(5.0, 2.0) - 10.0).abs() < 1e-4);
/// assert!((joints_2d::damping_force(0.0, 2.0)).abs() < 1e-4);
/// ```
pub fn damping_force(velocity: f32, damping: f32) -> f32 {
    velocity * damping
}

/// Returns a label string describing the number of active joints.
///
/// # Examples
/// ```
/// assert_eq!(joints_2d::joint_label(0), "Joints: 0");
/// assert_eq!(joints_2d::joint_label(2), "Joints: 2");
/// ```
pub fn joint_label(count: i32) -> String {
    format!("Joints: {}", count)
}

// ─── JointDemo node ───────────────────────────────────────────────────────────

/// Scene root that programmatically creates two pairs of physics bodies,
/// connecting them with `PinJoint2D` and `DampedSpringJoint2D`.
///
/// Add a `Label` child named `"Label"` in the Godot editor to see joint count.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct JointDemo {
    /// Stiffness of the damped spring joint.
    #[export]
    spring_stiffness: f32,
    /// Damping coefficient of the damped spring joint.
    #[export]
    spring_damping: f32,
    /// Number of joints created.
    joint_count: i32,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for JointDemo {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            spring_stiffness: 20.0,
            spring_damping: 1.0,
            joint_count: 0,
            base,
        }
    }

    fn ready(&mut self) {
        self.setup_pin_joint_pair();
        self.setup_spring_joint_pair();

        let count = self.joint_count;
        let text = joint_label(count);
        if let Some(mut label) = self.base_mut().try_get_node_as::<Label>("Label") {
            label.set_text(text.as_str());
        }

        godot_print!("[JointDemo] Ready — {} joints created.", self.joint_count);
    }
}

#[godot_api]
impl JointDemo {
    /// Returns the number of joints created during `ready`.
    #[func]
    pub fn get_joint_count(&self) -> i32 {
        self.joint_count
    }

    /// Spawns two `RigidBody2D` nodes and connects them with a `PinJoint2D`.
    fn setup_pin_joint_pair(&mut self) {
        // Body A — left side.
        let mut body_a = RigidBody2D::new_alloc();
        body_a.set_position(Vector2::new(-150.0, 0.0));
        body_a.set_name("BodyPinA");
        let shape_a = CircleShape2D::new_gd();
        let mut col_a = CollisionShape2D::new_alloc();
        col_a.set_shape(&shape_a);
        let col_a_node: Gd<Node> = col_a.upcast();
        body_a.add_child(&col_a_node);
        let body_a_node: Gd<Node> = body_a.clone().upcast();
        self.base_mut().add_child(&body_a_node);
        let path_a = body_a.get_path();

        // Body B — right side.
        let mut body_b = RigidBody2D::new_alloc();
        body_b.set_position(Vector2::new(150.0, 0.0));
        body_b.set_name("BodyPinB");
        let shape_b = CircleShape2D::new_gd();
        let mut col_b = CollisionShape2D::new_alloc();
        col_b.set_shape(&shape_b);
        let col_b_node: Gd<Node> = col_b.upcast();
        body_b.add_child(&col_b_node);
        let body_b_node: Gd<Node> = body_b.clone().upcast();
        self.base_mut().add_child(&body_b_node);
        let path_b = body_b.get_path();

        // PinJoint2D between them.
        let mut pin = PinJoint2D::new_alloc();
        pin.set_position(Vector2::ZERO);
        pin.set_node_a(&path_a);
        pin.set_node_b(&path_b);
        let pin_node: Gd<Node> = pin.upcast();
        self.base_mut().add_child(&pin_node);

        self.joint_count += 1;
    }

    /// Spawns two more `RigidBody2D` nodes and connects them with a `DampedSpringJoint2D`.
    fn setup_spring_joint_pair(&mut self) {
        let stiffness = self.spring_stiffness;
        let damping = self.spring_damping;

        // Body C — left side, lower on screen.
        let mut body_c = RigidBody2D::new_alloc();
        body_c.set_position(Vector2::new(-150.0, 200.0));
        body_c.set_name("BodySpringC");
        let shape_c = CircleShape2D::new_gd();
        let mut col_c = CollisionShape2D::new_alloc();
        col_c.set_shape(&shape_c);
        let col_c_node: Gd<Node> = col_c.upcast();
        body_c.add_child(&col_c_node);
        let body_c_node: Gd<Node> = body_c.clone().upcast();
        self.base_mut().add_child(&body_c_node);
        let path_c = body_c.get_path();

        // Body D — right side, lower on screen.
        let mut body_d = RigidBody2D::new_alloc();
        body_d.set_position(Vector2::new(150.0, 200.0));
        body_d.set_name("BodySpringD");
        let shape_d = CircleShape2D::new_gd();
        let mut col_d = CollisionShape2D::new_alloc();
        col_d.set_shape(&shape_d);
        let col_d_node: Gd<Node> = col_d.upcast();
        body_d.add_child(&col_d_node);
        let body_d_node: Gd<Node> = body_d.clone().upcast();
        self.base_mut().add_child(&body_d_node);
        let path_d = body_d.get_path();

        // DampedSpringJoint2D connecting them.
        let mut spring = DampedSpringJoint2D::new_alloc();
        spring.set_position(Vector2::new(0.0, 200.0));
        spring.set_node_a(&path_c);
        spring.set_node_b(&path_d);
        spring.set_stiffness(stiffness);
        spring.set_damping(damping);
        let spring_node: Gd<Node> = spring.upcast();
        self.base_mut().add_child(&spring_node);

        self.joint_count += 1;
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // spring_force ────────────────────────────────────────────────────────────

    #[test]
    fn spring_force_hooke_law() {
        assert!((spring_force(10.0, 20.0) - 200.0).abs() < 1e-4);
    }

    #[test]
    fn spring_force_zero_displacement() {
        assert!(spring_force(0.0, 20.0).abs() < 1e-4);
    }

    #[test]
    fn spring_force_zero_stiffness() {
        assert!(spring_force(10.0, 0.0).abs() < 1e-4);
    }

    // damping_force ───────────────────────────────────────────────────────────

    #[test]
    fn damping_force_product() {
        assert!((damping_force(5.0, 2.0) - 10.0).abs() < 1e-4);
    }

    #[test]
    fn damping_force_zero_velocity() {
        assert!(damping_force(0.0, 2.0).abs() < 1e-4);
    }

    // joint_label ─────────────────────────────────────────────────────────────

    #[test]
    fn joint_label_zero() {
        assert_eq!(joint_label(0), "Joints: 0");
    }

    #[test]
    fn joint_label_two() {
        assert_eq!(joint_label(2), "Joints: 2");
    }

    #[test]
    fn joint_label_large() {
        assert_eq!(joint_label(99), "Joints: 99");
    }
}
