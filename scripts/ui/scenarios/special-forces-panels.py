"""Starred special forces off the map: the player panel, the number input, the setup hints and the report."""

AREA = "special-forces"

STARRED = ".force-token.starred"
PANEL_HEADER = ".col-lg-3:last-child > .card .row"  # spice, forces and special forces in reserve
SETUP_HINTS = f".small.text-2:has({STARRED})"
REPORT_LINE = f".card.report .card-body > div:has({STARRED})"
PICKER = f".num:has({STARRED})"

# Each token's size and badge size, and in the number input the room above and
# right of the token inside the icon cell, which the badge needs so it isn't clipped.
BADGES = """sel => [...document.querySelectorAll(sel)].map(t => {
    const img = t.querySelector('img').getBoundingClientRect();
    const badge = getComputedStyle(t, '::after');
    const clip = t.closest('.ic');
    const c = clip && clip.getBoundingClientRect();
    return {
        size: [Math.round(img.width), Math.round(img.height)],
        badge: badge.width,
        inside: c ? [Math.round(img.top - c.top), Math.round(c.right - img.right)] : null,
    };
})"""


def run(r):
    with r.game("battle", "red") as g:
        g.element_shot("panel-red", PANEL_HEADER)

    with r.game("battle", "yellow") as g:
        g.element_shot("panel-yellow", PANEL_HEADER)

    # Emperor revival: the special force picker, alone and in its column.
    with r.game("revival", "red") as g:
        r.note("picker badges", g.eval(f"({BADGES})('{PICKER} {STARRED}')"))
        g.element_shot("picker", PICKER)
        g.element_shot("picker-column", ".col-lg-3", height=500)

    with r.game("setup", "yellow") as g:
        g.element_shot("setup-hints", SETUP_HINTS)

    # The Ship & Move report names Fedaykin that shipped.
    with r.game("ship-fremen", "yellow") as g:
        r.note("report badges", g.eval(f"({BADGES})('{REPORT_LINE} {STARRED}')"))
        g.element_shot("report-line", REPORT_LINE)
        g.element_shot("report", ".card.report", height=400)
