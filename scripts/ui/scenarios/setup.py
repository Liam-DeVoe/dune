"""Fremen initial deployment: click to stage a force, Shift-click for a Fedaykin, right-click to remove."""

AREA = "map-ux"
ZOOM = dict(size=(440, 300), dy=0)
EAST = "False Wall South (East Sector)"


def run(r):
    with r.game("setup", "yellow") as g:
        g.shot("setup-1-start")
        if r.after:
            for _ in range(3):
                g.click("Sietch Tabr")
            g.click(EAST)
            g.click(EAST)
            g.click(EAST, shift=True)
            g.click("False Wall West (Middle Sector)")
            g.click("False Wall West (Middle Sector)", shift=True)
            g.shot("setup-2-staged", zoom=EAST, **ZOOM)
            g.click(EAST, right=True)
            g.shot("setup-3-removed", zoom=EAST, **ZOOM)
