"""Where the pinned panel sits: long and short arrows, sources in the way, the top of the map, no source."""

AREA = "map-ux"

FAR_NORTH = "Pasty Mesa (Far North Sector)"
NORTH = "Pasty Mesa (North Sector)"
SOUTH = "Pasty Mesa (South Sector)"


def measure(r, g, key):
    r.note(key, g.eval("(() => { const p = document.querySelector('.map-pin'); return p ? [p.offsetWidth, p.offsetHeight] : null })()"))


def run(r):
    # Blue with 1 force in Carthag, moving to False Wall West (North).
    # A long arrow whose source is far above the destination.
    with r.game("ally", "blue") as g:
        g.click("Carthag")
        g.click("False Wall West (North Sector)")
        g.shot("pin-anchor-1-long-arrow", zoom=("Carthag", "False Wall West (North Sector)"), size=(560, 480), dy=-20)
        measure(r, g, "panel size (map units)")

    # Fremen in Pasty Mesa (Far North) and (North). The sectors lie in a
    # column, so the source is straight above the destination.
    with r.game("move-within", "yellow") as g:
        g.click(FAR_NORTH)
        g.click(SOUTH)  # empty, 100 map units below the source
        g.shot("pin-anchor-2-source-above", zoom=(FAR_NORTH, SOUTH), size=(480, 420), dy=-60)

    with r.game("move-within", "yellow") as g:
        g.click(FAR_NORTH)
        g.click(NORTH)  # holds forces, 54 map units below the source
        g.button("Move here", within=".map-pin")
        g.shot("pin-anchor-3-source-close-above", zoom=(FAR_NORTH, NORTH), size=(480, 420), dy=-60)

    with r.game("move-within", "yellow") as g:
        g.click(FAR_NORTH)
        g.click(NORTH)
        g.button("Add as source", within=".map-pin")
        g.click(SOUTH)
        g.shot("pin-anchor-4-two-sources", zoom=(FAR_NORTH, SOUTH), size=(480, 420), dy=-60)

    # Fremen with 6 forces in False Wall South (East), moving next door to Tuek's Sietch.
    with r.game("move", "yellow") as g:
        g.click("False Wall South (East Sector)")
        g.click("Tuek's Sietch")
        g.shot("pin-anchor-5-short-sideways", zoom=("False Wall South (East Sector)", "Tuek's Sietch"), size=(480, 360), dy=-60)

    # Blue advisors in Arrakeen moving up to Old Gap (Middle), which is close
    # to the top edge of the map, so the panel has to sit below.
    with r.game("blue", "blue") as g:
        g.click("Arrakeen")
        g.click("Old Gap (Middle Sector)")
        g.shot("pin-anchor-6-near-top", zoom=("Arrakeen", "Old Gap (Middle Sector)"), size=(480, 420), dy=40)

    # The Emperor shipping from reserves into Carthag. No source on the map.
    with r.game("ship", "red") as g:
        g.click("Carthag")
        g.shot("pin-anchor-7-no-source", zoom="Carthag", size=(480, 360), dy=-60)
