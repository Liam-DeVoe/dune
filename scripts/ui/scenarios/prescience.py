"""Prescience: the revealed element stays fixed, Atreides' Karma look stays private, Nexus prescience shows in the plan panel."""

AREA = "prescience"

PANEL = ".card:has(button:has-text('Battle!'))"


def dial_button(g, sign):
    return g.page.locator(
        f"{PANEL} .bp-ctl:has(.bp-what:text-is('Dial')) button:text-is('{sign}')"
    )


def bottom(g, name):
    """The strength row with the Battle! button, and the reason it is disabled when there is one."""
    total = g.page.locator(f"{PANEL} .bp-total")
    why = g.page.locator(f"{PANEL} .bp-why-not")
    (why if why.count() else total).scroll_into_view_if_needed()
    g.settle()
    box = total.bounding_box()
    end = box["y"] + box["height"]
    if why.count():
        w = why.bounding_box()
        end = max(end, w["y"] + w["height"])
    g.clip(name, box["x"] - 8, box["y"] - 8, box["width"] + 16, end - box["y"] + 16)


def run(r):
    # Atreides (defender) asked for the Guild's dial in Tuek's Sietch; the Guild has submitted.
    with r.game("prescience-seen", "atreides") as g:
        g.element_shot("prescience-1-atreides-sees-dial", PANEL)

    # The Guild withdrew that plan and is changing its dial before resubmitting.
    with r.game("prescience-revised", "guild") as g:
        g.element_shot("prescience-2-guild-revising", PANEL)
        sign = "−" if dial_button(g, "−").is_enabled() else "+"
        dial_button(g, sign).click()
        g.settle()
        r.note("dial step", sign)
        g.element_shot("prescience-3-guild-changes-dial", PANEL)
        bottom(g, "prescience-4-guild-confirm-row")

    # Atreides used Karma to see the Bene Gesserit's plan in Arrakeen. The Emperor is not in the battle.
    with r.game("karma-prescience", "emperor") as g:
        g.shot("karma-1-emperor-watching")

    # The Emperor played the Atreides Nexus card as a secret ally to see the Guild's dial; the Guild has submitted.
    with r.game("nexus-prescience", "emperor") as g:
        g.element_shot("nexus-1-emperor-plan", PANEL)
