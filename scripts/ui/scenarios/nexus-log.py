"""The Nexus in the spice blow report: alliance changes as two faction chips with a glyph between."""

AREA = "battle-log"

REPORT = "[data-nexus-report]"
PREV = f'{REPORT} button[aria-label="Previous phase"]'

MARK = """(() => {
    document.querySelectorAll('[data-nexus-report]').forEach(e => e.removeAttribute('data-nexus-report'));
    const col = [...document.querySelectorAll('.col-lg-3')].at(-1);
    [...col.children].filter(c => c.classList.contains('card')).at(-1).setAttribute('data-nexus-report', '');
})()"""


def run(r):
    # Mid-game with an alliance made in an earlier Nexus. Step back through the
    # report until a Nexus with an alliance change is showing.
    with r.game("ally", "green") as g:
        g.eval(MARK)
        for _ in range(60):
            if g.count(f"{REPORT} .blow-nexus .nexus-step"):
                break
            g.page.click(PREV)
            g.settle()
            g.eval(MARK)
        r.note("nexus", g.eval(f"[...document.querySelectorAll('{REPORT} .blow-nexus')].map(n => n.innerText.replace(/\\s+/g, ' ')).join(' | ')"))
        g.element_shot("nexus-1-report", REPORT)
