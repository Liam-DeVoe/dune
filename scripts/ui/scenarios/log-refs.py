"""References in the game log: place and faction chips with their hover highlights, and spice drawn as digits on the icon."""

from scenarios.intel import intel_shot

AREA = "log-refs"

# The right column holds the player panel and, under it, the report card.
# Neither selector names a class the change adds, so both builds match.
PANEL = ".side-column.cursor-hand > .card.mb-1"  # the player panel; the menu panels card above it is empty
REPORT = ".col-lg-3:last-child > .card:last-of-type"
PANEL_HEADER = f"{PANEL} .row"  # spice, forces and special forces in reserve
BID_PANEL = ".col-lg-3 > .card:has(.num)"  # the bidding card, found by its spice picker

# Report text as one string per line, with sigils and force icons written as [title].
REPORT_TEXT = f"""[...document.querySelector('{REPORT}').querySelectorAll('.card-body > div')]
    .map(d => [...d.querySelectorAll('*')].filter(e => !e.children.length)
        .map(e => e.tagName === 'IMG' ? '[' + (e.title || e.alt) + ']' : e.textContent).join('').replace(/\\s+/g, ' ').trim())
    .join(' / ')"""

PLACE = "Plastic Basin (North Sector)"


def hover_element(g, selector, has_text=None):
    """Move the real mouse onto the first element matching selector."""
    g.page.locator(selector, has_text=has_text).first.hover()
    g.settle()


def run(r):
    # Turn 3 Ship & Move report seen by the Bene Gesserit. It has moves into
    # Plastic Basin (North Sector) and Old Gap (Middle Sector), shipments paid in
    # spice, and the Harkonnen, whose leader is the Bene Gesserit player's traitor.
    with r.game("blue", "blue") as g:
        r.note("report", g.eval(REPORT_TEXT))
        g.element_shot("report-1-panel", REPORT)

        # Hover the move into Plastic Basin (North Sector). After the change the
        # chip lights up the one sector on the map; before, the bold text does nothing.
        if r.after:
            hover_element(g, f"{REPORT} .chip.place", has_text="Plastic Basin")
            r.note("place chips", g.count(f"{REPORT} .chip.place"))
        else:
            hover_element(g, f"{REPORT} .fw-bold", has_text=PLACE)
        g.shot("report-2-sector-hover", zoom=PLACE, dy=0, park=False)

        # Hover the first Harkonnen reference. After the change every sector with
        # Harkonnen forces tints and the traitor card in the panel is marked.
        if r.after:
            hover_element(g, f'{REPORT} .chip.faction[data-ref-faction="Black"]')
            r.note("faction chips", g.count(f"{REPORT} .chip.faction"))
            r.note("traitor marked", g.count(f"{PANEL} [data-faction-of=Black].ref-hi"))
        else:
            hover_element(g, f'{REPORT} img[title="Harkonnen"]')
        g.shot("report-3-faction-hover", park=False)
        g.element_shot("report-3-faction-hover-panel", PANEL, park=False)

        g.element_shot("panel-1-spice", PANEL_HEADER)

        # Hover the text of a line that tells of a journey. After the change the
        # line draws an arrow on the map from where the forces were to where
        # they went, and lights both ends. The mouse goes on the word "move" or
        # "ship" so it is the line, not a chip, that is hovered.
        move_line = f"{REPORT} .card-body > div"
        hover_element(g, f"{move_line} >> text=move", has_text=None)
        if r.after:
            r.note("arrow lines", g.eval("document.querySelectorAll('.maprefs line').length"))
        g.shot("report-4-move-hover", zoom=("Funeral Plain", PLACE), dy=0, park=False)

        hover_element(g, f"{move_line}:has-text(\"Tuek's Sietch\") >> text=ship", has_text=None)
        if r.after:
            r.note("arrow lines", g.eval("document.querySelectorAll('.maprefs line').length"))
        g.shot("report-5-ship-hover", park=False)

    # Spice just blew at Sihaya Ridge, so the popover has a spice amount to draw.
    with r.game("blow", "green") as g:
        intel_shot(g, "intel-1-spice", "Sihaya Ridge")

    # A control: the bidding card's spice picker should not change.
    with r.game("bid", "green") as g:
        g.element_shot("bid-1-panel", BID_PANEL)
