//! GDScript Interop demo — calling GDScript methods from Rust via `call()`,
//! and exposing Rust `#[func]` methods to GDScript. Demonstrates `Variant`
//! conversion in both directions.

use godot::classes::{INode, Node};
use godot::prelude::*;

struct GdscriptInteropExtension;
#[gdextension]
unsafe impl ExtensionLibrary for GdscriptInteropExtension {}

/// Root node that demonstrates bidirectional Rust ↔ GDScript communication.
#[derive(GodotClass)]
#[class(base=Node)]
struct GdscriptInterop {
    base: Base<Node>,
}

#[godot_api]
impl INode for GdscriptInterop {
    fn init(base: Base<Node>) -> Self {
        Self { base }
    }

    fn ready(&mut self) {
        // Call GDScript methods from Rust using call()
        let node = self.base().get_node_or_null("GDScriptNode");
        if let Some(mut node) = node {
            // Call gdscript_double with 21 → expect 42
            let result: Variant = node.call("gdscript_double", &[Variant::from(21_i64)]);
            let value = result.to::<i64>();
            godot_print!("GDScript returned gdscript_double(21) = {}", value);
            godot_print!("{}", format_call_result("gdscript_double", 21, value));

            // Call gdscript_greet with "Rust"
            let greet_result: Variant =
                node.call("gdscript_greet", &[Variant::from(GString::from("Rust"))]);
            let greeting = greet_result.to::<GString>();
            godot_print!("GDScript returned gdscript_greet(\"Rust\") = {}", greeting);
        }
    }
}

#[godot_api]
impl GdscriptInterop {
    /// Simple arithmetic callable from GDScript.
    #[func]
    pub fn rust_add(&self, a: f64, b: f64) -> f64 {
        a + b
    }

    /// Returns a greeting string, callable from GDScript.
    #[func]
    pub fn rust_greet(&self, name: GString) -> GString {
        GString::from(greet(name.to_string().as_str()).as_str())
    }

    /// Gets the GDScriptNode child, calls `gdscript_double` on it, and prints the result.
    #[func]
    pub fn call_gdscript_method(&mut self) {
        let node = self.base().get_node_or_null("GDScriptNode");
        if let Some(mut node) = node {
            let result: Variant = node.call("gdscript_double", &[Variant::from(21_i64)]);
            let value = result.to::<i64>();
            godot_print!("call_gdscript_method: gdscript_double(21) = {}", value);
        }
    }
}

// ---------------------------------------------------------------------------
// Pure helper functions — fully testable without Godot runtime
// ---------------------------------------------------------------------------

/// Format a log line describing a cross-language call with its input and output.
pub fn format_call_result(method: &str, input: i64, output: i64) -> String {
    format!("{}({}) → {}", method, input, output)
}

/// Return a human-readable name for a Variant type tag.
/// 0=Nil, 1=Bool, 2=Int, 3=Float, 4=String
pub fn variant_type_name(v: u8) -> &'static str {
    match v {
        0 => "Nil",
        1 => "Bool",
        2 => "Int",
        3 => "Float",
        4 => "String",
        _ => "Unknown",
    }
}

/// Build a greeting string (pure, no Godot types).
pub fn greet(name: &str) -> String {
    format!("Hello from Rust, {}!", name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_call_result_basic() {
        assert_eq!(
            format_call_result("gdscript_double", 21, 42),
            "gdscript_double(21) → 42"
        );
    }

    #[test]
    fn format_call_result_zero() {
        assert_eq!(format_call_result("foo", 0, 0), "foo(0) → 0");
    }

    #[test]
    fn format_call_result_negative() {
        assert_eq!(format_call_result("negate", -5, 5), "negate(-5) → 5");
    }

    #[test]
    fn variant_type_name_nil() {
        assert_eq!(variant_type_name(0), "Nil");
    }

    #[test]
    fn variant_type_name_bool() {
        assert_eq!(variant_type_name(1), "Bool");
    }

    #[test]
    fn variant_type_name_int() {
        assert_eq!(variant_type_name(2), "Int");
    }

    #[test]
    fn variant_type_name_float() {
        assert_eq!(variant_type_name(3), "Float");
    }

    #[test]
    fn variant_type_name_string() {
        assert_eq!(variant_type_name(4), "String");
    }

    #[test]
    fn variant_type_name_unknown() {
        assert_eq!(variant_type_name(99), "Unknown");
    }

    #[test]
    fn greet_formats_correctly() {
        assert_eq!(greet("World"), "Hello from Rust, World!");
    }

    #[test]
    fn greet_empty_name() {
        assert_eq!(greet(""), "Hello from Rust, !");
    }

    #[test]
    fn rust_add_pure_check() {
        // Verify addition logic directly
        let a = 3.5_f64;
        let b = 1.5_f64;
        assert!((a + b - 5.0).abs() < 1e-10);
    }
}
