"""The revival forms played from the hand (client-plan.md package 7d): the card opens its panel
beside the faction panel (CardPlayPopover) with Raise Dead or the Emperor's Karama revival.
Changing an amount or a leader asks the server for the draft's validation, and confirming
submits, so the web side runs on the Rust server (scene_server.py)."""

POP = ".card-play-pop"


def hide_flash(s) -> None:
    """Clicks away the flash of the fixture's last event (ImageFlashComponent), which hides by
    itself after a few seconds, a race for the shots that follow."""
    for page in s.pages.values():
        page.evaluate("() => document.querySelector(\"div[style*='fadeInFadeOut']\")?.click()")
    for page in s.pages.values():
        page.evaluate("q => window.treacheryScene.settled(q)", 150)


def run(c):
    # The Emperor holds Tleilaxu Ghola (Raise Dead) and may revive 3 forces or one of two leaders.
    with c.scene("face-danced", "Emperor", server=True) as s:
        hide_flash(s)
        s.click_on(".pp-card.playable")
        s.shot("ghola-open")
        s.click_on(f"{POP} .num >> nth=0 >> button[aria-label='Increase']")
        s.click_on(f"{POP} .num >> nth=0 >> button[aria-label='Increase']")
        s.shot("ghola-forces-2")
        s.click_on(f"{POP} .img-choices img >> nth=0")
        # Forces and a leader together are refused; back to the leader alone.
        s.shot("ghola-leader")
        s.click_on(f"{POP} .num >> nth=0 >> button[aria-label='Decrease']")
        s.click_on(f"{POP} .num >> nth=0 >> button[aria-label='Decrease']")
        s.shot("ghola-leader-only")
        s.button("Use Tleilaxu Ghola", within=POP)
        s.shot("ghola-confirmed")

    # Turn 5, start of the revival phase: the Emperor holds Karama (AdvancedKarama), whose panel
    # offers the advantage prevention and the free revival of 3 forces, 1 Sardaukar or a leader.
    with c.scene("karma-revival", "Emperor", server=True) as s:
        hide_flash(s)
        s.click_on(".pp-card.playable")
        s.shot("karama-open")
        revive = f"{POP} .card:has(.card-header:text('revive'))"
        s.click_on(f"{revive} .num >> nth=1 >> button[aria-label='Increase']")
        s.shot("karama-sardaukar")
        s.click_on(f"{revive} .img-choices img >> nth=1")
        # A leader and forces together are refused; back to the leader alone.
        s.shot("karama-leader")
        s.click_on(f"{revive} .num >> nth=1 >> button[aria-label='Decrease']")
        s.shot("karama-leader-only")
        s.click_on(f"{revive} button:text-is('Use Karama')")
        s.shot("karama-confirmed")
