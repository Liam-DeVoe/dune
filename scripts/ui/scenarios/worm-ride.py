"""The Fremen worm ride, from the board and from reserves (Great Maker).

After: also drives pin, unpin, re-pin, Cancel and one confirmed ride, and
notes whether each took effect.
"""

AREA = "map-ux"

OPEN_SAND = (400, 330)  # between Polar Sink and Tuek's; a place to aim the cursor arrow at nothing


def run(r):
    # The worm appeared in Red Chasm, where the Fremen hold 4 forces and a Fedaykin.
    with r.game("worm-ride", "yellow") as g:
        g.shot("worm-ride-1-start")
        if r.after:
            g.click("Red Chasm")
            g.hover(OPEN_SAND)  # so the cursor arrow shows
            g.hide_popovers()   # the one for the sector under the cursor would cover the sidebar
            g.shot("worm-ride-2-source", park=False)
            g.click("Tuek's Sietch")
            g.shot("worm-ride-3-pinned", park=False, zoom=("Red Chasm", "Tuek's Sietch"), size=(440, 330), dy=-85)
            r.note("pinned", g.count(".map-pin") == 1)
            g.click("Tuek's Sietch")
            r.note("unpinned", g.count(".map-pin") == 0)
            g.click("Imperial Basin (East Sector)")
            r.note("repinned", g.count(".map-pin") == 1)
            g.button("Cancel", within=".map-pin")
            r.note("cancelled", g.count(".map-pin") == 0)
            g.click("Red Chasm")
            g.click("Tuek's Sietch")
            g.button("Ride!", within=".map-pin")
            r.note("ride accepted", g.wait_gone("Use the map to ride the worm."))
            g.shot("worm-ride-4-confirmed")

    # The Great Maker appeared; the Fremen have 9 forces and a Fedaykin in reserve.
    with r.game("worm-ride-reserves", "yellow") as g:
        g.shot("worm-ride-reserves-1-start")
        if r.after:
            g.click("Polar Sink")
            g.shot("worm-ride-reserves-2-pinned", zoom="Polar Sink")
            r.note("reserves pinned", g.count(".map-pin") == 1)
