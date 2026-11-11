"""Log papercuts: halves as decimals, collection column order, storm box hover, empty phase at its start."""

AREA = "log-papercuts"

REPORT = ".report"
PREV = f'{REPORT} button[aria-label="Previous phase"]'
PANEL = ".card:has(button:has-text('Battle!'))"


def back(g, times):
    for _ in range(times):
        g.page.locator(PREV).click()
        g.settle()


def run(r):
    # Turn 1 Battle, the Guild's plan. One force token clicked once goes to
    # half strength, so the total ends in a half.
    with r.game("battle", "guild") as g:
        g.page.locator(f"{PANEL} .bp-pip").nth(0).click()
        g.settle()
        g.element_shot("1-half-strength", PANEL)

    # Turn 8 just after the Spice Blow; three phases back is turn 7's Collection.
    with r.game("blow", "atreides") as g:
        back(g, 3)
        g.element_shot("2-collection", REPORT)

    # Turn 5 Storm: boxes 3 and 4 share a sector.
    # Hover the third box.
    with r.game(STORM_FIXTURE, "atreides") as g:
        back(g, STORM_BACK)
        g.page.locator(".storm-place").nth(STORM_BOX).hover()
        g.settle()
        g.element_shot("3-storm-hover", REPORT, park=False)
        r.note("wedge cursor", g.eval("getComputedStyle(document.querySelector('.storm-ring .w')).cursor"))

    # The Charity phase has just started and nothing has been logged in it yet.
    with r.game("mc-charity", "atreides") as g:
        g.element_shot("4-phase-start", REPORT)


STORM_FIXTURE, STORM_BACK, STORM_BOX = "tanks-face-down", 40, 2
