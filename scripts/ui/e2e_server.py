"""End to end: the web client against the running Rust server, in a browser.

Starts rust/target/release/treachery-server on a fresh database with the web build
(web/dist) and dev scenes on, then drives Chromium through Playwright:

0. Before any other client connects, the scene route (/scene/alliance/Emperor) opens a game on
   the server through POST /dev/scene. The scene user must be seated as the Emperor (their
   player panel, named Emperor, and the alliance form), and passing must replace the
   view. The first client's serverInfo call on a fresh server restores the running games from the
   database, so this is the case where a seat that was never saved got lost.
1. Alice and Bob log in (accounts are created on first use) in two browser contexts.
2. Alice creates a game from the alliance fixture, takes the Emperor's seat and opens the
   Atreides seat; Bob joins it. The fixture's seed decides the seats, so they come from the
   view the server sends Alice.
3. Both open the game screen (/game/<gameId>), logged in with the session the setup kept in the
   tab's sessionStorage: no token in any URL. Alice pauses the bots from the menu, so that the
   only events are the test's own. She offers an alliance to the Atreides by clicking, Bob
   offers one back, and both screens show the alliance (the offer form leaves, and the toolbar
   has a button for the alliance with the other one's faction).
4. Join. Alice opens a third faction's seat from the players panel. Dave signs up through the
   login screen, and his lobby offers that seat and no other. He clicks it and plays that
   faction. Alice's and Bob's players panels show him in the seat, which is no longer open.
5. Leave. Dave clicks Leave game in his players panel and is back in the lobby. Alice and Bob
   see a bot in the seat again.
6. Kick. Alice reopens the seat and Dave joins again. Alice clicks Kick on his row: Dave's
   screen goes back to the lobby, and Alice and Bob see a bot in the seat. Alice opens the seat
   once more. Dave's click to join it is refused with "You were removed from this game", and
   so is his click to observe. Then Frank joins the seat and his WebSocket is cut; Alice kicks
   him meanwhile. When the connection comes back, the server refuses it and Frank's screen
   goes to the lobby.
7. Observer. Erin signs up and clicks Observe in the lobby. She sees the game screen, without a
   player panel or any action. Alice's players panel lists her as an observer.
8. Undo. Alice picks the moment before the last event (Bob's offer, which formed the alliance)
   on the undo timeline and confirms. Bob's offer form comes back, Alice's alliance button
   goes, and Erin's log shows the Emperor's offer instead of the alliance. Bob offers again,
   and all three see the alliance.
9. Reconnect. Bob's WebSocket is cut, and his screen says he was disconnected. While he's
   away, Alice undoes his offer again and opens a seat. The client reconnects on its own. Bob
   has the current view (his offer form is back) and the current participation (the open seat
   in his players panel), and when he offers again, Alice and Erin see the alliance.
10. Bob closes his tab, opens the site in a new one (whose sessionStorage is empty, so it shows
    the login form), logs in through the form and clicks Resume game in the lobby. He plays
    the Atreides, allied with the Emperor.
11. Carol logs in through the login form, asking to stay signed in. The session token is in
    localStorage and in no URL; a reload and a second tab are logged in with it. Logging out
    returns both tabs to the login form, and a reload stays logged out.
12. The scene route again, now on a server with other clients: passing in the alliance phase
    replaces the view.

Everything after step 2 is done by clicking, except where the client has no screen for it:
- Steps 1 and 2 log in, create, seat and join through the web client's connection code
  (window.treacheryClient). The lobby has screens for creating a game from a savegame and for
  seating, but the setup doesn't use them yet.
- Step 11 creates Carol's account through window.treacheryClient, and logs her in through it
  once, before logging in with the form.
- Steps 6 and 9 cut a connection by routing the page's WebSocket through Playwright
  (page.route_web_socket) and closing it. The client's own retry reconnects once the route
  lets it through again.

Build first: `cargo build --release -p treachery-server` (from rust/) and `npm run build`
(from web/, which also compiles the stylesheet into web/assets). Run: `python3 scripts/ui/e2e_server.py [--port 5077] [--shots /tmp/e2e]`.
Without the playwright package installed, `uv run --with playwright python3 ...` works, with a
playwright version whose Chromium is in ~/Library/Caches/ms-playwright.
Exits non-zero on the first failed check, after a screenshot of every open page.
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
# Every request that failed or got an error status, on any page: a missing image or stylesheet
# shows up here, not as a failed check.
FAILED_REQUESTS: list[str] = []
VIEWPORT = {"width": 1440, "height": 820}
# Faction names as the lobby and the skin show them, where they differ from the view's.
FACTION_LABELS = {"BeneGesserit": "Bene Gesserit", "Choam": "CHOAM"}
# Every page opened so far by name, for screenshots when a check fails.
PAGES: dict[str, Page] = {}


def watch(page: Page) -> Page:
    page.on("pageerror", lambda e: print(f"page error: {e}"))
    page.on("response", lambda r: r.status >= 400 and FAILED_REQUESTS.append(f"{r.status} {r.url}"))
    # Navigating away aborts requests still in flight, which is not an error.
    page.on("requestfailed", lambda r: "ERR_ABORTED" not in (r.failure or "") and FAILED_REQUESTS.append(f"{r.failure} {r.url}"))
    return page


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


# The fixture's seed decides which seat each faction gets, so the seats are read from the view
# the server sends the host after creating the game.
SETUP_HOST = """async ([savegame]) => {
  const { Connection, login, storeSession } = window.treacheryClient;
  const conn = new Connection();
  const me = await login(conn, "alice", "alice-password", { email: null, playerName: "Alice" });
  let view = null;
  conn.on("gameView", (v) => { view = v; });
  const info = await conn.invoke("createGame", { name: "E2E", passwordHash: null, savegame, skin: null });
  const seat = (faction) => view.players.find((p) => p.faction === faction).seat;
  const seats = { emperor: seat("Emperor"), atreides: seat("Atreides"), factions: view.players.map((p) => p.faction) };
  await conn.invoke("assignSeats", { game: info.gameId, seats: [[me.account.userId, seats.emperor]] });
  await conn.invoke("openOrCloseSeat", { game: info.gameId, seat: seats.atreides });
  conn.close();
  storeSession(me.token, false);
  return { gameId: info.gameId, ...seats };
}"""

SETUP_GUEST = """async ([gameId, seat]) => {
  const { Connection, login, storeSession } = window.treacheryClient;
  const conn = new Connection();
  const me = await login(conn, "bobby", "bob-password", { email: null, playerName: "Bobby" });
  await conn.invoke("joinGame", { game: gameId, passwordHash: null, seat });
  conn.close();
  storeSession(me.token, false);
  return { joined: true };
}"""


CREATE_ACCOUNT = """async ([name, password]) => {
  const { Connection, login } = window.treacheryClient;
  const conn = new Connection();
  await login(conn, name, password, { email: null, playerName: name });
  conn.close();
}"""

# Logs an existing account in through the same helper, which then sends `login` with the game
# version instead of creating the account.
LOG_IN = """async ([name, password]) => {
  const { Connection, login } = window.treacheryClient;
  const conn = new Connection();
  const me = await login(conn, name, password);
  conn.close();
  return me.account.userName;
}"""

SESSION_KEY = "treachery.online;session"


def log_in_and_out(browser, base: str, shots: Path) -> None:
    """Step 11: the login form, a kept session, and logging out."""
    context = browser.new_context(viewport=VIEWPORT)
    page = watch(context.new_page())
    page.goto(base + "/")
    page.evaluate(CREATE_ACCOUNT, ["carol", "carol-password"])
    check(page.evaluate(LOG_IN, ["carol", "carol-password"]) == "carol", "the client's login helper logs in an existing account")
    page.get_by_placeholder("User name").fill("carol")
    page.get_by_placeholder("Password").fill("carol-password")
    page.locator("#keepSignedIn").check()
    page.get_by_role("button", name="Log in").click()
    page.get_by_role("button", name="Log out").wait_for(timeout=15000)
    token = page.evaluate(f"localStorage.getItem('{SESSION_KEY}')") or ""
    check(len(token) == 64 and token not in page.url, "Carol logged in; the session token is kept in localStorage, not in the URL")
    page.reload()
    page.get_by_role("button", name="Log out").wait_for(timeout=15000)
    other = watch(context.new_page())
    other.goto(base + "/")
    other.get_by_role("button", name="Log out").wait_for(timeout=15000)
    check(True, "a reload and a second tab are logged in with the kept session")
    page.screenshot(path=str(shots / "4-logged-in.png"))
    page.get_by_role("button", name="Log out").click()
    for p, which in ((page, "this tab"), (other, "the other tab")):
        p.get_by_role("button", name="Log in").wait_for(timeout=15000)
        check(True, f"logging out shows the login form in {which}")
    check(page.evaluate(f"localStorage.getItem('{SESSION_KEY}')") is None, "the session token is gone from localStorage")
    page.reload()
    page.get_by_role("button", name="Log in").wait_for(timeout=15000)
    check(True, "a reload stays logged out")
    context.close()


def offer_alliance(page: Page, to: str) -> None:
    form = page.locator(".card", has_text="Offer a faction to form an Alliance")
    form.locator(f"img[alt='{to}']").click()
    button = form.get_by_role("button", name="Offer Alliance")
    button.wait_for()
    page.wait_for_function("() => [...document.querySelectorAll('button')].some(b => b.textContent.trim() === 'Offer Alliance' && !b.disabled)")
    button.click()


def play_scene(browser, base: str, shots: Path, name: str) -> None:
    """Opens /scene/alliance/Emperor, checks that it shows the Emperor's screen, and passes."""
    scene = watch(browser.new_context(viewport=VIEWPORT).new_page())
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


def titled(element: str, title: str, prefix: bool = False) -> str:
    """A selector for an element by its title. The page's tooltip script moves a hovered
    element's title to data-tip, so the selector looks for both."""
    op = "^=" if prefix else "="
    return ", ".join(f"{element}[{attribute}{op}'{title}']" for attribute in ("title", "data-tip"))


def new_page(browser, name: str) -> Page:
    page = watch(browser.new_context(viewport=VIEWPORT).new_page())
    PAGES[name] = page
    return page


def sign_up_with_form(page: Page, base: str, name: str) -> None:
    """Creates an account through the login screen's Create account tab, which logs the new
    user in, and waits for the lobby."""
    page.goto(base + "/")
    page.get_by_role("button", name="Create account").click()
    page.get_by_placeholder("User name (used to log in)").fill(name)
    page.get_by_placeholder("Password", exact=True).fill(f"{name}-password")
    page.get_by_placeholder("Repeat password").fill(f"{name}-password")
    page.get_by_placeholder("Player name (visible to other players)").fill(name)
    page.get_by_role("button", name="Create account").last.click()
    in_lobby(page)


def in_lobby(page: Page) -> None:
    page.get_by_text("Active games").wait_for(timeout=15000)


def lobby_row(page: Page):
    """The E2E game's row in the lobby's list of active games."""
    return page.locator("tr", has=page.get_by_text("E2E", exact=True))


def players_panel(page: Page):
    """The players panel of the game screen, opened from the toolbar if it isn't open."""
    panel = page.locator("div.mt-2.p-1.border")
    if not panel.is_visible():
        page.locator(titled(".toolbar button", "Players")).click()
    panel.wait_for(timeout=15000)
    return panel


def seat_row(page: Page, faction: str):
    """The row of one faction's seat in the players panel."""
    return players_panel(page).locator(":scope > div", has=page.locator(f"img[alt='{faction}']"))


def in_game_as(page: Page, faction: str) -> bool:
    bar = page.locator(".player-panel .pp-bar").first
    bar.wait_for(timeout=15000)
    return bar.inner_text().strip().startswith(faction)


def wait_gone(locator, timeout: int = 15000) -> None:
    locator.first.wait_for(state="detached", timeout=timeout)


def pause_bots(alice: Page) -> None:
    """The host pauses the bots from the menu, so that the only events are the test's own."""
    alice.locator(titled(".toolbar button", "Menu")).click()
    alice.locator(titled("button", "Bots are paused")).click()
    alice.keyboard.press("Escape")
    alice.locator(titled("button", "Bots are paused. Click to resume.")).wait_for(timeout=15000)
    check(True, "Alice paused the bots from the menu")


def open_or_close_seat(alice: Page, seat: str, open_it: bool) -> None:
    """Alice opens or closes a seat from her players panel."""
    row = seat_row(alice, seat)
    row.get_by_role("button", name="Open seat" if open_it else "Close seat").click()
    badge = row.locator(".badge", has_text="open")
    if open_it:
        badge.wait_for(timeout=15000)
    else:
        wait_gone(badge)


def join_from_lobby(page: Page, seat: str) -> None:
    join = lobby_row(page).locator(titled("button", f"Join game as {seat}"))
    # The lobby refreshes its game list with each heartbeat, every 6 seconds.
    join.wait_for(timeout=15000)
    join.click()


def join_leave_kick(browser, base: str, alice: Page, bob: Page, seat: str) -> None:
    """Steps 4 to 6: Dave joins an open seat from the lobby, leaves, joins again and is kicked."""
    open_or_close_seat(alice, seat, True)
    check(True, f"Alice opened the {seat} seat from her players panel")
    dave = new_page(browser, "dave")
    sign_up_with_form(dave, base, "dave")
    lobby_row(dave).locator(titled("button", f"Join game as {seat}")).wait_for(timeout=15000)
    check(lobby_row(dave).locator(titled("button", "Join game as", prefix=True)).count() == 1, f"Dave's lobby offers the {seat} seat and no other")
    join_from_lobby(dave, seat)
    check(in_game_as(dave, seat), f"Dave joined from the lobby and plays the {seat}")
    for p, who in ((alice, "Alice"), (bob, "Bob")):
        row = seat_row(p, seat)
        row.get_by_text("dave", exact=True).wait_for(timeout=15000)
        check(row.locator(titled("span", "Bot")).count() == 0 and row.locator(".badge", has_text="open").count() == 0,
              f"{who}'s players panel shows Dave in the {seat} seat instead of a bot, and the seat is no longer open")

    players_panel(dave).get_by_role("button", name="Leave game").click()
    in_lobby(dave)
    check(True, "Dave left from the players panel and is back in the lobby")
    for p, who in ((alice, "Alice"), (bob, "Bob")):
        wait_gone(seat_row(p, seat).get_by_text("dave", exact=True))
        check(seat_row(p, seat).locator(titled("span", "Bot")).count() == 1, f"{who}'s players panel shows a bot in the {seat} seat again")

    open_or_close_seat(alice, seat, True)
    join_from_lobby(dave, seat)
    check(in_game_as(dave, seat), f"Alice opened the {seat} seat again, and Dave joined it again")
    seat_row(alice, seat).get_by_text("dave", exact=True).wait_for(timeout=15000)
    seat_row(alice, seat).get_by_role("button", name="Kick").click()
    in_lobby(dave)
    check(True, "Alice kicked Dave from the players panel, and Dave's screen went back to the lobby")
    for p, who in ((alice, "Alice"), (bob, "Bob")):
        wait_gone(seat_row(p, seat).get_by_text("dave", exact=True))
        check(seat_row(p, seat).locator(titled("span", "Bot")).count() == 1, f"{who}'s players panel shows a bot in the {seat} seat again")

    open_or_close_seat(alice, seat, True)
    join_from_lobby(dave, seat)
    row = lobby_row(dave)
    row.get_by_text("You were removed from this game").wait_for(timeout=15000)
    check(True, "Alice opened the seat once more; Dave's click to join it is refused, and the lobby says he was removed")
    row.locator(titled("button", "Observe game")).click()
    dave.wait_for_timeout(1000)
    check(dave.locator(".toolbar").count() == 0 and row.get_by_text("You were removed from this game").count() == 1,
          "Dave can't observe the game either")
    dave.context.close()

    # Kicked while the connection is down: the server refuses the reconnection.
    frank = new_page(browser, "frank")
    plug = Plug(frank)
    sign_up_with_form(frank, base, "frank")
    join_from_lobby(frank, seat)
    check(in_game_as(frank, seat), f"Frank joined the {seat} seat")
    plug.cut()
    frank.get_by_text("You were disconnected from the game.").wait_for(timeout=15000)
    seat_row(alice, seat).get_by_text("frank", exact=True).wait_for(timeout=15000)
    seat_row(alice, seat).get_by_role("button", name="Kick").click()
    wait_gone(seat_row(alice, seat).get_by_text("frank", exact=True))
    plug.restore()
    in_lobby(frank)
    check(True, "Alice kicked Frank while his connection was down; when it came back, his screen went to the lobby")
    frank.context.close()


def observe(browser, base: str, alice: Page) -> Page:
    """Step 7: Erin observes the game from the lobby."""
    erin = new_page(browser, "erin")
    sign_up_with_form(erin, base, "erin")
    lobby_row(erin).locator(titled("button", "Observe game")).click()
    erin.locator(titled(".toolbar button", "Players")).wait_for(timeout=15000)
    check(True, "Erin clicked Observe in the lobby and sees the game screen")
    players_panel(alice).get_by_text("erin", exact=True).wait_for(timeout=15000)
    check(True, "Alice's players panel lists Erin among the observers")
    check(erin.locator(".player-panel").count() == 0 and erin.get_by_role("button", name="Pass").count() == 0
          and erin.get_by_text("Offer a faction to form an Alliance").count() == 0,
          "Erin has no player panel and no actions")
    return erin


class Plug:
    """Routes a page's WebSockets through Playwright, so the test can cut the connection and let
    the client's own reconnection back in."""

    def __init__(self, page: Page) -> None:
        self.down = False
        self.open: list = []
        page.route_web_socket(re.compile(r"/ws$"), self.handle)

    def handle(self, ws) -> None:
        if self.down:
            ws.close()
            return
        self.open.append((ws, ws.connect_to_server()))

    def cut(self) -> None:
        self.down = True
        for page_side, server_side in self.open:
            server_side.close()
            page_side.close()
        self.open.clear()

    def restore(self) -> None:
        self.down = False


def expect_alliance(alice: Page, bob: Page) -> None:
    for p, who, ally in ((alice, "Alice", "Atreides"), (bob, "Bob", "Emperor")):
        wait_gone(p.get_by_text("Offer a faction to form an Alliance"))
        p.locator(titled("button", f"Manage your Alliance with {ally}")).wait_for(timeout=15000)
        check(True, f"{who}'s screen shows the alliance: the offer form is gone, and the toolbar has the alliance with the {ally}")


def undo_last_event(alice: Page) -> None:
    """Alice picks the moment before the last event on the undo timeline and confirms."""
    alice.locator(titled(".toolbar button", "Menu")).click()
    alice.get_by_role("button", name="Undo…").click()
    zones = alice.locator(".undo-timeline .track .zone")
    zones.last.wait_for(timeout=15000)
    # The last zone is now; the one before it is the moment before the last event.
    zones.nth(zones.count() - 2).click()
    button = alice.get_by_role("button", name=re.compile(r"^Undo \d+ events?$"))
    check(button.inner_text().strip() == "Undo 1 event", "Alice picked the moment before the last event on the undo timeline")
    button.click()


def observer_sees(erin: Page, alliance: bool) -> None:
    """Waits until the observer's log shows the Emperor and the Atreides allied, or the
    Emperor's offer to the Atreides still open."""
    line = erin.locator(".nexus-step").filter(has_text="Emperor").filter(has_text="Atreides")
    glyph = "allied with" if alliance else "offer an alliance to"
    line.locator(f"svg[aria-label='{glyph}']").first.wait_for(timeout=15000)


def undo_alliance(alice: Page, bob: Page, erin: Page, shots: Path) -> None:
    """Step 8: Alice undoes Bob's offer, the event that formed the alliance, and Bob offers again."""
    observer_sees(erin, alliance=True)
    undo_last_event(alice)
    bob.get_by_text("Offer a faction to form an Alliance").wait_for(timeout=15000)
    check(True, "Bob's view rolled back: his offer form is back")
    wait_gone(alice.locator(titled("button", "Manage your Alliance with Atreides")))
    check(True, "Alice's view rolled back: the toolbar has no alliance")
    observer_sees(erin, alliance=False)
    check(True, "Erin's view rolled back: her log shows the Emperor's offer, not the alliance")
    erin.screenshot(path=str(shots / "8-erin-undone.png"))
    offer_alliance(bob, "Emperor")
    expect_alliance(alice, bob)
    observer_sees(erin, alliance=True)
    check(True, "Bob offered again, and Erin's log shows the alliance again")


def drop_connection(alice: Page, bob: Page, erin: Page, plug: Plug, seat: str) -> None:
    """Step 9: Bob's WebSocket closes. While it's down, Alice undoes his offer and opens a seat.
    When it's back, Bob has the current view and participation, and can offer again."""
    players_panel(bob)
    plug.cut()
    bob.get_by_text("You were disconnected from the game.").wait_for(timeout=15000)
    check(True, "Bob's connection dropped, and his screen says so")
    undo_last_event(alice)
    wait_gone(alice.locator(titled("button", "Manage your Alliance with Atreides")))
    open_or_close_seat(alice, seat, True)
    check(True, f"while Bob is away, Alice undid his offer and opened the {seat} seat")
    plug.restore()
    wait_gone(bob.get_by_text("You were disconnected from the game."))
    check(True, "Bob's client reconnected on its own")
    bob.get_by_text("Offer a faction to form an Alliance").wait_for(timeout=15000)
    check(True, "Bob has the current view: his offer form is back")
    seat_row(bob, seat).locator(".badge", has_text="open").wait_for(timeout=15000)
    check(True, f"Bob's players panel shows the {seat} seat Alice opened while he was away")
    offer_alliance(bob, "Emperor")
    expect_alliance(alice, bob)
    observer_sees(erin, alliance=True)
    check(True, "Bob's offer after reconnecting reached Alice and Erin")
    open_or_close_seat(alice, seat, False)


def reopen(browser, base: str, bob: Page) -> None:
    """Step 10: Bob closes his tab, opens the site in a new one, logs in and resumes the game."""
    context = bob.context
    bob.close()
    page = watch(context.new_page())
    PAGES["bob"] = page
    page.goto(base + "/")
    page.get_by_placeholder("User name").fill("bobby")
    page.get_by_placeholder("Password").fill("bob-password")
    page.get_by_role("button", name="Log in").click()
    in_lobby(page)
    lobby_row(page).locator(titled("button", "Resume game")).first.click()
    check(in_game_as(page, "Atreides"), "Bob logged in again in a new tab and resumed the game from the lobby")
    page.locator(titled("button", "Manage your Alliance with Emperor")).wait_for(timeout=15000)
    check(True, "Bob's new tab shows the alliance")
    context.close()


def run(browser, base: str, shots: Path) -> None:
    alice = new_page(browser, "alice")
    bob = new_page(browser, "bob")
    plug = Plug(bob)

    # 0: a scene first, on the fresh server.
    play_scene(browser, base, shots, "0-first-scene")

    # 1-2: log in, create, seat.
    alice.goto(base + "/")
    host = alice.evaluate(SETUP_HOST, [FIXTURE.read_text()])
    check(bool(host["gameId"]), "Alice logged in and created a game from the alliance fixture")
    bob.goto(base + "/")
    guest = bob.evaluate(SETUP_GUEST, [host["gameId"], host["atreides"]])
    check(bool(guest["joined"]), "Bob logged in and joined the Atreides seat")
    others = [f for f in host["factions"] if f not in ("Emperor", "Atreides")]
    check(bool(others), "the fixture has a third faction, for Dave to join")
    third = FACTION_LABELS.get(others[0], others[0])

    # 3: both screens, alliance events by clicking.
    alice.goto(f"{base}/game/{host['gameId']}")
    bob.goto(f"{base}/game/{host['gameId']}")
    for p, who, faction in ((alice, "Alice", "Emperor"), (bob, "Bob", "Atreides")):
        p.get_by_text("Offer a faction to form an Alliance").wait_for(timeout=15000)
        check(True, f"{who} sees the alliance form in a view from the server")
        check(in_game_as(p, faction), f"{who} plays the {faction}")
    alice.screenshot(path=str(shots / "1-alice-start.png"))
    pause_bots(alice)

    offer_alliance(alice, "Atreides")
    bob.get_by_text("Emperor").first.wait_for()
    offer_alliance(bob, "Emperor")
    expect_alliance(alice, bob)
    alice.screenshot(path=str(shots / "2-alice-allied.png"))
    bob.screenshot(path=str(shots / "2-bob-allied.png"))

    # 4-6: join, leave, kick.
    join_leave_kick(browser, base, alice, bob, third)

    # 7: an observer.
    erin = observe(browser, base, alice)

    # 8: undo.
    undo_alliance(alice, bob, erin, shots)

    # 9: Bob's connection drops and comes back.
    drop_connection(alice, bob, erin, plug, third)

    # 10: Bob closes the tab and opens the site again.
    reopen(browser, base, bob)

    # 11: logging in through the form, and out.
    log_in_and_out(browser, base, shots)

    # 12: the scene route on the server, after other clients.
    play_scene(browser, base, shots, "3-scene")


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
                try:
                    run(browser, base, args.shots)
                except Exception:
                    for name, page in PAGES.items():
                        if not page.is_closed():
                            page.screenshot(path=str(args.shots / f"failed-{name}.png"))
                    print(f"failed; screenshots of the open pages in {args.shots}")
                    raise
                browser.close()
                check(not FAILED_REQUESTS, "every request succeeded" + "".join(f"\n  {r}" for r in sorted(set(FAILED_REQUESTS))))
        finally:
            server.terminate()
            server.wait(timeout=10)
    print(f"all checks passed; screenshots in {args.shots}")


if __name__ == "__main__":
    main()
