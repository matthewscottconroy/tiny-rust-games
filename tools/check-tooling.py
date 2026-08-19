#!/usr/bin/env python3
"""Fail if `just ci` stops covering what CI runs.

The justfile promises, in four documents, that `just ci` runs what CI runs.
Keeping that true by hand failed: six checks were added to CI over time and none
reached `just ci`, so a contributor could run it, see green, and have CI fail on
something a laptop catches in a second.

This is the fifth variation on one theme here — hand-written lists of crates, of
lockfiles, of demos, of tools — and the fix is always the same: derive it, or
check it.

The check resolves `ci`'s recipe dependencies transitively and collects the
tools those recipes actually invoke. Merely *having* a recipe somewhere is not
enough, which is the trap the first version of this file fell into: a tool with
its own convenience recipe looked covered while `just ci` never ran it.

    python3 tools/check-tooling.py
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TOOL = re.compile(r"tools/[a-z-]+\.(?:py|sh)")
RECIPE = re.compile(r"^([a-z][a-z0-9-]*)\s*:([^=]*)$")

# Tools CI runs that `just ci` deliberately does not, because they need
# something a laptop may not have. Each is reachable from its own recipe, and
# the docs name all of them as CI-only.
CI_ONLY = {
    "tools/build-web.sh": "needs the wasm toolchain; run `just web`",
    "tools/smoke-web.py": "needs a browser and a built site; run `just smoke`",
    "tools/validate-godot.sh": "needs Godot installed; run `just validate-godot`",
}


def parse_recipes(text: str) -> dict[str, tuple[list[str], str]]:
    """Maps recipe name -> (dependencies, body)."""
    recipes: dict[str, tuple[list[str], str]] = {}
    current: str | None = None
    deps: list[str] = []
    body: list[str] = []
    for line in text.splitlines():
        if line.startswith((" ", "\t")) and current:
            body.append(line)
            continue
        match = RECIPE.match(line)
        if match and not line.startswith("#"):
            if current:
                recipes[current] = (deps, "\n".join(body))
            current = match.group(1)
            deps = match.group(2).split()
            body = []
        elif current and not line.strip():
            body.append(line)
    if current:
        recipes[current] = (deps, "\n".join(body))
    return recipes


def closure(recipes: dict[str, tuple[list[str], str]], start: str) -> set[str]:
    seen: set[str] = set()
    stack = [start]
    while stack:
        name = stack.pop()
        if name in seen or name not in recipes:
            continue
        seen.add(name)
        stack.extend(recipes[name][0])
    return seen


def main() -> int:
    workflows = sorted((ROOT / ".github" / "workflows").glob("*.yml"))
    in_ci: set[str] = set()
    for path in workflows:
        in_ci |= set(TOOL.findall(path.read_text(encoding="utf-8")))

    justfile = (ROOT / "justfile").read_text(encoding="utf-8")
    recipes = parse_recipes(justfile)
    if "ci" not in recipes:
        print("the justfile has no `ci` recipe")
        return 1

    covered: set[str] = set()
    for name in closure(recipes, "ci"):
        covered |= set(TOOL.findall(recipes[name][1]))

    missing = sorted(in_ci - covered - set(CI_ONLY))
    if missing:
        print("CI runs these, but `just ci` does not:")
        for tool in missing:
            print(f"  {tool}")
        print(
            "\nAdd it to a recipe `ci` depends on, or list it in CI_ONLY with a\n"
            "reason. `just ci` claims to cover everything a laptop can run."
        )
        return 1

    skipped = sorted(in_ci & set(CI_ONLY))
    print(f"`just ci` covers {len(in_ci & covered)} of the {len(in_ci)} tools CI runs.")
    for tool in skipped:
        print(f"  CI-only: {tool} — {CI_ONLY[tool]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
