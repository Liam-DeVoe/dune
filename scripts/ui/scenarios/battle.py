"""The battle plan prompt: forces, dial total, leader, weapon and defense."""

AREA = "battle"

PANEL = ".card:has(button:has-text('Battle!'))"
# The picker before the redesign had arrow icons instead of buttons.
OLD_INCREASE = ".number-picker-component span + svg"


def increase(g):
    new = g.page.get_by_role("button", name="Increase")
    (new if new.count() else g.page.locator(OLD_INCREASE)).first.click()
    g.settle()


def run(r):
    with r.game("battle", "orange") as g:
        g.element_shot("battle-1-plan", PANEL)
        increase(g)
        increase(g)
        g.element_shot("battle-2-forces", PANEL)
