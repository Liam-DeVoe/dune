"""Pinned-panel variants: the Bene Gesserit fighters/advisors toggle and the ally warning.

Only the new client has pins, so there is no before; run it with the default mode.
"""

AREA = "map-ux"


def run(r):
    # Blue moving 4 advisors from Arrakeen to Old Gap, where they may arrive as fighters or advisors.
    with r.game("blue", "blue", steps=["click:Arrakeen", "click:Old Gap (Middle Sector)"]) as g:
        g.shot("pin-blue-toggle-full")
        g.zoom("pin-blue-toggle", ("Arrakeen", "Old Gap (Middle Sector)"))

    # Blue with an ally moving from Polar Sink into False Wall West, where the ally has forces.
    with r.game("ally", "blue", steps=["click:Polar Sink", "click:False Wall West (Middle Sector)"]) as g:
        g.shot("pin-ally-warning-full")
        g.zoom("pin-ally-warning", ("Polar Sink", "False Wall West (Middle Sector)"))
