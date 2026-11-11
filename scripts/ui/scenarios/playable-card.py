"""Options for marking a treachery card you can play now in the player panel, each injected as CSS over the real client."""

AREA = "playable-card"

BASE = ".player-panel .pp-card.playable, .player-panel .pp-card.playable:hover, .player-panel .pp-card.playable.open { box-shadow: none; }\n"

VARIANTS = {
    "a-today": None,
    "c-gap-ring": BASE + """
.player-panel .pp-card.playable { box-shadow: 0 0 0 3px var(--ink-2), 0 0 0 5px var(--gold); }
.player-panel .pp-card.playable.open { box-shadow: 0 0 0 3px var(--ink-2), 0 0 0 5px var(--parchment); }
""",
    "c1-gap1-ring2": BASE + """
.player-panel .pp-card.playable { box-shadow: 0 0 0 1px var(--ink-2), 0 0 0 3px var(--gold); }
.player-panel .pp-card.playable.open { box-shadow: 0 0 0 1px var(--ink-2), 0 0 0 3px var(--parchment); }
""",
    "c2-gap2-ring2": BASE + """
.player-panel .pp-card.playable { box-shadow: 0 0 0 2px var(--ink-2), 0 0 0 4px var(--gold); }
.player-panel .pp-card.playable.open { box-shadow: 0 0 0 2px var(--ink-2), 0 0 0 4px var(--parchment); }
""",
    "c3-gap1-ring3": BASE + """
.player-panel .pp-card.playable { box-shadow: 0 0 0 1px var(--ink-2), 0 0 0 4px var(--gold); }
.player-panel .pp-card.playable.open { box-shadow: 0 0 0 1px var(--ink-2), 0 0 0 4px var(--parchment); }
""",
    "c4-gap2-ring3": BASE + """
.player-panel .pp-card.playable { box-shadow: 0 0 0 2px var(--ink-2), 0 0 0 5px var(--gold); }
.player-panel .pp-card.playable.open { box-shadow: 0 0 0 2px var(--ink-2), 0 0 0 5px var(--parchment); }
""",
    "g-play-tag": """
.player-panel .pp-card.playable { position: relative; }
.player-panel .pp-card.playable::after { content: "Play"; position: absolute; left: 0; right: 0; bottom: 0; line-height: 1; padding: 3px 0 2px;
  background: var(--gold); color: var(--ink-0); font-family: var(--font-label); font-size: 10px; letter-spacing: .12em; text-transform: uppercase; text-align: center;
  border-radius: 0 0 var(--r-1) var(--r-1); }
.player-panel .pp-card.playable.open::after { background: var(--parchment); }
""",
    "g2-tag-gap-ring": BASE + """
.player-panel .pp-card.playable { position: relative; box-shadow: 0 0 0 2px var(--ink-2), 0 0 0 4px var(--gold); }
.player-panel .pp-card.playable.open { box-shadow: 0 0 0 2px var(--ink-2), 0 0 0 4px var(--parchment); }
.player-panel .pp-card.playable::after { content: "Play"; position: absolute; left: 0; right: 0; bottom: 0; line-height: 1; padding: 3px 0 2px;
  background: var(--gold); color: var(--ink-0); font-family: var(--font-label); font-size: 10px; letter-spacing: .12em; text-transform: uppercase; text-align: center;
  border-radius: 0 0 var(--r-1) var(--r-1); }
.player-panel .pp-card.playable.open::after { background: var(--parchment); }
""",
}

CASES = [("mc-voice", "emperor"), ("revival", "emperor"), ("move", "fremen")]


def run(r):
    for fixture, faction in CASES:
        with r.game(fixture, faction) as g:
            g.settle()
            for key, css in VARIANTS.items():
                g.page.evaluate("""css => {
                    let s = document.getElementById('variant-css');
                    if (!s) { s = document.createElement('style'); s.id = 'variant-css'; document.head.appendChild(s); }
                    s.textContent = css || '';
                }""", css)
                g.settle()
                name = f"{key}-{fixture}"
                box = g.page.evaluate("""() => {
                    const r = document.querySelector('.player-panel').closest('.card').getBoundingClientRect();
                    return [r.x, r.y, r.width, r.height]; }""")
                x, y, w, h = box
                g.park()
                g.settle()
                g.clip(name, x - 8, y - 8, w + 16, h + 16)
                if fixture == "mc-voice":
                    g.shot(f"{name}-full")
                    g.page.locator(".pp-card.playable").first.click()
                    g.settle()
                    g.page.mouse.move(5, 995)
                    g.settle()
                    g.clip(f"{name}-open", x - 8, y - 8, w + 16, h + 16)
                    g.page.evaluate("document.querySelector('.menu-scrim').click()")
                    g.settle()
