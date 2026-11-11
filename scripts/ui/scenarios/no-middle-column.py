"""The game screen without the middle column: the board takes the width, one right column holds your faction, the action and the log. Also opens a playable card and the deals popover."""

import importlib.util
from pathlib import Path

AREA = "middle-column"

_spec = importlib.util.spec_from_file_location("mc", Path(__file__).with_name("middle-column.py"))
mc = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(mc)


def run(r):
    from harness import FIXTURES
    w, h = mc.SIZE
    for fixture, faction, what in mc.CASES:
        if not (FIXTURES / f"{fixture}.json").exists():
            continue
        faction = faction or mc.actor(fixture)
        name = f"{fixture}-{faction}"
        with r.game(fixture, faction) as g:
            g.page.set_viewport_size({"width": w, "height": h})
            g.page.evaluate("window.dispatchEvent(new Event('resize'))")
            g.settle()
            g.shot(name)
            r.note(name, g.page.evaluate("""() => {
                const doc = document.scrollingElement;
                const ring = document.getElementById('planetmap_inner').getBoundingClientRect();
                return {pageScrolls: doc.scrollHeight > doc.clientHeight, mapScale: +(ring.height / 626).toFixed(3),
                        playable: document.querySelectorAll('.pp-card.playable').length};
            }"""))
            if g.count(".pp-card.playable"):
                g.page.locator(".pp-card.playable").first.click()
                g.settle()
                g.shot(f"{name}-play", park=False)
                g.page.evaluate("document.querySelector('.menu-scrim').click()")
                g.settle()
            # The trade icons follow the separator in the menu bar; the first is Deals.
            deals = g.page.locator('.menu-bar .toolbar-sep + button')
            if name in ("ship-emperor", "ally-bene-gesserit", "bid-atreides") and deals.count():
                deals.click()
                g.settle()
                g.shot(f"{name}-deals", park=False)
                g.page.evaluate("document.querySelector('.menu-scrim').click()")
                g.settle()
            if name == "ally-bene-gesserit":
                g.page.locator('.menu-bar .toolbar-sep + button + button').click()
                g.settle()
                g.shot(f"{name}-alliance", park=False)
                g.page.evaluate("document.querySelector('.menu-scrim').click()")
                g.settle()
            if name == "bid-atreides" and g.count('.menu-bar .toolbar button:last-child'):
                g.page.locator('.menu-bar .toolbar button').last.click()
                g.settle()
                g.shot(f"{name}-cards", park=False)
