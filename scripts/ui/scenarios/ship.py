"""Shipment: from reserves, Guild site to site and back, Fremen free placement."""

AREA = "map-ux"
ZOOM = (450, 350)


def run(r):
    # The Emperor shipping from reserves. Carthag is a valid, expensive destination.
    with r.game("ship", "red") as g:
        g.shot("ship-1-start")
        if r.after:
            g.click("Carthag")
            g.shot("ship-2-pinned", zoom="Carthag", size=ZOOM)

    # The Guild, holding Habbanya Sietch and Sietch Tabr: site to site, then back to reserves.
    with r.game("ship-guild", "orange") as g:
        g.shot("ship-guild-1-start")
        if r.after:
            g.click("Habbanya Sietch")
            g.shot("ship-guild-2-source")
            g.click("Sietch Tabr")
            g.shot("ship-guild-3-pinned", zoom="Sietch Tabr", size=ZOOM)
            g.click("Sietch Tabr")  # unpin: back to the source panel
            g.shot("ship-guild-3b-source-panel")
            g.button("Here from reserves")  # ship onto the source sector from reserves
            g.shot("ship-guild-3c-from-reserves", zoom="Habbanya Sietch", size=ZOOM)
            g.button("Cancel")
            g.click("Habbanya Sietch")
            g.button("To reserves")
            g.shot("ship-guild-4-reserves", zoom="Habbanya Sietch", size=ZOOM)

    # The Fremen bringing forces onto the planet for free.
    with r.game("ship-fremen", "yellow") as g:
        g.shot("ship-fremen-1-start")
        if r.after:
            g.click("The Great Flat")
            g.shot("ship-fremen-2-pinned", zoom="The Great Flat", size=ZOOM)
