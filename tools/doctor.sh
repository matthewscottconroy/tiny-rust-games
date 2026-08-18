#!/usr/bin/env bash
#
# Check that this machine can build what it is about to try to build.
#
# The failures this catches are all ones that are confusing the first time:
#
#   * missing wayland/alsa/udev *headers* surface as a `wayland-sys` build
#     script panic reading "Package 'wayland-client' was not found", which looks
#     like a Rust error and is not;
#   * a missing `wasm32-unknown-unknown` target only fails once the web build is
#     several minutes in;
#   * `core.hooksPath` unset means a fresh clone silently has no git hooks, and
#     you find out what CI thinks much later than you needed to;
#   * bracket-lib's transitive expat-sys fails under CMake >= 4.0 unless
#     CMAKE_POLICY_VERSION_MINIMUM is set.
#
# Nothing here builds anything, so it takes about a second.
#
#   tools/doctor.sh      # or: just doctor

set -uo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

required_missing=0
optional_missing=0

ok()      { printf '  \033[32m✓\033[0m %s\n' "$1"; }
bad()     { printf '  \033[31m✗\033[0m %s\n' "$1"; required_missing=$((required_missing + 1)); }
warn()    { printf '  \033[33m!\033[0m %s\n' "$1"; optional_missing=$((optional_missing + 1)); }
fix()     { printf '      %s\n' "$1"; }

echo "Toolchain"
if command -v cargo >/dev/null 2>&1; then
  ok "cargo $(cargo --version | awk '{print $2}')"
else
  bad "cargo not found"
  fix "install Rust: https://rustup.rs"
fi
for component in rustfmt clippy-driver; do
  if command -v "$component" >/dev/null 2>&1; then
    ok "$component"
  else
    bad "$component not found"
    fix "rustup component add ${component/clippy-driver/clippy}"
  fi
done

echo
echo "Build helpers"
if command -v just >/dev/null 2>&1; then
  ok "just $(just --version 2>/dev/null | awk '{print $2}')"
else
  warn "just not found — every recipe has a raw cargo equivalent, but they are long"
  fix "cargo install just"
fi
if command -v python3 >/dev/null 2>&1; then
  ok "python3 $(python3 --version 2>&1 | awk '{print $2}') (catalogue, parity, learning path)"
else
  bad "python3 not found — the generated docs cannot be regenerated"
fi
if command -v cmake >/dev/null 2>&1; then
  version=$(cmake --version | head -1 | awk '{print $3}')
  major=${version%%.*}
  if [ "${major:-0}" -ge 4 ]; then
    warn "cmake $version — bracket-lib's expat-sys needs a policy override"
    fix "export CMAKE_POLICY_VERSION_MINIMUM=3.5"
  else
    ok "cmake $version"
  fi
else
  warn "cmake not found — only the bracket-lib demos need it"
fi

echo
echo "System libraries (Bevy and bracket-lib link against these)"
if command -v pkg-config >/dev/null 2>&1; then
  missing_pkgs=()
  for pkg in alsa libudev wayland-client wayland-cursor wayland-egl xkbcommon; do
    pkg-config --exists "$pkg" 2>/dev/null || missing_pkgs+=("$pkg")
  done
  if [ ${#missing_pkgs[@]} -eq 0 ]; then
    ok "all present"
  else
    bad "missing: ${missing_pkgs[*]}"
    fix "Fedora: sudo dnf install -y alsa-lib-devel systemd-devel wayland-devel libxkbcommon-devel"
    fix "Debian: sudo apt-get install -y libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev"
    fix "(the Godot demos and the rules libraries need none of these)"
  fi
else
  warn "pkg-config not found, so the Bevy system libraries cannot be checked"
fi

echo
echo "Targets and optional tools"
if rustup target list --installed 2>/dev/null | grep -q wasm32-unknown-unknown; then
  ok "wasm32-unknown-unknown (the web build)"
else
  warn "wasm32-unknown-unknown not installed — only 'just web' needs it"
  fix "rustup target add wasm32-unknown-unknown"
fi
if command -v wasm-bindgen >/dev/null 2>&1; then
  ok "wasm-bindgen $(wasm-bindgen --version 2>/dev/null | awk '{print $2}') (tools/build-web.sh checks the version matches)"
else
  warn "wasm-bindgen not found — only 'just web' needs it"
  fix "cargo install wasm-bindgen-cli"
fi
if command -v godot4 >/dev/null 2>&1 || command -v godot >/dev/null 2>&1; then
  godot=$(command -v godot4 || command -v godot)
  ok "godot $($godot --version 2>/dev/null | head -1)"
else
  warn "godot not found — needed to *run* a Godot demo, not to build one"
fi

echo
echo "Repository setup"
hooks=$(git config core.hooksPath 2>/dev/null || true)
if [ "$hooks" = ".githooks" ]; then
  ok "git hooks installed"
else
  warn "git hooks are not installed, so nothing is checked before a commit"
  fix "just install-hooks     # or: git config core.hooksPath .githooks"
fi

echo
if [ "$required_missing" -gt 0 ]; then
  echo "$required_missing required item(s) missing — the build will fail until they are fixed."
  exit 1
fi
if [ "$optional_missing" -gt 0 ]; then
  echo "Everything required is present; $optional_missing optional item(s) missing (see ! above)."
else
  echo "Everything is present."
fi
