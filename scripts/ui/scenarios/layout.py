"""The in-game screen at several window sizes: on desktop it fits the window, with the menu bar under the map."""

AREA = "layout"

# retreat.json has bots, so the host sees the bot controls in the menu.
FIXTURE = "retreat"

SIZES = [
    (1000, 604),   # a small laptop window
    (1440, 820),   # MacBook Air, browser window
    (1600, 1000),  # the harness default
    (1920, 960),   # 1080p monitor, browser window
    (2560, 1290),  # 1440p monitor, browser window
    (3440, 1300),  # ultrawide
    (1280, 600),   # laptop with devtools open
]


def measure(g):
    """How the page and the columns fit the window."""
    return g.page.evaluate("""() => {
        const doc = document.scrollingElement;
        const map = document.getElementById('mapdiv').getBoundingClientRect();
        const bar = document.querySelector('#mapdiv > .menu-bar')?.getBoundingClientRect();
        const sides = [...document.querySelectorAll('.side-column')].map(c => Math.round(c.getBoundingClientRect().width));
        return {
            pageScrolls: doc.scrollHeight > doc.clientHeight,
            pageHeight: doc.scrollHeight,
            mapWidth: Math.round(map.width),
            barBottom: bar && Math.round(bar.bottom),
            window: innerHeight,
            sides,
        };
    }""")


def popover_box(g):
    return g.page.evaluate("""() => {
        const p = document.querySelector('.popover').getBoundingClientRect();
        return {top: Math.round(p.top), bottom: Math.round(p.bottom), window: innerHeight};
    }""")


def run(r):
    with r.game(FIXTURE, "green") as g:
        for w, h in SIZES:
            g.page.set_viewport_size({"width": w, "height": h})
            g.settle()
            g.page.evaluate("window.dispatchEvent(new Event('resize'))")
            g.settle()
            g.shot(f"layout-{w}x{h}")
            r.note(f"{w}x{h}", measure(g))

        # The discard piles sit at the bottom corners of the map, so their
        # popovers are the ones that can run off the bottom of the window.
        g.page.set_viewport_size({"width": 1000, "height": 604})
        g.page.evaluate("window.dispatchEvent(new Event('resize'))")
        g.settle()
        piles = g.page.locator('#planetmap [data-bs-content*="Discard pile"]')
        for i in range(piles.count()):
            piles.nth(i).hover()
            g.page.wait_for_selector(".popover", state="visible")
            g.settle()
            g.shot(f"layout-pile-{i + 1}-hover", park=False)
            r.note(f"pile {i + 1} popover", popover_box(g))
            g.page.mouse.move(500, 10)
            g.page.wait_for_selector(".popover", state="detached")

            # The first time a pile is hovered its card image isn't loaded yet,
            # so Popper places a short popover that grows when the image
            # arrives. Imitate that: hide the images when the popover is
            # inserted, just before Popper places it, then show them.
            g.page.evaluate("""() => document.addEventListener('inserted.bs.popover', e => {
                bootstrap.Popover.getInstance(e.target).tip.querySelectorAll('img').forEach(i => i.style.display = 'none');
            }, {once: true})""")
            piles.nth(i).hover()
            g.page.wait_for_selector(".popover", state="visible")
            g.settle()
            g.page.evaluate("document.querySelectorAll('.popover img').forEach(i => i.style.display = '')")
            g.settle()
            g.shot(f"layout-pile-{i + 1}-hover-late", park=False)
            r.note(f"pile {i + 1} popover, image loaded late", popover_box(g))
            g.page.mouse.move(500, 10)
            g.page.wait_for_selector(".popover", state="detached")

        if not r.after:
            return

        g.page.set_viewport_size({"width": 1600, "height": 1000})
        g.page.evaluate("window.dispatchEvent(new Event('resize'))")
        g.settle()

        g.page.click('.menu-bar button[title="Menu"]')
        g.settle()
        g.shot("layout-menu-open")
        g.page.keyboard.press("Escape")

        g.page.click('.menu-bar button[title="To main screen"]')
        g.settle()
        g.shot("layout-leave-open")
        g.page.click('.menu-leave button:has-text("Stay")')

        g.page.click('.menu-bar button[title="Players"]')
        g.settle()
        g.shot("layout-players-open")
        g.page.click('.menu-bar button[title="Players"]')

        g.page.click('.menu-bar button[title="Menu"]')
        g.page.click('.menu-overflow button:has-text("Undo")')
        g.settle()
        g.shot("layout-undo-open")
        r.note("undo", measure(g))
        r.note("undo timeline height", g.page.evaluate(
            "(() => { const t = document.querySelector('.undo-timeline'); const s = getComputedStyle(t);"
            " return t.offsetHeight + parseFloat(s.marginTop) + parseFloat(s.marginBottom); })()"))
