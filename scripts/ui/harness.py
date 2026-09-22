"""Drive the client's scene page for design screenshots.

A scenario receives a Run and opens games from it:

    with run.game("ship", "red") as g:
        g.shot("ship-1-start")
        if run.after:
            g.click("Carthag")
            g.shot("ship-2-pinned", zoom="Carthag")

Run knows the mode (before or after), the client's URL and the output
directory. Game wraps one browser page on the scene page, which loads a
fixture from scripts/fixtures without a server, seats the given faction,
and exposes window.treacheryScene: the map's locations by name, a click
that goes straight to the component, and a promise that resolves once the
page has stopped changing. Everything here waits on that promise instead
of sleeping, so a scenario only has to say what to do.

Locations are named as the skin describes them, "Tuek's Sietch" or
"False Wall South (East Sector)". Anything that takes a location also
takes an (x, y) tuple in map coordinates, for points that are not a
location's centre.
"""

from __future__ import annotations

import difflib
from contextlib import contextmanager
from dataclasses import dataclass, field
from pathlib import Path
from urllib.parse import quote, urlencode

from playwright.sync_api import Browser, Page

SCRIPTS = Path(__file__).resolve().parent.parent
REPO = SCRIPTS.parent
FIXTURES = SCRIPTS / "fixtures"

FACTIONS = ("green", "black", "yellow", "red", "orange", "blue", "grey", "purple")

VIEWPORT = {"width": 1600, "height": 1000}
SCALE = 2  # device pixels per CSS pixel, so PNGs are 3200x2000
QUIET_MS = 150  # how long the DOM must stay unchanged to count as settled
LOAD_TIMEOUT_MS = (
    90_000  # the runtime downloads and starts on the first page of each browser
)
ZOOM_SIZE = (520, 360)  # default clip around a map point, in CSS pixels
ZOOM_DY = -60  # the pinned panel hangs above the point, so look further up

# Console errors that come from the browser's rules rather than the client:
# headless Chromium refuses to play the phase sounds without a user gesture.
IGNORED_CONSOLE = ("play() failed because the user didn't interact",)

Point = tuple[float, float]
Place = str | Point


def relative(path: Path) -> str:
    return str(path.relative_to(REPO)) if path.is_relative_to(REPO) else str(path)


class Log:
    """Prints lines with a prefix, so the output of parallel scenarios stays readable."""

    def __init__(self, prefix: str, write=print):
        self.prefix = prefix
        self.write = write

    def __call__(self, message: str) -> None:
        self.write(f"{self.prefix:14} {message}")


@dataclass
class Run:
    """One scenario's slice of an invocation of shots.py: mode, output, client URL, browser."""

    mode: str  # "before" or "after"
    out: Path
    url: str
    browser: Browser
    log: Log = field(default_factory=lambda: Log(""))
    notes: dict = field(default_factory=dict)
    errors: list[str] = field(
        default_factory=list
    )  # console errors from every game so far

    @property
    def after(self) -> bool:
        return self.mode == "after"

    @property
    def before(self) -> bool:
        return self.mode == "before"

    @contextmanager
    def game(self, fixture: str, faction: str, steps: list[str] | None = None):
        """Open a page on scripts/fixtures/<fixture>.json, seated as the faction.

        steps are applied by the page before it reports ready, in the scene
        page's own syntax: "click:Carthag", "shift:Sietch Tabr", "ctrl:...",
        "right:...", "button:Confirm". Handy for a starting state; when a shot
        is taken between clicks, use g.click so the sequence reads in one place.
        """
        path = FIXTURES / f"{fixture}.json"
        if not path.exists():
            raise FileNotFoundError(f"{path}; make it with: just fixture {fixture}")
        if faction not in FACTIONS:
            raise ValueError(
                f"faction must be one of {', '.join(FACTIONS)}, not {faction!r}"
            )
        self.out.mkdir(parents=True, exist_ok=True)
        params = {"fixture": fixture, "faction": faction}
        if steps:
            params["steps"] = "|".join(steps)
        url = f"{self.url}/scene?{urlencode(params, quote_via=quote)}"
        context = self.browser.new_context(viewport=VIEWPORT, device_scale_factor=SCALE)
        try:
            page = context.new_page()
            g = Game(self, page)
            g._open(url)
            yield g
        finally:
            context.close()

    def note(self, key: str, value) -> None:
        """Record something observed while driving, such as whether a pin appeared."""
        self.notes[key] = value
        self.log(f"{key}: {value}")


class Game:
    """One browser page on the scene page, with a game loaded."""

    def __init__(self, run: Run, page: Page):
        self.run = run
        self.page = page
        self.locations: dict[str, dict] = {}
        page.on("console", lambda m: self._error(m.text) if m.type == "error" else None)
        page.on("pageerror", lambda e: self._error(str(e)))

    # Map actions

    def click(
        self, name: str, shift: bool = False, ctrl: bool = False, right: bool = False
    ) -> None:
        """Click a location by name. The click is delivered to the component, not through the mouse."""
        self._check(name)
        self.page.evaluate(
            "([n, o]) => window.treacheryScene.click(n, o)",
            [name, {"shift": shift, "ctrl": ctrl, "right": right}],
        )
        self.settle()

    def hover(self, at: Place) -> None:
        """Move the mouse to a location's centre, or to an (x, y) map point."""
        self.page.mouse.move(*self.screen(at))
        self.settle()

    def park(self) -> None:
        """Move the mouse to the corner so nothing on the map is hovered."""
        self.page.mouse.move(5, 5)
        self.settle()

    def button(self, name: str, within: str | None = None) -> None:
        """Click a button by its label, optionally inside a CSS selector such as '.map-pin'."""
        scope = self.page.locator(within) if within else self.page
        scope.get_by_role("button", name=name).click()
        self.settle()

    def hide_popovers(self) -> None:
        """Hide the intel popover, for shots where it would cover the subject."""
        self.settle()
        self.page.evaluate(
            "document.querySelectorAll('.popover').forEach(p => p.style.display = 'none')"
        )

    def settle(self, quiet_ms: int = QUIET_MS) -> None:
        """Return once the page has stopped changing. Every action ends with this."""
        self.page.evaluate("q => window.treacheryScene.settled(q)", quiet_ms)

    # Looking at the page

    def count(self, selector: str) -> int:
        return self.page.locator(selector).count()

    def eval(self, js: str):
        """Run JavaScript in the page and return its result."""
        return self.page.evaluate(js)

    def wait_gone(self, text: str, timeout: float = 20) -> bool:
        """Wait for text to leave the page. Returns False on timeout instead of raising."""
        try:
            self.page.wait_for_selector(
                f"text={text}", state="detached", timeout=timeout * 1000
            )
            self.settle()
            return True
        except Exception:
            return False

    def screen(self, at: Place) -> tuple[float, float]:
        """Viewport coordinates of a location's centre or of an (x, y) map point."""
        x, y = self.point(at)
        box = self.page.evaluate("""() => {
            const inner = document.getElementById('planetmap_inner');
            const r = inner.getBoundingClientRect();
            return {left: r.left, top: r.top, scale: r.width / inner.offsetWidth};
        }""")
        return box["left"] + x * box["scale"], box["top"] + y * box["scale"]

    def point(self, at: Place) -> Point:
        """Map coordinates of a location, or the tuple itself."""
        if isinstance(at, str):
            self._check(at)
            return self.locations[at]["x"], self.locations[at]["y"]
        return at

    # Screenshots. Names are file names without the .png.

    def shot(
        self,
        name: str,
        zoom: Place | tuple[Place, Place] | None = None,
        size: tuple[int, int] = ZOOM_SIZE,
        dy: int = ZOOM_DY,
        park: bool = True,
    ) -> None:
        """Full-page shot. With zoom, also writes <name>-zoom.png clipped around that place.

        park=False keeps the mouse where it is, for shots of hover effects.
        """
        if park:
            self.park()
        self.settle()
        self._write(name, None)
        if zoom is not None:
            self.zoom(f"{name}-zoom", zoom, size, dy)

    def zoom(
        self,
        name: str,
        at: Place | tuple[Place, Place],
        size: tuple[int, int] = ZOOM_SIZE,
        dy: int = ZOOM_DY,
    ) -> None:
        """Clip around a location or (x, y) point, or around the midpoint of a pair of them."""
        if isinstance(at, tuple) and not isinstance(at[0], (int, float)):
            (ax, ay), (bx, by) = self.point(at[0]), self.point(at[1])
            at = ((ax + bx) / 2, (ay + by) / 2)
        sx, sy = self.screen(at)
        w, h = size
        self.clip(name, sx - w / 2, sy - h / 2 + dy, w, h)

    def element_shot(
        self, name: str, selector: str, height: int | None = None, park: bool = True
    ) -> None:
        """Clip to the first element matching a CSS selector, optionally capped in height."""
        if park:
            self.park()
        self.settle()
        r = self.page.locator(selector).first.bounding_box()
        if r is None:
            raise LookupError(f"nothing visible matches {selector!r}")
        self.clip(
            name, r["x"], r["y"], r["width"], min(r["height"], height or r["height"])
        )

    def clip(self, name: str, x: float, y: float, w: float, h: float) -> None:
        """Shot of a rectangle given in CSS pixels of the viewport."""
        self._write(name, {"x": x, "y": y, "width": w, "height": h})

    # Internals

    def _error(self, text: str) -> None:
        if not any(i in text for i in IGNORED_CONSOLE):
            self.run.errors.append(text)

    def _write(self, name: str, clip: dict | None) -> None:
        path = self.run.out / f"{name}.png"
        self.page.screenshot(
            path=str(path), clip=clip, animations="disabled", caret="hide"
        )
        self.run.log(f"wrote {relative(path)}")

    def _check(self, name: str) -> None:
        if name in self.locations:
            return
        close = difflib.get_close_matches(name, self.locations, n=5, cutoff=0.5)
        hint = f"; did you mean {', '.join(repr(c) for c in close)}?" if close else ""
        raise KeyError(f"no location named {name!r}{hint}")

    def _open(self, url: str) -> None:
        page = self.page
        page.goto(url)
        page.wait_for_function(
            "['ready', 'error'].includes(document.documentElement.dataset.scene)",
            timeout=LOAD_TIMEOUT_MS,
        )
        if page.evaluate("document.documentElement.dataset.scene") == "error":
            text = page.evaluate(
                "window.treacheryScene?.error ?? document.body.innerText"
            )
            raise RuntimeError(
                f"scene page failed for {url}: {' '.join(str(text).split())[:500]}"
            )
        self.locations = page.evaluate("window.treacheryScene.locations")
        self.settle()
