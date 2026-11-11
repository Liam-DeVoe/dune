"""Undo: the host picks a moment on a timeline under the map, and the board, player panel and report show the game as it was then."""

import time

AREA = "undo"

# retreat.json is a long game (1133 events, turn 4 battle), so each preview
# replays a lot of history. That makes it the fixture to time previews on.
FIXTURE = "retreat"
OLD_UNDO_BUTTON = 'a[title="Undo"]'  # the icon row before the menu redesign


def open_undo(g):
    if g.count(OLD_UNDO_BUTTON):
        g.page.click(OLD_UNDO_BUTTON)
    else:
        # The tooltip script moves title into data-tip once the button is hovered.
        g.page.click('.toolbar button[title="Menu"], .toolbar button[data-tip="Menu"]')
        g.page.click('.menu-overflow button:has-text("Undo")')
    g.settle()

TRACK = ".undo-timeline .track"
TURNS = ".undo-timeline .turns .turn"


def run(r):
    with r.game(FIXTURE, "atreides") as g:
        g.shot("undo-1-closed")
        open_undo(g)
        g.shot("undo-2-open")
        if not r.after:
            return

        track = g.page.locator(TRACK).bounding_box()
        y = track["y"] + track["height"] / 2

        # Drag from now back to a third of the way into the current turn.
        start = time.perf_counter()
        g.page.mouse.move(track["x"] + track["width"] - 2, y)
        g.page.mouse.down()
        g.page.mouse.move(track["x"] + track["width"] * 0.33, y, steps=12)
        g.page.mouse.up()
        g.settle()
        r.note("drag 12 steps, ms", round((time.perf_counter() - start) * 1000))
        g.shot("undo-3-dragged", park=False)

        # One more gap back, timed on its own: a single preview replay.
        start = time.perf_counter()
        g.page.mouse.click(track["x"] + track["width"] * 0.25, y)
        g.settle()
        r.note("one preview, ms", round((time.perf_counter() - start) * 1000))

        # Hover a gap to see what it keeps and undoes.
        g.page.mouse.move(track["x"] + track["width"] * 0.6, y)
        g.settle()
        g.shot("undo-4-hover", park=False)

        # An earlier turn, zoomed from the top strip.
        g.page.locator(TURNS).nth(1).click()
        g.settle()
        track = g.page.locator(TRACK).bounding_box()
        g.page.mouse.click(track["x"] + track["width"] * 0.5, track["y"] + track["height"] / 2)
        g.settle()
        g.shot("undo-5-earlier-turn")

        # Escape puts everything back to now.
        g.page.keyboard.press("Escape")
        g.settle()
        r.note("timeline after escape", g.count(".undo-timeline"))
        g.shot("undo-6-escaped")

        # Undo for real, then the redo toast over the map.
        def confirm_undo():
            open_undo(g)
            track = g.page.locator(TRACK).bounding_box()
            g.page.mouse.click(track["x"] + track["width"] * 0.5, track["y"] + track["height"] / 2)
            g.settle()
            g.page.locator(".panel.undo .btn-primary").click()
            g.settle()

        confirm_undo()
        r.note("redo toasts after undo", g.count(".redo-toast"))
        g.shot("undo-7-confirmed")

        # Redo puts the events back and the toast goes.
        g.page.locator('.redo-toast button:has-text("Redo")').click()
        g.settle()
        r.note("redo toasts after redo", g.count(".redo-toast"))
        g.shot("undo-8-redone")

        # × keeps the undo and the toast goes.
        confirm_undo()
        g.page.locator(".redo-toast .x").click()
        g.settle()
        r.note("redo toasts after ×", g.count(".redo-toast"))
        g.shot("undo-9-kept")
