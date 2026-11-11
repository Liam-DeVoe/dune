"""A leader lying face down in the tanks, seen by its owner and by another faction."""

AREA = "tanks"

# The tanks area in the map's top-left corner, in map coordinates.
TANKS = (40, 60)


def run(r):
    # Turn 9 revival. Atreides has a leader killed twice, so it lies face down.
    for faction in ("atreides", "harkonnen"):
        with r.game("tanks-face-down", faction) as g:
            g.zoom(f"tanks-{faction}", TANKS, size=(360, 420), dy=0)
