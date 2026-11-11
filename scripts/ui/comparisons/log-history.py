"""The log's history: step back through every phase of a game with the report panel's
previous-phase arrow, comparing the log at each step. The games cover every phase log design:
a nine-turn game (tanks-face-down), two spice decks with worms (blow), CHOAM and Moritani
(mc-discard) and a Fremen worm ride from reserves (worm-ride-reserves)."""

GAMES = [
    ("tanks-face-down", "Atreides"),
    ("blow", "Fremen"),
    ("mc-discard", "Moritani"),
    ("worm-ride-reserves", "Fremen"),
]

PREVIOUS = ".side-column .report button[aria-label='Previous phase']"


def history(c, fixture, faction):
    with c.scene(fixture, faction) as s:
        step = 0
        # Stop when either client is at the first phase.
        while not any(page.locator(PREVIOUS).first.is_disabled() for page in s.pages.values()):
            s.click_on(PREVIOUS)
            step += 1
            s.shot(f"{fixture}-{faction}-{step:02}")


def run(c):
    for fixture, faction in GAMES:
        history(c, fixture, faction)
