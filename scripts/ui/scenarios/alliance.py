"""The alliance phase: host Continue before, every faction passes or allies after."""

AREA = "alliance"


def run(r):
    # Turn 6, first alliance phase; nobody is allied. Seated as the Emperor, who is also the host.
    # The before build plays version 187 (host ends the phase), so it gets the 187 copy of the game.
    with r.game("alliance" if r.after else "alliance-before", "emperor") as g:
        g.shot("alliance-1-start")
        if r.after:
            g.button("Pass")
            g.shot("alliance-2-passed")

    # Turn 9, first alliance phase; everyone is allied, the Emperor with the Bene Gesserit.
    with r.game("alliance-allied" if r.after else "alliance-allied-before", "emperor") as g:
        g.shot("alliance-allied-1-start")
        if r.after:
            g.button("Keep alliance")
            g.shot("alliance-allied-2-kept")
