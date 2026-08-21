#!/usr/bin/env python3
"""Enforce the per-crate rules CLAUDE.md states but nothing checked.

Three invariants, all true today and none of them guaranteed:

  1. **Every Bevy workspace member sets `[lints] workspace = true`.** Without
     it a crate silently opts out of `-D warnings` — CLAUDE.md names this
     exact failure and it was enforced by nothing. A demo added without the
     stanza compiles, passes CI, and quietly stops being linted.
  2. **Every Bevy demo directory appears in `[workspace] members`.** The
     pre-commit hook checks this, but hooks are per-clone and bypassable, so
     CI should not depend on one having run.
  3. **Every non-Godot library crate enforces `missing_docs`.** The Godot
     demos are exempt because gdext's `#[export]` generates accessors that
     cannot carry docs, which is why the exemption is by path rather than by
     a per-crate opt-out that could spread.

    python3 tools/check-crate-hygiene.py
"""

from __future__ import annotations

import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BEVY_ROOT = ROOT / "tech-demos" / "bevy"

# Bevy workspace members that live outside the workspace directory.
OUT_OF_TREE = [
    "snake/snake-bevy",
    "breakout/breakout-bevy",
    "tic-tac-toe/tic-tac-toe-bevy",
    "benchmarks",
]

# Library crates whose public API must be documented.
DOCUMENTED = [
    "tic-tac-toe/tic-tac-toe-lib",
    "tic-tac-toe/tic-tac-toe-web",
    "snake/snake-lib",
    "snake/snake-lockstep",
    "breakout/breakout-lib",
]


def load(path: Path) -> dict:
    return tomllib.loads(path.read_text(encoding="utf-8"))


def main() -> int:
    problems: list[str] = []

    member_dirs = sorted(
        p for p in BEVY_ROOT.iterdir() if p.is_dir() and (p / "Cargo.toml").exists()
    )
    members = [*member_dirs, *(ROOT / rel for rel in OUT_OF_TREE)]

    # 1. The lints stanza.
    for crate in members:
        data = load(crate / "Cargo.toml")
        if data.get("lints", {}).get("workspace") is not True:
            problems.append(
                f"  {crate.relative_to(ROOT)}/Cargo.toml is missing the "
                f"[lints] workspace = true stanza.\n"
                f"      Without it the crate opts out of -D warnings silently."
            )

    # 2. Membership, independent of the pre-commit hook.
    declared = set(load(BEVY_ROOT / "Cargo.toml")["workspace"]["members"])
    for crate in member_dirs:
        if crate.name not in declared:
            problems.append(
                f"  tech-demos/bevy/{crate.name} is not in [workspace] members"
            )
    for rel in OUT_OF_TREE:
        expected = "../" * (len(Path(rel).parts)) + rel
        if not any(m.endswith(rel) for m in declared):
            problems.append(f"  {rel} is not in [workspace] members")

    # 3. Documentation.
    for rel in DOCUMENTED:
        data = load(ROOT / rel / "Cargo.toml")
        if data.get("lints", {}).get("rust", {}).get("missing_docs") is None:
            problems.append(
                f"  {rel}/Cargo.toml does not set missing_docs under "
                f"[lints.rust]"
            )

    if problems:
        print("Crate hygiene problems:")
        print("\n".join(problems))
        return 1

    print(
        f"{len(members)} Bevy members lint with the workspace, "
        f"{len(member_dirs)} demo directories are declared members, and "
        f"{len(DOCUMENTED)} library crates require docs."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
