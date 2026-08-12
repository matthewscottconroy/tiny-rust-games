#!/usr/bin/env bash
#
# Capture screenshots of the browser-playable games.
#
# The games already compile to WebAssembly for GitHub Pages, so the cheapest
# honest way to picture them is to load that build in a headless browser rather
# than to drive native windows. Same binary the reader will play, no display
# server, and it works in CI.
#
# Requires: tools/build-web.sh output in web/dist, and Google Chrome.
#
#   tools/screenshot.sh          # write docs/images/*.png

set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

dist="web/dist"
out="docs/images"
port=8099

[ -d "$dist" ] || { echo "run tools/build-web.sh first" >&2; exit 1; }

chrome=$(command -v google-chrome || command -v chromium || command -v chromium-browser || true)
[ -n "$chrome" ] || { echo "Google Chrome or Chromium is required" >&2; exit 1; }

mkdir -p "$out"

python3 -m http.server "$port" --directory "$dist" >/dev/null 2>&1 &
server=$!
trap 'kill $server 2>/dev/null || true' EXIT
sleep 2

for slug in snake tic-tac-toe; do
  echo "==> $slug"
  # `--virtual-time-budget` lets the page run a simulated 25 s in far less
  # wall-clock time, so the game is past its loading screen when the shot is
  # taken. swiftshader gives WebGL without a GPU, which is what CI has.
  "$chrome" --headless=new --no-sandbox --disable-gpu \
    --enable-unsafe-swiftshader \
    --virtual-time-budget=25000 --window-size=900,700 \
    --screenshot="$out/$slug.png" \
    "http://localhost:$port/$slug/" >/dev/null 2>&1

  [ -s "$out/$slug.png" ] || { echo "    failed to capture $slug" >&2; exit 1; }
  echo "    $(du -h "$out/$slug.png" | cut -f1)  $out/$slug.png"
done

echo "==> wrote $out"
