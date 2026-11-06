"""The report panel as a phase stepper: arrows and dots through every phase of the game, battles captioned."""

AREA = "turn-log"

# The report card is the last card in the last lg-3 column, under the player
# panel. Some fixtures put another element after that column, so
# ".col-lg-3:last-child" does not always match; mark() tags the card instead.
# Nothing here names a class the change adds, so both builds match.
REPORT = "[data-turnlog-report]"
COLUMN = "[data-turnlog-column]"
PREV = f'{REPORT} button[aria-label="Previous phase"]'
NEXT = f'{REPORT} button[aria-label="Next phase"]'
DOTS = f"{REPORT} .card-header span[title]"  # one per phase of the turn, Storm first, Mentat last

MARK = """(() => {
    document.querySelectorAll('[data-turnlog-report],[data-turnlog-column]').forEach(e => {
        e.removeAttribute('data-turnlog-report'); e.removeAttribute('data-turnlog-column'); });
    const col = [...document.querySelectorAll('.col-lg-3')].at(-1);
    col.setAttribute('data-turnlog-column', '');
    [...col.children].filter(c => c.classList.contains('card')).at(-1).setAttribute('data-turnlog-report', '');
})()"""

# The card's text as lines joined by " / ", with sigils and force icons written
# as [title]. A chip's sigil is skipped since its name follows. Header and body
# alike, so the phase title and turn label show.
REPORT_TEXT = f"""(() => {{
    const lines = [''];
    const BLOCK = /^(DIV|P|LI|UL|H[1-6]|HEADER|SECTION|TABLE|TR)$/;
    const walk = n => {{
        if (n.nodeType === 3) {{ lines[lines.length - 1] += n.textContent; return; }}
        if (n.nodeType !== 1) return;
        const cs = getComputedStyle(n);
        if (cs.display === 'none') return;
        if (n.tagName === 'IMG') {{
            if (!n.closest('.chip')) lines[lines.length - 1] += '[' + (n.title || n.alt || '') + ']';
            return;
        }}
        if (n.tagName.toLowerCase() === 'svg') {{ lines[lines.length - 1] += [...n.querySelectorAll('text')].map(t => t.textContent).join(''); return; }}
        const block = BLOCK.test(n.tagName) && !cs.display.startsWith('inline');
        if (block) lines.push('');
        n.childNodes.forEach(walk);
        if (block) lines.push('');
    }};
    walk(document.querySelector('{REPORT}'));
    return lines.map(l => l.replace(/\\s+/g, ' ').trim()).filter(Boolean).join(' / ');
}})()"""


def note_report(r, g, key):
    r.note(key, g.eval(REPORT_TEXT))
    r.note(f"{key} width", g.eval(f"Math.round(document.querySelector('{REPORT}').getBoundingClientRect().width)"))
    r.note(
        f"{key} overflow",
        g.eval(f"""[...document.querySelectorAll('{REPORT} *')]
            .filter(e => e.scrollWidth > e.clientWidth + 1 && getComputedStyle(e).overflowX !== 'visible').length"""),
    )


def report(r, g, name, key=None):
    g.eval(MARK)
    note_report(r, g, key or name)
    g.element_shot(name, REPORT)


def step(g, selector, times=1):
    for _ in range(times):
        g.eval(MARK)
        g.page.locator(selector).click()
        g.settle()


def run(r):
    # Turn 8, Battle phase, seen by the Harkonnen. Their battle in Habbanya Ridge
    # Flat against the Atreides is decided and the Harkonnen may retreat 1
    # force, so the current battle has a result and earlier battles this turn
    # (if any) are concluded.
    with r.game("retreat", "black") as g:
        g.shot("battle-0-page")
        report(r, g, "battle-1-default")
        if r.after:
            # Hover the left arrow, to see its hover state.
            g.eval(MARK)
            g.page.locator(PREV).hover()
            g.settle()
            g.element_shot("battle-2-arrow-hover", REPORT, park=False)
            # Three phases back: Battle -> Shipment & Move -> Bidding -> Charity (or whatever precedes).
            step(g, PREV, 3)
            report(r, g, "battle-3-back-3")
            # Back to the present with the right arrow.
            step(g, NEXT, 3)
            r.note("back at battle", g.eval(REPORT_TEXT)[:80])
            # Click the first dot, the Storm phase of this turn.
            g.eval(MARK)
            dots = g.eval(f"""[...document.querySelectorAll('{REPORT} [title]')]
                .map(e => e.tagName + ':' + e.getAttribute('title'))""")
            r.note("titled elements", dots)
            g.eval(MARK)
            g.page.locator(DOTS).first.click()
            g.settle()
            report(r, g, "battle-4-dot-storm")
            # And the dot after the current phase, which should do nothing.
            g.eval(MARK)
            g.page.locator(DOTS).last.click(force=True)
            g.settle()
            report(r, g, "battle-5-dot-future")

    # Turn 1, Battle phase, seen by the Guild: their battle in Habbanya Sietch
    # against the Atreides has started but no plans are in, so it is in progress.
    with r.game("battle", "orange") as g:
        report(r, g, "live-1-default")

    # Turn 8, Spice Blow just done, seen by the Atreides. The panel shows the
    # Spice Blow phase; left twice crosses into turn 7's Mentat phase, and on
    # back into turn 7's battles, which are all concluded.
    with r.game("blow", "green") as g:
        report(r, g, "turn-1-default")
        if r.after:
            step(g, PREV)
            report(r, g, "turn-2-storm")
            step(g, PREV)
            report(r, g, "turn-3-previous-mentat")
            step(g, PREV)
            report(r, g, "turn-4-previous-collection")
            step(g, PREV)
            report(r, g, "turn-5-previous-battle")
            g.element_shot("turn-5-previous-battle-column", COLUMN)
            # Right arrow back over the boundary: the turn label should go.
            step(g, NEXT, 4)
            report(r, g, "turn-6-forward-again")
            g.eval(MARK)
            r.note("next disabled at present", g.eval(f"document.querySelector('{NEXT}')?.disabled"))

    # Turn 1 Bidding: the left arrow walks back to turn 1's Storm, then the setup log, and stops.
    with r.game("bid", "green") as g:
        report(r, g, "start-1-default")
        if r.after:
            step(g, PREV, 3)
            report(r, g, "start-2-first-phase")
            # One more: the setup log, before turn 1's Storm.
            step(g, PREV)
            report(r, g, "start-3-setup")
            g.eval(MARK)
            r.note("prev disabled at start", g.eval(f"document.querySelector('{PREV}')?.disabled"))

    # Turn 3, the worm appeared in Red Chasm, where the Fremen hold 4 forces and
    # a Fedaykin. Ride to Tuek's Sietch so the report gains the ride line.
    with r.game("worm-ride", "yellow") as g:
        report(r, g, "ride-1-default")
        g.click("Red Chasm")
        g.click("Tuek's Sietch")
        g.button("Ride!", within=".map-pin")
        r.note("ride accepted", g.wait_gone("Use the map to ride the worm."))
        report(r, g, "ride-2-after-ride")
