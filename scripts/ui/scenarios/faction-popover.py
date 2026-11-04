"""The faction popover on the map, and the full rules sheet it shows while Shift is held."""

AREA = "popovers"


def run(r):
    with r.game("retreat", "green") as g:
        token = g.page.locator('#planetmap img[data-bs-content*="pop-faction"]:visible').first
        token.hover()
        g.page.wait_for_selector(".popover", state="visible")
        g.settle()
        g.shot("faction-hover", park=False)

        g.page.keyboard.down("Shift")
        g.page.wait_for_selector(".popover .pop-faction-full", state="visible")
        g.settle()
        g.shot("faction-rules", park=False)
        g.element_shot("faction-rules-popover", ".popover", park=False)

        g.page.keyboard.up("Shift")
        g.page.wait_for_selector(".popover .pop-faction-full", state="hidden")
        r.note("summary back after Shift released", g.count(".popover .pop-faction-brief:visible") == 1)
