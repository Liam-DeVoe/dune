"""The toolbar above the player panel: players, a menu holding everything else, and leave."""

AREA = "menu"

# retreat.json has bots, so the host sees the bot speed control.
FIXTURE = "retreat"
MENU = '.toolbar button[title="Menu"]'


def column(g, name):
    # The menu sits at the top of the rightmost column; the menu drops over the player panel.
    box = g.page.locator(".col-lg-3").last.bounding_box()
    g.clip(name, box["x"] - 40, 0, box["width"] + 40, 760)


def run(r):
    with r.game(FIXTURE, "green") as g:
        g.shot("menu-1-closed")
        column(g, "menu-1-closed-column")

        if not r.after:
            # Today's help and volume panels, to pair with the expanded rows.
            g.page.click('a[title="Help"] >> nth=1')
            g.settle()
            column(g, "menu-3-rules")
            g.page.click('a[title="Adjust music and sound volume"]')
            g.settle()
            column(g, "menu-4-sound")
            return

        g.page.click(MENU)
        g.settle()
        column(g, "menu-2-open")

        g.page.click('.menu-overflow button:has-text("Rules")')
        g.settle()
        column(g, "menu-3-rules")

        g.page.click('.menu-overflow button:has-text("Rules")')
        g.page.click('.menu-overflow button:has-text("Sound")')
        g.settle()
        column(g, "menu-4-sound")

        g.page.keyboard.press("Escape")
        g.settle()
        r.note("menu open after escape", g.count(".menu-overflow.show"))

        g.page.click('.toolbar button[title="To main screen"]')
        g.settle()
        column(g, "menu-5-leave")
        g.page.click('.menu-leave button:has-text("Stay")')
        g.settle()

        g.page.click(MENU)
        g.page.click('.menu-overflow .bots button:has-text("Pause")')
        g.settle()
        g.page.keyboard.press("Escape")
        g.settle()
        column(g, "menu-6-bots-paused")
        g.page.click(".menu-bar > .badge")
        g.settle()
        r.note("badge after resume", g.count(".menu-bar > .badge"))
