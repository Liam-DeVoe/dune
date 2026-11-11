"""The bidding panel's inputs: raising the bid relabels the confirm button, and turning auto-pass on
shows its threshold and the repeat option."""


def run(c):
    # Atreides is to bid on card 1 of 6 against Fremen's 1, and sees the card (prescience).
    with c.scene("bid", "Atreides") as s:
        s.click_on(".bidding-panel button[aria-label='Increase']")
        s.shot("raised")
        s.click_on(".bidding-panel .autopass input")
        s.shot("auto-pass")
