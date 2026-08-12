#!/usr/bin/env bash
#
# Load every Godot project headlessly and check its Rust classes register.
#
# CI compiles the Godot demos but, until this existed, never *loaded* one. That
# left a whole class of breakage invisible: a `.gdextension` pointing at the
# wrong library name, a scene referencing a class the Rust side does not
# register, a renamed node. All of it compiles perfectly and fails the moment
# anyone opens the project.
#
# Two things make this work, both of which cost an afternoon to discover:
#
#   * a project with no `run/main_scene` hangs forever under `--headless`
#     instead of erroring, because Godot waits on a UI that never appears. Every
#     project here declares one, and this script fails any that does not rather
#     than hanging.
#   * `.gdextension` files are only discovered during an import pass, which
#     writes `.godot/extension_list.cfg`. Without that the extension is never
#     loaded and every Rust class silently becomes a placeholder. So: import
#     first, then run.
#
#   tools/validate-godot.sh            # every project
#   tools/validate-godot.sh hex-grid   # just one

set -uo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

godot=$(command -v godot4 || command -v godot || true)
[ -n "$godot" ] || { echo "godot4 (or godot) is required" >&2; exit 1; }
echo "using $($godot --version 2>/dev/null | head -1)"

filter=${1:-}
projects=()
for p in tech-demos/godot/*/project.godot \
         snake/snake-godot/project.godot \
         tic-tac-toe/tic-tac-toe-godot/project.godot; do
  [ -f "$p" ] || continue
  dir=$(dirname "$p")
  [ -z "$filter" ] || [[ $(basename "$dir") == *"$filter"* ]] || continue
  projects+=("$dir")
done

failed=()
checked=0
skipped=0
for dir in "${projects[@]}"; do
  name=$(basename "$dir")
  printf '  %-24s' "$name"

  if ! grep -q 'run/main_scene' "$dir/project.godot"; then
    echo "FAIL (no run/main_scene — would hang headless)"
    failed+=("$name")
    continue
  fi

  # The extension has to exist at res://target/debug/ before Godot can load it.
  # CI builds every Godot crate into one shared target directory so gdext
  # compiles once instead of 69 times, which leaves the per-crate path empty —
  # so copy the artifact across rather than rebuilding it here.
  if ! ls "$dir"/target/debug/*.so >/dev/null 2>&1; then
    # `[lib] name` is optional: without it Cargo uses the package name with
    # dashes turned into underscores, and several crates here rely on that.
    lib=$(sed -n 's/^name *= *"\(.*\)"/\1/p' <(sed -n '/^\[lib\]/,/^\[/p' "$dir/Cargo.toml"))
    if [ -z "$lib" ]; then
      lib=$(sed -n 's/^name *= *"\(.*\)"/\1/p' <(sed -n '/^\[package\]/,/^\[/p' "$dir/Cargo.toml") | tr '-' '_')
    fi
    shared="tech-demos/godot/.shared-target/debug/lib${lib}.so"
    if [ -n "$lib" ] && [ -f "$shared" ]; then
      mkdir -p "$dir/target/debug"
      cp "$shared" "$dir/target/debug/"
    else
      echo "skip (not built)"
      skipped=$((skipped + 1))
      continue
    fi
  fi
  checked=$((checked + 1))

  log=$(mktemp)

  # Import populates .godot/extension_list.cfg. It exits non-zero on some
  # builds while still doing the work — Godot 4.7 aborts during editor teardown
  # — so its status is deliberately ignored and the run below is what judges.
  # `|| true` is not enough, and neither is a subshell: bash reports a
  # signal-killed child on the *script's* stderr after the fact. Running it
  # through a separate shell keeps that report inside a process whose stderr
  # can be discarded.
  bash -c "timeout 120 '$godot' --headless --import --path '$dir' >/dev/null 2>&1" 2>/dev/null || true

  timeout 120 "$godot" --headless --quit-after 10 --path "$dir" >"$log" 2>&1
  status=$?

  # Godot reports a missing class as an ERROR and carries on with a placeholder,
  # exiting 0 — so the log has to be inspected, not just the exit code.
  if [ $status -eq 124 ]; then
    echo "FAIL (timed out)"
    failed+=("$name")
  elif grep -qE "Cannot get class|cannot be created|Can't open dynamic library|Failed to load" "$log"; then
    echo "FAIL (extension did not register)"
    sed -n '1,6p' "$log" | sed 's/^/      /'
    failed+=("$name")
  elif [ $status -ne 0 ]; then
    echo "FAIL (exit $status)"
    sed -n '1,6p' "$log" | sed 's/^/      /'
    failed+=("$name")
  else
    echo "ok"
  fi
  rm -f "$log"
done

echo
if [ ${#failed[@]} -gt 0 ]; then
  echo "failed: ${failed[*]}"
  exit 1
fi
# Report what was actually exercised: "all N passed" would be a lie when most
# were skipped for want of a built extension.
echo "$checked of ${#projects[@]} Godot projects load and register their Rust classes"
if [ "$skipped" -gt 0 ]; then
  echo "$skipped skipped (extension not built — run: just test-godot)"
fi
