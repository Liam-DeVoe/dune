"""The battle plan prompt: matchup, dial and spice, force tokens, leader, weapon, defense and total strength."""

AREA = "battle"

PANEL = ".card:has(button:has-text('Battle!'))"


def step(g, row, sign, times=1):
    for _ in range(times):
        g.page.locator(f"{PANEL} .bp-ctl:has(.bp-what:text-is('{row}')) button:text-is('{sign}')").click()
        g.settle()


def run(r):
    with r.game("battle", "orange") as g:
        g.element_shot("battle-1-plan", PANEL)
        step(g, "Dial", "+", 4)
        g.element_shot("battle-2-dial", PANEL)
        step(g, "Spice", "+")
        g.element_shot("battle-3-spice", PANEL)
        g.page.locator(f"{PANEL} .bp-pip").last.click()
        g.settle()
        g.element_shot("battle-4-token", PANEL)

    # Emperor with 1 force and 3 Sardaukar in Tuek's Sietch.
    with r.game("battle-special", "red") as g:
        g.element_shot("battle-5-special", PANEL)
        step(g, "Dial", "+", 3)
        g.element_shot("battle-6-special-dial", PANEL)
