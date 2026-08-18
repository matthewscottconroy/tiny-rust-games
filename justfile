# Task runner for tiny-rust-games. `just` alone lists every recipe.
#
# The repository holds three kinds of crate — one Bevy workspace, 67 standalone
# Godot crates, and four other standalone crates — so "run this across the repo"
# is three different command shapes. These recipes wrap that up so you never
# have to remember which is which, and so `just ci` reproduces exactly what
# .github/workflows/ci.yml enforces.

bevy_manifest := "tech-demos/bevy/Cargo.toml"
# Godot crates compile gdext once each unless they share a target directory.
# Safe locally for the same reason it is safe in CI: only running a demo inside
# Godot depends on the per-crate target/ layout.
godot_target := justfile_directory() / "tech-demos/godot/.shared-target"

default:
    @just --list

# Everything CI enforces: format, lint, and test every crate.
ci: fmt-check clippy test

# --- Formatting ---

# Reformat every crate.
fmt:
    cargo fmt --manifest-path {{bevy_manifest}} --all
    @for m in tech-demos/godot/*/Cargo.toml tech-demos/brackets/*/Cargo.toml tic-tac-toe/*/Cargo.toml \
             snake/snake-lib/Cargo.toml snake/snake-godot/Cargo.toml \
             breakout/breakout-lib/Cargo.toml breakout/breakout-godot/Cargo.toml; do \
        cargo fmt --manifest-path "$m" --all; \
    done

# Fail if any crate is unformatted.
fmt-check:
    @failed=""; \
    for m in {{bevy_manifest}} tech-demos/godot/*/Cargo.toml tech-demos/brackets/*/Cargo.toml \
             tic-tac-toe/*/Cargo.toml snake/snake-lib/Cargo.toml snake/snake-godot/Cargo.toml \
             breakout/breakout-lib/Cargo.toml breakout/breakout-godot/Cargo.toml; do \
        cargo fmt --manifest-path "$m" --all --check >/dev/null 2>&1 || failed="$failed $m"; \
    done; \
    if [ -n "$failed" ]; then echo "Unformatted:$failed"; echo "Run 'just fmt'."; exit 1; fi; \
    echo "All crates formatted."

# --- Linting ---

# Clippy with warnings denied, across every crate.
clippy: clippy-bevy clippy-godot clippy-misc

clippy-bevy:
    cargo clippy --locked --workspace --all-targets --manifest-path {{bevy_manifest}} -- -D warnings

clippy-godot:
    @failed=""; \
    for m in tech-demos/godot/*/Cargo.toml tic-tac-toe/tic-tac-toe-godot/Cargo.toml \
             snake/snake-godot/Cargo.toml breakout/breakout-godot/Cargo.toml; do \
        CARGO_TARGET_DIR={{godot_target}} cargo clippy --locked --manifest-path "$m" --all-targets -- -D warnings \
            || failed="$failed $(basename $(dirname $m))"; \
    done; \
    if [ -n "$failed" ]; then echo "Clippy failed:$failed"; exit 1; fi

clippy-misc:
    @failed=""; \
    for m in tic-tac-toe/tic-tac-toe-lib/Cargo.toml tic-tac-toe/tic-tac-toe-cli/Cargo.toml \
             tic-tac-toe/tic-tac-toe-brackets/Cargo.toml snake/snake-lib/Cargo.toml \
             breakout/breakout-lib/Cargo.toml \
             tech-demos/brackets/*/Cargo.toml; do \
        CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo clippy --locked --manifest-path "$m" --all-targets -- -D warnings \
            || failed="$failed $m"; \
    done; \
    if [ -n "$failed" ]; then echo "Clippy failed:$failed"; exit 1; fi

# --- Tests ---

# Test every crate.
test: test-bevy test-godot test-misc

test-bevy:
    cargo test --locked --workspace --manifest-path {{bevy_manifest}}

test-godot:
    @failed=""; \
    for m in tech-demos/godot/*/Cargo.toml tic-tac-toe/tic-tac-toe-godot/Cargo.toml \
             snake/snake-godot/Cargo.toml breakout/breakout-godot/Cargo.toml; do \
        CARGO_TARGET_DIR={{godot_target}} cargo test --locked --manifest-path "$m" \
            || failed="$failed $(basename $(dirname $m))"; \
    done; \
    if [ -n "$failed" ]; then echo "Tests failed:$failed"; exit 1; fi

test-misc:
    @failed=""; \
    for m in tic-tac-toe/tic-tac-toe-lib/Cargo.toml tic-tac-toe/tic-tac-toe-cli/Cargo.toml \
             tic-tac-toe/tic-tac-toe-brackets/Cargo.toml snake/snake-lib/Cargo.toml \
             breakout/breakout-lib/Cargo.toml \
             tech-demos/brackets/*/Cargo.toml; do \
        CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo test --locked --manifest-path "$m" \
            || failed="$failed $m"; \
    done; \
    if [ -n "$failed" ]; then echo "Tests failed:$failed"; exit 1; fi

# --- Running demos ---

# Run a Bevy demo: `just bevy boids-flocking`
bevy demo:
    cargo run --manifest-path {{bevy_manifest}} -p {{demo}}

# Then open tech-demos/godot/<demo>/project.godot in Godot 4.3+.
# Build a Godot demo's extension: `just godot hex-grid`
godot demo:
    cargo build --manifest-path tech-demos/godot/{{demo}}/Cargo.toml

# Play Breakout in a Bevy window.
breakout:
    cargo run --manifest-path {{bevy_manifest}} -p breakout-bevy

# Play Snake in a Bevy window.
snake:
    cargo run --manifest-path {{bevy_manifest}} -p snake-bevy

# Play tic-tac-toe in the terminal.
play:
    cargo run --manifest-path tic-tac-toe/tic-tac-toe-cli/Cargo.toml

# Play tic-tac-toe in a Bevy window.
play-bevy:
    cargo run --manifest-path {{bevy_manifest}} -p tic-tac-toe-bevy

# Build the Godot tic-tac-toe extension, then open its project.godot in Godot.
play-godot:
    cargo build --manifest-path tic-tac-toe/tic-tac-toe-godot/Cargo.toml

# Needs Godot 4.3+ on PATH and the crates built (just test-godot).
# Load every Godot project headlessly and check its Rust classes register.
validate-godot:
    tools/validate-godot.sh

# Run the benchmarks that check the demos' documented performance claims.
bench:
    cargo bench --manifest-path benchmarks/Cargo.toml --bench claims

# --- Web ---

# Build the browser-playable games and the catalogue into web/dist.
web:
    python3 tools/catalogue.py
    tools/build-web.sh

# Then open http://localhost:8080.
# Build the web bundle and serve it locally.
web-serve:
    python3 tools/catalogue.py
    tools/build-web.sh --serve

# Check this machine can build what the repository needs.
doctor:
    bash tools/doctor.sh

# Regenerate web/catalogue.html from the demos' own module docs.
catalogue:
    python3 tools/catalogue.py

# Regenerate docs/PARITY.md: which concepts exist in which engine.
parity:
    python3 tools/parity.py

# Regenerate docs/LEARNING-PATH.md: a reading order through the demos.
learning-path:
    python3 tools/learning-path.py

# Regenerate everything that is generated from the demos.
generated: catalogue parity learning-path

# Refresh docs/images/*.png from the web build (needs Chrome).
screenshots: web
    tools/screenshot.sh

# --- Maintenance ---

# Git ignores .githooks until core.hooksPath points at it, so a fresh clone
# has no hooks until this runs.
# Enable the repository's git hooks (see .githooks/). Run once per clone.
install-hooks:
    git config core.hooksPath .githooks
    @chmod +x .githooks/*
    @echo "hooks enabled: $(git config core.hooksPath)"
    @echo "pre-commit: formatting + repo invariants (fast)"
    @echo "pre-push:   clippy + tests for affected crates"
    @echo "bypass either with --no-verify"

# Turn the hooks back off.
uninstall-hooks:
    git config --unset core.hooksPath || true
    @echo "hooks disabled"


# Run this after a gdext bump so the 67 lockfiles cannot drift apart.
# Re-pin every Godot crate to the same dependency versions.
sync-godot-locks:
    @for m in tech-demos/godot/*/Cargo.toml tic-tac-toe/tic-tac-toe-godot/Cargo.toml \
             snake/snake-godot/Cargo.toml breakout/breakout-godot/Cargo.toml; do \
        cargo update --manifest-path "$m" -q; \
    done
    @echo "godot versions now pinned at:"
    @for f in tech-demos/godot/*/Cargo.lock tic-tac-toe/tic-tac-toe-godot/Cargo.lock \
             snake/snake-godot/Cargo.lock breakout/breakout-godot/Cargo.lock; do \
        awk '/^name = "godot"$/{getline; print $3}' "$f"; \
    done | sort -u
