"""The menu bar under the map (package 5): the Deals popover, Atreides's card tracker, the
automation panel, and the menu with its rules and sound entries and the "List of Cards" modal."""


def run(c):
    # Bidding, seen by Atreides: bot deal offers are open, and Atreides has the card tracker.
    with c.scene("bid", "Atreides") as s:
        s.click_on(".toolbar button[title='Deals']")
        s.shot("deals")
        s.button("Offer a Deal", within=".trade-pop")
        s.shot("offer-a-deal")
        s.button("Give Spice", within=".trade-pop")
        s.shot("give-spice")
        s.click_on(".menu-scrim")
        s.click_on(".toolbar button[title='Card Tracker']")
        s.shot("tracker")
        s.click_on(".menu-scrim")
        s.click_on(".toolbar button[title='Automation']")
        s.shot("automation")

    with c.scene("bid", "Atreides") as s:
        s.click_on(".toolbar button[title='Menu']")
        s.button("Rules & help")
        s.button("Sound")
        s.shot("menu")
        s.button("List of Cards")
        s.shot("list-of-cards")
