"""The spice blow report under the advanced rules: a card-back heading for each deck, and the place of each blow as a chip."""

AREA = "spice-blow"

# The report card is the last card in the last lg-3 column.
REPORT = "[data-blow-report]"

MARK = """(() => {
    const col = [...document.querySelectorAll('.col-lg-3')].at(-1);
    [...col.children].filter(c => c.classList.contains('card')).at(-1).setAttribute('data-blow-report', '');
})()"""


def run(r):
    # Turn 2 or later, Spice Blow just done, with both decks drawn.
    with r.game("blow-two-decks", "green") as g:
        g.eval(MARK)
        r.note("report", g.eval(f"document.querySelector('{REPORT}').innerText.replace(/\\s+/g, ' ')"))
        r.note("headings", g.count(f"{REPORT} .heading"))
        g.element_shot("headings-1-report", REPORT)
