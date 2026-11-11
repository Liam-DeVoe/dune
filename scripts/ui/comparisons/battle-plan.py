"""The battle plan's inputs: token clicks, the dial and spice steppers, a typed dial, picking a
leader, a weapon and a defense, and confirming the plan. Runs the web side on the Rust server
(scene_server.py), because a token click or a leader pick asks it for the draft's options."""


def each(s, act) -> None:
    """Runs act(page) in both clients, then waits for both to settle."""
    for page in s.pages.values():
        act(page)
    for page in s.pages.values():
        page.evaluate("q => window.treacheryScene.settled(q)", 150)


DIAL = ".act-stack .bp-ctl:has(.bp-what:text-is('Dial'))"
SPICE = ".act-stack .bp-ctl:has(.ic)"


def run(c):
    # The Emperor (aggressor, 10 spice) fights the Guild with 1 force and 3 Sardaukar, which
    # count at half strength when not paid for; defenses in hand, no weapon.
    with c.scene("battle-special", "Emperor", server=True) as s:
        # The first token of the top row, a Sardaukar: out, half, full.
        s.click_on(".act-stack .bp-pip >> nth=0")
        s.shot("token-half")
        s.click_on(".act-stack .bp-pip >> nth=0")
        s.shot("token-full")
        s.click_on(f"{DIAL} button >> nth=1")
        s.shot("dial-up")
        s.click_on(f"{SPICE} button >> nth=0")
        s.shot("spice-down")
        # A typed dial goes to the nearest one a mix allows.
        each(s, lambda p: (p.locator(f"{DIAL} input").fill("0"), p.locator(f"{DIAL} input").press("Enter")))
        s.shot("dial-typed")
        each(s, lambda p: p.locator(f"{DIAL} input").press("ArrowUp"))
        s.shot("dial-key-up")
        # The second leader, then the first defense.
        s.click_on(".act-stack .bp-section:has(.bp-label:text-is('Leader')) img >> nth=1")
        s.shot("leader")
        s.click_on(".act-stack .bp-cards > div:nth-child(2) img >> nth=0")
        s.shot("defense")
        s.button("Battle!")
        s.shot("confirmed")

    # The Guild (defender, 13 spice) has a weapon and five leaders; no leader means no weapon.
    with c.scene("battle", "Guild", server=True) as s:
        s.click_on(".act-stack .bp-cards img >> nth=0")
        s.shot("guild-weapon")
        s.click_on(".act-stack .bp-pip >> nth=2")
        s.shot("guild-token")
