"""The sector hover highlight while browsing, with a source selected, and while shipping."""

AREA = "map-ux"

THIN = "False Wall West (Middle Sector)"


def run(r):
    with r.game("move", "fremen") as g:
        g.hover("Pasty Mesa (South Sector)")
        g.shot("hover-1-browse", park=False)
        if r.after:
            g.hover(THIN)
            g.hide_popovers()  # it would cover the thin sector in the close crop
            x, y = g.point(THIN)
            g.shot("hover-4-thin", park=False, zoom=(x + 10, y - 10), size=(320, 240), dy=0)
        g.click("False Wall South (East Sector)")
        g.hover("Pasty Mesa (South Sector)")
        g.shot("hover-2-source", park=False)

    if r.after:
        with r.game("ship", "emperor") as g:
            g.hover((200, 230))  # plain sand in Hagga Basin (West), so the hover wash reads clearly
            g.shot("hover-3-ship", park=False)
