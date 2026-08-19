#!/usr/bin/env python3
"""Enforce the discipline that makes cross-engine duplication defensible.

`tech-demos/bevy/DEMO_ANATOMY.md` says duplication between the Bevy and Godot
suites is deliberate: a demo whose purpose is "read this one file and learn hex
grids" is ruined by sending the reader to a shared crate for the half that
matters, and goal #1 beats goal #4 when they conflict. The games are where the
engine-agnostic argument is made properly.

But that trade is only defensible if two conditions hold, and the document says
so. Both were unenforced, and one of them was not holding at all:

  1. **Paired demos name their counterpart**, so a reader can find the other
     half of the comparison goal #2 is asking for. Fourteen of fifteen pairs
     did not, in either direction.

  2. **Shared pure functions stay in sync.** Both copies of `cube_round` once
     carried the same dead-assignment bug; fixing one and not the other is the
     failure mode duplication invites. They are identical today, which is
     luck plus diligence rather than a guarantee.

    python3 tools/check-paired-demos.py
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from catalogue import ROOT, collect  # noqa: E402

# Functions documented as byte-identical between the two suites. Drift here is
# the bug DEMO_ANATOMY warns about, so it is an error rather than a note.
MUST_MATCH: dict[str, list[str]] = {
    "hex-grid": ["cube_round", "hex_neighbors", "axial_distance"],
}

FN = re.compile(r"^pub fn (\w+)", re.M)


def source(demo: dict) -> Path:
    return ROOT / demo["path"] / "src" / "lib.rs"


def function_body(text: str, name: str) -> str | None:
    """The normalised body of `pub fn name`, or None if absent."""
    match = re.search(rf"^pub fn {re.escape(name)}\b.*?^}}", text, re.M | re.S)
    if not match:
        return None
    body = match.group(0)
    # Strip comments and collapse whitespace: a reformat is not drift.
    body = re.sub(r"//.*", "", body)
    return " ".join(body.split())


def main() -> int:
    demos = collect()
    bevy = {d["concept"]: d for d in demos if d["engine"] == "bevy"}
    godot = {d["concept"]: d for d in demos if d["engine"] == "godot"}
    pairs = sorted(set(bevy) & set(godot))

    problems: list[str] = []

    for concept in pairs:
        b, g = bevy[concept], godot[concept]
        b_src, g_src = source(b), source(g)
        if not b_src.exists() or not g_src.exists():
            continue
        b_text = b_src.read_text(encoding="utf-8")
        g_text = g_src.read_text(encoding="utf-8")

        # 1. Each side names its counterpart.
        for text, this, other, other_path in (
            (b_text, b, g, f"tech-demos/godot/{g['name']}"),
            (g_text, g, b, f"tech-demos/bevy/{b['name']}"),
        ):
            head = "\n".join(l for l in text.splitlines() if l.startswith("//!"))
            if other_path not in head:
                problems.append(
                    f"  {this['path']}/src/lib.rs: module docs do not name its "
                    f"counterpart ({other_path}).\n"
                    f"      Add a line like: //! Counterpart: {other_path}"
                )

        # 2. Functions that must stay identical, do.
        for name in MUST_MATCH.get(concept, []):
            b_body = function_body(b_text, name)
            g_body = function_body(g_text, name)
            if b_body is None or g_body is None:
                problems.append(
                    f"  {concept}: `{name}` is listed as shared but is missing "
                    f"from {'bevy' if b_body is None else 'godot'}"
                )
            elif b_body != g_body:
                problems.append(
                    f"  {concept}: `{name}` has drifted between the two suites.\n"
                    f"      {b['path']}/src/lib.rs\n"
                    f"      {g['path']}/src/lib.rs\n"
                    f"      Duplication is deliberate here; divergence is not — "
                    f"fix both copies in the same change."
                )

    if problems:
        print(f"Paired-demo problems ({len(pairs)} pairs checked):")
        print("\n".join(problems))
        return 1

    shared = sum(len(v) for v in MUST_MATCH.values())
    print(
        f"{len(pairs)} cross-engine pairs cross-link, and {shared} shared "
        f"functions are still identical."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
