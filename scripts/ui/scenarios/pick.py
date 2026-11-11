"""Single-pick actions that moved from sidebar dropdowns onto the map: retreat, worm target, BeneGesserit accompanies."""

AREA = "map-ux"

CASES = [
    # name, fixture, faction, where to pin
    ("retreat", "retreat", "harkonnen", "Habbanya Erg (West Sector)"),  # clear of the combat wheels
    ("worm", "worm", "fremen", "Imperial Basin (Center Sector)"),
    ("bene_gesserit", "bene-gesserit-accompanies", "bene_gesserit", "Habbanya Sietch"),
]


def run(r):
    for name, fixture, faction, location in CASES:
        with r.game(fixture, faction) as g:
            g.shot(f"pick-{name}-1-start")
            if r.after:
                g.click(location)
                g.shot(f"pick-{name}-2-pinned", zoom=location)
