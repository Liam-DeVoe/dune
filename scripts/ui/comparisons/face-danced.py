"""FaceDanced (client-plan.md package 7b): the Tleilaxu replace the battle winner's forces with
their own, from reserves and from the planet, with the source and target pickers. Changing a
picker asks the server for the draft's validation, and confirming submits, so the web side runs
on the Rust server (scene_server.py)."""


def each(s, act) -> None:
    """Runs act(page) in both clients, then waits for both to settle."""
    for page in s.pages.values():
        act(page)
    for page in s.pages.values():
        page.evaluate("q => window.treacheryScene.settled(q)", 150)


# The three pickers, in order: from reserves, from Sietch Tabr, to Tuek's Sietch.
RESERVES, SOURCE, TARGET = (f".act-stack .num >> nth={i}" for i in range(3))


def run(c):
    # Tleilaxu revealed a face dancer in Tuek's Sietch and may place up to 3 forces there, from
    # 16 in reserve or from the 4 in Sietch Tabr.
    with c.scene("face-danced", "Tleilaxu", server=True) as s:
        s.shot("open")
        # The face dancer's flash hides after 6 s, which the next shots would race: click it away.
        each(s, lambda p: p.evaluate("() => document.querySelector(\"div[style*='fadeInFadeOut']\")?.click()"))
        s.click_on(f"{RESERVES} >> button[aria-label='Increase']")
        s.shot("reserves-1")
        s.click_on(f"{SOURCE} >> button[aria-label='Increase']")
        s.click_on(f"{TARGET} >> button[aria-label='Increase']")
        s.shot("source-1-target-1")
        # A typed target over the limit of 3. As in C#, typing changes only what the picker shows;
        # the next step goes from the placement's 1.
        each(s, lambda p: (p.locator(f"{TARGET} >> input").fill("5"), p.locator(f"{TARGET} >> input").press("Enter")))
        s.shot("target-typed")
        s.click_on(f"{TARGET} >> button[aria-label='Decrease']")
        s.shot("target-0")
        s.click_on(f"{TARGET} >> button[aria-label='Increase']")
        s.click_on(f"{TARGET} >> button[aria-label='Increase']")
        s.shot("target-2")
        s.button("Confirm")
        s.shot("confirmed")
