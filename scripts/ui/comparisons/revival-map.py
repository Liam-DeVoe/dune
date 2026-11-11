"""The revival form's map placement (client-plan.md package 7d): Tleilaxu at high threshold
(Homeworlds) may place free revivals straight on the planet. The planet amount's range is a draft
query, the pick on the map pins the confirm button, and confirming submits, so the web side runs
on the Rust server (scene_server.py)."""

FORCES, PLANET = ".act-stack .num >> nth=0", ".act-stack .num >> nth=1"


def run(c):
    # Turn 2, revival phase: Tleilaxu (2 spice) may revive from 5 forces in the tanks.
    with c.scene("revival-planet", "Tleilaxu", server=True) as s:
        s.shot("open")
        s.click_on(f"{FORCES} >> button[aria-label='Increase']")
        s.click_on(f"{FORCES} >> button[aria-label='Increase']")
        s.shot("forces-2")
        s.click_on(f"{PLANET} >> button[aria-label='Increase']")
        s.shot("planet-1")
        s.click("The Great Flat")
        s.shot("picked")
        s.button("Revive", within=".map-pin")
        s.shot("confirmed")
