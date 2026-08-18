#!/usr/bin/env python3
"""Generate the demo catalogue from the demos themselves.

Every demo already states what it teaches — Godot demos in a `//! Teaches:`
block (enforced by the pre-commit hook), Bevy demos in their module rustdoc.
This reads that metadata and writes a single searchable page, so the catalogue
cannot drift from the source the way a hand-maintained table does.

    python3 tools/catalogue.py            # write web/catalogue.html
    python3 tools/catalogue.py --check    # fail if it is out of date

The `--check` mode is what CI runs: it regenerates into memory and compares, so
a demo added without regenerating is caught rather than silently missing.
"""

from __future__ import annotations

import argparse
import html
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "web" / "catalogue.html"

# Demo suites to scan: (label, directory, engine tag).
SUITES = [
    ("Bevy", ROOT / "tech-demos" / "bevy", "bevy"),
    ("Godot", ROOT / "tech-demos" / "godot", "godot"),
    ("bracket-lib", ROOT / "tech-demos" / "brackets", "brackets"),
]

# Directories that are not demos.
SKIP = {"target", "_template", ".shared-target"}

# The demos the web build publishes as playable. Read out of build-web.sh rather
# than repeated here: two lists that could disagree about what is playable would
# put a dead "Play" link in the catalogue, which is worse than no link at all.
BUILD_WEB = ROOT / "tools" / "build-web.sh"


def playable_slugs() -> set[str]:
    """Slugs from the `demos=(...)` array in tools/build-web.sh."""
    if not BUILD_WEB.exists():
        return set()
    text = BUILD_WEB.read_text(encoding="utf-8")
    match = re.search(r"^demos=\((.*?)^\)", text, re.S | re.M)
    if not match:
        return set()
    slugs = set()
    for line in match.group(1).splitlines():
        line = line.strip().strip('"')
        if not line or line.startswith("#"):
            continue
        parts = line.split(":")
        if len(parts) >= 2:
            slugs.add(parts[1])
    return slugs


def module_doc(src: Path) -> list[str]:
    """The leading `//!` block of a source file, as plain lines."""
    lines: list[str] = []
    for raw in src.read_text(encoding="utf-8").splitlines():
        if not raw.startswith("//!"):
            if lines:
                break
            continue
        lines.append(raw[3:].strip())
    return lines


def strip_markup(text: str) -> str:
    """Turn rustdoc into prose: drop links, backticks and bold markers."""
    text = re.sub(r"\[`?([^\]`]+)`?\]\([^)]*\)", r"\1", text)  # [x](y)
    text = re.sub(r"\[([^\]]+)\]", r"\1", text)  # intra-doc [x]
    return text.replace("`", "").replace("**", "").strip()


def parse_demo(demo_dir: Path, engine: str) -> dict | None:
    """Extract catalogue metadata from one demo crate."""
    src = demo_dir / "src" / "lib.rs"
    if not src.exists():
        src = demo_dir / "src" / "main.rs"
    if not src.exists():
        return None

    doc = module_doc(src)
    if not doc:
        return None

    # The first line is the summary. It may wrap onto the next one, which is
    # why the continuation is joined rather than dropped.
    summary_lines = [doc[0]]
    for follow in doc[1:]:
        if not follow:
            break
        summary_lines.append(follow)
    summary = strip_markup(" ".join(summary_lines)).rstrip(".")

    # What the demo teaches lives under one of two headings: Godot demos use
    # "Teaches:" (enforced by the pre-commit hook), Bevy demos use "Key ideas:".
    # Both may be a trailing clause, a bulleted block, or both.
    teaches = ""
    for i, line in enumerate(doc):
        if not re.match(r"^(Teaches|Key ideas)\b", line):
            continue
        parts: list[str] = []
        if ":" in line:
            head = strip_markup(line.split(":", 1)[1])
            if head:
                parts.append(head)
        for follow in doc[i + 1 :]:
            # Stop at a blank line that ends the block, a heading, or controls.
            if follow.startswith(("# ", "Controls", "**Controls")):
                break
            if not follow:
                if parts:
                    break
                continue
            parts.append(strip_markup(follow.lstrip("- ")))
        teaches = " ".join(p for p in parts if p).strip()
        break

    controls = ""
    for line in doc:
        if "Controls:" in line:
            controls = strip_markup(line.split("Controls:", 1)[1])
            break

    return {
        "name": demo_dir.name,
        "engine": engine,
        "summary": summary,
        "teaches": teaches,
        "controls": controls,
        "path": str(demo_dir.relative_to(ROOT)),
        "tests": len(re.findall(r"#\[test\]", src.read_text(encoding="utf-8"))),
    }


def collect() -> list[dict]:
    playable = playable_slugs()
    demos: list[dict] = []
    for _label, directory, engine in SUITES:
        if not directory.exists():
            continue
        for demo_dir in sorted(directory.iterdir()):
            if not demo_dir.is_dir() or demo_dir.name in SKIP:
                continue
            if not (demo_dir / "Cargo.toml").exists():
                continue
            entry = parse_demo(demo_dir, engine)
            if entry:
                # Only the Bevy demos are built for the web; a Godot demo of the
                # same name is not playable just because its Bevy twin is.
                entry["playable"] = engine == "bevy" and entry["name"] in playable
                demos.append(entry)
    return demos


PAGE = """<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>tiny-rust-games — demo catalogue</title>
<style>
  :root {{
    --bg: #14161a; --panel: #1c1f25; --text: #e8eaed; --dim: #9aa0a6;
    --line: #2c3038; --bevy: #b48ead; --godot: #81a1c1; --brackets: #a3be8c;
  }}
  * {{ box-sizing: border-box; }}
  body {{ margin: 0; background: var(--bg); color: var(--text);
         font: 15px/1.55 ui-sans-serif, system-ui, -apple-system, sans-serif; }}
  header {{ padding: 2rem 1.5rem 1rem; max-width: 1100px; margin: 0 auto; }}
  h1 {{ margin: 0 0 .3rem; font-size: 1.6rem; }}
  p.lede {{ margin: 0 0 1.2rem; color: var(--dim); }}
  .controls {{ display: flex; gap: .6rem; flex-wrap: wrap; align-items: center; }}
  input[type=search] {{ flex: 1 1 260px; padding: .6rem .8rem; border-radius: 8px;
    border: 1px solid var(--line); background: var(--panel); color: var(--text); font-size: 1rem; }}
  button {{ padding: .5rem .9rem; border-radius: 999px; cursor: pointer;
    border: 1px solid var(--line); background: var(--panel); color: var(--dim); font-size: .9rem; }}
  button[aria-pressed="true"] {{ color: var(--text); border-color: currentColor; }}
  button[data-engine="bevy"][aria-pressed="true"] {{ color: var(--bevy); }}
  button[data-engine="godot"][aria-pressed="true"] {{ color: var(--godot); }}
  button[data-engine="brackets"][aria-pressed="true"] {{ color: var(--brackets); }}
  main {{ max-width: 1100px; margin: 0 auto; padding: 1rem 1.5rem 4rem;
          display: grid; gap: .8rem; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); }}
  article {{ background: var(--panel); border: 1px solid var(--line);
             border-radius: 10px; padding: .9rem 1rem; }}
  article h2 {{ margin: 0 0 .2rem; font-size: 1rem; font-family: ui-monospace, monospace; }}
  article h2 a {{ color: inherit; text-decoration: none; }}
  article h2 a:hover {{ text-decoration: underline; }}
  .tag {{ font-size: .72rem; text-transform: uppercase; letter-spacing: .06em; }}
  .tag.bevy {{ color: var(--bevy); }} .tag.godot {{ color: var(--godot); }}
  .tag.brackets {{ color: var(--brackets); }}
  .summary {{ margin: .35rem 0 .4rem; }}
  .teaches {{ color: var(--dim); font-size: .9rem; margin: 0; }}
  .meta {{ margin-top: .5rem; color: var(--dim); font-size: .78rem; }}
  .play {{ color: var(--accent); font-weight: 600; text-decoration: none; }}
  .play:hover {{ text-decoration: underline; }}
  #count {{ color: var(--dim); font-size: .9rem; }}
  .empty {{ grid-column: 1/-1; color: var(--dim); padding: 2rem 0; }}
</style>
</head>
<body>
<header>
  <h1>tiny-rust-games — demo catalogue</h1>
  <p class="lede">{count} demos across three engines. Generated from each demo's
  own module documentation, so it cannot drift from the source.</p>
  <div class="controls">
    <input type="search" id="q" placeholder="Search concepts, e.g. pathfinding, shader, signal…" autofocus>
    <button data-engine="all" aria-pressed="true">All</button>
    <button data-engine="bevy" aria-pressed="false">Bevy</button>
    <button data-engine="godot" aria-pressed="false">Godot</button>
    <button data-engine="brackets" aria-pressed="false">bracket-lib</button>
  </div>
  <p id="count"></p>
</header>
<main id="grid"></main>
<script>
const DEMOS = {data};
const REPO = "https://github.com/matthewscottconroy/tiny-rust-games/tree/main/";
const grid = document.getElementById("grid");
const count = document.getElementById("count");
const q = document.getElementById("q");
let engine = "all";

function render() {{
  const needle = q.value.trim().toLowerCase();
  const shown = DEMOS.filter(d =>
    (engine === "all" || d.engine === engine) &&
    (!needle || (d.name + " " + d.summary + " " + d.teaches).toLowerCase().includes(needle)));

  count.textContent = shown.length + " of " + DEMOS.length + " demos";
  grid.innerHTML = shown.length ? shown.map(d => `
    <article>
      <span class="tag ${{d.engine}}">${{d.engine}}</span>
      <h2><a href="${{REPO}}${{d.path}}">${{d.name}}</a></h2>
      <p class="summary">${{d.summary}}</p>
      ${{d.teaches ? `<p class="teaches">${{d.teaches}}</p>` : ""}}
      <p class="meta">${{d.tests}} test${{d.tests === 1 ? "" : "s"}}${{d.controls ? " · " + d.controls : ""}}${{d.playable ? ` · <a class="play" href="demos/${{d.name}}/">▶ play in your browser</a>` : ""}}</p>
    </article>`).join("") : `<p class="empty">Nothing matches “${{needle}}”.</p>`;
}}

q.addEventListener("input", render);
document.querySelectorAll("button[data-engine]").forEach(b =>
  b.addEventListener("click", () => {{
    engine = b.dataset.engine;
    document.querySelectorAll("button[data-engine]").forEach(o =>
      o.setAttribute("aria-pressed", String(o === b)));
    render();
  }}));
render();
</script>
</body>
</html>
"""


def build() -> str:
    demos = collect()
    return PAGE.format(
        count=len(demos),
        data=html.escape(json.dumps(demos, ensure_ascii=False), quote=False),
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="exit non-zero if the committed catalogue is stale",
    )
    args = parser.parse_args()

    page = build()
    if args.check:
        if not OUTPUT.exists():
            print(f"{OUTPUT} does not exist; run: python3 tools/catalogue.py")
            return 1
        if OUTPUT.read_text(encoding="utf-8") != page:
            print(f"{OUTPUT} is out of date; run: python3 tools/catalogue.py")
            return 1
        print(f"{OUTPUT} is up to date")
        return 0

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(page, encoding="utf-8")
    print(f"wrote {OUTPUT.relative_to(ROOT)} ({len(collect())} demos)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
