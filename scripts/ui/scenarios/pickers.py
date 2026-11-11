"""The sidebar number picker in three panels, cropped to the actions column.

Bidding shows the spice picker, the battle plan shows force and spice pickers
once a force is dialled, and revival shows force and special force pickers.
Also notes each picker's size and its children's widths and font sizes.
"""

AREA = "map-ux"

PANELS = [
    # fixture, faction, dial a force first?
    ("bid", "atreides", False),
    ("battle", "guild", True),
    ("revival", "emperor", False),
]

# Tall enough to include the panel under the phase card, without the unrelated panels below.
COLUMN_HEIGHT = 700

PICKER_SIZES = """() => [...document.querySelectorAll('.num')].map(n => {
    const r = n.getBoundingClientRect();
    const kids = [...n.children].map(k => [k.tagName, Math.round(k.getBoundingClientRect().width), getComputedStyle(k).fontSize]);
    return [Math.round(r.width), Math.round(r.height), kids];
})"""


def run(r):
    for fixture, faction, dial_force in PANELS:
        with r.game(fixture, faction) as g:
            if dial_force:
                # The spice picker only appears once at least one force is in the plan.
                g.page.locator(".num button[aria-label=Increase]").first.click()
            g.element_shot(f"picker-{fixture}", ".col-lg-3", height=COLUMN_HEIGHT)
            r.note(f"{fixture} pickers (w, h, children)", g.eval(PICKER_SIZES))
