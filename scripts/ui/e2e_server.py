"""End to end: the web client against the running Rust server, in a browser.

Starts rust/target/release/treachery-server on a fresh database with the web build
(web/dist) and dev scenes on, then drives Chromium through Playwright:

0. Before any other client connects, the scene route (/scene/alliance/Emperor) opens a game on
   the server through POST /dev/scene. The scene user must be seated as the Emperor (their
   player panel, named Emperor, and the alliance form), and passing must replace the
   view. The first client's Connect on a fresh server restores the running games from the
   database, so this is the case where a seat that was never saved got lost.
1. Two users log in (accounts are created on first use) in two browser contexts, through the
   web client's own connection code (window.treacheryClient). The lobby screens don't exist yet
   (client-plan.md, package 8), so this is the only step not done by clicking.
2. Alice creates a game from the alliance fixture, takes the Emperor's seat and opens the
   Atreides seat; Bob joins it.
3. Both open the game screen (/<token>/<gameId>). Alice offers an alliance to the Atreides by
   clicking, Bob offers one back, and both screens update to show the alliance (the log's Nexus
   line, and the offer form leaves).
4. The scene route again, now on a server with other clients: passing in the alliance phase
   replaces the view.

Build first: `cargo build --release -p treachery-server` (from rust/) and `npm run build`
(from web/). Run: `python3 scripts/ui/e2e_server.py [--port 5077] [--shots /tmp/e2e]`.
Exits non-zero on the first failed check.
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path

from playwright.sync_api import Page, sync_playwright

ROOT = Path(__file__).resolve().parents[2]
SERVER = ROOT / "rust/target/release/treachery-server"
FIXTURE = ROOT / "scripts/fixtures/alliance.json"
EMPEROR_SEAT, ATREIDES_SEAT = 0, 3


def start_server(port: int, db: Path) -> subprocess.Popen:
    env = {
        **os.environ,
        "TREACHERY_DATABASE": str(db),
        "TREACHERY_LISTEN": f"127.0.0.1:{port}",
        "TREACHERY_DEV_SCENE": "true",
    }
    proc = subprocess.Popen([str(SERVER)], env=env, cwd=tempfile.gettempdir())
    for _ in range(100):
        try:
            urllib.request.urlopen(f"http://127.0.0.1:{port}/", timeout=1)
            return proc
        except Exception:
            if proc.poll() is not None:
                sys.exit(f"the server exited with {proc.returncode}")
            time.sleep(0.1)
    proc.kill()
    sys.exit("the server did not start")


def check(condition: bool, what: str) -> None:
    if not condition:
        raise AssertionError(what)
    print(f"ok  {what}")


SETUP_HOST = """async ([savegame, emperorSeat, atreidesSeat]) => {
  const { Connection, login } = window.treacheryClient;
  const conn = new Connection();
  const me = await login(conn, "alice", "alice-password", { email: null, playerName: "Alice" });
  const info = await conn.invoke("RequestCreateGame", "E2E", me.Token, "", savegame, "");
  await conn.invoke("RequestAssignSeats", me.Token, info.GameId, [[me.UserId, emperorSeat]]);
  await conn.invoke("RequestOpenOrCloseSeat", me.Token, info.GameId, atreidesSeat);
  conn.close();
  return { token: me.Token, gameId: info.GameId };
}"""

SETUP_GUEST = """async ([gameId, seat]) => {
  const { Connection, login } = window.treacheryClient;
  const conn = new Connection();
  const me = await login(conn, "bobby", "bob-password", { email: null, playerName: "Bobby" });
  await conn.invoke("RequestJoinGame", me.Token, gameId, "", seat);
  conn.close();
  return { token: me.Token };
}"""


def offer_alliance(page: Page, to: str) -> None:
    form = page.locator(".card", has_text="Offer a faction to form an Alliance")
    form.locator(f"img[alt='{to}']").click()
    button = form.get_by_role("button", name="Offer Alliance")
    button.wait_for()
    page.wait_for_function("() => [...document.querySelectorAll('button')].some(b => b.textContent.trim() === 'Offer Alliance' && !b.disabled)")
    button.click()


def play_scene(browser, base: str, shots: Path, name: str) -> None:
    """Opens /scene/alliance/Emperor, checks that it shows the Emperor's screen, and passes."""
    scene = browser.new_context(viewport={"width": 1440, "height": 820}).new_page()
    scene.on("pageerror", lambda e: print(f"page error: {e}"))
    scene.goto(base + "/scene/alliance/Emperor")
    scene.wait_for_function("() => document.documentElement.dataset.scene !== 'loading'", timeout=20000)
    check(scene.evaluate("document.documentElement.dataset.scene") == "ready", f"{name}: the scene opened on the server")
    scene.screenshot(path=str(shots / f"{name}.png"))
    panel = scene.locator(".player-panel")
    check(panel.count() == 1 and panel.locator(".pp-bar").first.inner_text().strip().startswith("Emperor"),
          f"{name}: the scene user is seated, with the Emperor's player panel")
    check(scene.get_by_text("Offer a faction to form an Alliance").count() == 1, f"{name}: the Emperor's alliance form is shown")
    scene.evaluate("window.treacheryScene.button('Pass')")
    scene.get_by_role("button", name="Pass").wait_for(state="detached", timeout=15000)
    check(True, f"{name}: the scene user passed, and the server sent a new view")
    scene.screenshot(path=str(shots / f"{name}-passed.png"))
    scene.context.close()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, default=5077)
    parser.add_argument("--shots", type=Path, default=Path(tempfile.gettempdir()) / "treachery-e2e")
    parser.add_argument("--headed", action="store_true")
    args = parser.parse_args()
    args.shots.mkdir(parents=True, exist_ok=True)
    check(SERVER.exists(), f"{SERVER.relative_to(ROOT)} is built")
    check((ROOT / "web/dist/index.html").exists(), "web/dist is built")

    with tempfile.TemporaryDirectory() as tmp:
        server = start_server(args.port, Path(tmp) / "e2e.db")
        base = f"http://127.0.0.1:{args.port}"
        try:
            with sync_playwright() as pw:
                browser = pw.chromium.launch(headless=not args.headed)
                alice = browser.new_context(viewport={"width": 1440, "height": 820}).new_page()
                bob = browser.new_context(viewport={"width": 1440, "height": 820}).new_page()
                for p in (alice, bob):
                    p.on("pageerror", lambda e: print(f"page error: {e}"))

                # 0: a scene first, on the fresh server.
                play_scene(browser, base, args.shots, "0-first-scene")

                # 1-2: log in, create, seat.
                alice.goto(base + "/")
                host = alice.evaluate(SETUP_HOST, [FIXTURE.read_text(), EMPEROR_SEAT, ATREIDES_SEAT])
                check(bool(host["gameId"]), "Alice logged in and created a game from the alliance fixture")
                bob.goto(base + "/")
                guest = bob.evaluate(SETUP_GUEST, [host["gameId"], ATREIDES_SEAT])
                check(bool(guest["token"]), "Bob logged in and joined the Atreides seat")

                # 3: both screens, alliance events by clicking.
                alice.goto(f"{base}/{host['token']}/{host['gameId']}")
                bob.goto(f"{base}/{guest['token']}/{host['gameId']}")
                for p, who in ((alice, "Alice"), (bob, "Bob")):
                    p.get_by_text("Offer a faction to form an Alliance").wait_for(timeout=15000)
                    check(True, f"{who} sees the alliance form in a view from the server")
                alice.screenshot(path=str(args.shots / "1-alice-start.png"))

                offer_alliance(alice, "Atreides")
                bob.get_by_text("Emperor").first.wait_for()
                offer_alliance(bob, "Emperor")
                for p, who in ((alice, "Alice"), (bob, "Bob")):
                    try:
                        p.get_by_text("Offer a faction to form an Alliance").wait_for(state="detached", timeout=15000)
                        # The log shows the new alliance as "Nexus: <Emperor> <ally> <Atreides>".
                        p.get_by_text(re.compile("^nexus$", re.I)).first.wait_for(timeout=15000)
                    except Exception:
                        p.screenshot(path=str(args.shots / f"failed-{who}.png"))
                        raise
                    check(True, f"{who}'s screen updated: the offer form is gone, and the log shows the alliance")
                alice.screenshot(path=str(args.shots / "2-alice-allied.png"))
                bob.screenshot(path=str(args.shots / "2-bob-allied.png"))

                # 4: the scene route on the server, after other clients.
                play_scene(browser, base, args.shots, "3-scene")
                browser.close()
        finally:
            server.terminate()
            server.wait(timeout=10)
    print(f"all checks passed; screenshots in {args.shots}")


if __name__ == "__main__":
    main()
