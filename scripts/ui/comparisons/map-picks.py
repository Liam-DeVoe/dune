"""The "other events" forms that pick on the map (client-plan.md package 7e): the territory lights
up and the pinned panel holds the confirm button. The pick is the client's own state, so saved
views are enough; confirming would need the server."""


def run(c):
    # Fremen holds a Karama card in the Thumper phase of turn 4 (AdvancedKarama). The card's
    # panel offers KarmaMonster beside Karma; Funeral Plain is a valid territory.
    with c.scene("karma-monster", "Fremen") as s:
        s.click_on(".pp-card.playable")
        s.shot("karma-open")
        s.click("Funeral Plain")
        s.shot("karma-picked")
    # Harkonnen holds the Moritani Nexus card as betrayal; Carthag and Sietch Tabr hold terror.
    with c.scene("nexus-moritani", "Harkonnen") as s:
        s.click("Carthag")
        s.shot("moritani-picked")
