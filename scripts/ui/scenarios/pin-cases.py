"""Pinned-panel variants: the Bene Gesserit fighters/advisors toggle and the ally warning.

Only the new client has pins, so there is no before; run it with the default mode.
"""

AREA = "map-ux"


def run(r):
    # BeneGesserit moving 4 advisors from Arrakeen to Old Gap, where they may arrive as fighters or advisors.
    with r.game("bene_gesserit", "bene_gesserit", steps=["click:Arrakeen", "click:Old Gap (Middle Sector)"]) as g:
        g.shot("pin-bene-gesserit-toggle-full")
        g.zoom("pin-bene-gesserit-toggle", ("Arrakeen", "Old Gap (Middle Sector)"))

    # BeneGesserit with an ally moving from Polar Sink into False Wall West, where the ally has forces.
    with r.game("ally", "bene_gesserit", steps=["click:Polar Sink", "click:False Wall West (Middle Sector)"]) as g:
        g.shot("pin-ally-warning-full")
        g.zoom("pin-ally-warning", ("Polar Sink", "False Wall West (Middle Sector)"))
