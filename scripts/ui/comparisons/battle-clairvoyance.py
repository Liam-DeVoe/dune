"""Clairvoyance from the hand: the card opens its panel beside the faction panel
(CardPlayPopover), and the question and parameter selects work by keyboard and by mouse as
SelectComponent and js/listbox.js do. Runs the web side on the Rust server (scene_server.py),
because changing the question asks it for the draft's options."""


def each(s, act) -> None:
    """Runs act(page) in both clients, then waits for both to settle."""
    for page in s.pages.values():
        act(page)
    for page in s.pages.values():
        page.evaluate("q => window.treacheryScene.settled(q)", 150)


QUESTION = ".card-play-pop .select-trigger >> nth=0"
PARAMETER = ".card-play-pop .select-trigger >> nth=1"


def run(c):
    # The Emperor holds Clairvoyance (playable) during a battle, so every question is offered,
    # the battle ones included.
    with c.scene("karma-prescience", "Emperor", server=True) as s:
        s.click_on(".pp-card.playable")
        s.shot("open")
        # Closed, ArrowDown opens the list on the chosen question.
        each(s, lambda p: p.locator(QUESTION).press("ArrowDown"))
        s.shot("question-list")
        # Down twice, then typing "i" jumps to the first label starting with it.
        each(s, lambda p: (p.locator(QUESTION).press("ArrowDown"), p.locator(QUESTION).press("ArrowDown")))
        s.shot("question-arrows")
        each(s, lambda p: p.locator(QUESTION).press("i"))
        s.shot("question-typed")
        each(s, lambda p: p.locator(QUESTION).press("Enter"))
        s.shot("question-chosen")
        # The parameter by mouse: open, then the third option.
        s.click_on(PARAMETER)
        s.shot("parameter-list")
        s.click_on(".card-play-pop .select-list:visible .select-option >> nth=2")
        s.shot("parameter-chosen")
        # Escape closes without choosing.
        each(s, lambda p: p.locator(PARAMETER).press("End"))
        each(s, lambda p: p.locator(PARAMETER).press("Escape"))
        s.shot("parameter-escaped")
