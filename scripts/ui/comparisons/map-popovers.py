"""Map popovers whose text comes from the server's skin (cleanup B): the faction popover on a
seat with its rules sheet summary, the same with Shift held (the full sheet), and the turn
popover with the ruleset and rules described. Each is opened programmatically in both clients
and compared as the "popover" region."""

FACTION = "[data-bs-content*='pop-faction-head']"
TURN = "[data-bs-content*='Game version']"


def run(c):
    # Six factions, advanced rules (mc-*), and a basic game.
    for fixture, faction in (("mc-discard", "Atreides"), ("bid", "Fremen")):
        with c.scene(fixture, faction) as s:
            for i in range(6):
                s.show_popover(FACTION, i)
                s.shot(f"{fixture}-faction-{i}")
            s.show_popover(FACTION, 0, shift=True)
            s.shot(f"{fixture}-faction-full")
            s.show_popover(TURN)
            s.shot(f"{fixture}-turn")
