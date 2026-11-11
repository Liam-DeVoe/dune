"""The battle plan prompt: matchup, dial and spice, force tokens, leader, weapon, defense and total strength."""

AREA = "battle"

PANEL = ".card:has(button:has-text('Battle!'))"


def step(g, row, sign, times=1):
    for _ in range(times):
        g.page.locator(f"{PANEL} .bp-ctl:has(.bp-what:text-is('{row}')) button:text-is('{sign}')").click()
        g.settle()


def states(g):
    return g.page.eval_on_selector_all(f"{PANEL} .bp-pip", "els => els.map(e => [...e.classList].find(c => ['full', 'half', 'out'].includes(c)))")


def click_token(g, i):
    before = states(g)
    g.page.locator(f"{PANEL} .bp-pip").nth(i).click()
    g.settle()
    after = states(g)
    changed = [j for j, (a, b) in enumerate(zip(before, after)) if a != b]
    assert changed == [i], f"clicking token {i} changed {changed}: {before} -> {after}"


def run(r):
    with r.game("battle", "guild") as g:
        g.element_shot("battle-1-plan", PANEL)
        click_token(g, 2)
        click_token(g, 2)
        click_token(g, 4)
        g.element_shot("battle-1b-clicks", PANEL)
        click_token(g, 2)
        click_token(g, 4)
        step(g, "Dial", "+", 4)
        g.element_shot("battle-2-dial", PANEL)
        step(g, "Spice", "+")
        g.element_shot("battle-3-spice", PANEL)
        click_token(g, 5)
        g.element_shot("battle-4-token", PANEL)

    # Emperor with 1 force and 3 Sardaukar in Tuek's Sietch.
    with r.game("battle-special", "emperor") as g:
        g.element_shot("battle-5-special", PANEL)
        step(g, "Dial", "+", 3)
        g.element_shot("battle-6-special-dial", PANEL)
