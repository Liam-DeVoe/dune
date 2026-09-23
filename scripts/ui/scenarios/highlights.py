"""Every highlight variant in the same three states, for design/map-ux/review-highlights.html."""

AREA = "map-ux/highlights"

SOURCE = "False Wall South (East Sector)"
VALID = "Pasty Mesa (South Sector)"
INVALID = "Bight Of The Cliff (North Sector)"  # far from any Fremen force, so never a destination; no token or spice on its centre to end the hover
SAND = (200, 230)
ZOOM_AT = (405, 345)  # between Polar Sink and Tuek's Sietch, so the source, the target and the sectors between them fit  # plain sand in Hagga Basin (West), where a wash reads clearly

# "current" is the client as it is. The rest are hlv-<name> classes on body,
# defined at the end of Styles/components/_map.scss.
ROUND_1 = ["quiet", "fill", "ink", "faction", "territory", "edge-hover"]
ROUND_2 = ["cursor", "dots", "spotlight", "contour", "reveal"]
VARIANTS = ["current"] + ROUND_1 + ROUND_2


def set_variant(g, name, show_all=False):
    classes = [] if name == "current" else [f"hlv-{name}"]
    if show_all:
        classes.append("show-all")
    g.page.evaluate(
        "cls => { for (const c of [...document.body.classList]) if (c.startsWith('hlv-') || c === 'show-all') document.body.classList.remove(c);"
        " for (const c of cls) document.body.classList.add(c); }",
        classes,
    )
    g.settle()


def map_shot(g, name):
    g.element_shot(name, "#planetmap_inner", park=False)


def run(r):
    # Fremen with 6 forces and a Fedaykin in False Wall South (East) and 2 forces
    # and a Fedaykin in False Wall West (Middle). The source is selected and the
    # mouse is over a reachable sector, so selectable, selected, target and hover
    # tints are all on screen at once.
    with r.game("move", "yellow") as g:
        g.click(SOURCE)
        for v in VARIANTS:
            set_variant(g, v)
            g.hover(VALID)
            g.hide_popovers()
            map_shot(g, f"{v}-move")
            g.zoom(f"{v}-move-zoom", ZOOM_AT, size=(560, 420), dy=0)
            if v in ("cursor", "reveal"):
                g.hover(INVALID)
                g.hide_popovers()
                g.zoom(f"{v}-move-invalid", INVALID, size=(400, 300), dy=0)
                r.note(f"{v} invalid class", g.eval("document.getElementById('planetmap_inner').className"))
            if v == "reveal":
                set_variant(g, v, show_all=True)
                g.hover(VALID)
                g.hide_popovers()
                map_shot(g, f"{v}-move-all")

        # The sidebar list idea, mocked by writing the real target names into
        # the Move panel. Only a picture; nothing in the client does this.
        set_variant(g, "cursor")
        ids = g.eval("(document.querySelector('.maparrows')?.dataset.targets || '').split(',').filter(Boolean).map(Number)")
        names = [n for n, l in g.locations.items() if l["id"] in ids]
        r.note("list targets", len(names))
        g.page.evaluate(
            """names => {
                const hint = [...document.querySelectorAll('.card .small')].find(e => e.textContent.trim() === 'Use the map to move forces.');
                if (!hint) return;
                hint.textContent = 'Move 6 forces and 1 Fedaykin from False Wall South (East) to:';
                const wrap = document.createElement('div');
                wrap.id = 'hl-list-mock';
                wrap.style.cssText = 'display:flex;flex-wrap:wrap;gap:4px;margin-top:6px;max-width:420px';
                names.forEach((n, i) => {
                    const b = document.createElement('button');
                    b.className = 'btn btn-sm';
                    b.textContent = n.replace(' Sector)', ')');
                    b.style.cssText = 'font-size:11px;padding:2px 8px;border:1px solid rgba(255,255,255,.25);background:rgba(255,255,255,.06);color:#fcf9d1;border-radius:4px;'
                        + (i === 2 ? 'background:rgba(231,191,60,.35);border-color:#e7bf3c' : '');
                    wrap.appendChild(b);
                });
                hint.after(wrap);
            }""",
            sorted(names),
        )
        g.settle()
        g.park()
        g.shot("list-move")
        g.element_shot("list-move-panel", ".card:has(#hl-list-mock)")

    # The Emperor shipping from reserves: every sector on the planet is a target.
    with r.game("ship", "red") as g:
        for v in VARIANTS:
            set_variant(g, v)
            g.hover(SAND)
            g.hide_popovers()
            map_shot(g, f"{v}-ship")
            if v == "reveal":
                set_variant(g, v, show_all=True)
                g.hover(SAND)
                g.hide_popovers()
                map_shot(g, f"{v}-ship-all")
