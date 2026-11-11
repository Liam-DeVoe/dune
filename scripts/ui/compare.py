#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12,<3.14"
# dependencies = ["playwright==1.58.0", "pillow>=11"]
# ///
# Not Python 3.14: its asyncio.wait records the waiting task on every future it waits for and
# never removes it from a future that stays pending. Playwright waits on its connection's error
# future (which never completes) for every call, so every call's task stays alive, with the
# stack Playwright saves on it and every local of those frames: each screenshot and image. A
# full run grew past 3 GB.
"""Compare the web client with the C# client, scene by scene, and write a gallery.

    just compare                       # every fixture, every faction plus an observer
    just compare alliance mc-ended     # the scenes of these fixtures
    just compare alliance/Emperor          # one scene
    just compare --require side-column # exit 1 unless every scene matches there
    just compare --no-build            # reuse the last builds of both clients and the views

A scene is a fixture from scripts/fixtures seen by one faction, or by an observer. The C#
client shows it on its /scene page (fixture replayed in the browser, no server). The web
client shows it on /scene/<fixture>/<Faction>, which renders a view written by the Rust view
crate (rust/view/examples/scene_views.rs) from the same fixture and seat. For each scene and
viewport, both pages are screenshotted and the named regions are compared by pixels and by a
DOM digest, after masking the expected differences (MASKS). Each run writes its own directory,
design/compare/runs/<time>-<pid> (or --out DIR): gallery.html (a matrix per region), one page
per scene, and results.json. It also merges its results into the shared
design/compare/gallery.html, which shows the latest result of every scene across runs. At the
end it prints every comparison of this run that did not match.

rust/docs/client-plan.md ("Comparing with the C# client") is the design; scripts/README.md
has the workflow.
"""

from __future__ import annotations

import argparse
import difflib
import importlib.util
import io
import json
import os
import queue
import shutil
import subprocess
import sys
import threading
import time
import traceback
from contextlib import contextmanager
from dataclasses import asdict, dataclass, field
from functools import partial
from pathlib import Path
from urllib.parse import quote, urlsplit

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from PIL import Image, ImageChops, ImageDraw  # noqa: E402
from playwright.sync_api import Browser, Page, sync_playwright  # noqa: E402
from server import FIXTURES, Handler, Server, StaticServer  # noqa: E402

REPO = HERE.parent.parent
WEB = REPO / "web"
WWWROOT = REPO / "treachery-online" / "Client" / "wwwroot"
# Builds and views; nothing here is kept. Each build goes to its own directory under
# WORK/builds, with the C# client, the web client and the views, and WORK/current links to the
# newest complete one. A run uses one build from start to end (use_build), so another agent's
# build can't change what a run compares, and no agent needs its own COMPARE_WORK.
WORK = Path(os.environ.get("COMPARE_WORK", "/tmp/treachery-compare"))
BUILDS = WORK / "builds"
CURRENT = WORK / "current"
# This run's build. Set by use_build().
BUILD = WORK
CSHARP_CLIENT = BUILD / "csharp"
WEB_DIST = BUILD / "web"
VIEWS = BUILD / "views"
# The shared gallery: the latest result of every scene, across runs (compare_report.publish).
ROOT = REPO / "design" / "compare"
# This run's directory: --out, else COMPARE_OUT, else ROOT/runs/<time>-<pid>. Set by main().
OUT = ROOT

# The sizes the plan names: the desktop layout and a phone.
VIEWPORTS = {"desktop": {"width": 1440, "height": 820}, "phone": {"width": 390, "height": 844}}

# Screen areas, by a selector that holds in both clients (the web client keeps the C# class
# names, client-plan.md "Structure"). "page" is the whole document.
REGIONS = {
    "page": None,
    "turn-banner": ".turn-banner",
    "board": "#mapdiv",
    "side-column": ".side-column",
    "player-panel": ".side-column .card:has(.player-panel)",
    "prompt": ".side-column .act-stack",
    "log": ".side-column .report",
    # End of game (package 9): WinnerPanel, the only card in the act stack with an h1, and the
    # story modal while it is open (scripts/ui/comparisons/story.py opens it). The region is the
    # modal's fixed frame, the window: its pixels are the part of the story on screen and its
    # digest the whole story. A box around the content would differ in height whenever the
    # story-hide-secrets mask applies, since the view's story is shorter.
    "winner": ".side-column .act-stack .card:has(> .card-body > h1.h1)",
    "story": "#storyModal.show",
    # Login, lobby, setup and seating (package 8): the start screen below the maintenance banner.
    "start-screen": ".start-screen",
    # The panel for playing a treachery card from the hand (CardPlayPopover), over the board;
    # scripts/ui/comparisons/battle-clairvoyance.py opens it.
    "card-play": ".card-play-pop",
    # A popover a script opened with show_popover (scripts/ui/comparisons/map-popovers.py).
    "popover": "body > .popover.show",
}

# A pixel differs when a channel differs by more than this, and a region matches when no more
# than this share of its pixels differ: room for font antialiasing, nothing more.
CHANNEL_TOLERANCE = 24
PIXEL_SHARE = 0.0005
LOAD_TIMEOUT_MS = 90_000
QUIET_MS = 150
MASK_FILL = (255, 0, 255)
FIXED_TIME = "2027-01-01T00:00:00Z"  # after every fixture's events, or the clocks hide
SELF_CHECK = False  # --self-check: the C# client in place of the web client

# The expected differences: places where the C# client shows hidden information that the view
# deliberately leaves out (protocol.md, "Where the C# client shows more than the rules allow"),
# and where the C# client fails and the web client deliberately does something else.
# rust/view/examples/scene_views.rs decides per scene which apply (manifest "leaks"); `find` is
# JavaScript run in both pages that returns the rectangles to mask, in page coordinates. A
# masked rectangle is painted over in both screenshots and dropped from both DOM digests.
MASKS = {
    "partial-prescience": {
        "reason": "C# draws what prescience revealed of a battle plan for every viewer "
        "(BattlePlans.razor:19-25); the view shows it to the prescience side only.",
        # The partial plan is drawn at half opacity on the wheel image, and everything it
        # shows (the dial, a weapon, a leader) lies on the wheel.
        "find": """() => [...document.querySelectorAll('#mapdiv img')]
            .filter(i => (i.getAttribute('src') || '').endsWith('art/wheel.png') && Math.abs(parseFloat(getComputedStyle(i).opacity) - 0.5) < 0.05)
            .map(i => { const r = i.getBoundingClientRect();
                return {x: r.left + scrollX - 4, y: r.top + scrollY - 4, width: r.width + 8, height: r.height + 8}; })""",
    },
    "story-hide-secrets": {
        "reason": "C#'s end-of-game story shows the cards, spice and Nexus cards of factions "
        "that chose HideSecrets; the view leaves them out.",
        "find": """() => [...document.querySelectorAll('#storyModal .modal-body')]
            .map(e => e.getBoundingClientRect()).filter(r => r.width > 0)
            .map(r => ({x: r.left + scrollX, y: r.top + scrollY, width: r.width, height: r.height}))""",
    },
    "silent-auction-pass": {
        "reason": "C# colours a silent-auction bid by whether it passed (Bids.razor:25); the "
        "view sends 'ready' only.",
        "find": """() => [...document.querySelectorAll('#mapdiv *')]
            .filter(e => [...e.childNodes].some(n => n.nodeType === 3 && n.textContent.trim() === 'READY'))
            .map(e => e.getBoundingClientRect())
            .map(r => ({x: r.left + scrollX - 2, y: r.top + scrollY - 2, width: r.width + 4, height: r.height + 4}))""",
    },
    "observer-bidding-error": {
        "reason": "C#'s bidding panel throws for an observer (BiddingPanel.razor: Player is null), "
        "and Blazor shows its error bar over the bottom of the page; the view gives observers "
        "the panel of a player without prescience (protocol.md, deviation 5).",
        # The bar is fixed to the bottom of the window. A full-page screenshot draws it where it
        # is in the window, not at the bottom of a longer page. The C# page has no panel, so the
        # web client's is taken out of the layout (a mask would leave the log below it shifted).
        # It loses its card class too, or the prompt card above would lose its bottom border
        # (`.card:has(+ .card)`).
        "find": """() => { document.querySelectorAll('.bidding-panel').forEach(e => { e.style.display = 'none'; e.classList.remove('card'); });
            return [...document.querySelectorAll('#blazor-error-ui')]
            .map(e => e.getBoundingClientRect()).filter(r => r.width > 0 && r.height > 0)
            .map(r => ({x: r.left + scrollX, y: r.top + scrollY, width: r.width, height: r.height})); }""",
    },
    "flash-after-event": {
        "reason": "C# flashes cards only when the game page opens with a flash current "
        "(ImageFlashComponent sets the time on its first render); the web client also flashes "
        "after each new event (protocol.md, deviation 6).",
        # Only the web client marks its flash; the one shown on opening is "opened" and compared.
        "find": """() => [...document.querySelectorAll('[data-flash="event"]')]
            .map(e => e.getBoundingClientRect()).filter(r => r.width > 0 && r.height > 0)
            .map(r => ({x: r.left + scrollX, y: r.top + scrollY, width: r.width, height: r.height}))""",
    },
}

# Masks applied in every comparison, wherever their elements are found.
ALWAYS_MASKED = ["flash-after-event"]

# The DOM digest: one line per visible element, indented by depth, with its tag, its classes
# (sorted, without Blazor's b-xxxxxxxxxx scope classes), an image's src, a title, and its own
# text (a textarea's value) with whitespace collapsed. Comments, scripts, and React's and Blazor's internals leave
# no trace, and neither do elements inside a mask: a mask often covers something that only one
# client draws, so a marker line would differ too.
DIGEST_JS = """([selector, masks]) => {
    const root = selector ? document.querySelector(selector) : document.getElementById('app');
    if (!root) return null;
    const lines = [];
    const masked = r => masks.find(m => r.left + scrollX >= m.x - 1 && r.top + scrollY >= m.y - 1
        && r.right + scrollX <= m.x + m.width + 1 && r.bottom + scrollY <= m.y + m.height + 1);
    const clean = s => s.replace(/\\s+/g, ' ').trim();
    function walk(el, depth) {
        if (['SCRIPT', 'STYLE', 'TEMPLATE', 'NOSCRIPT'].includes(el.tagName)) return;
        const style = getComputedStyle(el);
        if (style.display === 'none') return;
        const r = el.getBoundingClientRect();
        const m = r.width > 0 && r.height > 0 && masked(r);
        if (m) return;
        const pad = '  '.repeat(depth);
        if (style.visibility !== 'hidden') {
            let line = el.tagName.toLowerCase();
            const cls = [...el.classList].filter(c => !/^b-[a-z0-9]{10}$/.test(c)).sort();
            if (cls.length) line += '.' + cls.join('.');
            if (el.tagName === 'IMG') line += ' src=' + (el.getAttribute('src') || '').split('?')[0].replace(/^\\.\\//, '/');
            const title = el.getAttribute('title');
            if (title) line += ' title="' + clean(title).slice(0, 80) + '"';
            // A textarea's text is its value: React writes a controlled value into the text
            // child too (defaultValue), Blazor sets only the value.
            const text = el.tagName === 'TEXTAREA' ? clean(el.value)
                : clean([...el.childNodes].filter(n => n.nodeType === 3).map(n => n.textContent).join(' '));
            if (text) line += ' "' + text + '"';
            lines.push(pad + line);
        }
        for (const c of el.children) walk(c, depth + 1);
    }
    walk(root, 0);
    return lines.join('\\n');
}"""

BOX_JS = """(selector) => {
    const e = document.querySelector(selector);
    if (!e) return null;
    const r = e.getBoundingClientRect();
    if (r.width === 0 || r.height === 0) return null;
    return {x: r.left + scrollX, y: r.top + scrollY, width: r.width, height: r.height};
}"""


def relative(path: Path) -> str:
    return os.path.relpath(path, REPO) if path.is_relative_to(REPO) else str(path)


# Building what is compared


def build() -> Path:
    """Publishes the C# client, builds the web client and writes the views of every fixture into
    a new directory under BUILDS, and makes it CURRENT. About a minute warm.

    A failed build is deleted and CURRENT stays, so --no-build still works while another agent's
    change leaves rust/view broken. The views are always written for every fixture (a few
    seconds), so CURRENT never holds only the scenes of one agent's partial run."""
    BUILDS.mkdir(parents=True, exist_ok=True)
    name = f"{time.strftime('%Y%m%d-%H%M%S')}-{os.getpid()}"
    building = BUILDS / f"{name}.building"
    building.mkdir()
    label = f"compare-{name}"  # publish-client.sh publishes to /tmp/treachery-client-<label>
    env = {**os.environ, "DOTNET_ROOT": os.environ.get("DOTNET_ROOT", str(Path.home() / "dotnet10"))}
    env["PATH"] = f"{env['DOTNET_ROOT']}:{env['PATH']}"
    steps = [
        ("C# client", [str(REPO / "scripts" / "publish-client.sh"), label], REPO),
        # vite build without tsc: a type error elsewhere in web/ should not stop a comparison.
        ("web client", ["npx", "vite", "build", "--outDir", str(building / "web"), "--emptyOutDir", "--logLevel", "warn"], WEB),
        ("views", ["cargo", "run", "--release", "-q", "-p", "treachery-view", "--example", "scene_views", "--", str(building / "views")], REPO / "rust"),
    ]
    procs = []
    for step, cmd, cwd in steps:
        log = building / f"build-{step.replace(' ', '-').replace('#', 'sharp')}.log"
        procs.append((step, log, subprocess.Popen(cmd, cwd=cwd, env=env, stdout=log.open("w"), stderr=subprocess.STDOUT)))
    failed = False
    for step, log, p in procs:
        if p.wait() != 0:
            failed = True
            print(f"building the {step} failed:\n{log.read_text()[-3000:]}", file=sys.stderr)
        else:
            print(f"built the {step}")
    published = Path(f"/tmp/treachery-client-{label}")
    if failed:
        shutil.rmtree(building, ignore_errors=True)
        shutil.rmtree(published, ignore_errors=True)
        sys.exit("build failed" + ("; `--no-build` compares the previous build" if CURRENT.exists() else ""))
    published.rename(building / "csharp")
    done = BUILDS / name
    building.rename(done)
    with build_lock():
        link = WORK / f".current-{os.getpid()}"
        link.unlink(missing_ok=True)
        link.symlink_to(done.relative_to(WORK))
        link.replace(CURRENT)  # atomic: a run sees the old build or the new one
    return done


@contextmanager
def build_lock():
    """Held while choosing a build to use, switching CURRENT, or deleting old builds."""
    import fcntl

    WORK.mkdir(parents=True, exist_ok=True)
    with open(WORK / ".lock", "w") as f:
        fcntl.flock(f, fcntl.LOCK_EX)
        try:
            yield
        finally:
            fcntl.flock(f, fcntl.LOCK_UN)


def alive(pid: int) -> bool:
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        pass
    return True


def use_build(build_dir: Path | None) -> None:
    """Makes this run use build_dir (else CURRENT's build) until it ends: the build is pinned
    with a file named by this process's id, which prune_builds respects."""
    global BUILD, CSHARP_CLIENT, WEB_DIST, VIEWS
    with build_lock():
        if build_dir is None:
            if not CURRENT.exists():
                sys.exit(f"no build in {WORK}; run without --no-build first")
            build_dir = CURRENT.resolve()
        (build_dir / "pins").mkdir(exist_ok=True)
        (build_dir / "pins" / str(os.getpid())).touch()
    BUILD, CSHARP_CLIENT, WEB_DIST, VIEWS = build_dir, build_dir / "csharp", build_dir / "web", build_dir / "views"
    prune_builds()


def release_build() -> None:
    (BUILD / "pins" / str(os.getpid())).unlink(missing_ok=True)


def prune_builds() -> None:
    """Deletes every build that is not CURRENT, not being built and not used by a running run."""
    with build_lock():
        current = CURRENT.resolve() if CURRENT.exists() else None
        for d in BUILDS.iterdir() if BUILDS.exists() else []:
            if d == current or not d.is_dir():
                continue
            if d.name.endswith(".building") and alive(int(d.name.removesuffix(".building").rsplit("-", 1)[1])):
                continue
            pins = [int(p.name) for p in (d / "pins").iterdir()] if (d / "pins").exists() else []
            if not any(alive(pid) for pid in pins):
                shutil.rmtree(d, ignore_errors=True)


# Serving the web client


class WebHandler(Handler):
    """The built web client, with the C# client's wwwroot (art, css, fonts) and the views.

    Production serves wwwroot next to dist/ the same way (vite.config.ts)."""

    SHARED = ("/art/", "/css/", "/fonts/", "/js/", "/docs/")

    def __init__(self, *args, dist: Path, views: Path, **kwargs):
        self.dist, self.views = dist, views
        super().__init__(*args, wwwroot=dist, **kwargs)

    def translate_path(self, path: str) -> str:
        route = urlsplit(path).path
        if route.startswith("/views/") and (self.views / route.removeprefix("/views/")).exists():
            return str(self.views / route.removeprefix("/views/"))
        if route.startswith(self.SHARED) or (route.count("/") == 1 and route.endswith((".ttf", ".ico"))):
            return str(WWWROOT / route.lstrip("/"))
        return super().translate_path(path)


class WebServer(StaticServer):
    def __init__(self, dist: Path, views: Path):
        if not (dist / "index.html").exists():
            raise FileNotFoundError(f"{dist}/index.html; build without --no-build")
        self._httpd = Server(("127.0.0.1", 0), partial(WebHandler, dist=dist, views=views))
        self._thread = threading.Thread(target=self._httpd.serve_forever, daemon=True)


# Scenes


@dataclass
class Scene:
    fixture: str
    faction: str  # enum name, or "Observer"
    leaks: list[str]

    @property
    def key(self) -> str:
        return f"{self.fixture}/{self.faction}"

    def csharp_url(self, base: str) -> str:
        return f"{base}/scene?fixture={quote(self.fixture)}&faction={self.faction.lower()}"

    def web_url(self, base: str) -> str:
        # When the Rust server's dev scene endpoint exists (client-plan.md package 2), the web
        # client's route opens the scene there instead of loading /views/; this URL stays.
        return f"{base}/scene/{quote(self.fixture)}/{self.faction}"


def load_manifest() -> tuple[list[Scene], dict[str, str]]:
    manifest = json.loads((VIEWS / "manifest.json").read_text())
    scenes, errors = [], {}
    for f in manifest["fixtures"]:
        # A fixture at an older game version is "skipped": the C# scene page refuses it, and it
        # is kept only for the Rust tests (scene_views.rs).
        if f.get("error"):
            errors[f["fixture"]] = f["error"]
        for s in f["scenes"]:
            scenes.append(Scene(f["fixture"], s["faction"], s["leaks"]))
    return scenes, errors


def select(scenes: list[Scene], wanted: list[str]) -> list[Scene]:
    if not wanted:
        return scenes
    chosen = [s for s in scenes if s.fixture in wanted or s.key.lower() in (w.lower() for w in wanted)]
    unknown = [w for w in wanted if not any(s.fixture == w or s.key.lower() == w.lower() for s in scenes)]
    if unknown:
        sys.exit(f"no scene for {', '.join(unknown)}; scenes look like alliance or alliance/Emperor")
    return chosen


# Capturing one client


@dataclass
class Capture:
    """One client's page at one viewport: a full-page screenshot, region boxes, digests."""

    image: Image.Image | None = None
    boxes: dict[str, dict | None] = field(default_factory=dict)
    digests: dict[str, str | None] = field(default_factory=dict)
    masks: list[dict] = field(default_factory=list)
    error: str | None = None


def open_scene(page: Page, url: str) -> str | None:
    """Loads a scene page and waits until it is ready. Returns the page's error, if any."""
    try:
        page.goto(url)
        page.wait_for_function("['ready', 'error'].includes(document.documentElement.dataset.scene)", timeout=LOAD_TIMEOUT_MS)
    except Exception as e:
        return f"did not load: {type(e).__name__}: {str(e).splitlines()[0]}"
    if page.evaluate("document.documentElement.dataset.scene") == "error":
        text = page.evaluate("window.treacheryScene?.error ?? document.body.innerText")
        return " ".join(str(text).split())[:500]
    page.evaluate("q => window.treacheryScene.settled(q)", QUIET_MS)
    return None


def find_masks(page: Page, leaks: list[str]) -> list[dict]:
    rects = []
    for name in leaks:
        for r in page.evaluate(MASKS[name]["find"]):
            rects.append({**r, "name": name})
    return rects


def capture(page: Page, masks: list[dict]) -> Capture:
    c = Capture(masks=masks)
    page.mouse.move(0, 0)
    # The C# client's follow-log script sometimes stops short of the newest line (a timing race;
    # the web log is always at the bottom), so both logs are scrolled to the bottom first.
    page.evaluate("() => document.querySelectorAll('.report .card-body').forEach(e => { e.scrollTop = e.scrollHeight; })")
    page.evaluate("q => window.treacheryScene.settled(q)", QUIET_MS)
    png = page.screenshot(full_page=True, animations="disabled", caret="hide")
    c.image = Image.open(io.BytesIO(png)).convert("RGB")
    draw = ImageDraw.Draw(c.image)
    for m in masks:
        draw.rectangle((m["x"], m["y"], m["x"] + m["width"], m["y"] + m["height"]), fill=MASK_FILL)
    for region, selector in REGIONS.items():
        if selector is None:
            c.boxes[region] = {"x": 0, "y": 0, "width": c.image.width, "height": c.image.height}
        else:
            c.boxes[region] = page.evaluate(BOX_JS, selector)
        c.digests[region] = page.evaluate(DIGEST_JS, [selector, masks]) if c.boxes[region] else None
    return c


# Comparing


@dataclass
class RegionResult:
    region: str
    viewport: str
    status: str  # match, differ, missing (C# only), extra (web only), absent (neither), error
    pixel_share: float | None = None  # share of differing pixels
    size: dict | None = None  # {"csharp": [w, h], "web": [w, h]}
    dom_equal: bool | None = None
    dom_diff: str | None = None  # unified diff of the digests, cut at DOM_DIFF_LINES
    # The region in each client's page shot as [x, y, width, height]; the shots are
    # <viewport>.<client>.webp in the scene's directory. "diff" is the diff image's file name.
    boxes: dict | None = None
    diff: str | None = None


DOM_DIFF_LINES = 400


def box(c: Capture, region: str) -> tuple[int, int, int, int]:
    b = c.boxes[region]
    x, y = round(b["x"]), round(b["y"])
    return x, y, round(b["x"] + b["width"]), round(b["y"] + b["height"])


def pixel_diff(a: Image.Image, b: Image.Image) -> tuple[float, Image.Image]:
    """The share of differing pixels over the larger canvas, and a diff image: the C# shot
    dimmed, with differing pixels in emperor. Where only one shot has pixels, they differ."""
    w, h = max(a.width, b.width), max(a.height, b.height)
    pa, pb = Image.new("RGB", (w, h), MASK_FILL), Image.new("RGB", (w, h), (0, 255, 0))
    pa.paste(a)
    pb.paste(b)
    bands = ImageChops.difference(pa, pb).split()
    worst = ImageChops.lighter(ImageChops.lighter(bands[0], bands[1]), bands[2])
    mask = worst.point(lambda v: 255 if v > CHANNEL_TOLERANCE else 0)
    differing = mask.histogram()[255]
    base = Image.blend(Image.new("RGB", (w, h)), pa.convert("L").convert("RGB"), 0.35)
    diff = Image.composite(Image.new("RGB", (w, h), (255, 40, 40)), base, mask)
    return differing / (w * h), diff


def save(img: Image.Image, path: Path) -> None:
    # Lossy WebP keeps a full run's gallery small; the comparison itself uses the lossless shot.
    img.save(path, "WEBP", quality=70, method=4)


def compare_region(region: str, viewport: str, cs: Capture, web: Capture, out: Path) -> RegionResult:
    r = RegionResult(region, viewport, "absent")
    has_cs, has_web = cs.boxes.get(region) is not None, web.boxes.get(region) is not None
    if not has_cs and not has_web:
        return r
    r.boxes = {}
    if has_cs:
        x0, y0, x1, y1 = box(cs, region)
        r.boxes["csharp"] = [x0, y0, x1 - x0, y1 - y0]
    if has_web:
        x0, y0, x1, y1 = box(web, region)
        r.boxes["web"] = [x0, y0, x1 - x0, y1 - y0]
    lines = list(difflib.unified_diff(
        (cs.digests.get(region) or "").splitlines(), (web.digests.get(region) or "").splitlines(),
        "csharp", "web", n=2, lineterm=""))
    r.dom_equal = not lines
    if len(lines) > DOM_DIFF_LINES:
        lines = lines[:DOM_DIFF_LINES] + [f"... {len(lines) - DOM_DIFF_LINES} more lines"]
    r.dom_diff = "\n".join(lines) or None
    if not has_web:
        r.status = "missing"
        return r
    if not has_cs:
        r.status = "extra"
        return r
    a, b = cs.image.crop(box(cs, region)), web.image.crop(box(web, region))
    r.size = {"csharp": list(a.size), "web": list(b.size)}
    r.pixel_share, diff = pixel_diff(a, b)
    same_pixels = a.size == b.size and r.pixel_share <= PIXEL_SHARE
    r.status = "match" if same_pixels and r.dom_equal else "differ"
    if r.pixel_share > 0:
        r.diff = f"{viewport}-{region}.diff.webp"
        save(diff, out / r.diff)
    return r


@dataclass
class SceneResult:
    """One comparison: a scene as loaded (step "start"), or a shot of a scripted comparison."""

    fixture: str
    faction: str
    leaks: list[str]
    dir: str = ""  # the result's directory, relative to OUT
    script: str | None = None  # the scripted comparison that took this shot
    step: str = "start"
    seconds: float = 0
    error: str | None = None  # the harness or the script failed
    csharp_error: str | None = None  # the C# scene page could not show the scene
    web_error: str | None = None
    masks: list[dict] = field(default_factory=list)
    regions: list[RegionResult] = field(default_factory=list)
    console: dict = field(default_factory=dict)

    @property
    def key(self) -> str:
        return f"{self.script}:{self.step}" if self.script else f"{self.fixture}/{self.faction}"


_stills: dict[str, bytes] = {}
_stills_lock = threading.Lock()


def still_gif(route) -> None:
    url = route.request.url
    with _stills_lock:
        body = _stills.get(url)
    if body is None:
        response = route.fetch()
        if response.status != 200:
            route.fulfill(response=response)
            return
        img = Image.open(io.BytesIO(response.body()))
        img.seek(0)
        buf = io.BytesIO()
        img.save(buf, "GIF")
        body = buf.getvalue()
        with _stills_lock:
            _stills[url] = body
    route.fulfill(status=200, body=body, content_type="image/gif")


class Contexts:
    """One browser context per client and viewport, kept for a worker's whole run so that the
    browser's caches keep the Blazor runtime between scenes. Local and session storage are
    cleared at the start of every page load, so no scene sees what an earlier one stored."""

    def __init__(self, browser: Browser):
        self.browser, self.contexts = browser, {}

    def get(self, client: str, viewport: str):
        key = (client, viewport)
        if key not in self.contexts:
            ctx = self.browser.new_context(viewport=VIEWPORTS[viewport], device_scale_factor=1, reduced_motion="reduce")
            ctx.add_init_script("try { localStorage.clear(); sessionStorage.clear(); } catch (e) {}")
            # Clocks (the turn banner's, the prompt's timer) show the time since an event, so the
            # pages see one fixed time: the same in both clients and in every run. Timers still run.
            ctx.clock.set_fixed_time(FIXED_TIME)
            # Animated GIFs (the worm around the acting faction) can't be paused, so both
            # clients get their first frame.
            ctx.route("**/*.gif", still_gif)
            self.contexts[key] = ctx
        return self.contexts[key]


class Pair:
    """Both clients on one scene at one viewport. Actions act on both; a shot compares them."""

    def __init__(self, contexts: "Contexts", scene: Scene, urls: dict[str, str], viewport: str, console: dict):
        self.scene, self.viewport = scene, viewport
        self.pages, self.errors = {}, {}
        # --server: the dev scene endpoint seats a faction only, so observers keep the saved views.
        web_base = urls.get("web-views", urls["web"]) if scene.faction == "Observer" else urls["web"]
        web_url = scene.csharp_url(urls["csharp"]) if SELF_CHECK else scene.web_url(web_base)
        for client, url in (("csharp", scene.csharp_url(urls["csharp"])), ("web", web_url)):
            page = contexts.get(client, viewport).new_page()
            sink = console.setdefault(client, [])
            page.on("console", lambda m, c=sink: c.append(m.text) if m.type == "error" and "play() failed" not in m.text else None)
            page.on("pageerror", lambda e, c=sink: c.append(str(e)))
            self.errors[client] = open_scene(page, url)
            self.pages[client] = page

    def close(self) -> None:
        for page in self.pages.values():
            page.close()

    @property
    def failed(self) -> bool:
        return any(self.errors.values())

    def click(self, name: str, shift: bool = False, ctrl: bool = False, right: bool = False) -> None:
        """Click a map location in both clients, as scripts/ui/harness.py's Game.click does."""
        for page in self.pages.values():
            page.evaluate("([n, o]) => window.treacheryScene.click(n, o)", [name, {"shift": shift, "ctrl": ctrl, "right": right}])
            page.evaluate("q => window.treacheryScene.settled(q)", QUIET_MS)

    def button(self, label: str, within: str | None = None) -> None:
        """Click a button by its label in both clients."""
        for page in self.pages.values():
            scope = page.locator(within) if within else page
            scope.get_by_role("button", name=label).first.click()
            page.evaluate("q => window.treacheryScene.settled(q)", QUIET_MS)

    def click_on(self, selector: str) -> None:
        """Click the first element matching a Playwright selector in both clients."""
        for page in self.pages.values():
            page.locator(selector).first.click()
            page.evaluate("q => window.treacheryScene.settled(q)", QUIET_MS)

    def show_popover(self, selector: str, index: int = 0, shift: bool = False) -> None:
        """Opens the Bootstrap popover of the index-th element matching a CSS selector in both
        clients, without hovering it (the shot moves the mouse away, which would close a hover
        popover). With shift, the page is put in the Shift-held state first."""
        for page in self.pages.values():
            ok = page.evaluate(
                """([sel, i, shift]) => {
                    document.body.classList.toggle('shift-held', shift);
                    document.querySelectorAll('[data-bs-toggle=popover]').forEach(e => window.bootstrap.Popover.getInstance(e)?.dispose());
                    document.querySelectorAll('.popover').forEach(p => p.remove());
                    const el = document.querySelectorAll(sel)[i];
                    if (!el) return false;
                    window.bootstrap.Popover.getOrCreateInstance(el).show();
                    return true;
                }""",
                [selector, index, shift],
            )
            if not ok:
                raise RuntimeError(f"no element {selector} [{index}]")
            page.wait_for_selector("body > .popover.show")
            page.evaluate("q => window.treacheryScene.settled(q)", QUIET_MS)

    def compare(self, res: SceneResult, out: Path) -> None:
        """Captures both pages and adds a result per region to res."""
        res.csharp_error, res.web_error = self.errors["csharp"], self.errors["web"]
        if self.failed:
            res.regions += [RegionResult(region, self.viewport, "error") for region in REGIONS]
            return
        masks = [m for p in self.pages.values() for m in find_masks(p, self.scene.leaks + ALWAYS_MASKED)]
        masks = list({tuple(sorted(m.items())): m for m in masks}.values())  # the same area in both
        if self.viewport == "desktop" or not res.masks:
            res.masks = masks
        cs, web = capture(self.pages["csharp"], masks), capture(self.pages["web"], masks)
        save(cs.image, out / f"{self.viewport}.csharp.webp")
        save(web.image, out / f"{self.viewport}.web.webp")
        res.regions += [compare_region(region, self.viewport, cs, web, out) for region in REGIONS]


def fresh_dir(rel: str) -> Path:
    out = OUT / rel
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)
    return out


def run_scene(scene: Scene, browser: Contexts, urls: dict[str, str]) -> list[SceneResult]:
    res = SceneResult(scene.fixture, scene.faction, scene.leaks, dir=f"{scene.fixture}/{scene.faction}")
    out = fresh_dir(res.dir)
    for viewport in VIEWPORTS:
        pair = Pair(browser, scene, urls, viewport, res.console)
        try:
            pair.compare(res, out)
        finally:
            pair.close()
    return [res]


class Script:
    """What a scripted comparison (scripts/ui/comparisons/<name>.py) gets: scenes to drive.

        def run(c):
            with c.scene("alliance", "Emperor") as s:
                s.shot("start")
                s.button("Pass")
                s.shot("passed")

    Each shot compares both clients and becomes one result, <name>:<shot> in the gallery.
    """

    def __init__(self, name: str, browser: Contexts, urls: dict[str, str], scenes: dict[str, Scene]):
        self.name, self.browser, self.urls, self.scenes = name, browser, urls, scenes
        self.results: list[SceneResult] = []

    @contextmanager
    def scene(self, fixture: str, faction: str, viewport: str = "desktop", server: bool = False):
        """`server=True` opens the web side on the Rust server's dev scene endpoint
        (scene_server.py), for steps that query or submit; saved views can't."""
        if server and not SELF_CHECK:
            from scene_server import SceneServer

            with SceneServer(WEB_DIST, tag=f"{self.name}-{fixture}-{faction}-{viewport}", clock=FIXED_TIME) as url:
                with Script(self.name, self.browser, {**self.urls, "web": url}, self.scenes).scene(fixture, faction, viewport) as s:
                    s.script = self
                    yield s
            return
        key = f"{fixture}/{faction}"
        scene = self.scenes.get(key) or Scene(fixture, faction, [])
        console: dict = {}
        pair = Pair(self.browser, scene, self.urls, viewport, console)
        script = self

        class Driver:
            script = self  # the Script whose results the shots join

            def __getattr__(self, attr):
                return getattr(pair, attr)

            def shot(self, step: str) -> None:
                res = SceneResult(fixture, faction, scene.leaks, dir=f"scripts/{self.script.name}/{step}", script=self.script.name, step=step)
                res.console = console
                pair.compare(res, fresh_dir(res.dir))
                self.script.results.append(res)

        try:
            if pair.failed:
                raise RuntimeError(f"{key} did not open: {pair.errors}")
            yield Driver()
        finally:
            pair.close()

    @contextmanager
    def lobby(self, viewport: str = "desktop"):
        """Both servers on empty databases; `user(name)` opens a user's page pair at /.

            with c.lobby() as lobby:
                alice = lobby.user("alice")
                alice.button("Log in")
                alice.shot("lobby")
        """
        from lobby_servers import LobbyServers

        script = self
        with LobbyServers(WEB_DIST, tag=self.name) as servers:
            pairs: list[LobbyPair] = []
            console: dict = {}

            class User:
                def __init__(self, name: str, path: str):
                    self.name = name
                    self.pair = LobbyPair(script.browser.browser, servers.urls, viewport, console, path)
                    pairs.append(self.pair)
                    if self.pair.failed:
                        raise RuntimeError(f"{name}: a page did not open: {self.pair.errors}")

                def __getattr__(self, attr):
                    return getattr(self.pair, attr)

                def shot(self, step: str) -> None:
                    res = SceneResult("lobby", self.name, [], dir=f"scripts/{script.name}/{step}", script=script.name, step=step)
                    res.console = console
                    self.pair.compare(res, fresh_dir(res.dir))
                    script.results.append(res)

            class Lobby:
                urls = servers.urls

                def user(self, name: str, path: str = "/") -> User:
                    return User(name, path)

            try:
                yield Lobby()
            finally:
                for p in pairs:
                    p.close()


# Lobby comparisons (client-plan.md package 8): login, the lobby, setup and seating need a game
# server, so they run against the C# server and the Rust server (lobby_servers.py) instead of the
# scene pages. A user is a pair of pages, one per client, that every action drives alike.

LOBBY_INIT_JS = """(() => {
    // The scene pages' settled() and still style, for pages that are not scenes.
    const quiet = q => new Promise(resolve => {
        let timer;
        const check = () => {
            if (document.getAnimations().some(a => a.playState === 'running')) { schedule(); return; }
            observer.disconnect(); resolve();
        };
        const schedule = () => { clearTimeout(timer); timer = setTimeout(check, q || 100); };
        const observer = new MutationObserver(schedule);
        observer.observe(document.documentElement, { subtree: true, childList: true, attributes: true, characterData: true });
        schedule();
    });
    if (!window.treacheryScene) window.treacheryScene = { settled: quiet };
    else if (!window.treacheryScene.settled) window.treacheryScene.settled = quiet;
    document.addEventListener('DOMContentLoaded', () => {
        const style = document.createElement('style');
        style.textContent = '*, *::before, *::after { transition: none !important; animation: none !important; }';
        document.head.appendChild(style);
    });
})()"""


class LobbyPair(Pair):
    """One user in both clients: the C# client on the C# server and the web client on the Rust
    server, each opened at `path` in a fresh browser context (its own local storage)."""

    def __init__(self, browser: Browser, urls: dict[str, str], viewport: str, console: dict, path: str = "/"):
        self.scene, self.viewport = Scene("lobby", "", []), viewport
        self.pages, self.errors, self.contexts = {}, {}, []
        for client in ("csharp", "web"):
            ctx = browser.new_context(viewport=VIEWPORTS[viewport], device_scale_factor=1, reduced_motion="reduce")
            ctx.add_init_script(LOBBY_INIT_JS)
            ctx.route("**/*.gif", still_gif)
            self.contexts.append(ctx)
            page = ctx.new_page()
            sink = console.setdefault(client, [])
            page.on("console", lambda m, c=sink: c.append(m.text) if m.type == "error" and "play() failed" not in m.text and "play() request" not in m.text else None)
            page.on("pageerror", lambda e, c=sink: c.append(str(e)))
            try:
                page.goto(urls[client] + path)
                self.errors[client] = None
            except Exception as e:
                self.errors[client] = f"did not load: {e}"
            self.pages[client] = page

    def close(self) -> None:
        for ctx in self.contexts:
            ctx.close()

    def settle(self) -> None:
        for page in self.pages.values():
            page.evaluate("q => window.treacheryScene.settled(q)", QUIET_MS)

    def wait_text(self, text: str, timeout_ms: int = 30_000) -> None:
        """Waits until both pages show `text`."""
        for page in self.pages.values():
            page.get_by_text(text, exact=False).first.wait_for(timeout=timeout_ms)
        self.settle()

    def fill(self, selector: str, text: str) -> None:
        """Types into the first element matching a Playwright selector in both clients."""
        for page in self.pages.values():
            page.locator(selector).first.fill(text)
            page.locator(selector).first.dispatch_event("change")
        self.settle()

    def wait(self, ms: int) -> None:
        for page in self.pages.values():
            page.wait_for_timeout(ms)
        self.settle()


COMPARISONS = HERE / "comparisons"


def script_names() -> list[str]:
    return sorted(p.stem for p in COMPARISONS.glob("*.py"))


def run_script(name: str, browser: Contexts, urls: dict[str, str], scenes: dict[str, Scene]) -> list[SceneResult]:
    spec = importlib.util.spec_from_file_location(f"comparisons.{name}", COMPARISONS / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    script = Script(name, browser, urls, scenes)
    try:
        module.run(script)
    except Exception:
        script.results.append(SceneResult("", "", [], script=name, step="(failed)", error=traceback.format_exc()))
    return script.results


def worker(jobs: queue.Queue, results: list[SceneResult], urls: dict[str, str], scenes: dict[str, Scene], echo, headed: bool) -> None:
    with sync_playwright() as p:
        chromium = p.chromium.launch(headless=not headed)
        browser = Contexts(chromium)
        try:
            while True:
                try:
                    job = jobs.get_nowait()
                except queue.Empty:
                    return
                started = time.monotonic()
                name = job.key if isinstance(job, Scene) else f"{job}:"
                try:
                    batch = run_scene(job, browser, urls) if isinstance(job, Scene) else run_script(job, browser, urls, scenes)
                except Exception as e:
                    batch = [SceneResult(job.fixture, job.faction, job.leaks, error=traceback.format_exc())]
                    echo(f"{name:36} FAILED: {type(e).__name__}: {str(e).splitlines()[0] if str(e) else ''}")
                seconds = time.monotonic() - started
                for res in batch:
                    res.seconds = seconds / len(batch)
                    results.append(res)
                    counts: dict[str, int] = {}
                    for r in res.regions:
                        counts[r.status] = counts.get(r.status, 0) + 1
                    summary = ", ".join(f"{n} {s}" for s, n in sorted(counts.items()))
                    for what, err in (("C#", res.csharp_error), ("web", res.web_error), ("error", res.error)):
                        if err:
                            summary += f"; {what}: {err.strip().splitlines()[-1][:120]}"
                    echo(f"{res.key:36} {res.seconds:5.1f}s  {summary}")
        finally:
            chromium.close()


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("only", nargs="*", metavar="NAME",
                    help="fixtures (alliance), scenes (alliance/Emperor) or scripted comparisons (scripts/ui/comparisons); all if none")
    ap.add_argument("--no-build", action="store_true", help="reuse the newest complete build in " + str(CURRENT))
    ap.add_argument("--workers", type=int, default=4, help="scenes at once (default 4; each opens two pages)")
    ap.add_argument("--require", action="append", default=[], metavar="REGION",
                    help="exit 1 unless every compared scene matches in REGION (repeatable; 'all' for every region): "
                    "a client package's done check")
    ap.add_argument("--self-check", action="store_true",
                    help="compare the C# client with itself, which must match everywhere: a test of the harness "
                    "and its masks. Writes to $COMPARE_WORK/self-check unless --out is given")
    ap.add_argument("--out", type=Path, metavar="DIR",
                    help="this run's directory (default $COMPARE_OUT, else design/compare/runs/<time>-<pid>). "
                    "Only a directory under design/compare/runs is merged into the shared design/compare/gallery.html")
    ap.add_argument("--headed", action="store_true", help="show the browsers")
    ap.add_argument("--server", action="store_true",
                    help="open the web side on the Rust server's dev scene endpoint (scene_server.py) instead of the "
                    "saved views, so forms that submit by themselves (TreacheryCalled's pass, BattleInitiated) "
                    "advance as in the C# scene page")
    ap.add_argument("--list", action="store_true", help="list the scenes and scripted comparisons, and exit")
    args = ap.parse_args()
    for r in args.require:
        if r != "all" and r not in REGIONS:
            sys.exit(f"--require takes one of {', '.join(REGIONS)} or all, not {r!r}")

    global SELF_CHECK, OUT
    SELF_CHECK = args.self_check
    if args.out:
        OUT = args.out.resolve()
    elif os.environ.get("COMPARE_OUT"):
        OUT = Path(os.environ["COMPARE_OUT"]).resolve()
    elif args.self_check:
        OUT = WORK / "self-check"
    else:
        OUT = ROOT / "runs" / f"{time.strftime('%Y%m%d-%H%M%S')}-{os.getpid()}"
    scripts = [n for n in args.only if n in script_names()] if args.only else script_names()
    wanted = [n for n in args.only if n not in scripts]
    use_build(None if args.no_build else build())
    try:
        run(args, scripts, wanted)
    finally:
        release_build()


def run(args, scripts: list[str], wanted: list[str]) -> None:
    all_scenes, fixture_errors = load_manifest()
    scenes = select(all_scenes, wanted) if wanted or not args.only else []
    if args.list:
        for s in scenes:
            print(f"{s.key:36} {' '.join(s.leaks)}")
        for name in scripts:
            spec = importlib.util.spec_from_file_location(name, COMPARISONS / f"{name}.py")
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            print(f"{name + ':':36} {(module.__doc__ or '').strip().splitlines()[0]}")
        return

    by_key = {s.key: s for s in all_scenes}
    OUT.mkdir(parents=True, exist_ok=True)  # now, so pruning by another run sees it in progress
    print(f"writing to {relative(OUT)}")
    from contextlib import ExitStack
    n_workers = max(1, min(args.workers, len(scenes) + len(scripts)))
    with StaticServer(CSHARP_CLIENT) as cs_server, WebServer(WEB_DIST, VIEWS) as web_server, ExitStack() as stack:
        urls = {"csharp": cs_server.url, "web": web_server.url}
        # --server: one Rust server per worker, because opening a scene closes the scene user's
        # earlier games on that server (rust/server/src/dev.rs).
        worker_urls = [urls] * n_workers
        if args.server:
            from scene_server import SceneServer
            servers = [stack.enter_context(SceneServer(WEB_DIST, tag=f"compare-{i}", clock=FIXED_TIME)) for i in range(n_workers)]
            worker_urls = [{**urls, "web": server, "web-views": web_server.url} for server in servers]
            urls = worker_urls[0]
        print(f"C# client {CSHARP_CLIENT} at {urls['csharp']}; web client {WEB_DIST} at {urls['web']}; "
              f"{len(scenes)} scenes, {len(scripts)} scripted comparisons")
        jobs: queue.Queue = queue.Queue()
        for job in [*scripts, *scenes]:  # scripts first: they run longest
            jobs.put(job)
        results: list[SceneResult] = []
        lock = threading.Lock()

        def echo(line: str) -> None:
            with lock:
                print(line, flush=True)

        started = time.monotonic()
        threads = [threading.Thread(target=worker, args=(jobs, results, worker_urls[i], by_key, echo, args.headed), daemon=True)
                   for i in range(n_workers)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()
        elapsed = time.monotonic() - started

    from compare_report import publish, write_run
    results.sort(key=lambda r: (r.script or "", r.fixture, r.faction, r.step))
    data = {
        "written": time.strftime("%Y-%m-%d %H:%M:%S"),
        "partial": bool(args.only),
        "selfCheck": SELF_CHECK,
        "viewports": VIEWPORTS,
        "regions": dict(REGIONS),
        "tolerance": {"channel": CHANNEL_TOLERANCE, "pixelShare": PIXEL_SHARE},
        "masks": {k: v["reason"] for k, v in MASKS.items()},
        "fixtureErrors": fixture_errors,
        "scenes": [asdict(r) for r in results],
        "scripts": scripts,
    }
    gallery = write_run(OUT, data)
    shared = publish(ROOT, OUT, data)
    print(f"\n{len(results)} comparisons in {elapsed:.0f}s; wrote {relative(gallery)} and {relative(OUT / 'results.json')}"
          + (f"; merged into {relative(shared)}" if shared else ""))

    # --require and everything below use this run's results only.
    ok = not any(r.error or r.web_error for r in results)
    for line in summary_lines(results) + failure_lines(results):
        print(line)
    required = list(REGIONS) if "all" in args.require else args.require
    for region in required:
        bad = [r for r in results if any(x.region == region and x.status not in ("match", "absent") for x in r.regions)]
        if bad:
            ok = False
            print(f"--require {region}: {len(bad)} of {len(results)} comparisons differ, e.g. {', '.join(r.key for r in bad[:5])}")
    sys.exit(0 if ok else 1)


def summary_lines(results: list[SceneResult]) -> list[str]:
    lines = [f"{'region':14} " + "  ".join(f"{v:>16}" for v in VIEWPORTS)]
    for region in REGIONS:
        cells = []
        for viewport in VIEWPORTS:
            rs = [x for r in results for x in r.regions if x.region == region and x.viewport == viewport]
            good = sum(x.status in ("match", "absent") for x in rs)
            cells.append(f"{good:>5}/{len(rs):<4} match")
        lines.append(f"{region:14} " + "  ".join(f"{c:>16}" for c in cells))
    return lines


STATUS_TEXT = {"differ": "differs", "missing": "only in C#", "extra": "only in web", "error": "failed"}


def failure_lines(results: list[SceneResult]) -> list[str]:
    """Every comparison of this run that did not match, with its regions per viewport."""
    lines = []
    for res in results:
        parts = []
        for what, err in (("harness", res.error), ("C#", res.csharp_error), ("web", res.web_error)):
            if err:
                parts.append(f"{what} error: {err.strip().splitlines()[-1][:120]}")
        for viewport in VIEWPORTS:
            bad = []
            for r in res.regions:
                if r.viewport != viewport or r.status in ("match", "absent", "error"):
                    continue
                note = STATUS_TEXT.get(r.status, r.status)
                if r.status == "differ":
                    why = [f"{r.pixel_share:.2%} px" if r.pixel_share else "", "" if r.dom_equal else "DOM"]
                    if r.size and r.size["csharp"] != r.size["web"]:
                        why.append("size")
                    note += f" ({', '.join(w for w in why if w)})"
                bad.append(f"{r.region} {note}")
            if bad:
                parts.append(f"{viewport}: {', '.join(bad)}")
        if parts:
            page = f"{relative(OUT / res.dir)}/index.html" if res.dir else ""
            lines.append(f"  {res.key:36} {'; '.join(parts)}" + (f"\n  {'':36} {page}" if page else ""))
    if not lines:
        return [f"\nall {len(results)} comparisons match"]
    return [f"\n{len(lines)} of {len(results)} comparisons do not match:"] + lines


if __name__ == "__main__":
    main()
