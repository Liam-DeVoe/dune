"""The battle forms that pick on the map (client-plan.md package 7b): Retreat and the Bene
Gesserit battle announcement. A pick lights the territory and pins a panel with the confirm
button; confirming submits, so the web side runs on the Rust server (scene_server.py)."""


def run(c):
    # Harkonnen lost the battle in Habbanya Ridge Flat (LeaderSkills) and may retreat one force to an
    # adjacent empty territory (False Wall West, Habbanya Erg, Cielago West).
    with c.scene("retreat", "Harkonnen", server=True) as s:
        s.shot("retreat-open")
        s.click("False Wall West (Middle Sector)")
        s.shot("retreat-picked")
        s.click("Habbanya Erg (West Sector)")
        s.shot("retreat-repicked")
        s.button("Retreat", within=".map-pin")
        s.shot("retreat-confirmed")

    # Bene Gesserit (advisors) in the blow report of turn 2: advisors alone in Carthag, Tuek's
    # Sietch and Habbanya Sietch may flip to fighters; Sietch Tabr is in the storm.
    with c.scene("bg-announcement", "BeneGesserit", server=True) as s:
        s.shot("announce-open")
        s.click("Carthag")
        s.shot("announce-picked")
        s.button("Flip to fighters", within=".map-pin")
        s.shot("announce-confirmed")
