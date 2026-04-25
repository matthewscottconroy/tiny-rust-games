# Simulated 2D Lighting

A demo of **per-sprite distance-based lighting**: three coloured lights orbit the arena and the player carries a fourth. Every tile's colour is recomputed each frame based on the combined contribution of all nearby lights, using a quadratic falloff. An adjustable ambient level prevents total darkness.

**WASD / Arrow keys** — move your light. **+** / **-** — raise or lower ambient.

---

## What This Is Illustrating

Atmospheric lighting as a game mechanic — how light and shadow communicate danger, guide attention, and shape mood. True GPU-based dynamic lighting is complex to implement. This demo shows a CPU-side approximation that is simple to understand, fully portable, and effective for 2D grid games.

---

## Game Design Principle: Light as Communication and Atmosphere

Light in a game world is not just a visual detail — it is a gameplay signal. Players move toward light (safety, resources, exits) and away from darkness (threat, the unknown). A game that controls lighting controls the player's emotional state.

**Distance-based falloff:** The most important property of a point light is that its intensity decreases with distance. A linear falloff (`1 − dist/radius`) looks flat and artificial. A quadratic falloff (`(1 − dist/radius)²`) produces a bright centre with a softer outer ring that matches how light actually spreads. The demo uses quadratic falloff for each light source.

**Additive multi-light blending:** Multiple lights contribute additively — a tile illuminated by both a red and a blue light blends both contributions and clamps the result to 1.0. This is physically accurate (light adds, not averages) and produces the characteristic colour overlap you see in real theatrical or game lighting.

**Ambient as a floor:** A small ambient level (default 0.04) ensures that even tiles receiving no light from any source remain barely visible. This models the scattered background light present in any real environment. Setting ambient to 0 produces pure black tiles outside all light radii, which is useful for horror settings; raising it above 0.3 begins to wash out the lighting effect.

---

## How Bevy Achieves It

**Pure lighting functions.** `light_intensity(pos: Vec2, lights: &[(Vec2, f32)]) -> f32` accumulates quadratic contributions from a slice of `(position, radius)` pairs. `apply_light_intensity(base: (f32, f32, f32), intensity: f32, ambient: f32) -> (f32, f32, f32)` scales a base RGB colour by `(intensity + ambient).min(1.0)`. Both operate on plain Rust types with no Bevy dependency and are independently unit-tested.

**`LitTile` component caches world position.** Each tile sprite carries a `LitTile { world_pos: Vec2, base_color: (f32, f32, f32) }` component. The world position is set once at spawn and never changes (tiles do not move). Caching it on the component avoids extracting position from `Transform` every frame, which would require reading three floats and ignoring the Z.

**Per-channel coloured light accumulation.** Rather than calling the pure `light_intensity` function (which produces a greyscale scalar), the `apply_lighting` system accumulates per-channel RGB contributions from each coloured orbit light. Each light's colour `(r, g, b)` multiplies its intensity contribution before adding. This produces the coloured overlap effect. The player's light is hardcoded as warm white `(1.0, 0.95, 0.7)`.

**`OrbitLights` resource drives autonomous animation.** Three `OrbitLight` structs stored in a resource each carry an angle, orbit radius, angular speed, colour, and reach radius. The `advance_orbits` system increments each light's angle by `speed × delta_secs` every frame. No entity is needed for the lights themselves — they exist only as data. Light visual dots are separate entities updated from the resource in `apply_lighting`.

**No shader, no render plugin.** The entire effect is achieved by setting `Sprite::color` on existing sprite entities every frame. This is slower than a GPU shader (O(tiles × lights) CPU work per frame) but requires zero graphics API knowledge, works on all backends, and is completely debuggable with print statements.

---

## Running

```
cargo run
```

Controls: **WASD / Arrow keys** — move player light · **+** — brighter ambient · **-** — darker ambient.
