#!/usr/bin/env bash
#
# Build the browser-playable games into web/dist/.
#
# The Bevy frontends are built here because Bevy targets wasm, while gdext and
# bracket-lib do not and the terminal frontend has nothing to render into. That
# is the point of the split — the same rules crate compiles for the web because
# it never depended on an engine in the first place.
#
# `tic-tac-toe-web` is built the same way but is not a Bevy crate at all: it is
# the same rules drawn on a raw canvas with no engine. It ships alongside the
# Bevy build of the same game so the two can be compared, and the comparison is
# stark — tens of kilobytes against tens of megabytes.
#
#   tools/build-web.sh          # build everything into web/dist
#   tools/build-web.sh --serve  # ...then serve it on http://localhost:8080

set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

manifest="tech-demos/bevy/Cargo.toml"
profile="wasm-release"
target_dir="tech-demos/bevy/target/wasm32-unknown-unknown/$profile"
dist="web/dist"

# package : url slug : title : controls : rules crate
games=(
  "snake-bevy:snake:Snake:Arrows or WASD to steer · R to restart:snake-lib"
  "tic-tac-toe-bevy:tic-tac-toe:Tic-Tac-Toe:Click a cell to play · R to restart:tic-tac-toe-lib"
  "breakout-bevy:breakout:Breakout:Left/right or A/D to move · Space to launch · R to restart:breakout-lib"
)

# A curated handful of demos, playable from the catalogue.
#
# Deliberately not all 82. Each Bevy wasm build is roughly 25 MB and a fat-LTO
# link of its own, so publishing every demo would mean two gigabytes of
# WebAssembly and an hour of linking to make the catalogue slightly livelier.
# These are the ones that are worth watching move and that load no assets, since
# an asset a demo cannot fetch is a demo that fails in the browser only.
#
# package : slug : title : controls
demos=(
  "boids-flocking:boids-flocking:Boids flocking:Watch — separation, alignment and cohesion"
  "particle-system:particle-system:Particle system:Watch — pooled particles with lifetimes"
  "rope-simulation:rope-simulation:Rope simulation:Drag the rope — Verlet integration"
)

command -v wasm-bindgen >/dev/null || {
  echo "wasm-bindgen not found. Install the version matching the lockfile:" >&2
  awk '/^name = "wasm-bindgen"$/{getline; print "  cargo install wasm-bindgen-cli --version " $3}' \
    tech-demos/bevy/Cargo.lock >&2
  exit 1
}

rustup target list --installed | grep -q wasm32-unknown-unknown || {
  echo "run: rustup target add wasm32-unknown-unknown" >&2
  exit 1
}

echo "==> building ${#games[@]} games for wasm32 ($profile)"
# One package per invocation, deliberately. The wasm-release profile uses fat
# LTO with a single codegen unit, and each final link peaks at several GB;
# passing all the -p flags at once lets cargo run those links in parallel,
# which has OOM-killed a 54 GB dev machine and would sink a 7 GB CI runner.
# Serial links cost a little wall-clock and bound the memory to one link.
for entry in "${games[@]}"; do
  cargo build --locked --target wasm32-unknown-unknown --profile "$profile" \
    --manifest-path "$manifest" -p "${entry%%:*}"
done

echo "==> building ${#demos[@]} demos for wasm32 ($profile)"
# Serial for the same reason as the games above: one fat-LTO link at a time.
for entry in "${demos[@]}"; do
  cargo build --locked --target wasm32-unknown-unknown --profile "$profile" \
    --manifest-path "$manifest" -p "${entry%%:*}"
done

# The engine-free frontend: its own crate, its own target dir, ordinary release
# profile. It has no Bevy in it, so none of the fat-LTO memory caution above
# applies and it builds in seconds.
echo "==> building tic-tac-toe-web (no engine) for wasm32"
cargo build --locked --target wasm32-unknown-unknown --release \
  --manifest-path tic-tac-toe/tic-tac-toe-web/Cargo.toml

rm -rf "$dist"
mkdir -p "$dist"
cp web/index.html "$dist/"
cp web/catalogue.html "$dist/" 2>/dev/null || {
  echo "==> generating catalogue"; python3 tools/catalogue.py; cp web/catalogue.html "$dist/"
}

for entry in "${games[@]}"; do
  IFS=: read -r pkg slug title controls lib <<<"$entry"
  echo "==> $slug"
  mkdir -p "$dist/$slug"
  wasm-bindgen --no-typescript --target web \
    --out-dir "$dist/$slug" --out-name "$pkg" \
    "$target_dir/$pkg.wasm"

  # wasm-opt shrinks the module substantially; skip it rather than fail if the
  # binaryen toolchain is not installed.
  if command -v wasm-opt >/dev/null; then
    wasm-opt -Os "$dist/$slug/${pkg}_bg.wasm" -o "$dist/$slug/${pkg}_bg.wasm"
  else
    echo "    (wasm-opt not found — shipping unoptimised wasm)"
  fi

  sed -e "s|__TITLE__|$title|g" \
      -e "s|__CONTROLS__|$controls|g" \
      -e "s|__MODULE__|$pkg|g" \
      -e "s|__LIB__|$lib|g" \
      web/game.html > "$dist/$slug/index.html"

  size=$(du -h "$dist/$slug/${pkg}_bg.wasm" | cut -f1)
  echo "    $slug: $size"
done

# --- Playable demos ----------------------------------------------------------
mkdir -p "$dist/demos"
for entry in "${demos[@]}"; do
  IFS=: read -r pkg slug title controls <<<"$entry"
  echo "==> demo $slug"
  mkdir -p "$dist/demos/$slug"
  wasm-bindgen --no-typescript --target web \
    --out-dir "$dist/demos/$slug" --out-name "$pkg" \
    "$target_dir/$pkg.wasm"
  if command -v wasm-opt >/dev/null; then
    wasm-opt -Os "$dist/demos/$slug/${pkg}_bg.wasm" -o "$dist/demos/$slug/${pkg}_bg.wasm"
  fi
  sed -e "s|__TITLE__|$title|g" \
      -e "s|__CONTROLS__|$controls|g" \
      -e "s|__MODULE__|$pkg|g" \
      -e "s|__LIB__|tech-demos/bevy/$slug|g" \
      web/game.html > "$dist/demos/$slug/index.html"
  echo "    demos/$slug: $(du -h "$dist/demos/$slug/${pkg}_bg.wasm" | cut -f1)"
done

# --- The engine-free frontend ------------------------------------------------
#
# Deliberately published next to the Bevy build of the same game. Both play the
# same tic-tac-toe from the same rules crate; one carries a whole engine and one
# carries a canvas and a click handler.
echo "==> tic-tac-toe-no-engine"
mkdir -p "$dist/tic-tac-toe-no-engine"
wasm-bindgen --no-typescript --target web \
  --out-dir "$dist/tic-tac-toe-no-engine" --out-name tic_tac_toe_web \
  "tic-tac-toe/tic-tac-toe-web/target/wasm32-unknown-unknown/release/tic_tac_toe_web.wasm"
if command -v wasm-opt >/dev/null; then
  wasm-opt -Os "$dist/tic-tac-toe-no-engine/tic_tac_toe_web_bg.wasm" \
    -o "$dist/tic-tac-toe-no-engine/tic_tac_toe_web_bg.wasm"
fi
cp web/no-engine.html "$dist/tic-tac-toe-no-engine/index.html"
echo "    tic-tac-toe-no-engine: $(du -h "$dist/tic-tac-toe-no-engine/tic_tac_toe_web_bg.wasm" | cut -f1)"

# --- API documentation -------------------------------------------------------
#
# Every demo carries module-level rustdoc and `///` docs on every public item
# (CI enforces the latter), which is invisible to anyone who has not cloned the
# repository. Publishing it alongside the games is most of the payoff for that
# effort.
#
# All the crates share one target directory so their output merges into a single
# doc/ tree rather than overwriting each other.
if [ "${SKIP_DOCS:-}" = "1" ]; then
  echo "==> skipping rustdoc (SKIP_DOCS=1)"
else
  echo "==> building API documentation"
  doc_target="$root/target-doc"
  CARGO_TARGET_DIR="$doc_target" cargo doc --locked --workspace --no-deps \
    --manifest-path "$manifest" --quiet

  # The standalone crates are separate builds; the shared target dir merges them.
  for m in snake/snake-lib/Cargo.toml \
           tic-tac-toe/tic-tac-toe-lib/Cargo.toml \
           breakout/breakout-lib/Cargo.toml \
           snake/snake-godot/Cargo.toml \
           breakout/breakout-godot/Cargo.toml \
           tic-tac-toe/tic-tac-toe-godot/Cargo.toml \
           tic-tac-toe/tic-tac-toe-web/Cargo.toml \
           snake/snake-lockstep/Cargo.toml; do
    CARGO_TARGET_DIR="$doc_target" cargo doc --locked --no-deps \
      --manifest-path "$m" --quiet
  done

  cp -r "$doc_target/doc" "$dist/doc"
  # rustdoc only writes a redirect index when one crate is unambiguous, so point
  # at the library a reader most likely wants.
  cat > "$dist/doc/index.html" <<'HTML'
<!doctype html>
<meta charset="utf-8">
<title>tiny-rust-games — API documentation</title>
<meta http-equiv="refresh" content="0; url=snake_lib/index.html">
<p><a href="snake_lib/index.html">API documentation</a></p>
HTML
  echo "    doc: $(du -sh "$dist/doc" | cut -f1)"
fi

echo "==> built $dist"

if [ "${1:-}" = "--serve" ]; then
  echo "==> http://localhost:8080  (Ctrl-C to stop)"
  python3 -m http.server 8080 --directory "$dist"
fi
