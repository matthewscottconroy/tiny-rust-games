#!/usr/bin/env python3
"""Check that colours carrying meaning stay distinguishable.

A game that encodes information in colour has to survive two things: a player
who cannot separate red from green, and a glance rather than a stare. Neither is
checkable by looking at a screenshot with ordinary vision, which is how both of
the defects below survived review.

Colours are compared as CIE ΔE in Lab, under normal vision and under simulated
protanopia, deuteranopia and tritanopia (Viénot, Brettel & Mollon 1999). ΔE is
roughly "just noticeable" at 2.3; the threshold here is far higher because these
are moving objects seen for a fraction of a second, not paint chips held side by
side.

What it found when written:

  * Snake's food was red on a green snake — ΔE 98 normally, and **10.9** under
    deuteranopia, the commonest colour blindness. The food was invisible to
    roughly one man in twelve.
  * Breakout's damaged bricks were drawn at 88% brightness, which the docs
    described as "dimmed, so the player can see it is weakened". That is ΔE 6.
    Nobody could see it, colour vision or not.

    python3 tools/check-palette.py
"""

from __future__ import annotations

import sys

import numpy as np

# Minimum ΔE between two colours that a player must be able to tell apart, in
# the worst case across all four vision types.
THRESHOLD = 25.0

# sRGB -> LMS, and the dichromat projections. Viénot et al. (1999).
RGB2LMS = np.array(
    [
        [17.8824, 43.5161, 4.11935],
        [3.45565, 27.1554, 3.86714],
        [0.0299566, 0.184309, 1.46709],
    ]
)
LMS2RGB = np.linalg.inv(RGB2LMS)
DICHROMAT = {
    "protanopia": np.array([[0, 2.02344, -2.52581], [0, 1, 0], [0, 0, 1]]),
    "deuteranopia": np.array([[1, 0, 0], [0.494207, 0, 1.24827], [0, 0, 1]]),
    "tritanopia": np.array([[1, 0, 0], [0, 1, 0], [-0.395913, 0.801109, 0]]),
}
VISION = [None, *DICHROMAT]

BRICK_ROWS = [
    (0.90, 0.35, 0.35),
    (0.90, 0.60, 0.30),
    (0.85, 0.85, 0.35),
    (0.40, 0.80, 0.45),
    (0.40, 0.65, 0.90),
]
DAMAGED = 0.5

SNAKE = {
    "background": (0.10, 0.11, 0.13),
    "head": (0.55, 0.95, 0.55),
    "body": (0.25, 0.70, 0.35),
    "food": (0.95, 0.40, 0.85),
}
BREAKOUT = {
    "background": (0.08, 0.09, 0.11),
    "paddle": (0.85, 0.88, 0.92),
    "ball": (1.00, 0.95, 0.85),
}
TIC_TAC_TOE = {
    "bevy X": (0.35, 0.75, 1.00),
    "bevy O": (1.00, 0.55, 0.35),
    "web X": (0xE0 / 255, 0x6C / 255, 0x75 / 255),
    "web O": (0x61 / 255, 0xAF / 255, 0xEF / 255),
}

# (label, colour a, colour b) — pairs a player must be able to tell apart.
def required_pairs() -> list[tuple[str, tuple, tuple]]:
    pairs = [
        ("snake: food vs body", SNAKE["food"], SNAKE["body"]),
        ("snake: food vs head", SNAKE["food"], SNAKE["head"]),
        ("snake: body vs background", SNAKE["body"], SNAKE["background"]),
        ("snake: food vs background", SNAKE["food"], SNAKE["background"]),
        ("breakout: ball vs background", BREAKOUT["ball"], BREAKOUT["background"]),
        ("breakout: paddle vs background", BREAKOUT["paddle"], BREAKOUT["background"]),
        ("tic-tac-toe: X vs O (bevy)", TIC_TAC_TOE["bevy X"], TIC_TAC_TOE["bevy O"]),
        ("tic-tac-toe: X vs O (web)", TIC_TAC_TOE["web X"], TIC_TAC_TOE["web O"]),
    ]
    # A damaged brick must be visibly weaker than a fresh one of the same row:
    # that difference is the only cue that it takes another hit.
    for index, row in enumerate(BRICK_ROWS):
        pairs.append(
            (
                f"breakout: brick row {index} fresh vs damaged",
                row,
                tuple(channel * DAMAGED for channel in row),
            )
        )
    return pairs


def to_linear(colour) -> np.ndarray:
    c = np.asarray(colour, dtype=float)
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def to_lab(linear: np.ndarray) -> np.ndarray:
    m = np.array(
        [
            [0.4124, 0.3576, 0.1805],
            [0.2126, 0.7152, 0.0722],
            [0.0193, 0.1192, 0.9505],
        ]
    )
    xyz = m @ linear
    white = np.array([0.95047, 1.0, 1.08883])
    t = xyz / white
    f = np.where(t > 0.008856, np.cbrt(t), 7.787 * t + 16 / 116)
    return np.array([116 * f[1] - 16, 500 * (f[0] - f[1]), 200 * (f[1] - f[2])])


def as_seen(linear: np.ndarray, vision: str | None) -> np.ndarray:
    if vision is None:
        return linear
    lms = RGB2LMS @ linear
    return np.clip(LMS2RGB @ (DICHROMAT[vision] @ lms), 0, None)


def delta_e(a, b, vision: str | None) -> float:
    la = as_seen(to_linear(a), vision)
    lb = as_seen(to_linear(b), vision)
    return float(np.linalg.norm(to_lab(la) - to_lab(lb)))


def main() -> int:
    failures = []
    print(f"{'pair':38} {'normal':>7} {'protan':>7} {'deuter':>7} {'tritan':>7}")
    for label, a, b in required_pairs():
        scores = [delta_e(a, b, vision) for vision in VISION]
        worst = min(scores)
        row = " ".join(f"{score:7.1f}" for score in scores)
        mark = "" if worst >= THRESHOLD else "   TOO CLOSE"
        print(f"{label:38} {row}{mark}")
        if worst < THRESHOLD:
            failures.append((label, worst))

    if failures:
        print(f"\n{len(failures)} pair(s) below ΔE {THRESHOLD:.0f}:")
        for label, worst in failures:
            print(f"  {label}: worst ΔE {worst:.1f}")
        print(
            "\nThese carry meaning in the game, so a player who cannot separate "
            "them\nloses information. Pick colours further apart, or add a cue "
            "that is not\ncolour at all."
        )
        return 1

    print(f"\nAll {len(required_pairs())} meaningful pairs stay above ΔE {THRESHOLD:.0f}.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
