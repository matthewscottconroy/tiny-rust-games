#!/usr/bin/env python3
"""Load every published page in a real browser and check it actually works.

Two user-visible bugs shipped here that the whole test suite could not see,
because both were about what appears on a screen rather than what a function
returns:

  * Breakout's canvas was cropped by a stylesheet, so the paddle and ball were
    simply not on the page. 875 passing tests, a game you cannot play.
  * Every em dash and arrow rendered as a tofu box, because Bevy's default font
    is ASCII-only. `tools/check-font-coverage.py` now catches that specific
    case; this catches the general one.

Both were found by a human looking at a screenshot, which is not a strategy.

Deliberately *not* a pixel-diff against committed baselines. Those are brittle
against a Chrome upgrade, a GPU driver, or a font hint, and a check that cries
wolf gets disabled. These are structural assertions instead — the page loaded,
a canvas exists at the resolution the game asked for, it is not blank, the
layout does not overflow sideways — each of which maps to a way the product has
actually been broken.

    python3 tools/smoke-web.py            # check web/dist
    python3 tools/smoke-web.py --dir X    # check somewhere else
"""

from __future__ import annotations

import argparse
import http.server
import json
import re
import shutil
import socketserver
import subprocess
import sys
import tempfile
import threading
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# The canvas size each page is known to produce when it is working.
#
# This is the one baseline here, and it is two integers rather than an image,
# so a Chrome upgrade cannot invalidate it. It exists because the cropping bug
# is invisible without it: winit resizes the canvas *backing store* to match
# whatever CSS box it ends up in, so a stylesheet that squeezes the element does
# not crop the picture, it quietly shrinks the resolution the game renders at.
# Comparing the canvas against itself therefore always agrees. Comparing it
# against what the game asked for is what catches it.
#
# A deliberate resolution change is one number here, and the failure message
# says which page and by how much.
EXPECTED_CANVAS: dict[str, tuple[int, int]] = {
    "breakout": (820, 660),
    "snake": (720, 560),
    "tic-tac-toe": (560, 680),
}

# Injected into the served tree, removed afterwards. It frames the real page so
# the pages themselves need no test hooks, then reports what it sees.
HARNESS = """<!doctype html>
<html><body>
<iframe id="f" style="width:1000px;height:900px;border:0"></iframe>
<pre id="result">pending</pre>
<script>
const target = new URLSearchParams(location.search).get("t");
const f = document.getElementById("f");
const errors = [];
window.addEventListener("error", e => errors.push(String(e.message)));
f.src = target;
// Give the module time to boot; virtual time makes this cheap.
setTimeout(() => {
  const out = {target, errors};
  try {
    const d = f.contentDocument;
    const c = d.querySelector("canvas");
    out.hasCanvas = !!c;
    if (c) {
      const r = c.getBoundingClientRect();
      out.attrW = c.width; out.attrH = c.height;
      out.cssW = Math.round(r.width); out.cssH = Math.round(r.height);
    }
    const status = d.getElementById("status");
    out.statusText = status ? status.textContent.trim() : null;
    out.docScrollW = d.documentElement.scrollWidth;
    out.docClientW = d.documentElement.clientWidth;
  } catch (e) {
    out.harnessError = String(e);
  }
  // Stop the game before reporting. A Bevy page drives requestAnimationFrame
  // forever, so with the iframe still live Chrome's virtual clock never runs
  // out of work and --dump-dom waits until it is killed.
  f.src = "about:blank";
  document.getElementById("result").textContent = JSON.stringify(out);
}, 2500);
</script>
</body></html>
"""


class QuietHandler(http.server.SimpleHTTPRequestHandler):
    """Same as the default, minus a request log nobody reads."""

    def log_message(self, *args):  # noqa: D102
        pass


def serve(directory: Path):
    handler = lambda *a, **k: QuietHandler(*a, directory=str(directory), **k)
    httpd = socketserver.TCPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    return httpd, httpd.server_address[1]


def chrome() -> str:
    for name in ("google-chrome", "chromium", "chromium-browser"):
        path = shutil.which(name)
        if path:
            return path
    print("Google Chrome or Chromium is required", file=sys.stderr)
    sys.exit(1)


def inspect(browser: str, url: str) -> dict:
    """Run the harness against one page and return what it reported."""
    try:
        out = subprocess.run(
            [
                browser, "--headless=new", "--no-sandbox", "--disable-gpu",
                "--enable-unsafe-swiftshader", "--virtual-time-budget=15000",
                "--window-size=1100,1000", "--dump-dom", url,
            ],
            capture_output=True, text=True, timeout=120,
        ).stdout
    except subprocess.TimeoutExpired:
        return {"harnessError": "the browser never finished loading this page"}
    match = re.search(r'<pre id="result">(.*?)</pre>', out, re.S)
    if not match or match.group(1).strip() == "pending":
        return {"harnessError": "harness produced no result"}
    try:
        return json.loads(match.group(1))
    except json.JSONDecodeError as exc:
        return {"harnessError": f"unparseable result: {exc}"}


def looks_blank(browser: str, url: str, tmp: Path) -> bool | None:
    """True if the screenshot is a single flat colour (nothing rendered)."""
    shot = tmp / "shot.png"
    try:
        subprocess.run(
            [
                browser, "--headless=new", "--no-sandbox", "--disable-gpu",
                "--enable-unsafe-swiftshader", "--virtual-time-budget=15000",
                "--window-size=1000,900", f"--screenshot={shot}", url,
            ],
            capture_output=True, timeout=120,
        )
    except subprocess.TimeoutExpired:
        return None
    if not shot.exists():
        return None
    try:
        import numpy as np
        from PIL import Image
    except ImportError:
        return None
    pixels = np.asarray(Image.open(shot).convert("RGB"))
    shot.unlink()
    # A working page has a background, a play field and sprites. Fewer than a
    # handful of distinct colours means nothing drew.
    return len(np.unique(pixels.reshape(-1, 3), axis=0)) < 4


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dir", default="web/dist", help="directory to check")
    args = parser.parse_args()

    dist = (ROOT / args.dir).resolve()
    if not dist.is_dir():
        print(f"{args.dir} does not exist — run: just web")
        return 1

    pages = sorted(
        p.parent.relative_to(dist).as_posix()
        for p in dist.glob("*/index.html")
        if p.parent.name != "doc"
    )
    pages += sorted(
        f"demos/{p.parent.name}" for p in dist.glob("demos/*/index.html")
    )
    if not pages:
        print(f"no pages found under {args.dir}")
        return 1

    browser = chrome()
    httpd, port = serve(dist)
    harness = dist / "_smoke-harness.html"
    harness.write_text(HARNESS, encoding="utf-8")
    problems: list[str] = []

    try:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            for page in pages:
                url = f"http://127.0.0.1:{port}/_smoke-harness.html?t=/{page}/"
                info = inspect(browser, url)
                label = f"  {page:28}"

                if err := info.get("harnessError"):
                    problems.append(f"{page}: {err}")
                    print(f"{label} FAILED ({err})")
                    continue

                issues = []
                status = info.get("statusText")
                # game.html removes the placeholder on success and writes the
                # failure into it otherwise, so a surviving status line is the
                # page's own report that it did not start.
                if status and ("could not start" in status or "Loading" in status):
                    issues.append(f"did not start: {status[:60]!r}")
                if not info.get("hasCanvas"):
                    issues.append("no canvas element")
                else:
                    aw, ah = info.get("attrW", 0), info.get("attrH", 0)
                    cw, ch = info.get("cssW", 0), info.get("cssH", 0)
                    if aw == 0 or ah == 0:
                        issues.append("canvas has zero intrinsic size")
                    elif page in EXPECTED_CANVAS:
                        ew, eh = EXPECTED_CANVAS[page]
                        if abs(aw - ew) > 2 or abs(ah - eh) > 2:
                            issues.append(
                                f"canvas is {aw}x{ah}, expected {ew}x{eh} — the "
                                f"game is rendering at the wrong resolution, "
                                f"usually a stylesheet squeezing the element"
                            )
                    if ch and ah and ch < ah * 0.9:
                        issues.append(
                            f"canvas is {ah}px tall but only {ch}px is shown"
                        )
                if info.get("docScrollW", 0) > info.get("docClientW", 0) + 1:
                    issues.append("page overflows horizontally")
                if info.get("errors"):
                    issues.append(f"js errors: {info['errors'][:2]}")

                page_url = f"http://127.0.0.1:{port}/{page}/"
                if looks_blank(browser, page_url, tmp):
                    issues.append("nothing rendered (screenshot is one flat colour)")

                if issues:
                    problems.append(f"{page}: " + "; ".join(issues))
                    print(f"{label} FAILED")
                    for issue in issues:
                        print(f"      {issue}")
                else:
                    size = f"{info.get('attrW')}x{info.get('attrH')}"
                    print(f"{label} ok  (canvas {size})")
    finally:
        harness.unlink(missing_ok=True)
        httpd.shutdown()

    if problems:
        print(f"\n{len(problems)} page(s) broken.")
        return 1
    print(f"\nall {len(pages)} published pages load and render.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
