"""The territory intel popover: feature rows, hazard chips, and the occupant strip."""

AREA = "intel"

# Bootstrap binds the hover popover to some territory shapes only after a
# first hover elsewhere, and to a few not at all under Playwright. So the
# shots read the popover's HTML straight off the shape and show it inside a
# copy of a popover frame that did open. Same markup, same CSS.
CLONE = """([x, y, html]) => {
    document.getElementById('intel-clone')?.remove();
    const pop = document.querySelector('.popover');
    if (!pop) return false;
    const k = pop.cloneNode(true);
    k.id = 'intel-clone';
    k.querySelector('.popover-body').innerHTML = html;
    Object.assign(k.style, {position: 'fixed', left: x + 'px', top: y + 'px', transform: 'none', zIndex: 99999});
    document.body.appendChild(k);
    return true;
}"""

WARMUP = ("Hagga Basin (West Sector)", "Tuek's Sietch")


def intel_shot(g, name, location):
    sx, sy = g.screen(location)
    html = g.page.evaluate(
        "([x, y]) => document.elementFromPoint(x, y).parentElement.getAttribute('data-bs-content')", [sx, sy])
    for warmup in WARMUP * 2:
        g.hover(warmup)
        if g.page.evaluate(CLONE, [700, 120, html]):
            break
    else:
        raise LookupError("no popover opened to clone")
    g.element_shot(name, "#intel-clone")
    g.eval("document.getElementById('intel-clone').remove()")


def run(r):
    # Atreides hold Arrakeen with 10 forces; the Fremen player is looking.
    with r.game("move", "yellow") as g:
        intel_shot(g, "intel-arrakeen", "Arrakeen")
        intel_shot(g, "intel-tueks", "Tuek's Sietch")
    # Spice just blew at Sihaya Ridge, so the blow row and the spice strip both show.
    with r.game("blow", "green") as g:
        intel_shot(g, "intel-sihaya", "Sihaya Ridge")
