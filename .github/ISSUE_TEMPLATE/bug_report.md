---
name: Bug report
about: Something does not work as documented
labels: bug
---

**What happened, and what you expected instead**

**Which crate**
e.g. `tech-demos/bevy/boids-flocking`, or `snake-lib`.

**How to reproduce**
The exact command, please — the three crate layouts here take different ones:

```bash
# e.g.
just bevy boids-flocking
cargo test --manifest-path snake/snake-lib/Cargo.toml
```

**Environment**
- OS:
- `rustc --version`:
- For Godot demos, `godot --version`:

**Does `just ci` pass on your machine?**
If it fails, the output is usually the fastest route to the cause.
