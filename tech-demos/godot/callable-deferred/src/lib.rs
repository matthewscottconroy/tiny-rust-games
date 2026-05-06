//! Callable Deferred demo — `call_deferred` for safe scene-tree mutation from
//! signal handlers; `Callable::from_fn` for inline Rust callbacks.
//!
//! Teaches: why direct `add_child` inside a signal handler is unsafe; how
//! `call_deferred` schedules an operation for the next idle frame; and how
//! `Callable::from_fn` creates anonymous callable objects that Godot can
//! invoke without a method name on a registered class.

use godot::classes::{INode, Label, Node, Timer};
use godot::prelude::*;

// ---------------------------------------------------------------------------
// Extension entry-point
// ---------------------------------------------------------------------------

struct CallableDeferredExtension;
#[gdextension]
unsafe impl ExtensionLibrary for CallableDeferredExtension {}

// ---------------------------------------------------------------------------
// DeferredDemo node
// ---------------------------------------------------------------------------

/// Demonstrates `call_deferred` and `Callable::from_fn`.
///
/// Place this as the scene root with a child `Label` named `"Log"`.
/// A `Timer` child is created in `ready()` and its `timeout` signal is
/// connected to a `Callable::from_fn` closure, which appends a message each
/// tick without needing a named method on the class.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct DeferredDemo {
    /// Simulated delay hint (exported for inspector visibility; not used for
    /// an actual sleep — just illustrates `#[export]` on a f64 field).
    #[export]
    deferred_delay: f64,

    /// Rolling log of operations that have been requested or executed.
    pending_ops: Vec<String>,

    base: Base<Node>,
}

#[godot_api]
impl INode for DeferredDemo {
    fn init(base: Base<Node>) -> Self {
        Self {
            deferred_delay: 0.5,
            pending_ops: Vec::new(),
            base,
        }
    }

    fn ready(&mut self) {
        godot_print!("[DeferredDemo] ready");

        // Build a one-second repeating timer and wire its timeout to a
        // Callable::from_fn closure — no named method required.
        let mut timer = Timer::new_alloc();
        timer.set_wait_time(1.0);
        timer.set_one_shot(false);

        // Callable::from_fn creates an anonymous callable.  The closure
        // receives a slice of Variant arguments and must return a type that
        // implements ToGodot — returning Variant directly satisfies that.
        let callable = Callable::from_fn("log_tick", |_args: &[&Variant]| {
            godot_print!("[DeferredDemo] Callable::from_fn tick fired");
            Variant::nil()
        });
        timer.connect("timeout", &callable);

        let timer_node = timer.upcast::<Node>();
        self.base_mut().add_child(&timer_node);

        // Retrieve the just-added Timer by name and start it.
        let mut timer_ref = self.base().get_node_as::<Timer>("Timer");
        timer_ref.start();

        // Demonstrate request_add_child immediately — it defers the actual
        // add_child to the next frame to show call_deferred in action.
        self.request_add_child();
        self.update_log_label();
    }
}

#[godot_api]
impl DeferredDemo {
    /// Queues `do_add_child` for the next idle frame via `call_deferred`.
    ///
    /// Calling `add_child` directly from within a signal handler while the
    /// scene tree is locked causes a Godot error.  `call_deferred` is the
    /// correct way to schedule tree mutations safely.
    #[func]
    pub fn request_add_child(&mut self) {
        self.pending_ops.push("add_child deferred".into());
        godot_print!("[DeferredDemo] scheduling do_add_child via call_deferred");
        self.base_mut()
            .call_deferred("do_add_child", &[]);
        self.update_log_label();
    }

    /// Actually spawns a `Label` child node.  Called by `call_deferred`; runs
    /// when the scene tree is in a safe writeable state (next idle frame).
    #[func]
    pub fn do_add_child(&mut self) {
        self.pending_ops.push("add_child executed".into());
        godot_print!("[DeferredDemo] do_add_child running — tree is now safe");

        let mut label = Label::new_alloc();
        label.set_text("Deferred child");
        let label_node = label.upcast::<Node>();
        self.base_mut().add_child(&label_node);

        self.update_log_label();
    }

    /// Returns the number of logged operations (deferred requests + executions).
    #[func]
    pub fn get_pending_count(&self) -> i32 {
        self.pending_ops.len() as i32
    }

    // Internal: refresh the Label child with the current op log.
    fn update_log_label(&mut self) {
        if let Some(mut label) = self.base().try_get_node_as::<Label>("Log") {
            let text = format_op_log(&self.pending_ops);
            label.set_text(text.as_str());
        }
    }
}

// ---------------------------------------------------------------------------
// Pure helper functions
// ---------------------------------------------------------------------------

/// Formats a slice of operation names into a newline-delimited string for
/// display on a Label.
///
/// # Examples
/// ```
/// let ops = vec!["add_child deferred".to_string(), "add_child executed".to_string()];
/// let s = callable_deferred::format_op_log(&ops);
/// assert!(s.contains("add_child deferred"));
/// assert!(s.contains("add_child executed"));
/// ```
pub fn format_op_log(ops: &[String]) -> String {
    if ops.is_empty() {
        return "No operations yet.".into();
    }
    ops.iter()
        .enumerate()
        .map(|(i, op)| format!("{}: {}", i + 1, op))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Returns `true` when it is safe to call `add_child` directly — i.e. when
/// we are NOT currently inside a signal handler.  In practice you should
/// always use `call_deferred` if there is any doubt.
///
/// # Examples
/// ```
/// assert!(callable_deferred::is_safe_to_call_direct(false));
/// assert!(!callable_deferred::is_safe_to_call_direct(true));
/// ```
pub fn is_safe_to_call_direct(in_signal: bool) -> bool {
    !in_signal
}

/// Returns a human-readable label for the count of pending deferred calls.
///
/// # Examples
/// ```
/// assert_eq!(callable_deferred::deferred_label(0), "No pending ops");
/// assert_eq!(callable_deferred::deferred_label(1), "1 pending op");
/// assert_eq!(callable_deferred::deferred_label(3), "3 pending ops");
/// ```
pub fn deferred_label(count: i32) -> String {
    match count {
        0 => "No pending ops".into(),
        1 => "1 pending op".into(),
        n => format!("{} pending ops", n),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // format_op_log -----------------------------------------------------------

    #[test]
    fn format_op_log_empty_returns_placeholder() {
        assert_eq!(format_op_log(&[]), "No operations yet.");
    }

    #[test]
    fn format_op_log_single_op() {
        let ops = vec!["add_child deferred".to_string()];
        let s = format_op_log(&ops);
        assert!(s.contains("1: add_child deferred"));
    }

    #[test]
    fn format_op_log_multiple_ops_numbered() {
        let ops = vec!["op_a".to_string(), "op_b".to_string()];
        let s = format_op_log(&ops);
        assert!(s.contains("1: op_a"));
        assert!(s.contains("2: op_b"));
    }

    #[test]
    fn format_op_log_newline_separated() {
        let ops = vec!["x".to_string(), "y".to_string()];
        let s = format_op_log(&ops);
        assert!(s.contains('\n'));
    }

    // is_safe_to_call_direct --------------------------------------------------

    #[test]
    fn is_safe_outside_signal() {
        assert!(is_safe_to_call_direct(false));
    }

    #[test]
    fn is_not_safe_inside_signal() {
        assert!(!is_safe_to_call_direct(true));
    }

    // deferred_label ----------------------------------------------------------

    #[test]
    fn deferred_label_zero() {
        assert_eq!(deferred_label(0), "No pending ops");
    }

    #[test]
    fn deferred_label_one() {
        assert_eq!(deferred_label(1), "1 pending op");
    }

    #[test]
    fn deferred_label_many() {
        assert_eq!(deferred_label(5), "5 pending ops");
    }
}
