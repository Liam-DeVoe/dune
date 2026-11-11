"""Every place the skin's worm image appears, for design/worm-icon.html.

Here before and after mean the old and new monster.svg: publish the client
with each Client/wwwroot/art/monster.svg under its own label. The crops are
named by position; worm-icon.html uses copies renamed by hand. The
worm-ride-ssw fixture is worm-ride with the Shield Wall worm rule (105)
added by hand so the shield wall marker renders.
"""

AREA = "worm-icon/sites"

WORM_IMAGES = """() => [...document.querySelectorAll("img[src*='monster.svg']")].map(i => {
    const r = i.getBoundingClientRect();
    return {x: r.left + r.width / 2, y: r.top + r.height / 2, w: r.width, map: !!i.closest('#planetmap')};
})"""


def run(r):
    with r.game("worm-ride-ssw", "fremen") as g:
        g.shot("game-full")
        seen = []
        for i in g.eval(WORM_IMAGES):
            if i["w"] == 0 or any(abs(i["x"] - x) < 60 and abs(i["y"] - y) < 30 for x, y in seen):
                continue  # hidden, or a duplicate right next to one already taken
            seen.append((i["x"], i["y"]))
            pad = 70 if i["map"] else 90
            where = "map" if i["map"] else "ui"
            g.clip(f"game-{where}-{int(i['x'])}x{int(i['y'])}", i["x"] - pad, i["y"] - 40, 2 * pad, 80)
        for name, location in (("hazard-rock", "Pasty Mesa (South Sector)"), ("hazard-sand", (200, 230))):
            g.hover(location)  # (200, 230) is plain sand in Hagga Basin (West)
            if g.count(".popover"):
                g.element_shot(name, ".popover", park=False)
            else:
                r.note(name, "no popover")
