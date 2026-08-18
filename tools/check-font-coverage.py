#!/usr/bin/env python3
"""Fail if a Bevy demo draws text its font cannot render.

Bevy embeds `FiraMono-subset.ttf`, which covers 95 codepoints — printable
ASCII and nothing else. A string containing an em dash, an arrow, a card suit
or an emoji does not fail to build, does not fail a test, and does not warn.
It renders as a tofu box, and the only way to find out is to look at the
running game.

That happened twice here: em dashes in all three games' HUDs, and an arrow in
`boids-flocking`, both found by squinting at a screenshot. This makes it a
check instead.

Window titles are exempt: those are drawn by the window manager in a system
font, not by Bevy, so non-ASCII is fine there.

    python3 tools/check-font-coverage.py
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Crates whose lib.rs draws text with Bevy.
SOURCES = sorted(ROOT.glob("tech-demos/bevy/*/src/lib.rs")) + [
    ROOT / "snake" / "snake-bevy" / "src" / "lib.rs",
    ROOT / "breakout" / "breakout-bevy" / "src" / "lib.rs",
    ROOT / "tic-tac-toe" / "tic-tac-toe-bevy" / "src" / "lib.rs",
]

STRING = re.compile(r'"(?:[^"\\\n]|\\.)*"')

# Suggestions for the characters that have actually turned up here.
SUGGEST = {
    "—": "-", "–": "-", "−": "-", "×": "x", "…": "...",
    "→": "->", "←": "<-", "↑": "^", "↓": "v",
    "♥": "H", "♠": "S", "♦": "D", "♣": "C",
    "○": "( )", "□": "[ ]", "△": "Triangle",
}


def main() -> int:
    problems: list[str] = []
    for path in SOURCES:
        if not path.exists():
            continue
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            for literal in STRING.findall(line):
                bad = sorted({c for c in literal if ord(c) > 127})
                if not bad:
                    continue
                hints = ", ".join(
                    f"{c!r} -> {SUGGEST.get(c, 'an ASCII spelling')}" for c in bad
                )
                rel = path.relative_to(ROOT)
                problems.append(f"  {rel}:{number}: {hints}")

    if problems:
        print("Text Bevy cannot render (its font is ASCII-only):")
        print("\n".join(problems))
        print(
            "\nThese render as tofu boxes in the running game. Use an ASCII\n"
            "spelling, or ship a font with the glyphs and set it explicitly."
        )
        return 1

    print(f"{len(SOURCES)} Bevy sources draw only ASCII text.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
