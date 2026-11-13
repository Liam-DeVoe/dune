# Design tooling

Everything here exists to look at the client with a game in a chosen state,
so design work can be reviewed from screenshots and reproduced later.

| Piece | What it does |
|---|---|
| `make-fixture.cs` | Plays bots until a situation comes up, then saves the game to `fixtures/<name>.json` |
| `publish-client.sh [label]` | Publishes the client on its own to `/tmp/treachery-client-<label>` (default `after`) |
| `build-client.sh` | Compiles the client without publishing, for a quick error check (`just build`) |
| `ui/shots.py` | Serves a published client, drives the scenarios in `ui/scenarios/` in parallel, writes PNGs under `design/` |
| `ui/gallery.py` | Writes `design/<area>/gallery.html` pairing every before and after shot by name |
| `ui/serve.py` | Serves a published client with the fixtures on a fixed port, for looking at a scene by hand |
| `ui/compare.py` | Compares the new web client (`web/`) with this client on every fixture and seat, and writes `design/compare/gallery.html` (`just compare`; see [Comparing with the web client](#comparing-with-the-web-client)) |

No game server is involved. The client has a `/scene` page that loads a
fixture over HTTP, seats one faction and renders the in-game screen, with
bots standing still. `shots.py` serves the published client and the
fixtures from a port it picks, and each scenario opens that page.

## The three commands

```sh
just publish                  # after any client change; about 10 s incremental
just shots ship worm-ride     # writes design/map-ux/after/{ship,worm-ride}-*.png
just gallery map-ux           # writes design/map-ux/gallery.html
just serve                    # serves the published client on port 8765 and prints scene URLs
```

`just shots` with no names runs every scenario, four at a time
(`--workers N` to change). `--list` shows what exists. `--out DIR` writes
somewhere else, for checking a scenario without touching reviewed images.
`--gallery` writes the gallery for each area touched once the shots are
done. Fixtures come from `just fixture <case>`, only when a scenario needs
a situation that no existing fixture holds.

`shots.py` runs under `uv` and installs its own Playwright, so there is
nothing to set up beyond `uv` and the dotnet 10 in `~/dotnet10`.

## Before and after

Before and after are two published builds. `--before` serves
`/tmp/treachery-client-before` and writes to `design/<area>/before/`; the
default serves `/tmp/treachery-client-after` and writes to `after/`. A
review page in `design/<area>/` embeds both by name.

To take the before shots from an older commit, publish from a worktree:

```sh
git worktree add /tmp/treachery-before <commit>
/tmp/treachery-before/scripts/publish-client.sh before
just shots --before
git worktree remove /tmp/treachery-before
```

The older commit must contain the scene page (`Client/Pages/Scene.razor`),
because that is what the scenarios drive. For commits from before it
existed, cherry-pick the commit that added it into the worktree first.

Any published directory can be served with `--client DIR`. `--url URL`
skips the built-in server and drives a server that is already running,
which then has to serve the scene page and `/fixtures/<name>.json` itself.
The dev server from `just run` serves the client but not the fixtures, so
it does not work for this without help.

## Writing a scenario

A scenario is one file in `ui/scenarios/`. It declares which `design/`
area it belongs to and a `run` function that opens games and takes shots:

```python
"""One line saying what this shows; --list prints it."""

AREA = "map-ux"


def run(r):
    # The Emperor shipping from reserves. Carthag is a valid, expensive destination.
    with r.game("ship", "emperor") as g:
        g.shot("ship-1-start")
        if r.after:
            g.click("Carthag")
            g.shot("ship-2-pinned", zoom="Carthag")
```

`r.game(fixture, faction, steps=None)` opens a page on
`fixtures/<fixture>.json` seated as that faction and closes it when the
block ends. Factions are `atreides`, `harkonnen`, `fremen`, `emperor`, `guild`,
`bene_gesserit`, `ixian`, `tleilaxu`. `steps` is a list of clicks the page makes before
it counts as loaded, written `"click:Carthag"`, `"shift:Sietch Tabr"`,
`"ctrl:..."`, `"right:..."` or `"button:Confirm"`. It suits a starting
state; when a shot is taken between clicks, write them as `g.click` lines so
the sequence reads in one place.

Locations are named the way the client names them: `"Carthag"` for a
territory with one sector, `"False Wall South (East Sector)"` otherwise. A
name that does not exist raises an error listing close matches. Anything
that takes a location also takes an `(x, y)` tuple in map coordinates, for
a point that is not a location's centre. Say in a comment what the point is.

Inside the block:

| Call | Effect |
|---|---|
| `g.click(name, shift=False, ctrl=False, right=False)` | Click a location. Goes straight to the component, not through the mouse |
| `g.hover(name_or_point)` | Move the real mouse to a location's centre, for hover effects and popovers |
| `g.park()` | Move the mouse off the map |
| `g.button("Ride!", within=".map-pin")` | Click a button by label, optionally inside a selector |
| `g.hide_popovers()` | Hide the intel popover when it would cover the subject |
| `g.shot(name, zoom=None, size=(520, 360), dy=-60, park=True)` | Full page to `name.png`; with `zoom`, also `name-zoom.png` around that place |
| `g.zoom(name, place_or_pair, size, dy)` | Just the clip around a location or point, or the midpoint of two |
| `g.element_shot(name, selector, height=None, park=True)` | Clip to an element |
| `g.clip(name, x, y, w, h)` | Clip to viewport pixels |
| `g.point(name)`, `g.screen(name_or_point)` | A location's map coordinates; a place's viewport coordinates |
| `g.count(selector)`, `g.eval(js)`, `g.wait_gone(text)` | Look at the page |
| `g.settle(quiet_ms=150)` | Wait until the page has stopped changing. Every action above already does this |
| `g.locations`, `g.page` | The location table and the Playwright page, for anything else |
| `r.note(key, value)` | Print something observed, such as whether a pin appeared |

Conventions worth keeping:

- Shots are named `<flow>-<step>-<state>`, so a review page can pair
  `ship-2-pinned.png` with its before.
- Say in a comment what the fixture holds (which faction has forces where),
  since that is what makes the clicks meaningful.
- Shots park the mouse first. Pass `park=False` when the hover is the point.
- There are no sleeps. Actions wait for the page to stop changing, which
  covers a phase advancing too. If something still renders late, call
  `g.settle()` with a longer quiet time rather than `time.sleep`.
- Screenshots are taken with animations disabled and the caret hidden, and
  the scene page switches CSS transitions off, so a shot right after an
  action shows the finished state.

## Fixtures

A fixture is a saved game in a chosen situation. `fixture-specs.json`
holds one spec per fixture: the situation to reach, any rules the case
adds to the six-faction basic game, and the seed the generator pinned.
`make-fixture.cs` plays bots forward from that seed until the situation
comes up and writes `fixtures/<name>.json`:

```sh
just fixture worm-ride                        # regenerate from its spec
just fixture describe worm-ride               # print what the fixture holds: forces per location, spice, hints
just fixture --locations                      # every location name
```

The printout is what a scenario author reads to know what to click. The
condition vocabulary and the optional `place` section, which states
exactly which forces start where, are documented at the top of
`make-fixture.cs`. Bot randomness is seeded, so a spec and seed give the
same file every time. The fixtures that exist today predate that, so
regenerating one produces a different game in the same situation; do it
only when the scenario that uses it changes. `worm-ride-ssw.json` was made
by hand from `worm-ride.json` by adding rule 105 and has no generator.
Every fixture is at the latest game version; the engine loads no other. The
reskin screenshots in `design/before` and `design/after` were also taken
by hand, not by a scenario.

## Comparing with the web client

The React client in `web/` must look like this one (`rust/docs/client-plan.md`, "Comparing
with the C# client"). `just compare` checks that, scene by scene:

```sh
just compare                          # every fixture, each faction in it plus an observer; about 40 minutes
just compare alliance mc-ended        # the scenes of two fixtures
just compare alliance/Emperor             # one scene
just compare --no-build alliance/Emperor  # skip the three builds below
just compare --require side-column    # exit 1 unless every scene matches in that region
just compare --list
```

It first builds both clients and the views, in parallel, into a new directory
`/tmp/treachery-compare/builds/<time>-<pid>`: it publishes this client to `csharp/`, builds
`web/` to `web/` (without the type check, so a type error elsewhere does not stop a
comparison), and runs `rust/view/examples/scene_views.rs`, which writes the view of every
scene to `views/` with a manifest. When all three succeed, `/tmp/treachery-compare/current`
points to the new build. `--no-build` uses the build `current` points to, so if the view crate
does not build because someone is in the middle of a change, `--no-build` compares the
previous build.

A run keeps using the build it started with, even when another agent's build replaces
`current` in the meantime, so agents can compare at the same time without a private work
directory. A build is deleted once it is neither `current` nor used by a running comparison.

For each scene and for two viewports, desktop (1440×820) and phone (390×844), it opens this
client's `/scene?fixture=alliance&faction=emperor` and the web client's `/scene/alliance/Emperor`. The
web client renders the view of that scene. The scene page takes `faction=observer` for a
non-host observer. Both pages are shot in full, and these regions are compared:

| Region | Selector |
|---|---|
| page | the whole document |
| turn-banner | `.turn-banner` |
| board | `#mapdiv` |
| side-column | `.side-column` |
| player-panel | `.side-column .card:has(.player-panel)` |
| prompt | `.side-column .act-stack` |
| log | `.side-column .report` |
| card-play | `.card-play-pop` (the panel a playable card in the hand opens) |

A region matches when its DOM digests are equal and at most 0.05% of its pixels differ by
more than 24 in a channel. The DOM digest lists every visible element with its tag, classes,
image source, title and own text, so its diff names a missing chip or a wrong label. A region
neither client shows counts as a match. `--self-check` compares this client with itself,
which must match everywhere. It tests the harness and the masks.

Both pages see the same fixed time (1 January 2027), so the clocks agree, and animated GIFs
are served as their first frame. Animations and transitions are off.

Four differences are expected (`rust/docs/protocol.md`, "Visibility"). In three, this client
shows hidden information that the web client leaves out. In the fourth, this client's bidding
panel crashes for an observer and Blazor's error bar covers the bottom of the page, where the
web client shows no bidding panel. They are masked: painted magenta in both shots, and the
elements inside a mask are left out of both digests without a trace. The masks are `MASKS` in `compare.py`,
and `scene_views.rs` decides which apply to each scene.

Each run writes its results to its own directory, `design/compare/runs/<time>-<pid>`, so
agents comparing at the same time don't overwrite each other. `--out DIR` (or `COMPARE_OUT`)
picks another directory. The run prints its directory when it starts. When it ends, it prints a
table of matches per region and then every comparison that did not match, with the
regions that differ per viewport and the path of the scene's page:

```
2 of 5 comparisons do not match:
  move/Fremen                          desktop: board differs (1.20% px, DOM), page differs (0.40% px); phone: board only in C#
                                       design/compare/runs/20261110-202233-46455/move/Fremen/index.html
```

`--require` checks only the current run's results.

In the run's directory, `gallery.html` has one matrix per region and viewport, with a square
per scene. Each square links to the scene's page, which shows both shots, the differing pixels
and the DOM diff. `results.json` holds the same data for scripts.

`design/compare/gallery.html` and `results.json` are shared. They hold the latest result of
every scene across all runs and link into the run directories. A run of every scene replaces
them, and a partial run replaces only the scenes it compared. Runs merge into them one at a time
under a lock, and the files are replaced by rename, so a reader never sees half a file. Only
runs under `design/compare/runs` are merged in. A run with `--out` elsewhere or with
`--self-check` is not.

The newest 20 run directories are kept. An older one is deleted once the shared gallery no
longer links to it, which is when later runs have compared all of its scenes again. A run still
in progress is never deleted. Directories named `design/compare/<fixture>` are from before run
directories existed, and the shared gallery links to them until those scenes are compared again.

Scripted comparisons in `ui/comparisons/` drive both clients through the same clicks, for
screens that need input to reach:

```python
"""Passing on alliances."""

def run(c):
    with c.scene("alliance", "Emperor") as s:   # viewport="phone" for the phone
        s.shot("start")
        s.button("Pass")
        s.shot("passed")
```

`s.click(location)`, `s.button(label, within=None)` and `s.click_on(selector)` (a Playwright
selector that holds in both clients) act on both pages. `s.shot(name)`
compares them and becomes `<file>:<name>` in the gallery. Map clicks fail in the web client
until it has a board.

The saved views can't answer a draft query or take an event, so a web page on them can't
change a form that asks the server (a battle plan's leader, Clairvoyance's question) or advance
after a confirm. `c.scene(fixture, faction, server=True)` opens the web side on the Rust server
instead (`ui/scene_server.py`: `treachery-server` with `DevScene` on, serving the web build), where
the same `/scene/<fixture>/<Faction>` route plays the scene over the WebSocket, as the C# scene
page plays it in the browser. `comparisons/battle-plan.py` and `battle-clairvoyance.py` work this
way. `just compare --server ...` does the same for every scene. That matters for forms that
submit by themselves when they appear, as C# does: in `mc-treachery/Emperor` the Emperor, who can't
call treachery, passes at once in C#, and only a server lets the web client do the same.

## Troubleshooting

**The shots do not show my change.** The published client is stale; run
`just publish` again. `shots.py` serves whatever is in
`/tmp/treachery-client-after`, and prints which directory that is.

**`static server: 404 /fixtures/x.json`.** The scenario names a fixture that
does not exist under `scripts/fixtures/`. Make it with `just fixture x`,
or check the name. With `--url`, the server you pointed at has to serve
the fixtures itself.

**`scene page failed for ...: Unknown location ...`.** A `steps=` entry
names a location the page does not know. Names are checked before a
`g.click`, but steps are checked by the page.

**Publish hangs at the start.** It is waiting for another build of the
same tree to finish; `build-client.sh` and `publish-client.sh` share a
lock at `/tmp/treachery-build.lock`. If nothing else is building, a
crashed build left the lock behind: `rmdir /tmp/treachery-build.lock`.

**A scenario times out waiting for the page.** With many workers, the
first page of each browser downloads the whole runtime at once. The
server allows for that, but if the machine is busy, try `--workers 2`.
The summary at the end lists console errors per scenario, which usually
say what went wrong.
