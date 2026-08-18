#!/usr/bin/env python3
"""Generate the guided reading order through the demos.

The catalogue is searchable and `docs/PARITY.md` is a coverage map, but neither
tells a newcomer where to *start*. 151 demos in alphabetical order is a
reference, not a curriculum, and the repository's stated first goal is teaching.

The ordering below is a judgement call and deliberately lives here rather than
in the generated file: the sequence is the content. What the tool adds is that
every demo named is checked to exist, so the path cannot quietly rot as demos
are renamed — the failure mode of every hand-written tutorial index.

    python3 tools/learning-path.py            # write docs/LEARNING-PATH.md
    python3 tools/learning-path.py --check    # fail if stale or broken
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from catalogue import ROOT, collect  # noqa: E402

OUTPUT = ROOT / "docs" / "LEARNING-PATH.md"

# (heading, why this stage exists, [demo names in reading order])
STAGES: list[tuple[str, str, list[str]]] = [
    (
        "Open a window",
        "Start with the smallest thing that runs, then add exactly one idea at a "
        "time. `draw-window` is the only demo in the repository with no tests, "
        "because it has no logic to test — that is the point of it.",
        ["draw-window", "hello-world", "hello-plugin", "sprite-demo", "movable-sprite"],
    ),
    (
        "How an ECS actually fits together",
        "Bevy's data model is the part that feels alien coming from an "
        "object-oriented engine. These four are the vocabulary everything later "
        "assumes: entities and components, messages between systems, components "
        "that pull in their own dependencies, and systems you call yourself "
        "instead of scheduling.",
        ["events", "observer-events", "required-components", "one-shot-systems"],
    ),
    (
        "Input and cameras",
        "Enough to make something you can steer and follow around a world larger "
        "than the window.",
        ["mouse-input", "gamepad-input", "two-players", "camera-follow", "screen-wrap"],
    ),
    (
        "Time, and why it is the hard part",
        "The most important stage. A frame's `delta` varies; a simulation that "
        "consumes it directly becomes frame-rate dependent and stops being "
        "reproducible. `fixed-timestep` is the demo the games' whole "
        "architecture rests on — read it before `snake/` or `breakout/`.",
        ["fixed-timestep", "time-scale", "scene-pause", "tween-animation"],
    ),
    (
        "Space: grids, tiles and neighbours",
        "Most 2D games are a grid wearing a costume. `spatial-partitioning` is "
        "worth reading together with `benchmarks/`, which measured it and found "
        "the grid 15x *slower* than brute force at the demo's own default — a "
        "reminder that an optimisation is a hypothesis until someone times it.",
        [
            "grid-movement",
            "tilemap",
            "hex-grid",
            "collision-detection",
            "spatial-partitioning",
        ],
    ),
    (
        "Finding a way through",
        "Pathfinding, from the textbook version to the one that scales to a crowd "
        "by inverting the problem.",
        ["pathfinding", "flow-field-pathfinding", "line-of-sight", "fog-of-war"],
    ),
    (
        "Making it feel like a game",
        "None of these change what the game *does*. All of them change how it "
        "feels to play, which is most of the difference between a prototype and "
        "something people enjoy.",
        [
            "screen-shake",
            "knockback-hitstop",
            "floating-text",
            "particle-system",
            "y-sort",
        ],
    ),
    (
        "The systems every game re-implements",
        "Health, effects, inventories, drops, persistence. Read these when you "
        "want to see how much of a game is bookkeeping.",
        [
            "health-and-damage",
            "status-effects",
            "pickup-and-inventory",
            "inventory-ui",
            "loot-table",
            "save-load",
        ],
    ),
    (
        "Making things decide",
        "Four escalating answers to 'what should this enemy do?', in order of how "
        "much structure they impose.",
        [
            "enemy-chase-ai",
            "state-machine-ai",
            "behavior-tree",
            "stealth-ai",
            "boids-flocking",
        ],
    ),
    (
        "Simulation without a physics engine",
        "Verlet integration, springs and cellular updates — enough physics to be "
        "convincing, written out where you can read it.",
        ["rope-simulation", "soft-body", "water-ripple", "destructible-terrain"],
    ),
]

# The games are the argument the demos exist to support, so they close the path.
GAMES = [
    (
        "tic-tac-toe/",
        "turn-based",
        "One set of rules driving four frontends — terminal, ASCII console, "
        "Bevy's ECS and Godot's scene tree — with no rule of the game in any of "
        "them. Start here: it is the simplest possible version of the whole "
        "argument.",
    ),
    (
        "snake/",
        "real-time",
        "The library owns the rules, never the clock. `step()` advances one tick "
        "and never reads a clock, which is what makes it both engine-agnostic "
        "and deterministic enough to record: a replay is a board size, a seed "
        "and the turns queued on each tick.",
    ),
    (
        "breakout/",
        "continuous physics",
        "The case that was supposed to break the pattern — floating-point "
        "positions and two engines that ship their own physics. It holds, on "
        "the condition that `step()` advances a fixed timestep. What it adds is "
        "interpolated *rendering*, which the discrete games never need.",
    ),
]


def build() -> str:
    demos = collect()
    known = {demo["name"] for demo in demos}
    bevy = {demo["name"] for demo in demos if demo["engine"] == "bevy"}
    other_suites = len(demos) - len(bevy)

    missing = [
        name for _title, _why, names in STAGES for name in names if name not in known
    ]
    if missing:
        raise SystemExit(
            "learning path names demos that do not exist: " + ", ".join(sorted(missing))
        )

    listed = {name for _t, _w, names in STAGES for name in names}

    lines = [
        "<!-- Generated by tools/learning-path.py — do not edit by hand. -->",
        "# A path through the demos",
        "",
        "The [catalogue](../web/catalogue.html) is searchable and "
        "[`PARITY.md`](PARITY.md) is a coverage map. Neither tells you where to "
        "start, and 151 demos in alphabetical order is a reference rather than a "
        "curriculum.",
        "",
        f"This is a reading order: {len(listed)} of the Bevy demos, arranged so "
        "each one only assumes what came before it, ending at the three games "
        "the demos exist to support. It is not a syllabus to finish — stop "
        "anywhere, or jump to the stage that matches what you are building.",
        "",
        "Every demo named here is checked to exist when this file is generated, "
        "so the path cannot rot as demos are renamed.",
        "",
        "Run any of them with `just bevy <name>`.",
        "",
    ]

    for index, (title, why, names) in enumerate(STAGES, start=1):
        lines.append(f"## {index}. {title}")
        lines.append("")
        lines.append(why)
        lines.append("")
        for name in names:
            lines.append(f"- [`{name}`](../tech-demos/bevy/{name}/) — `just bevy {name}`")
        lines.append("")

    lines.append("## Then: the games")
    lines.append("")
    lines.append(
        "The demos teach an engine. The games are the reason the repository "
        "exists — the same rules running under engines that genuinely differ, "
        "with the rules living in none of them. Read them in this order; each "
        "one adds exactly one problem the previous one did not have."
    )
    lines.append("")
    for path, kind, why in GAMES:
        lines.append(f"### [`{path}`]({'../' + path}) — {kind}")
        lines.append("")
        lines.append(why)
        lines.append("")

    lines.append("## What is deliberately not here")
    lines.append("")
    lines.append(
        f"{len(bevy - listed)} of the {len(bevy)} Bevy demos are not on this "
        f"path, and neither are the {other_suites} Godot and bracket-lib ones. "
        "That is not a judgement on them — they are reference material for a "
        "problem you either have or do not, and the catalogue is the right way "
        "to find those. A path that included everything would be the "
        "alphabetical list again."
    )
    lines.append("")

    return "\n".join(lines).rstrip("\n") + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if out of date")
    args = parser.parse_args()

    page = build()
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_text(encoding="utf-8") != page:
            print(f"{OUTPUT} is out of date; run: python3 tools/learning-path.py")
            return 1
        print(f"{OUTPUT} is up to date")
        return 0

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(page, encoding="utf-8")
    print(f"wrote {OUTPUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
