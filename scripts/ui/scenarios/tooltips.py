"""The small tooltip that replaces the browser's own: on a map icon and on a menu button."""

AREA = "popovers"


def hover_shot(g, locator, name):
    locator.hover()
    g.page.wait_for_selector("#tip.on", state="visible", timeout=3000)
    g.settle()
    box = g.page.evaluate("""() => {
        const r = document.getElementById('tip').getBoundingClientRect();
        return {x: r.left, y: r.top, w: r.width, h: r.height};
    }""")
    g.shot(name, park=False)
    g.page.mouse.move(5, 5)
    g.page.wait_for_selector("#tip.on", state="detached", timeout=3000)
    return box


def run(r):
    with r.game("retreat", "atreides") as g:
        # The first titled map icon that the mouse can reach: some are covered
        # by other pieces' hover areas.
        g.page.evaluate("""() => {
            const hit = [...document.querySelectorAll('#planetmap img[title]')].find(i => {
                const r = i.getBoundingClientRect();
                return r.width > 0 && document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2) === i;
            });
            if (hit) hit.dataset.shot = 'tip';
        }""")
        mapicon = g.page.locator('#planetmap img[data-shot=tip]')
        if mapicon.count():
            r.note("map tooltip text", mapicon.get_attribute("title"))
            r.note("map tooltip", hover_shot(g, mapicon, "tip-map"))
        r.note("menu tooltip", hover_shot(g, g.page.locator('.menu-bar button[title="Menu"]'), "tip-menu"))
