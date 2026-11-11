"""The story of a finished game: the modal WinnerPanel's button opens, in full and with secrets hidden."""

STORY = "Read the story of this game..."


def run(c):
    # mc-ended: a two-turn game that Guild won. Nobody hid their secrets, so the whole story
    # is compared, the cards, traitors and spice of every faction included.
    with c.scene("mc-ended", "Atreides") as s:
        s.button(STORY)
        s.shot("story")

    # mc-ended-hidden: mc-ended plus HideSecrets by Guild and Bene Gesserit. The C# story still
    # shows their cards, traitors, spice and prediction; the view leaves them out, so the
    # story-hide-secrets mask covers the modal's body and only its frame is compared.
    # HideSecrets is applicable only up to game version 123, so the fixture was made by hand:
    # mc-ended.json with two HideSecrets events appended.
    with c.scene("mc-ended-hidden", "Observer") as s:
        s.button(STORY)
        s.shot("story-hidden")
