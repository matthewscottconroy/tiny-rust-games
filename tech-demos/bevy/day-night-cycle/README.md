# Day-Night Cycle

A demo of **time-of-day ambient colour transitions**: a virtual 24-hour clock drives smooth blends between four key sky colours (midnight → dawn → day → dusk), updating the background, a sun/moon disc, and star visibility every frame. The clock speed is adjustable at runtime.

Press **+** to speed up time, **-** to slow it down, and **R** to reset to midnight.

---

## What This Is Illustrating

Ambient world systems — background processes that run continuously and shape the mood and rules of the game world without requiring direct player interaction. Day-night cycles are one of the most impactful polish additions a game can have; they make the world feel alive and provide a natural rhythm to gameplay sessions.

---

## Game Design Principle: Time as a World Resource

A static world is a backdrop. A world with time feels inhabited. Day-night cycles serve several gameplay functions simultaneously:

- **Mood and atmosphere.** Warm dawn colours feel safe; deep night blues feel tense. Colour alone shifts player psychology.
- **Gameplay modifiers.** Enemies may spawn only at night, shops may close at dusk, crops may grow during daylight. The cycle creates a structure players learn and plan around.
- **Pacing.** A visible time indicator gives players a soft deadline (reach the dungeon before dark) without a hard timer, preserving agency while creating urgency.

**Colour interpolation between key times:**

Rather than storing a colour for every minute of the 24-hour day, the demo defines four palette anchors — Night (0:00), Dawn (6:00), Day (12:00), Dusk (18:00) — and linearly interpolates between adjacent pairs. This produces smooth transitions with minimal data. The interpolation parameter `t` is simply `(current_hour − start_hour) / segment_duration`, clamped to [0, 1].

Linear interpolation of sRGB values is perceptually approximate (gamma-correct interpolation would use linear light space), but for ambient sky colours the difference is invisible and linear interpolation is far cheaper.

---

## How Bevy Achieves It

**Pure colour function.** `time_of_day_to_rgb(hours: f32) -> (f32, f32, f32)` returns a plain RGB tuple with no Bevy dependency. Four constant `(f32, f32, f32)` tuples define the palette; `lerp_f32(a, b, t)` (also pure and tested) handles the blending. The tuple return type means tests can assert channel values with simple floating-point comparisons.

**`ClearColor` resource as the sky.** Bevy clears the framebuffer to the `ClearColor` resource value before rendering each frame. The `update_sky` system updates `clear.0` with `Color::srgb(r, g, b)` from the interpolated tuple, making the background itself the sky — no additional backdrop sprite is needed.

**`DayClock` resource with a speed multiplier.** The `DayClock` resource stores `hours: f32` and `speed: f32` (virtual hours per real second). `advance_clock` adds `speed × delta_time` to `hours` and wraps with `.rem_euclid(24.0)`. Pressing `+` multiplies speed by 1.5 (up to a cap); pressing `-` divides it. This makes the clock rate a first-class runtime value rather than a compile-time constant.

**Separate queries for sun/moon and stars.** `update_sky` holds two queries: `Query<(&mut Transform, &mut Sprite), With<SunMoon>>` and `Query<(&StarSprite, &mut Sprite), Without<SunMoon>>`. Bevy requires that overlapping component access in the same system uses disjoint filters (`With`/`Without`) to prove they cannot alias. The `StarSprite(f32)` component stores each star's base alpha; the system multiplies it by the computed `star_alpha` to fade stars in and out.

---

## Running

```
cargo run
```

Controls: **+/=** — faster · **-** — slower · **R** — reset to midnight.
