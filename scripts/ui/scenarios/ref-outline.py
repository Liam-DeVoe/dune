"""Chip hover on the map: lit sectors of one territory share a single outline instead of one per sector."""

AREA = "ref-outline"

# Each case lights a set of locations the way a chip naming them would. A
# stand-in chip is added at the page's corner so every case, including ones no
# log line in the fixture names, goes through the real mouseover handler.
# (name, locations, zoom centre, zoom size)
CASES = [
    # The reported case: a two-sector territory.
    ("hagga-basin", ["Hagga Basin (East Sector)", "Hagga Basin (West Sector)"],
     "Hagga Basin (East Sector)", (420, 320)),
    # Three sectors, all lit.
    ("cielago-north", ["Cielago North (West Sector)", "Cielago North (Center Sector)", "Cielago North (East Sector)"],
     "Cielago North (Center Sector)", (420, 320)),
    # Five thin sectors, all lit.
    ("false-wall-east", [f"False Wall East ({s} Sector)" for s in ("Far North", "North", "Middle", "South", "Far South")],
     "False Wall East (Middle Sector)", (420, 360)),
    # Sectors 17 and 0, across the wrap of the sector numbers.
    ("cielago-west-wrap", ["Cielago West (North Sector)", "Cielago West (South Sector)"],
     (203, 432), (420, 360)),
    # Two neighbouring sectors of four: they merge, the unlit ones stay out.
    ("pasty-mesa-partial", ["Pasty Mesa (North Sector)", "Pasty Mesa (South Sector)"],
     "Pasty Mesa (North Sector)", (420, 360)),
    # The two end sectors of three, not touching: each keeps its own outline.
    ("cielago-north-ends", ["Cielago North (West Sector)", "Cielago North (East Sector)"],
     "Cielago North (Center Sector)", (420, 320)),
    # Two whole territories side by side: each merges, the border between them stays.
    ("two-territories", ["Hagga Basin (East Sector)", "Hagga Basin (West Sector)",
                         "Tsimpo (East Sector)", "Tsimpo (Middle Sector)", "Tsimpo (West Sector)"],
     "Tsimpo (Middle Sector)", (460, 360)),
    # One sector of a territory: unchanged.
    ("one-sector", ["Tsimpo (Middle Sector)"], "Tsimpo (Middle Sector)", (420, 320)),
    # A place without sectors: unchanged.
    ("polar-sink", ["Polar Sink"], "Polar Sink", (420, 320)),
]

ADD_CHIP = """ids => {
    let chip = document.getElementById('ref-outline-chip');
    if (chip == null) {
        chip = document.createElement('span');
        chip.id = 'ref-outline-chip';
        chip.textContent = 'chip';
        chip.style.cssText = 'position:fixed;left:0;bottom:0;width:40px;height:20px;z-index:99999;opacity:0';
        document.body.appendChild(chip);
    }
    chip.dataset.refLocations = ids.join(',');
}"""


def run(r):
    # Turn 3 Ship & Move seen by the Bene Gesserit; the map itself is all that matters here.
    with r.game("bene_gesserit", "bene_gesserit") as g:
        for name, places, centre, size in CASES:
            g.park()
            g.eval(f"({ADD_CHIP})({[g.locations[p]['id'] for p in places]!r})")
            g.page.hover("#ref-outline-chip")
            g.settle()
            g.zoom(name, centre, size=size, dy=0)
        g.park()

        # A real faction chip in the report: every sector with Harkonnen forces, in their colour.
        g.page.locator('.chip.faction[data-ref-faction="Harkonnen"]').first.hover()
        g.settle()
        g.shot("faction-chip", park=False)

        # Hovering the map itself lights only the sector under the mouse, as before.
        g.hover("Hagga Basin (East Sector)")
        g.zoom("map-hover", "Hagga Basin (East Sector)", size=(420, 320), dy=0)
