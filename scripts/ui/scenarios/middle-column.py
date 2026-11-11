"""The actions column in every situation the fixtures hold: the whole window, and the column's full contents."""

AREA = "middle-column"

SIZE = (1440, 820)  # MacBook Air, browser window

# (fixture, faction to seat, what the shot shows)
CASES = [
    ("battle", "guild", "battle plan"),
    ("battle-special", "emperor", "battle plan, special forces"),
    ("bid", "atreides", "bidding"),
    ("ship", "emperor", "shipment"),
    ("ship-guild", "guild", "Guild shipment"),
    ("move", "fremen", "movement"),
    ("ally", "bene_gesserit", "movement with an ally"),
    ("revival", "emperor", "revival"),
    ("retreat", "harkonnen", "retreat"),
    ("blow", "atreides", "spice blow report"),
    ("setup", "fremen", "Fremen setup"),
    ("setup-host", "emperor", "host custom setup"),
    ("worm", "fremen", "Fremen sends the worm"),
    ("bene-gesserit-accompanies", "bene_gesserit", "Bene Gesserit accompany"),
    ("facedancer", "tleilaxu", "Tleilaxu turn"),
    ("tanks-face-down", "atreides", "face-down leader in tanks"),
    ("bid", "emperor", "waiting for someone else"),
    ("mc-traitors", None, "traitor selection"),
    ("mc-prediction", "bene_gesserit", "Bene Gesserit prediction"),
    ("mc-voice", None, "voice"),
    ("mc-prescience", None, "prescience"),
    ("mc-treachery", None, "call treachery"),
    ("mc-concluded", None, "battle concluded"),
    ("mc-charity", None, "charity"),
    ("mc-clairvoyance", None, "clairvoyance"),
    ("mc-thumper", None, "thumper"),
    ("mc-harvester", None, "harvester"),
    ("mc-handswap", "harkonnen", "karama hand swap"),
    ("mc-discard", "moritani", "discard over the hand limit"),
    ("mc-ended", None, "game ended"),
]

FACTION_NAMES = {
    "Atreides": "atreides", "Harkonnen": "harkonnen", "Fremen": "fremen", "Emperor": "emperor",
    "Guild": "guild", "BeneGesserit": "bene_gesserit", "Ixian": "ixian", "Tleilaxu": "tleilaxu",
    "Choam": "choam", "Richese": "richese", "Ecaz": "ecaz", "Moritani": "moritani",
}


def actor(fixture):
    """The seat make-fixture.cs chose, read from the spec's describe output."""
    import os
    import subprocess
    from harness import REPO
    env = dict(os.environ)
    env["DOTNET_ROOT"] = os.path.expanduser("~/dotnet10")
    env["PATH"] = os.path.expanduser("~/dotnet10") + ":" + env["PATH"]
    out = subprocess.run(
        ["dotnet", "run", "scripts/make-fixture.cs", "--", "describe", fixture],
        cwd=REPO, env=env, capture_output=True, text=True).stdout
    for line in out.splitlines():
        if line.startswith("act as "):
            return FACTION_NAMES.get(line.split()[2])
    return None


def measure(g):
    return g.page.evaluate("""() => {
        const map = document.getElementById('mapdiv').getBoundingClientRect();
        const cols = [...document.querySelectorAll('.side-column')];
        const mid = cols[0];
        const kids = [...mid.children];
        const bottom = kids.length ? Math.max(...kids.map(k => k.getBoundingClientRect().bottom)) : 0;
        return {
            mapWidth: Math.round(map.width),
            sides: cols.map(c => Math.round(c.getBoundingClientRect().width)),
            midContent: Math.round(bottom - mid.getBoundingClientRect().top + mid.scrollTop),
            window: innerHeight,
        };
    }""")


def run(r):
    from harness import FIXTURES
    w, h = SIZE
    for fixture, faction, what in CASES:
        if not (FIXTURES / f"{fixture}.json").exists():
            r.note(fixture, "no fixture")
            continue
        faction = faction or actor(fixture)
        if faction is None:
            r.note(fixture, "seat not in harness")
            continue
        name = f"{fixture}-{faction}"
        try:
            with r.game(fixture, faction) as g:
                g.page.set_viewport_size({"width": w, "height": h})
                g.page.evaluate("window.dispatchEvent(new Event('resize'))")
                g.settle()
                g.shot(name)
                # Pieces for the mockups: the board, your shield, the report.
                g.element_shot(f"{name}-board", "#planetmap-filler", park=False)
                g.element_shot(f"{name}-shield", ".side-column >> nth=1 >> .card.p-1.mb-1", park=False)
                g.element_shot(f"{name}-report", ".side-column >> nth=1 >> .card.report", park=False)
                m = measure(g)
                r.note(f"{name} ({what})", m)
                # The column's whole contents, however long.
                tall = max(h, m["midContent"] + 40)
                g.page.set_viewport_size({"width": w, "height": tall})
                g.page.evaluate("window.dispatchEvent(new Event('resize'))")
                g.settle()
                box = g.page.locator(".side-column").first.bounding_box()
                g.clip(f"{name}-column", box["x"], box["y"], box["width"], m["midContent"])
                # Each panel in the column on its own, for mockups that reuse them unchanged.
                panels = g.page.locator(".side-column >> nth=0 >> :scope > *")
                for i in range(panels.count()):
                    b = panels.nth(i).bounding_box()
                    if b and b["height"] > 4:
                        g.clip(f"{name}-panel-{i}", b["x"], b["y"], b["width"], b["height"])
        except Exception as e:  # keep going; one bad fixture shouldn't stop the set
            r.note(name, f"failed: {e}")
