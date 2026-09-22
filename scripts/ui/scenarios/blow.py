"""The spice blow sector tint, on a turn where older piles compete with the new one."""

AREA = "spice-blow"


def run(r):
    with r.game("blow", "green") as g:
        g.element_shot("blow", "#planetmap_inner")
        g.zoom("blow-zoom", "Sihaya Ridge", size=(340, 230), dy=0)  # where the spice blew
