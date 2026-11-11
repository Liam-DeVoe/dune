"""Card popovers: a leader and a treachery card in the player panel, the treachery and spice discard piles, and spice cards with their place chip."""

AREA = "popovers"


def hover_shot(g, locator, name):
    locator.hover()
    g.page.wait_for_selector(".popover", state="visible")
    g.settle()
    g.shot(name, park=False)
    g.page.mouse.move(5, 5)
    g.page.wait_for_selector(".popover", state="detached")


def run(r):
    # Increased spice flow, so there are two spice discard piles.
    with r.game("blow-two-decks", "atreides") as g:
        leader = g.page.locator('[data-bs-content*="pop-leader"]:not(#planetmap *):visible')
        if leader.count():
            hover_shot(g, leader.first, "card-leader")

        hand = g.page.locator('[data-bs-content*="pop-card"]:not(#planetmap *):visible')
        if hand.count():
            hover_shot(g, hand.first, "card-hand")

        piles = g.page.locator('#planetmap :is([data-bs-content*="pop-card"], [data-bs-content*="pop-spice"], [data-bs-content*="pop-pile"]):visible')
        for i in range(piles.count()):
            hover_shot(g, piles.nth(i), f"card-pile-{i + 1}")
