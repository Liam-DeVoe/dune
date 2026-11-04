"""Chips on the parchment prompt, and the traitor and face dancer marks for players who hold several."""

AREA = "log-refs"

PANEL = ".side-column.cursor-hand > .card.mb-1"  # the player panel; the menu panels card above it is empty
REPORT = ".col-lg-3:last-child > .card:last-of-type"
PROMPT = ".prompt-act"

# Faction enum names, as data-faction-of carries them, to the skin's names, as the old sigils' titles carry them.
NAMES = {"Green": "Atreides", "Black": "Harkonnen", "Yellow": "Fremen", "Red": "Emperor", "Orange": "Guild", "Blue": "Bene Gesserit", "Purple": "Tleilaxu"}


def hover_faction_of_a_card(g, r, name):
    """Hover the report's chip (or sigil) for a faction that has a traitor or face dancer card in the panel."""
    if r.after:
        cards = g.eval(f"[...document.querySelectorAll('{PANEL} [data-faction-of]')].map(e => e.dataset.factionOf)")
        chips = g.eval(f"[...document.querySelectorAll('{REPORT} .chip.faction')].map(e => e.dataset.refFaction)")
        faction = next((f for f in cards if f in chips), None)
        r.note(f"{name} cards", cards)
        r.note(f"{name} hovered", faction)
        if faction is None:
            return False
        g.page.locator(f'{REPORT} .chip.faction[data-ref-faction="{faction}"]').first.hover()
    else:
        # The before build has no tags; hover the first sigil whose faction also has a card image in the panel.
        titles = g.eval(f"[...document.querySelectorAll('{REPORT} img[title]')].map(e => e.title)")
        r.note(f"{name} report sigils", sorted(set(titles)))
        target = next((t for t in titles if t in NAMES.values()), None)
        if target is None:
            return False
        g.page.locator(f'{REPORT} img[title="{target}"]').first.hover()
    g.settle()
    return True


def run(r):
    # The Atreides player waiting on the Fremen: the prompt names a faction on the parchment card.
    with r.game("move", "green") as g:
        r.note("prompt", g.eval(f"document.querySelector('{PROMPT}').innerText.replace(/\\\\s+/g, ' ')"))
        g.element_shot("prompt-1-waiting", PROMPT)

    # A battle prompt names the territory and both factions.
    with r.game("battle", "green") as g:
        r.note("prompt", g.eval(f"document.querySelector('{PROMPT}').innerText.replace(/\\\\s+/g, ' ')"))
        g.element_shot("prompt-2-battle", PROMPT)

    # The Harkonnen hold four traitors; hovering a faction in the report marks the traitor of theirs.
    with r.game("move", "black") as g:
        if hover_faction_of_a_card(g, r, "traitors"):
            g.element_shot("traitors-1-harkonnen", PANEL, park=False)

    # The Tleilaxu hold three face dancers; the same mark applies to them.
    with r.game("facedancer", "purple") as g:
        if hover_faction_of_a_card(g, r, "face dancers"):
            g.element_shot("facedancers-1-tleilaxu", PANEL, park=False)
