#!/usr/bin/env bash
#
# Build the browser-playable games into web/dist/.
#
# Only the Bevy frontends are built: Bevy targets wasm, while gdext and
# bracket-lib do not, and the terminal frontend has nothing to render into.
# That is the point of the split — the same rules crate compiles for the web
# because it never depended on an engine in the first place.
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
pkg_args=()
for entry in "${games[@]}"; do
  pkg_args+=(-p "${entry%%:*}")
done
cargo build --locked --target wasm32-unknown-unknown --profile "$profile" \
  --manifest-path "$manifest" "${pkg_args[@]}"

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

echo "==> built $dist"

if [ "${1:-}" = "--serve" ]; then
  echo "==> http://localhost:8080  (Ctrl-C to stop)"
  python3 -m http.server 8080 --directory "$dist"
fi
