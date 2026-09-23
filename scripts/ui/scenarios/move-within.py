"""Movement within one territory: a second sector with forces asks whether to add or move; an empty sector pins."""

AREA = "map-ux"

FAR_NORTH = "Pasty Mesa (Far North Sector)"
NORTH = "Pasty Mesa (North Sector)"
SOUTH = "Pasty Mesa (South Sector)"
ZOOM = dict(zoom=(FAR_NORTH, SOUTH), size=(560, 420), dy=-40)


def run(r):
    # Fremen with 4 forces and a Fedaykin in Pasty Mesa (Far North) and 3 forces
    # in Pasty Mesa (North). Pasty Mesa (South) is empty.
    with r.game("move-within", "yellow") as g:
        g.click(FAR_NORTH)
        g.shot("move-within-1-source", **ZOOM)
        # The second sector holds forces too, so the click could add a source or
        # set the destination. Before, it silently added a source.
        g.click(NORTH)
        g.shot("move-within-2-second-sector", **ZOOM)
        if r.after:
            g.button("Move here", within=".map-pin")
            g.shot("move-within-3-move-here", **ZOOM)
        r.note("pin buttons", g.count(".map-pin button"))

    with r.game("move-within", "yellow") as g:
        g.click(FAR_NORTH)
        # An empty sector of the source territory. Before, the click did nothing.
        g.click(SOUTH)
        g.shot("move-within-4-empty-sector", **ZOOM)
        r.note("pinned to empty sector", g.count(".map-pin"))
