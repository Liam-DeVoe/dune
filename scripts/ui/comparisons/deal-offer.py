"""DealOfferedForm in the Deals popover (client-plan.md package 7e): the targets, the payment,
the deal type and its end phase, the text, and offering the deal. Offering submits, so the web
side runs on the Rust server (scene_server.py)."""

POP = ".trade-pop"


def each(s, act) -> None:
    """Runs act(page) in both clients, then waits for both to settle."""
    for page in s.pages.values():
        act(page)
    for page in s.pages.values():
        page.evaluate("q => window.treacheryScene.settled(q)", 150)


def run(c):
    # The Emperor in the alliance phase of turn 6; nobody is allied, so all five are targets.
    with c.scene("alliance", "Emperor", server=True) as s:
        s.click_on(".toolbar button[title='Deals']")
        s.button("Offer a Deal", within=POP)
        s.shot("open")
        # Unpick Guild and Atreides: the offer goes to the other three.
        s.click_on(f"{POP} .img-choices img[alt='Guild']")
        s.click_on(f"{POP} .img-choices img[alt='Atreides']")
        s.shot("targets")
        s.click_on(f"{POP} button:text-is('You pay them')")
        s.click_on(f"{POP} .num button[aria-label='Increase']")
        s.click_on(f"{POP} .num button[aria-label='Increase']")
        s.shot("you-pay-2")
        s.click_on(f"{POP} button:text-is('They pay you')")
        s.shot("they-pay-2")
        s.click_on(f"{POP} .select-trigger")
        s.click_on(f"{POP} .select-option >> nth=2")
        s.shot("type-traitors")
        s.click_on(f"{POP} button:text-is('End of game')")
        each(s, lambda p: p.locator(f"{POP} textarea").fill("Tell me your traitors"))
        s.shot("filled")
        s.button("Offer Deal", within=POP)
        s.shot("offered")
