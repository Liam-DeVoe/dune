"""Choosing whom to offer an alliance: the chosen faction lights up and Offer Alliance enables."""


def run(c):
    # The Emperor (Emperor) in the alliance phase; the other five factions are valid targets.
    with c.scene("alliance", "Emperor") as s:
        # Both clients name the choice images after the faction. The prompt above the form
        # has faction images too, so take the last match, which is in the form.
        s.click_on(".act-stack img[alt='Guild'] >> nth=-1")
        s.shot("guild-chosen")
