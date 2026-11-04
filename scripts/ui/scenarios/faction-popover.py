"""The faction popover on the map, the rules panel its click opens, and the leader tokens with their strength badge."""

AREA = "popovers"


def run(r):
    with r.game("retreat", "green") as g:
        token = g.page.locator('#planetmap img[data-bs-content*="pop-faction"]:visible').first
        token.hover()
        g.page.wait_for_selector(".popover", state="visible")
        g.settle()
        g.shot("faction-hover", park=False)

        token.click()
        g.page.mouse.move(5, 5)
        g.page.wait_for_selector(".panel.faction-rules", state="visible")
        g.settle()
        g.shot("faction-rules", park=False)
        g.element_shot("faction-rules-panel", ".panel.faction-rules", park=False)
