"""The star badge on Fedaykin, Sardaukar and cyborg tokens: map stacks, intel popover, staged forces and the shipment counter."""

from scenarios.intel import intel_shot

AREA = "special-forces"
ZOOM = dict(size=(300, 220), dy=0)
EAST = "False Wall South (East Sector)"
MIDDLE = "False Wall West (Middle Sector)"


def run(r):
    # Emperor with 1 + 1 in Tuek's Sietch, Fremen with 6 + 1 and 2 + 1 on the False Wall.
    with r.game("battle", "red") as g:
        g.zoom("sf-map-tueks", "Tuek's Sietch", **ZOOM)
        g.zoom("sf-map-false-wall", (EAST, MIDDLE), size=(360, 260), dy=0)
        intel_shot(g, "sf-map-intel-tueks", "Tuek's Sietch")
        intel_shot(g, "sf-map-intel-east", EAST)

    # Fremen staging forces and a Fedaykin during setup.
    with r.game("setup", "yellow") as g:
        g.click(EAST)
        g.click(EAST, shift=True)
        g.zoom("sf-map-staged", EAST, **ZOOM)

    # The Emperor's shipment panel, with its Sardaukar counter.
    with r.game("ship", "red") as g:
        g.click("Carthag")
        g.park()
        g.settle()
        g.zoom("sf-map-stepper", "Carthag", size=(450, 350))
