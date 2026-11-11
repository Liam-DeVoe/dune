"""Movement: pick a source, pin a destination, clear it, play Hajr, reinforce."""

AREA = "map-ux"

SOURCE = "False Wall South (East Sector)"


def run(r):
    # Fremen with 6 forces and a Fedaykin in False Wall South (East)
    # and 2 forces and a Fedaykin in False Wall West (Middle).
    with r.game("move", "fremen") as g:
        g.shot("move-1-start")
        if r.after:
            g.click(SOURCE)
            g.hover("Pasty Mesa (South Sector)")
            g.shot("move-2-source", park=False)
            g.click("Tuek's Sietch")
            g.shot("move-3-pinned", park=False, zoom=(SOURCE, "Tuek's Sietch"), size=(440, 300), dy=-50)
            g.click(SOURCE)  # clicking the source again clears the selection
            g.shot("move-4-cleared")
            # Hajr is played from the card in the faction card, which opens its panel.
            if g.count(".pp-card.playable"):
                g.page.locator(".pp-card.playable").first.click()
                g.settle()
            g.button("Play Hajr")
            g.click("False Wall West (Middle Sector)")
            g.click("Cielago West (North Sector)")
            g.shot("move-5-hajr", park=False)

    # Fremen with 4 forces and a Fedaykin in False Wall South (East) and 3 forces in Wind Pass (North).
    # With a source selected, a click on its own forces elsewhere pins the destination
    # instead of switching sources.
    if r.after:
        with r.game("move-reinforce", "fremen", steps=[f"click:{SOURCE}", "click:Wind Pass North (North Sector)"]) as g:
            g.shot("move-reinforce-pinned")
