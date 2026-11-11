"""Toasts for deals offered to you: shown above the Deals icon, gone after five seconds or held by hovering, opening Deals with the offer marked, and saying when an offer is withdrawn."""

AREA = "deal-toast"


def offer(g, frm, price=7):
    g.page.evaluate(f"treacheryScene.offerDeal('{frm}', {price}, false)")
    g.settle()


def withdraw(g, frm):
    g.page.evaluate(f"treacheryScene.offerDeal('{frm}', 0, true)")
    g.settle()


def toasts(g):
    return g.count(".deal-toast")


def run(r):
    if not r.after:
        return
    with r.game("ship", "emperor") as g:
        g.page.set_viewport_size({"width": 1440, "height": 820})
        g.page.evaluate("window.dispatchEvent(new Event('resize'))")
        g.settle()
        r.note("toasts on load", toasts(g))

        offer(g, "atreides")
        g.shot("toast-1-one")
        r.note("after one offer", {"toasts": toasts(g), "count": g.eval("document.querySelector('.menu-bar .count')?.textContent")})

        offer(g, "bene_gesserit", 8)
        g.shot("toast-2-two")

        g.page.wait_for_timeout(5600)
        r.note("after 5.6s", toasts(g))

        # Held while hovered.
        offer(g, "atreides", 3)
        g.page.locator(".deal-toast").first.hover()
        g.page.wait_for_timeout(6000)
        r.note("hovered for 6s", toasts(g))
        g.page.locator(".deal-toast").first.click()
        g.settle()
        g.shot("toast-3-opened", park=False)
        r.note("marked rows", g.count(".deal-marked"))
        g.page.evaluate("document.querySelector('.menu-scrim').click()")
        g.settle()

        # × leaves the offer waiting.
        offer(g, "bene_gesserit", 2)
        g.page.locator(".deal-toast .x").first.click()
        g.settle()
        r.note("after ×", {"toasts": toasts(g), "count": g.eval("document.querySelector('.menu-bar .count')?.textContent")})

        # Withdrawn while the toast is up.
        offer(g, "atreides", 9)
        withdraw(g, "atreides")
        g.shot("toast-4-withdrawn")
        r.note("withdrawn", g.eval("document.querySelector('.deal-toast .withdrawn')?.textContent"))
        g.page.wait_for_timeout(2400)
        r.note("2.4s after withdrawal", toasts(g))
