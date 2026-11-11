"""The spice blow report under the advanced rules: a card-back heading for each deck, and the place of each blow as a chip."""

AREA = "spice-blow"

# The report card (the log), at the bottom of the right column.
REPORT = "[data-blow-report]"

MARK = """(() => {
    document.querySelector('.side-column > .card.report').setAttribute('data-blow-report', '');
})()"""


def run(r):
    # Turn 2 or later, Spice Blow just done, with both decks drawn.
    with r.game("blow-two-decks", "atreides") as g:
        g.eval(MARK)
        r.note("report", g.eval(f"document.querySelector('{REPORT}').innerText.replace(/\\s+/g, ' ')"))
        r.note("headings", g.count(f"{REPORT} .heading"))
        g.element_shot("headings-1-report", REPORT)
