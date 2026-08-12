## What this changes

## Why

The diff shows the what; this is the part a future reader cannot reconstruct.

## Checklist

- [ ] `just ci` passes (fmt, clippy `-D warnings` including `missing_docs`,
      tests, `--locked`)
- [ ] hooks installed (`just install-hooks`) — they catch most of the above in
      under a second
- [ ] new public items have `///` docs
- [ ] new demo: read the relevant `DEMO_ANATOMY.md`, copied `_template/`, has a
      `#[cfg(test)] mod tests`, and (Godot only) a `//! Teaches:` line
- [ ] added or changed a demo: ran `just catalogue`
- [ ] touched a shared pure function that exists in both engine suites: changed
      both copies

Delete whatever does not apply.
