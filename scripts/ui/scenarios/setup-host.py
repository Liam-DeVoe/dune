"""The host placing each faction's starting forces under the custom setup rule."""

AREA = "map-ux"


def run(r):
    # The host loaded the game and takes the seat of the faction being set up, the Emperor.
    with r.game("setup-host", "emperor") as g:
        g.shot("setup-host-1-start")
        if r.after:
            g.click("Tuek's Sietch")
            g.click("Tuek's Sietch")
            g.click("Tuek's Sietch", shift=True)
            g.click("Pasty Mesa (South Sector)")
            g.click("Sietch Tabr")
            g.click("Sietch Tabr", shift=True)
            g.shot("setup-host-2-staged")
            # Confirming moves the panel on to the next faction, whose staged
            # forces then sit next to real Emperor tokens.
            g.button("Confirm")
            g.click("Tuek's Sietch")
            g.click("Tuek's Sietch")
            g.shot("setup-host-3-next-faction", zoom="Tuek's Sietch", size=(320, 220), dy=0)
