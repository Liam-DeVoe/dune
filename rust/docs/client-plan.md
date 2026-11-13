# Client plan

The plan for the new web client and the view layer behind it. Read [PORTING.md](../../PORTING.md) ("Decisions for phases 3 and 4") first. The wire format is the Game view section of [protocol.md](protocol.md#game-view); what the C# client reads, per screen area, is [client-inventory.md](client-inventory.md).

## The decision in one paragraph

The server holds the game and runs the Rust engine. After each event it sends every participant a view: what that user may see, the actions they can take now, and the valid choices for each. The client is a React + TypeScript app with no engine. It draws views, sends draft queries while a player fills in a form, and submits finished events. It must look and behave like the C# Blazor client (`treachery-online/Client`, 241 razor components), but its code is a redesign.

## Pieces

| Piece | Where | What |
|---|---|---|
| Inventory tool | `rust/view/csharp-inventory/` | Roslyn over the client's C# and the Razor generator's output. Lists every `Shared` member each client file uses, grouped by screen area, and the helpers each action form calls. Writes `rust/docs/client-inventory.md`. Run `rust/view/csharp-inventory/run.sh` after the C# client changes |
| View crate | `rust/view/` (`treachery-view`, in the workspace) | Builds views and answers draft queries from the engine. `schema.rs` is the wire format |
| TypeScript types | `web/src/generated/` | Generated from `schema.rs` by `rust/view/tests/ts_bindings.rs` (ts-rs), checked in; the test fails when they are stale |
| Web client | `web/` | React + TypeScript + Vite |
| Skin | `rust/skin` (package S, another agent) and `web/public/skin/default.json` | Text on the server, images and colours on the client; see [Skin](#skin) |

### Why `web/` at the top level

The client is not Rust, has its own toolchain (npm, node_modules of about 80 MB), and is not a member of the Cargo workspace. Under `rust/` it would sit inside the tree that `rust/tools/check.sh`, `cargo` and the porting docs treat as the port. At the top level it sits beside `treachery-online/`, whose `Client/wwwroot` it serves its CSS and art from, and beside `scripts/ui`, whose Playwright tooling the comparison harness extends. The obsolete Leptos crate `rust/client` (a WASM client that ran the engine) should be removed from the workspace by the coordinator; nothing depends on it.

## The view replica

Building a view runs `GameStatus.DetermineStatus` and `Game.GetApplicableEvents`, and draft queries run `Validate()` and the `Valid*` helpers. Some of these write game state: `NextPlayerToBattle` advances the battle sequence (hazards 4.3), and pathfinding fills the neighbour cache, which serves stale results until the next event (hazards 6.1). In C# these calls ran only in each client's own copy of the game. The C# server's game never saw them, and its state after each event is what the Rust server must reproduce.

So the view layer never touches the server's game (see [server.md](server.md)). Each game on the server has a `ViewReplica` (`rust/view/src/replica.rs`): a second `Game` that plays the part of a C# client's copy.

- It executes each accepted event as `Client.HandleGameEvent` does, `Execute(false, false)`: no validation.
- After each event, every view is built from it, which runs `DetermineStatus` and `GetApplicableEvents` as `Client.PerformPostEventTasks` did.
- Draft queries run on it.
- Undo, restore and dismiss replace it: replay the server's new history into a fresh replica, as `Client.HandleUndo` ran `Game.Undo` on the client's copy.

The replica is disposable in the sense the coordinator asked for: nothing flows back from it, and it can be rebuilt from the history at any time. It is kept between events instead of being cloned per view because `Game` has no `Clone` (C# `Game.Clone` is not ported, engine-conventions.md "Not ported"), and replaying the whole history for each view costs O(events) per event.

One replica serves every viewer of a game, whereas each C# client had its own copy. That is equivalent:

- `NextPlayerToBattle` advances the sequence until it finds a player with a battle and then stops, so later reads in the same state return the same player without writing anything. The first viewer's build leaves the state that one build in a C# client would leave.
- The neighbour cache's results depend on the order of queries within one event. Each C# client also had its own query order, so no single order is "the" C# one. The replica's results are those of a client that made the same queries.

To check this, `rust/view/tests` builds views for all viewers in different orders and compares them (package 1 adds the test over the bot-game corpus). If a difference shows up, the fallback is one replica per viewer, exactly as in C#, at the cost of memory (a game state is a few MB).

## Visibility audit

The rules are in [protocol.md](protocol.md#visibility). They come from reading every client component that reads hidden data, with the inventory as the index (the audit's notes are summarised here so the next agent can check them):

- Every action form renders only when its event type is in `Client.Actions = Game.GetApplicableEvents(Player, IsHost)`. Forms are therefore per player by construction; their options can contain hidden data of the acting player (traitor choices, the decipherer's traitors, Ixian's starting cards, CHOAM's audited cards) and that is correct.
- Hidden data shown outside forms: `PlayerPanel` (own seat only), `NoteComponent` (notekeeping), `CardPiles` (discard pile, resource deck top, Richese's cache), `BiddingPanel` (card on auction), `Bids.razor`, `BattlePlans.razor`/`BattlePlan.razor`, `Tanks.razor` (face-down leaders), `ForcesOnPlanet`/`MapComponent` (terror, discoveries, No-Field), `DealComponent`, `TrackerComponent` (Atreides), `Story*` (game end).
- The engine's knowledge sets the client relies on: `Player.KnownCards` (through `RegisterKnown`/`UnregisterKnown`, `Game.cs:414-457`), `Game.KnownCards(p)` (adds the ally's), `RevealedTraitors`, `RevealedFaceDancers`, `KnownNonTraitors`, and the prescience predicates `HasBiddingPrescience`, `HasResourceDeckPrescience`, `HasStormPrescience`. The view builder calls these; it invents no rule of its own.
- `GameColumns.razor:33` shows every player's panel when the viewer's player has `Faction.None`. For current versions that can't happen: factions are assigned before `TradingFactions` ends (`Game_Setup.cs:41-44`). The view treats such a viewer like an observer.
- Skin popups (`Skin.GetPopup(IHero, Game)` and the faction sheets) read only public data (`Game.Skill`).
- No log record attached to a public message carries hidden data; the private ones (`CardWonShown`, `ExtraCardShown`) are sent with `ExpressTo`.
- Three C# leaks (partial prescience shown to all, the story ignoring `HideSecrets`, silent-auction pass colours) are listed in protocol.md with the view's behaviour.

## The web app

### Structure

```
web/
  index.html            loads the compiled CSS from /css, then src/main.tsx
  vite.config.ts        serves assets/ (art, css, fonts, js, docs) in dev and preview
  assets/               art, fonts, scripts, PDFs, Bootstrap; css/treachery.css is built from styles/
  styles/               the SCSS, compiled by `npm run build:css` (Dart Sass)
  public/skin/          the default skin as JSON, edited by hand
  public/static/        engine static data: locations (rust/view/examples/static_data.rs)
  public/views/         saved views for the skeleton and component tests (rust/view/examples/scene_view.rs)
  src/generated/        TypeScript types of the view (generated, do not edit)
  src/skin.ts           names, images, colours from the skin JSON
  src/state/            view sources (static JSON now, WebSocket next) and the React context
  src/components/       one component per C# screen part, named after it
```

Components map to the C# ones by name and keep their DOM structure and class names (`card`, `card-header`, `prompt-act`, `pp-bar`, `chip faction`, ...), because the look comes from the C# stylesheet unchanged. A component's header comment names the razor file it stands for. The C# client's own structure is worth keeping where it is the layout (GameColumns, ActionPanel, PlayerPanel, ReportComponent, the map's layers) and worth dropping where it is engine plumbing (components reaching into `Game`).

### State

- One store per connection: login, lobby status (the server plan's `ServerStatus`), chat, and the current game's latest `GameView`. A small external store (`useSyncExternalStore`) is enough; no Redux.
- The view replaces the previous one whole. Components derive what they draw from it and from the skin, and keep only UI state: a form's draft, which log phase is shown, collapsed panels, the map's selection.
- A form's draft is local to the form component and is discarded when its event type leaves `actions`.
- Draft queries go through the source (`ViewSource.query`); answers older than the form's latest query are dropped.
- Browser preferences the C# client kept in local storage (sound volumes, auto-pass, collapse states, the Atreides tracker's notes) stay in local storage under the same keys where possible.

### Map interaction (board and forms)

Agreed between package 4 (board) and package 7c (ship and move). The API is `web/src/components/board/mapInteraction.ts`, a port of the C# `MapInteraction.cs`, `MapPicker.cs` and `Client.LocationClick`:

- `mapInteraction` is the page's one `MapInteraction`. One form owns it at a time (`claim`, `release`, `isFree`, `isOwnedBy`). The owner fills `selectable`, `selected`, `targets`, `target`, `origin`, `endPoint`, `pin` (a React node drawn in the panel pinned to the arrow), `showCursorArrow`, `staged` and `onCancel`, then calls `refresh()`. The board's layers (`MapSelectionHighlights`, `MapSelectionOverlay`, `StagedForces`) only read it.
- Every click on a location goes to `onLocationClick` listeners as `{location, shift, ctrl, alt, right}`. A form acts on a click only while it owns the map.
- `useMapPicker` is `MapPicker`/`MapPickComponent` for the forms that pick one place, optionally after source sectors. Forms that drive the map in their own way (`ShipmentComponent`, `PlacementComponent`, `StagedPlacementComponent`) claim the map when it is free, push their state after every render and release it on unmount, as the C# components do.
- Which locations are valid comes from the action's options (ids of locations; a territory pick lists every sector of each territory, as `MapPickComponent.SectorsOf`). The form filters out what the board can't draw with `Board.hasCenter` (`SectorGeometry.HasCenter`), and finds the player's token with `Board.playerTokenPosition`, constructing `new Board(view, skin)` itself because it sits outside the board's context.
- When a click changes the form's inputs, the form sends a draft query and pushes the answer's options (for example a move's destinations for the forces selected so far) to the map.

### Routing

| Route | Screen |
|---|---|
| `/` | login, then lobby (`Main.razor`'s branches) |
| `/:userToken/:gameId` | reconnect to a game, as the C# client's second route |
| `/scene/:fixture/:faction` | the test scene (see below) |
| `/admin` | admin page |

Inside a game there are no routes; the screens follow the game's phase as `Main.razor` does (awaiting players, seating, the game screen).

The app can be served under a path prefix: `TREACHERY_BASE=/beta/ npx vite build` sets Vite's `base`, and the routes above are relative to it. The deploy at https://treachery.exe.xyz/beta/ is built this way, behind a proxy that strips the prefix, so the server always serves at `/`. Write every absolute URL (fetches, the WebSocket, links, art) with `withBase` from `src/base.ts`, or relative to `<base href>`. A literal `"/art/..."` works locally and breaks under `/beta/`.

### CSS, fonts, art

The C# client's look is its SCSS (`treachery-online/Client/Styles`, 6200 lines) compiled to `wwwroot/css/treachery.css`, Bootstrap, the three shipped fonts and 48 MB of art. The web client loads the compiled CSS and the art from `wwwroot` unchanged: `index.html` links `/css/bootstrap.min.css` and `/css/treachery.css`, and Vite (dev, preview) and the Rust server (production) serve `/art`, `/css`, `/fonts`, `/js`, `/docs` and the fonts from `treachery-online/Client/wwwroot`. Nothing is copied, so a style fix in the C# client reaches both clients. The few razor-scoped styles (`*.razor.css`: `Image`, `NumberPickerComponent`, `SimpleNumberComponent`, `DateTimePicker`, `DealComponent`, `RunningGamesComponent`) become CSS modules next to their React components. When the C# client is retired, `Styles/` moves into `web/` and is compiled by Vite's Sass support. Design work follows `design/design-language.html`.

Since native-plan.md step 5 the SCSS and the art, fonts, scripts and PDFs the site uses live in `web/styles` and `web/assets`, and `npm run build:css` compiles the SCSS with Dart Sass (pinned in `package.json`) to `web/assets/css/treachery.css`. The paths above are from before that move.

The client-side JavaScript of `wwwroot/index.html` and `wwwroot/js` (board scaling `SetPlanetMapScale`, tooltips, the listbox, hover tinting through `data-ref-*`, `follow-log.js`) is ported to TypeScript modules as the components that need them are built, not loaded as globals.

### Skin

Coordinator decision 1: `Skin.cs` and `DefaultSkin.cs` are ported as the shared crate `rust/skin`. The server renders all text with it: every `RichText` part has a `label` from `Skin.Describe`, so the client needs no port of `Describe` or `Format`. `describe::Describer` is implemented for `treachery_skin::Skin`; the replica holds `DefaultSkin.Default` until the server gives it the game's custom skin.

The client still needs the skin for images, colours, map geometry and the popup texts (card descriptions, faction sheets), so it loads the same skin as JSON: `web/public/skin/default.json`, exported from `DefaultSkin` once and edited by hand since native-plan.md step 5 (`rust/skin` holds the server's separate copy), and the game's custom skin when the host sets one (the `SetSkin` notification carries it). Popups that C# builds as HTML in `Skin.GetPopup` (and the client's `Support.GetPopup(ResourceCard)` and place chip) are ported as string builders in `web/src/popups.ts`, reading the skin JSON, and shown by Bootstrap as in C#.

## Comparing with the C# client

Playwright drives both clients into the same game state and compares screenshots and DOM.

### Getting both into the same state

The C# client already has what is needed: the `/scene` page (`Client/Pages/Scene.razor`, `Scene/SceneGameService.cs`) loads a savegame fixture from `scripts/fixtures/` over HTTP, seats one user (id 1, named "Scene", host) as a chosen faction, leaves the other seats to bots that never move, and executes events locally. `scripts/ui/shots.py` drives it with scripted clicks, and `just fixture` makes new fixtures by playing bots until a situation comes up. No C# server is involved.

The Rust side mirrors it:

- `ViewReplica::scene(savegame, faction)` builds the same game: same replay, same single seated user, same name, host. Tested in `tests/slice.rs`.
- The Rust server gets a development-only endpoint (off by default, a config flag): `POST /dev/scene {fixture, faction}` loads `scripts/fixtures/<fixture>.json` as a game with that participation and returns a user token and game id. Events submitted to it run through the normal server path, so scripted clicks advance it as in C#.
- The web client's `/scene/:fixture/:faction` route calls that endpoint and then connects normally. Without a server, it renders the saved view `public/views/<fixture>-<faction>.json` (what the skeleton does now).

So both clients start from the same savegame, seated the same way, and the same `steps` (`click:Carthag`, `button:Confirm`) drive both. The C# server is not needed for the comparison. Comparing against the C# server and the Rust server directly (same events through both hubs) is the server plan's business.

### What is compared

- Screenshots of the whole page and of named regions (the side column, the prompt, the player panel, the log, the map), at 1440×820 and at a phone size, with a small per-pixel tolerance for font antialiasing. Animations off (`prefers-reduced-motion`, and the `fadeIn` keyframes disabled by an injected style).
- A DOM digest per region: the tree of elements with their class lists and visible text, ignoring Blazor's comment nodes, `b-xxxx` scope attributes and React's internals. This catches a missing chip or a wrong label more readably than pixels.
- Popovers: hover each chip, card and territory that has one and compare the popover.

The expected differences (the C# leaks in protocol.md) are listed per scenario as masked regions, each with the reason.

### Harness layout

Built by package 10. How to run it and read the results: [scripts/README.md](../../scripts/README.md#comparing-with-the-web-client).

- `just compare` (`scripts/ui/compare.py`) runs every scene: each fixture in `scripts/fixtures`, seen by each of its factions and by an observer, at desktop (1440×820) and phone (390×844) size. Names narrow it (`just compare alliance`, `just compare alliance/Emperor`).
- Each build of both clients and the views goes to its own `/tmp/treachery-compare/builds/<time>-<pid>`, and `/tmp/treachery-compare/current` points to the newest complete one. A run uses one build from start to end, so concurrent runs and builds don't disturb each other.
- The C# side is the `/scene` page, published to the build's `csharp/`. It takes `faction=observer` for a non-host observer.
- The web side is `web/` built to the build's `web/`. Its `/scene/:fixture/:faction` route renders `/views/<fixture>-<Faction>.json`, which `rust/view/examples/scene_views.rs` writes for every scene, with a manifest. `web/src/scene.ts` gives the harness the same hooks as the C# `wwwroot/js/scene.js` (`data-scene`, `treacheryScene.settled`, `click`, `button`).
- Regions are compared by pixels and by a DOM digest. The regions are page, turn-banner, board, side-column, player-panel, prompt and log, each found by a selector that holds in both clients.
- The three C# leaks, and the C# error bar for observers in bidding, are masked where `scene_views.rs` says they apply. Elements inside a mask are left out of both digests. Both pages see a fixed time, and GIFs are served as their first frame. `--self-check` compares the C# client with itself and must match everywhere.
- Each run writes `gallery.html` (a matrix per region and viewport, linking to a page per scene with both shots, the differing pixels and the DOM diff) and `results.json` to its own `design/compare/runs/<time>-<pid>`, and prints every comparison that did not match. `design/compare/gallery.html` merges the latest result of every scene across runs.
- `--require <region>` exits 1 unless every scene matches in that region. This is a client package's done check: package 5 runs `just compare --require side-column`, package 4 `--require board`. There is no CI in this repo, so this command is the CI step when one exists.
- Scripted comparisons in `scripts/ui/comparisons/` drive both clients through the same clicks (`s.click`, `s.button`, `s.click_on`) before a shot. The existing `scripts/ui/scenarios` stay C#-only design tools. Port one to `comparisons/` when the web client can show what it clicks through.

**Switching to the dev scene endpoint.** When package 2 adds `POST /dev/scene`, the web route opens the scene on the server instead of fetching `/views/`. The URL `/scene/<fixture>/<Faction>` and the hooks stay the same, so the harness only has to start the Rust server and serve the web build next to it, in place of `WebServer` in `compare.py`. `scene_views.rs` is then needed only for the manifest (scene list and leaks), and that could move to the endpoint too. Scripted comparisons that submit events (a confirm button) need the server. With saved views, the web client can't advance.

## Packages

Each package is one agent's work, with tests, against the files named. Packages 1 to 3 come first; 4 to 9 can then run in parallel; 10 runs alongside all of them.

| # | Package | Contents | Done when |
|---|---|---|---|
| 1 | View core | The planned `GameView` fields that are not panels: `piles`, `tanks`, the rest of `PublicPlayer`, `PrivatePlayer` and `BoardView`, `timing`, `flash`. Order-independence test for the replica over the bot-game corpus. `CardPlays.Playable` for `hand[].playableWith`. Static data export completed (leaders' factions, card types, territory flags) | every field of the inventory's "Board", "Your faction panel" and "Prompt" areas maps to a view field, with tests per field from fixtures |
| 2 | Server glue | With the server agent: the replica per game, `GameView` after each event and participation change, `QueryDraft`, `RequestReport`, `RequestUndoList`, the dev scene endpoint. The WebSocket `ViewSource` in `web/src/state` | the web client plays a scene against the Rust server |
| 3 | Rich text and skin | `Describer` for `rust/skin`; `LogLine.record` for every log record type; client `RichText` with every part drawn as `ExpressionComponent` does, with popovers | every log line in the fixtures renders the same text as C# |
| 4 | Board | `MapComponents/*`: the planet SVG, forces, spice, storm, strongholds, tokens, the ring, card piles, tanks, battle plans, bids, map interaction (selection and highlights from action options) | map comparison passes for every fixture |
| 5 | Side column | `PlayerPanel`, `GameStatusComponent`, `MenuComponent`/`MenuPanels`, `TradePopover`, `NoteComponent`, `TrackerComponent`, `UndoComponent`/`UndoTimeline`, `ChatComponent` | comparison passes for the side column |
| 6 | Log | `ReportComponent` and the eight phase logs (`OtherComponents/Log`), history browsing | log comparison passes for every fixture and every report |
| 7a–7e | Action forms | One option builder in `rust/view/src/actions.rs` and one React form per event component, split by the inventory's areas: 7a bidding (3 panels and the bidding events), 7b battle (25 components), 7c ship and move (27), 7d revival (8), 7e other events (47). Each form: options, default draft, dependent queries, validation, the C# layout | the form's options equal the C# helper results for every fixture state where the event is applicable, and the screenshot matches |
| 8 | Lobby and setup | Login, game lists, scheduling, `EstablishPlayersComponent`, `PerformSetupComponent`, seating, awaiting game, admin | comparison against the C# client with a C# server |
| 9 | End of game | `WinnerPanel`, `Story*`, the `ending` view field with `HideSecrets` | comparison on `mc-ended` |
| 10 | Comparison harness | `just compare`, both clients in one scenario, DOM digest, masks for expected differences, gallery | runs in CI for every scenario |

Package 7 is the largest: 140 event types. The inventory's "Action forms" table lists the helpers per component, and the C# component is the specification for layout and defaults. Packages 7a–7e can each start as soon as 1 to 3 are done.

## Open questions

For the coordinator. Nothing is blocked on them; the current choice is in brackets.

1. C# leaks: should the view reproduce the three places where the C# client shows hidden information (partial prescience to all, story ignoring `HideSecrets`, silent-auction pass colours)? [No; listed as expected differences.]
2. Ecaz's unplaced ambassadors and `ResourcesAfterBidding`/bribes/extortion/banked spice are shown to everyone by C#. Public under the rules? [Shown, as in C#.]
3. Should the black-market Richese-betrayal Nexus form name `CardJustWon`? [As C#: yes.]
4. Encoding: the Game view uses camelCase and enum names, while the rest of the protocol mirrors the C# hub's PascalCase and enum numbers. [Keep: the view is new, and the skin JSON is keyed by enum names.]
5. View size: full views of 15–30 KB after every event, mostly the log. Fine for now; later, send `log.latest` only when it changed, or JSON diffs. [Full views.]
6. Undo list: the C# host saw `GetShortMessage` of every event. `RequestUndoList` sends the same. Any reason to restrict it further? [No.]
7. The obsolete `rust/client` (Leptos, ran the engine in WASM): remove it from the workspace? [Coordinator's call; nothing depends on it.]
8. Can an observer reach the bidding panel? `HasBiddingPrescience(null)` looks like it would throw in C#. [It throws. Since 2026-11-11 observers get the panel of a player without prescience: protocol.md, deviation 5.]

## Coordinator answers (2026-11-09)

1–6 and 8: the current choices in brackets stand.
7: `rust/client` is removed from the workspace and deleted.

## Progress and next steps

Done (2026-11-08):

- `rust/view/csharp-inventory` and the generated [client-inventory.md](client-inventory.md): 231 client files, 1268 `Shared` members, by area, by type, and the action-form helper table with dependent helpers marked.
- The visibility audit, in protocol.md and above.
- The Game view section of protocol.md.
- `rust/view`: schema, message rendering, the view builder (header, players, own shield, board basics, status, log, actions), option builders for `AllianceOffered`, `AlliancePassed`, `AllianceBroken` and `StormDialled`, draft queries, the replica with `scene()`. Tests (`tests/slice.rs`) on the alliance fixtures and a storm-dial state cut from one: options equal the engine helpers, the observer and allied cases, private log lines over every report of four games, own secrets only in `me`, draft queries, the replica following events, the scene seating. `tests/ts_bindings.rs` keeps `web/src/generated` current.
- `web/`: Vite + React + TypeScript skeleton that renders the side column of a saved view with the C# client's CSS (`npm run build` passes; `npx vite`, then `/scene/alliance-allied/BeneGesserit`).

Next: package 2 (packages 1 and 3 are done, below).

Package 1, view core (2026-11-09): done. Every field of the inventory's "Board", "Your faction panel" and "Prompt" areas maps to a view field or to static data; the mapping is below. What was added:

- `GameView`: `maximumPlayers`, `ruleset`, `rules`, `rulesForBots`, `economicsStatus`, `piles`, `tanks`, `bidding` (what the map shows; 7a adds the panels' data), `battle` (the plans as `BattlePlan.razor` draws them; 7b adds the forms' data), `notes`, `timing`. `PublicPlayer`, `PrivatePlayer`, `BoardView` and `StatusView` are complete. protocol.md lists every field.
- `hand[].playableWith` is a list: a Karma card can play several actions at once (`Karma` and `KarmaPrescience`, for example). An action listed for some card is played from the hand, so the action column leaves it out (`CardPlays.PlayedFromHand`).
- The prompt's "Continue" is the `EndPhase` action, with a `Simple` form and its validation.
- Static data (`src/static_data.rs`, written to `web/public/static/data.json` by `examples/static_data.rs`, typed by the generated `StaticData`): locations with territory, sector and kind; territory flags and advantages; leaders' factions; card types; resource cards by `SkinId`; homeworlds; each faction's force types; rule groups; the places `Map` names. `enums.ts` gained the enums these use, and `PhaseValue`/`MainPhaseValue` for the C# client's phase comparisons.
- The replica's `timer` carries `Client.Timer` (set by the hub's `SetTimer`) into `timing.timer`. Package 2 sets it.
- Code: `src/board.rs` (map, piles, tanks, bids, plans, flash, timing), `src/shield.rs` (faction panel, notes, card plays).

Two choices the C# client doesn't make the same way:

- `me.traitors` is empty while the player is still picking a traitor. The C# panel hides them too (`PlayerPanel.StillPickingTraitor`), and the candidates are in the `TraitorsSelected` form.
- Partial prescience goes to the prescience user and their ally only. This is deviation 1 in protocol.md.

Tests:

- `tests/view_core.rs` tests each field against the engine and against the C# component's gate. It uses the fixtures and, for the expansions the fixtures lack (terror, discoveries, ambassadors, the Hidden Mobile Stronghold, No-Field, Ecaz, Moritani, silent auctions), the first state of a sample bot game that has them. `tests/data` holds three small bot traces (seed 3141, games 2, 103 and 114; the last two cut short). They reach an identifiable discovery, an Ecaz loyal leader with `SetShipmentPermission`, and Moritani terror on the planet with `LoserConcluded`, which `rust/traces/sample` doesn't. (Game 9, a silent auction at version 107, was replaced when old versions were dropped; the sample now has silent bids.)
- `tests/corpus.rs` runs over every bot trace. After every accepted event it builds the view of every player and an observer on two replicas, in different orders, and requires equal views. It also checks every view against the visibility rules, derived from the engine state. With `VIEW_CORPUS_DIR` it runs on any directory of bot traces. It passed on 40 fresh games (`Treachery.Oracle bots --seed 3141 --games 40`, 25,190 steps), so one replica per game is enough.
- `tests/static_data.rs` keeps `data.json` current and equal to the engine's tables.

Inventory to view, for the three areas (static members of `Map`, `Location`, `Territory`, `Leader`, `TreacheryCard`, `Homeworld` and `ResourceCard` are in the static data, `Skin.*` is the skin JSON, and `PointD`/`Utilities` are client geometry):

| C# | View |
|---|---|
| `Game.ForcesOnPlanetExcludingEmptyLocations`, `BattalionsIn`, `AnyForcesIn`, `ThreatenedByAllyPresence`, `Player.ForcesOnPlanet` | `board.forces` |
| `StrongholdOwnership`, `TerrorIn`, `AmbassadorIn`, `DiscoveriesOnPlanet`, `Discovery.*`, `Location.Visible`/`Sector` of attached locations | `board.strongholds`, `terror`, `ambassadors`, `discoveries`, `attached` |
| `LeaderState` (`Alive`, `CurrentTerritory`), `Monsters`, `NumberOfMonsters`, `IsSpecialStronghold`, `AtomicsAftermath`, `IsProtectedFromStorm`, `CurrentNoFieldValue`, `HasStormPrescience` + `NextStormMoves` + `NextStormWillPassOver`, `RecentMoves` + `FindFirstShortestPath` | `board.*` |
| `GameStatus.HighlightedTerritories`, `PlayerShouldBeHighlighted` | `board.highlightedTerritories`, `status.waitingFor` |
| `Deck.*` of the decks and discard piles, `LatestSpiceCardA/B`, `SandTrout(Occured)`, `RicheseCache`, `HasResourceDeckPrescience`, `KnownCards`, `AssistedNoteKeepingEnabled` | `piles` |
| `Player.ForcesKilled`, `SpecialForcesKilled`, `Leaders`, `LeaderState.IsFaceDownDead`/`TimeOfDeath`, `IsAlive`, `MessiahIsAlive` | `tanks` |
| `Bids`, `IBid.*`, `CurrentAuctionType`, `HasRoomForCards` | `bidding.bids`, `players[].hasRoomForCards` |
| `CurrentBattle`, `AggressorPlan`/`DefenderPlan`, `Battle.*`, `AtreidesKarma`, `CurrentPrescience`, `CurrentNexusPrescience`, traitor actions, `BattleWinner`, Ecaz's contribution and bonus, `GetSkill`, `Skill`, `HasStrongholdAdvantage`, homeworld bonus | `battle` |
| `Player.HomeWorlds`, `IsOccupied`, `OccupierOf`, thresholds, `GetSkilledLeaders` + `IsInFrontOfShield`, `EcazLoyalLeader`, `LatestRevealedNoFieldValue`, `TechTokens`, `Ambassadors`, spice to collect, `Seat`, `MaximumPlayers` | `players[]`, `maximumPlayers` |
| `CurrentTurn`, `MaximumTurns`, `Rules`, `Ruleset`, `Version`, `Started`, `Duration`, `TimeSpent`, `History.Last().Time`, `EconomicsStatus` | header, `timing`, `economicsStatus` |
| `PlayerPanel`: `Resources`, the spice texts' members, reserves, `HasSpecialForces`, `Leaders`, `IsSkilled`, `Traitors`, `FaceDancers`, revealed sets, `HasActedOrPassed`, `PredictedFaction/Turn`, `MessiahAvailable`, `TotalForcesKilledInBattle`, `UnplacedTerrorTokens`, `TreacheryCards` + `CardPlays`, `GetCardSetAsideForBid`, `Nexus` | `me` |
| `NoteComponent`: `CardsOwnedBy`, `Player.KnownCards`, `NumberOfVictoryPoints`, revealed traitors and face dancers | `notes` |
| `CardPickerComponent`: `CardJustWon` known to Atreides | `me.cardJustWon` |
| `GameStatusComponent`: `Bid.PlayersToBid`, `BlackMarketBid.PlayersToBid`, `CardsOwnedBy`, `IsBot`, `IsPlaying`, `FlashInfo`, `TimedEvent`, `WaitingForMe`, `WaitingForPlayers`, `WaitingInSequence`, `GetMessage` | `status`, `players[].cardsOwned`, `isBot` |

Next for the view: packages 2 and 3, then 7a–7e. The `bidding` and `battle` fields grow with 7a and 7b.

Package 3, rich text and skin (2026-11-10): done. Every log line of the fixtures renders the same text as C#.

- `rust/view/src/describe.rs`: `Describer` for `treachery_skin::Skin`, and `InGame` (skin plus game), which `build_view` and draft queries use, so live collections in old messages are read from the game and leader parts carry `Game.Skill`. `MemberNames` is gone.
- `rust/view/src/expr.rs` follows `ExpressionComponent`'s switch case by case (a `long` is text, floats are rounded like `Support.Round`, leaders carry `skill`).
- `rust/view/src/log.rs` (moved from build.rs): `log_view`, `report_lines`, `log_line`, and `LogLine.record`/`route`/`plans`/`fact` as typed records for all 41 record types of `Shared/Model/Log/` plus `Route`, `BattlePlans`, `BattleFact` (protocol.md, Rich text).
- `rust/view/csharp-ref` (`ViewRef`): renders every message of every fixture (8,328) and two of each shape of the bot-game sample (235) with the C# engine and `DefaultSkin`. `tests/log_lines.rs` compares parts, target, heading and all four records with the view; another test fails when a C# log record has no Rust conversion. `tests/data/log` holds two bot games for `WormSentLog` and `ShipMoveLoss`.
- `web/`: `RichText.tsx` (ExpressionComponent) with `SimpleNumber`, `SpiceComponent`, `MonsterIcon`, `PlaceChip`, `PopoverText` (RichText.razor) and `Image`; `popups.ts` (popover HTML, place chip and its geometry), `text.ts` (HtmlEncode, `Support.Round`, the spice regex), `popover.ts` (Bootstrap popovers with the C# options). `index.html` gets `<base href="/">` and the C# Bootstrap bundle, as the C# page has. `npm run check:rich-text` renders all 8,563 golden lines and compares their text with the C# component's.

Not covered or known differences:

- Four record types occur in neither the fixtures nor the bot games checked (`BiddingSkipped`, `BiddingPassedByAll`, `BlowReshuffleLog`, `FamilyAtomicsKilled`), and neither do `messiah`, `resourceCard`, `bool` and `long` parts. Their conversion is a field-by-field mapping, untested against C#. A fixture or bot game that reaches them only needs adding to the C# reference's inputs.
- `TextBorder`: C# doesn't pass it to the parts of a nested `Message` or `MessagePart` (lists keep it). The view flattens nested messages, so `FlashView.bordered` marks per part whether the border applies, and `RichText` takes it as `bordered` (see "Fixes after package 3 and 7c/8"). Only `ImageFlashComponent` and the Cyan terror tokens use a border.
- Popover HTML is checked by reading against `rust/skin` (itself byte-checked), not by a test. Package 10's popover comparison (hover targets in both clients) is the place to check it. `WedgePath` uses JavaScript's `Math.cos`, which may differ from .NET in the last digit of a path coordinate.
- The phase log designs (`OtherComponents/Log`) are package 6; `LogPanel.tsx` still draws plain lines.

Package 10, comparison harness (2026-11-09): done as described in [Harness layout](#harness-layout). Remaining:

- Popovers are not compared yet (hover each chip, card and territory). Add this once the web client has popovers (package 3). Hovering needs a common way to find hover targets in both clients.
- The `story-hide-secrets` mask only matters once a comparison opens the story modal. `comparisons/story.py` (package 9) does.
- Fixtures at game version 187 (`alliance-before`, `alliance-allied-before`) can't be compared. The C# scene page refuses them, and the report lists them.

Package 2, server glue (2026-11-10), with the server's package T (the server plan of the time, since replaced by [server.md](server.md)):

- Server: `server/src/views.rs` keeps a `ViewReplica` per game with connected members and sends each member their `GameView` after events, undo/restore/dismiss/load, participation changes, `SetSkin` and `SetTimer`, and the caller theirs after create, join, observe, reconnect and `RequestGameState`. `QueryDraft`, `RequestReport` and `RequestUndoList` are answered from the replica. `server/src/dev.rs` is `POST /dev/scene`.
- `view/src/replica.rs` gained `report`, `undo_list` (`UndoEntry`), `set_skin` and `has_view_for`.
- `web/src/state/connection.ts`: the protocol client (`Connection`: calls, notifications, reconnecting; `hashPassword` as `Support.GetHash`; `login`). `web/src/state/source.ts`: `WebSocketSource` (views by subscription, draft queries, submissions with `Initiator` set to the viewer's faction, reports, undo list, heartbeats). `ViewSource` has an optional `subscribe`.
- Routes: `/scene/:fixture/:faction` opens the scene on the server when `POST /dev/scene` answers, and falls back to the saved view otherwise, so `just compare` keeps working without a server. `/:userToken/:gameId` reconnects to a game. Vite's dev server proxies `/ws` and `/dev` to `TREACHERY_SERVER` (default `http://localhost:5000`).
- Forms send events in the savegame format: enums as numbers (`ts_bindings.rs` writes `FactionValue` beside `PhaseValue`). `WebSocketSource` sets `Initiator` to the viewer's faction on every draft and submission, whatever the form put there; members the engine doesn't know (a form's `eventNumber`) are ignored by the server.
- End to end: `python3 scripts/ui/e2e_server.py` starts the release server on a fresh database with the web build, logs two users in through the client's connection code (there are no lobby screens yet, package 8), creates a game from the alliance fixture, seats both, plays alliance events by clicking in two browsers and checks that both screens update, then plays the scene route on the server. It opens a scene before any other client too, and checks that the scene user sees the Emperor's screen and can pass.
- `POST /dev/scene` calls `Connect` before anything else, as a client does (fixed 2026-11-10). Without it, the first client `Connect` on a fresh server ran `RestoreGamesAndUserCache` after the scene existed, and that replaced the scene game with the copy `RequestCreateGame` had saved before the seat was assigned (`RequestAssignSeats` saves only every few minutes), so the scene user was unseated. `server/tests/dev_scene.rs` opens scenes as the web client does (`Connect`, `GetLoginInfo`, `RequestReconnectGame`) on a fresh hub and checks the faction's view and actions.

Package 7e, other events (2026-11-10): done. Options, tests and React forms exist for all 47 components, and the prompt matches C# in every fixture.

Done (Rust options, default draft, `normalize`, validation, React form):

- Own components (43): AllyPermission, AutomationConfigured, BeneGesseritPrediction, CardTraded, ChoamDiscarded, ChoamEconomics, ChoamKarmaPrevention, DealOffered, Discarded, DiscardedSearched, DiscardedTaken, DistransUsed, DivideResources, Donated, EmperorBidSupport, EmperorDiscarded, FactionSelected, FactionTradeOffered, IxianRemovedCardFromAuction, IxianSelectedStartingCard, IxianSwappedCardOnBid, Karma, KarmaChoamDiscard, KarmaHandSwap, KarmaHandSwapInitiated, KarmaMonster, KarmaRicheseBuy, MulliganPerformed, NexusCardDrawn, NexusPlayed, PlayerReplaced, ReplacedCardWon, ResourcesAudited, ResourcesTransferred, SkillAssigned, StormSpellPlayed, TraitorDiscarded, TraitorsSelected, RicheseAnnouncesAuction, RicheseAnnouncesBlackMarket, RicheseGaveCard, RicheseKeepsUnsoldCard, RicheseSpecifiesAuction. AllianceOffered and StormDialled (in `actions.rs`) got proper React forms; AllianceOffered's default draft now starts at the first target, as the required image choice does.
- `SimpleActionComponent` for the 20 event types in `other::SIMPLE` plus AlliancePassed/AllianceBroken (`actions.rs`); `YesNoActionComponent` for the 8 in `other::YES_NO`. `Simple` gained `dismissLabel` (HideSecrets' "No") and `allowAutoConfirm` (GuildDelay's box). A battle-package builder that handles one of these first (BattleRevision, KarmaPrescience, PoisonToothCancelled, PortableAntidoteUsed, BattleClaimed, Captured) wins, because `battle::` is dispatched before `other::`.

Where things are:

- `rust/view/src/schema/other.rs`: payloads. Several variants share one payload (`CardChoiceOptions`, `FactionChoiceOptions`, `HeroChoiceOptions`, `DraftOptions`, `MayOptions`). Every payload carries `draft`, the event after `normalize`, as in 7d.
- `rust/view/src/actions/other.rs`: `default_draft` (field initializers), `normalize` (required image/select/radio choices fall back to the first value, number pickers clamp, hidden widgets reset, `Donated.FromBank` only for hosts), `options_and_validation` (validation is of the normalized draft). Helpers `picker`, `required`, `faction_options`, `hero_options`, `card_ids`, `draft_json`, `describe`, `format`, `enum_option` are `pub(super)` for reuse. `actions/other_nexus.rs`: `NexusPlayedComponent`'s switch as `Case`, the map choice, the per-case text as rich text.
- `rust/view/tests/other_forms.rs`: every fixture state and every state of the sample bot and fuzz traces (fuzz up to the first undo), every player: options equal the engine helpers, the draft parses as the form's event, validation is the draft's; fixtures: a query of each form's own draft returns the same options and validation; dependent inputs (Donated from the bank, required fallback and clamping). Passes.
- `web/src/components/other/`: `parts.tsx` (`Layout` = GameEventComponentLayout with optional header and `startCollapsed`; `useOtherDraft`; `Radio`, `Select` (listbox popover), `Segmented`, `SelectNumber`, `SelectManyFromImage`, `CommandButton`, `cardChoices`, `factionChoices`, `idList`/`idString` for `_cardIds`), `ChoiceForms.tsx`, `TradeForms.tsx`, `MapForms.tsx`, `SimpleForms.tsx`, `OtherForms.tsx` (`otherForm(action, inPopover)` and `shownInActionPanel`). `ActionPanel.tsx` sends every form but the bidding panel to `otherForm`, leaves out cards played from the hand and `Trade.Actions`, and draws PlayerReplaced last. `SelectFromImage` gained an optional `tooltip` per choice (factions' `title`).
- Comparison: `just compare mc-prediction mc-discard mc-traitors mc-charity alliance-allied` matches in every region at both sizes (35 scenes).

Second session (2026-11-10):

- Prompt comparison over every fixture (one `compare.py` process per fixture: the harness's own process grows by about 200 MB per scene, so a whole run goes over `heavy.sh`'s 3 GB cap): the prompt matches in every scene at both sizes, except the two version-187 fixtures that the C# scene page refuses (known). No prompt difference belongs to 7a–7d at the time of the run. Fixed here: Nexus case text is written as bare text between chips (RichText wrapped it in spans), and headers and labels that interpolate a skin name are JSX fragments, so they split into text nodes as the razor does (a single template string shaped a few glyphs a subpixel apart).
- New fixtures: `karma-monster` (Fremen may play KarmaMonster) and `nexus-moritani` (Harkonnen holds the Moritani Nexus card as betrayal, Carthag and Sietch Tabr hold terror). `make-fixture.cs` gained the `can` checks `nexus-ecaz-betrayal`, `nexus-moritani-betrayal`, `nexus-tleilaxu-location` and the spec key `ally` (two factions offer each other an alliance whenever they may). Scripted comparison `comparisons/map-picks.py`: the Karama card's panel, the KarmaMonster pick on Funeral Plain and the Moritani pick on Carthag (pinned panel with the confirm button).
- Not reached by bots: Ecaz betrayal (1,600 seeds, and 600 more with Ecaz forced into an alliance: in 40 seeds no other player ever held the Ecaz Nexus card while it applied) and the Tleilaxu secret-ally location (a Fremen at high threshold holding the Tleilaxu card in Resurrection; 2,000 seeds). Those two map cases are checked only by construction.
- `other_forms.rs` reads `tests/data/other`: 15 bot and fuzz games (oracle bots seed 7702, fuzz seeds 7701 and 7703) cut after the state that reaches a form nothing else reaches, 112 KB. The corpus test requires every form and Nexus case they reach. `OTHER_FORMS_CORPUS_DIR=<dir>` runs the test over a fresh oracle run instead and prints each trace's first state per form, which is how these were picked (60 bot and 279 fuzz games passed). `dependent_inputs` also checks both map picks by query.
- Reached nowhere (checked only by construction): ResourcesAudited, ResourcesTransferred, KarmaEcazDial, CardGiven, and the Ecaz-betrayal and Tleilaxu-location map cases above.
- The trade forms and card plays are drawn through `otherForm` by package 5's popovers (`CardPlayPopover` renders `ActionPanel` with a filter), so item 2 of the handoff is done.

Remaining:

1. Fixed (2026-11-10): `POST /dev/scene` left the scene user unseated (`SeatedPlayers {"1": -1}`) on the first scene of a fresh server; see the package 2 notes. Map picks can now be confirmed in a browser against the server.
2. `DealOfferedForm` keeps "they pay"/amount in component state and writes `Price`/`Benefit`; the "Until" control assumes the server's `endPhases` per deal type. Verify against C# when a scene shows the Deals popover.

Package 7a, bidding (2026-11-10): done.

- `rust/view/src/actions/bidding.rs` builds `NormalBiddingPanel` (`Bid`) and `BlackMarketBiddingPanel` (`BlackMarketBid`). It covers normal, Richese once-around and silent, and black-market auctions. It provides the options, the default draft (`BiddingPanel.OnParametersSetAsync` after the pickers clamp it) and the lot that every player sees (`bidding.panel`). Only `maxAmount` depends on the draft (the Karma card).
- Observers: in C#, `HasBiddingPrescience(null)` throws for them, and the C# client stops rendering ("An unhandled error has occurred"; see `design/compare/bid/Observer`). This settles open question 8. Since 2026-11-11 observers get the lot a player without prescience sees, in normal and black-market auctions: the card back (the card itself only in Richese's auctions), "Card n of m" and the current bid, with no inputs or auto-pass (protocol.md, deviation 5).
- Web: `BiddingPanel.tsx` (the lot, inputs, Pass / Win immediately / Bid, auto-pass with threshold and repeat, the bidding-timer pass), plus generic `NumberPicker` (`SelectResources`, `SelectResourcesFromFaction`), `ButtonRow`, `Checkbox` and `SelectFromImage`. `Image` gained `pointer` and `highlightOnHover`.
- Tests: `rust/view/tests/bidding.rs` checks every state of every fixture and sample bot trace. The options must equal the engine helpers, the validation must be that of the default draft, the default bid must be valid when affordable, and the lot must show the card exactly to the players with prescience or in Richese auctions. Over 8,000 bidder states were checked, covering normal, Richese once-around and silent, and black-market once-around and silent auctions. No state has Emperor paying or a black-market normal auction. Scripted comparison `scripts/ui/comparisons/bid-inputs.py`.
- Comparison: `prompt` matches in all `bid` scenes and both `bid-inputs` steps. No fixture is in a black-market, Richese or silent auction. A fixture for each would make the screenshots cover them, but every package's `--require` would then run on them too, so that is the coordinator's call.

Package 4, board (2026-11-10):

- `web/src/components/board/`: one component per `MapComponents` razor file, keeping their elements, classes and inline styles (`GameBoard`, `Planet` with `MapHover` and `RecentBlows`, `Pieces` = `ResourcesOnPlanet`, `Monsters`, `SSW`, `ForcesOnPlanet`, `ForcesInLocation`; `Phases` with `CardPiles`; `Players` with `Bids`; `Tanks`; `BattlePlans`; `RecentMoves` with `Arrow`; `MapSelection` = `MapSelectionHighlights`, `MapSelectionOverlay`, `StagedForces`; `primitives` = `MapImage`, `MapText`, `Circle`, `Rectangle`, `MapPopoverArea`, `MapForce`, `MapLeader`). `geometry.ts` is the board model (view + skin + static data) with `Skin.GetCenter`, `GetSpiceLocation` and `SectorGeometry`; C# float arithmetic is kept with `Math.fround` where a cast to int follows.
- `mapScripts.ts` ports the board's scripts from `wwwroot/index.html`: `SetPlanetMapScale` (run after every render and on resize), the hover shape, the cursor arrows, Escape/right-click cancel, `ShowMapRefs` (place chips light their sectors) and `ClampMapPin`.
- Popovers are Bootstrap's, through the shared `usePopover` (`board/popover.ts` adds the map's attributes and `PopoverTemplate`). Markup: `src/popups.ts` (package 3) plus `board/popups.ts` and `board/intel.ts` (territory intel, stronghold advantage, homeworld card, discard piles, Richese cache, tanks, turn). Not yet complete: the faction popover on a seat has no faction sheet (the skin JSON lacks `GetFactionSheet`/`GetFactionInfo_HTML`), and the turn popover prints rule, rule group and ruleset member names (no TS port of those `Describe` tables).
- Map interaction (`board/mapInteraction.ts`): `MapInteraction`/`MapPicker`/`Client.LocationClick` as a store. Forms use `useMapPicker` (MapPickComponent) or `mapInteraction` + `onLocationClick` (staged placements). `refresh()` only notifies when the map state changed, because forms push after every render and redraw on changes; the pinned panel has its own channel (`usePin`).
- Also added because they sit in the board's region: `components/MainLayout.tsx` (`Pages/MainLayout.razor`'s `.page > .main > .content` shell, which sets the columns' offsets; used by both game routes in `main.tsx`) and `components/ImageFlash.tsx` (`ImageFlashComponent`, drawn over the board after events that flash cards). `scene.ts` gained `registerSceneBoard`: `treacheryScene.locations` and `click` now work in the web client.
- Not ported: `Disconnection.razor` (the connection has no status for it yet).

Board comparison (2026-11-10, 48 fixtures, both viewports, 625 comparisons that both clients load): 621 match. The board is the same as C# in the other four too; what differs comes from outside it:
- `partial-prescience` scenes (`nexus-prescience/*`, `prescience-seen/*`; the remaining 18): fixed in the harness. Masked elements are now left out of both digests, and these scenes match.
- `bid/Observer` (desktop): the C# page shows "An unhandled error has occurred" (open question 8, `HasBiddingPrescience(null)` throws), which covers the board's bottom edge. Now masked as the expected difference `observer-bidding-error` (protocol.md, deviation 4), and the scene matches. Since observers get a bidding panel (deviation 5), the mask also takes the web panel out of the layout before the shot.
- `mc-treachery/Emperor`: the web client shows the turn banner and C# doesn't, so the board is 40px shorter (the turn banner's owner).
- The game version 187 fixtures can't be compared (C# refuses them).

Package 5, side column (2026-11-10): handoff, not done. Second agent's additions are under "Second pass" below.

Done (typechecks, workspace compiles):

- View (`rust/view`): `src/side.rs` builds `deals` (acceptable, cancellable and current deals, with the Accept/Decline/Withdraw events in savegame JSON and `Acceptance().Validate()`; an offer's `id` is its event number), `botsSpeed` and `tracker` (Atreides from `SelectingTraitors`: cards in play, `TraitorsInPlay`, per-player counts, resource deck as a sorted set and discard piles). `OwnLeader` and `HeroCard` gained `skill` (popovers). `replica.undo_list` entries gained `listed`, `turn`, `phase` (`UndoSession.IsListed`, `MomentOf`). protocol.md documents all of it. Tests: `tests/side.rs` (second pass).
- `examples/scene_views.rs`: applies the pass `TreacheryCalledComponent` submits by itself for a player who can't call treachery, as the C# scene page executes it (fixed mc-treachery/Emperor's turn banner and board height). The Rust server's dev scene gets the same from the web form's own auto-pass (package 7b).
- Web: `TurnBanner` (+ `--turn-banner-h` on `.game-screen`), `turnClock.ts`, `PlayerPanel` (all extras: storm prescience, prediction, Messiah, Ecaz ambassadors, Moritani terror, resource deck top, playable cards, set-aside card, Nexus), `GameStatusComponent` (timer, bidders with card counts, worm, sequence, message, Continue, the phase pause from the lobby client's `PhaseControl`, host bidding time limit), `MenuComponent` (toolbar incl. deal count, ally and card tracker buttons; menu; Load game / Apply skin via `RequestLoadGame`/`RequestSetSkin`; bot speed), `MenuPanels`, `menuState.ts`, `TradePopover`, `DealComponent`, `DealToasts`, `NoteComponent`, `TrackerComponent` (+ card picker; notes in local storage keyed by `timing.started` instead of `Game.Seed`), `UndoComponent`, `UndoTimeline`, `undoSession.ts`, `JoinedPlayersPanel` (fallback when there is no lobby client; inside the lobby the lobby's panel is used), `ChatComponent` (props-driven; not mounted: Main.razor's owner, package 8, has the messages and user list). Widgets: `ForceToken`, `FactionComponent`, `NumberComponent`, `SpiceLine`, `CollapseButton`, `Hoverable`, `PickFromImageComponent`; `Image` gained state classes and `Image.css`; `SimpleNumber` now scopes its CSS by a data attribute (the CSS-module class showed up in every DOM digest). `ViewSource` gained optional `undoList` and `gameCall(method, ...args)` (implemented by `WebSocketSource`).

Verified: subset comparison (bid, ally, mc-discard, nexus-prescience-base; 27 scenes, both viewports): `turn-banner` 27/27, `player-panel` 24/27 where the 3 differences were only the SimpleNumber class (fixed since, pixels were equal), `prompt` matched except where an action form is missing (package 7).

Not verified: a full run. Two full runs (`--workers 4`, then `--workers 2`) were killed by heavy.sh's 3 GB cap partway (the second during `ally`), and another agent's `just compare <subset>` replaced the views mid-run. Both are fixed in the harness (package 10, 2026-11-10): compare.py no longer runs on Python 3.14, whose asyncio kept every Playwright call's stack alive, and each run uses its own build snapshot. A private work dir such as `/tmp/side_dump/cmp.py` is no longer needed.

Left:

- `--require side-column` can't pass until the log (package 6) and action forms (package 7) match; the side column contains both.
- Deal toasts tell a withdrawn offer from an answered one only for answers made in this client.
- Notes' revealed traitors show no skill (as C#, which passes no Game there).

Second pass (2026-11-10):

- Tests, `rust/view/tests/side.rs`: `deals` over every state of blow, ally, alliance-allied and ship-guild (observers get none; acceptable = `AcceptableDeals` and only offers addressed to the viewer; own offers = `CancellableDeals` with `DealResponse` per recipient, and no answers on anyone else's offers; current deals equal for everyone; the Accept/Decline/Withdraw JSON parses as the engine's events for the viewer; Accept disabled exactly when `Acceptance().Validate()` fails). `tracker` (Atreides only, from `SelectingTraitors`; contents equal the engine's; deck sorted). `botsSpeed`, `mentat`, leader `skill` on own leaders, traitors and face dancers (a skill is public once assigned: `SkillAssigned` logs it to everyone, so the traitor popover's skill is not a leak). Undo list `listed`/`turn`/`phase` (`IsListed`, `Moments[n-1]`). The undo preview equals a replica loaded from the savegame cut at the gap, minus actions, for every viewer, and leaves the replica alone.
- Host save game: `RequestSaveGame(userToken, gameId)`, a view call (`server/src/views.rs`), for hosts and the admin, returns `GameState.GetStateAsString` of the server's game. `MenuComponent` downloads it as `savegame.json`; the admin is recognised from the lobby's `ServerInfo.AdminName`.
- Undo preview: `RequestUndoPreview(userToken, gameId, gap)`, host only, returns the caller's view at the gap with no actions and no `playableWith` (C# showed the preview with `Actions` empty). `ViewReplica::preview` replays a copy of the history into a new game. `undoSession.ts` asks for it on every move of the timeline; `GameScreen` puts the preview in the game context, so board, panels and log show the earlier moment. Server test in `server/tests/transport_views.rs`; protocol.md documents both calls.
- "List of Cards" (`MentatComponent.tsx`, data from the new `mentat` view field) and "Credits" (`CreditComponent.tsx`) in the rules entry, as Bootstrap modals after the menu bar, as in C#.
- Music: `MusicComponent.tsx` and `src/sound.ts` port `MusicComponent.razor` and the `Browser` sound functions (soundtrack per main phase, volume curve, fade-out, settings under the C# local-storage keys). The effect volume is stored in `soundSettings.effect`, but nothing plays sound effects yet: C# plays `Skin.GetSound(milestone)` for `Game.RecentMilestones` after each event (`Client.PlaySoundsForMilestones`) and the your-turn and chat sounds; the view has no milestones. That needs a `recentMilestones` view field and a hook in the game screen. Unlike C#, leaving the game screen stops the music.
- Comparison (2026-11-10, subset bid, ally, blow, retreat, facedancer, mc-discard, mc-treachery, nexus-prescience-base; 55 scenes, both viewports): turn-banner 55/55, prompt 55/55, player-panel 54/55, side-column 43/55. Every side-column difference comes with a log difference of the same scene (ally, facedancer, mc-treachery: package 6). The player-panel one, mc-treachery/Guild desktop, is resampling noise in one leader portrait (same image and DOM, mean colour equal; that leader is also drawn large in the battle plan), not a layout difference. New scripted comparison `scripts/ui/comparisons/side-menu.py` (Deals with its Offer a Deal and Give Spice tabs, card tracker, Automation, the menu with rules and sound open, List of Cards): 7/7 match after adding `DealComponent.css` (the C# component's scoped CSS, which the web lacked: deal answers were unstyled). A full `--require side-column` run is still to do, and can't pass before package 6.
- Forms in panels: `TradePopover` draws DealOffered, Donated and AllyPermission and `MenuPanels` draws AutomationConfigured through `ActionPanel`'s `filter` (7e's forms, `inPopover` as C# cascades it). `CardPlayPopover` already existed (another agent) and matches the C# component.


Package 6, log (2026-11-10):

- `web/src/components/LogPanel.tsx` is `ReportComponent`: the stepper (arrows, turn and phase, the turn's dots), the battle reports (faceoff, revealed plans with verdict and aftermath, `OutcomeFacts`), "Nothing to report", and the plain line. `web/src/components/log/` has the eight phase logs (`StormLog`, `BlowLog`, `CharityLog`, `BiddingLog`, `RevivalLog`, `ShipMoveLog`, `CollectionLog`, `MentatLog`), each following its razor file, and `common.tsx` (`Line`, `Plan`, asides with faction advantages, route hover attributes, part identity in place of C# reference equality). `web/src/followLog.ts` ports `follow-log.js`.
- History: `ViewSource.report(index)` (`RequestReport`). The panel caches answers per source, keyed by report index and the view's `eventNumber`. When a newer view arrives, it asks again and shows the cached lines meanwhile, because old lines can change (hazards 4.5) and undo can replace a report. It also asks for the phase before the one shown, so stepping back is immediate. Without a server, `StaticSource` reads `<view>.reports.json`, which `scene_views.rs` now writes next to every view. Checked against the Rust server by stepping back 12 phases of `/scene/blow/Fremen` opened through `POST /dev/scene`.
- `web/index.html` loads the C# client's `wwwroot/js/tooltip.js`, as the C# page does. A hovered element's `title` moves to `data-tip`, so after a click the C# DOM digest has no `title` on the hovered button. Without the script, every scripted step differed.
- `RichText.tsx` (package 3): the bold enum set named `BrownEconomicsStatus`, while the part's type is `ChoamEconomicsStatus`, so "play Inflation: **Double**" wasn't bold. `npm run check:rich-text` compares text only, so it couldn't see this.
- Comparison: `scripts/ui/comparisons/log-history.py` steps back through every phase of four games (tanks-face-down, blow, mc-discard, worm-ride-reserves) and compares each step. Once, outside the repo, the same walk ran over the longest fixture of every other distinct game.
- `followLog.ts` follows more reliably than `follow-log.js`. It also scrolls when the panel or a line changes size (ResizeObserver), and it ignores scroll events that find the panel where it last put it. The C# script stops following when a line grows between its scroll and that scroll's event. So where the C# log ends up depends on timing. For the same scene, the C# panel stopped at 306, 383 and 330 px out of 384 in three runs (`ally/Guild`); the web one is always at the bottom.
- The alliance log after two players ally on the server shows the Nexus aside with two chips and the "allied with" glyph, with no "are now allies" text. That is `BlowLog.razor`'s design: in C# the Nexus aside draws `NexusLog` alliance changes the same way. The steps with alliances in `log-history` (tanks-face-down, blow, mc-discard, worm-ride-reserves) match C# in DOM and pixels.

Comparison status (2026-11-11). The 369 fixture scenes, with `--require log`: every phone scene matches, and 26 desktop scenes differ. All 26 have an equal DOM; only the pixels differ.
- 23 are the C# scroll race above (bene_gesserit, facedancer, move-within, ship, ship-fremen, prescience-*, nexus-prescience-base, mc-concluded, mc-treachery, and some face-danced). Scrolled to the bottom in both clients before the shot, all 23 match. That run wrapped `capture` in a private copy of the harness. To fix this, `compare.py`'s `capture` should set every `.report .card-body` to `scrollTop = scrollHeight` before the screenshot. That is what a player at the bottom sees, and it removes the C# timing. This needs the harness owner.
- 3 are `face-danced/{Atreides,Emperor,Guild}` at about 0.1%: the battle plan's leader discs are resampled slightly differently in the C# shot. DOM, sizes and positions are equal. Not resolved.
- The two game version 187 fixtures can't be compared (C# refuses them).
- `log-history` matches C# in the log at all 224 steps (2026-11-11). Outside the repo, the same walk also ran over ally, battle, battle-special, bene-gesserit-accompanies, bene_gesserit, bid and blow-two-decks: 110 steps, all matching. The walks over the other distinct games (facedancer, karma-prescience, mc-handswap, mc-harvester, mc-thumper, move, move-reinforce, nexus-prescience, retreat, revival, ship, ship-fremen, ship-guild, worm, worm-ride, worm-ride-ssw, mc-ended) were killed by the harness's old memory leak and haven't been rerun.


Restored (2026-11-10, package 7b's second agent): the sections for packages 9, 7d, 7b, 7c and 8 below went missing from this file when another agent wrote it back from an older copy. They are restored as they stood, except package 7b, which is updated.

Package 9, end of game (2026-11-10):

- `view/src/ending.rs`: the `ending` field (protocol.md). In `Phase.GameEnded`, the winners and, unless `DisableEndOfGameReport` applies, the story: the history replayed in a fresh game as `StoryComponent` does, recorded as blocks in drawing order. The story is the same for every viewer. Two places where the C# story reads the end of the game rather than the moment, because its tables and messages are child components that render after the loop, on shallow clones: discarded traitors (`Player.Clone` shares `DiscardedTraitors`) and the live lists in milestone messages. Both are reproduced. Not reproduced: `HomeworldOccupation`, which the clone also shares, in the points column (homeworld games only).
- `HideSecrets` (deviation 2): the view asks the finished game's `SecretsRemainHidden`, so a faction that hid its secrets has no cards (held or bought), traitors, face dancers, spice, Nexus card or prediction in the story; the cells read "?".
- Web: `WinnerPanel.tsx`, `StoryComponent.tsx` (with the player-info and battle tables), `ModalComponent.tsx` (Bootstrap's data API opens it, as in C#). `ActionPanel.tsx` draws the winner panel first.
- Harness: regions `winner` and `story` in `compare.py`; scripted comparison `comparisons/story.py` opens the story on `mc-ended` (compared whole) and on `mc-ended-hidden` (the `story-hide-secrets` mask covers the body). `scripts/fixtures/mc-ended-hidden.json` is `mc-ended.json` with HideSecrets by Guild and Bene Gesserit appended by hand: the event is applicable only up to version 123, so no spec makes it.
- Tests: `view/tests/ending.rs`.

Package 7d, revival (2026-11-10): the eight revival forms.

- `view/src/actions/revival.rs` builds the options, default drafts and validation for `Revival`, `RaiseDeadPlayed`, `KarmaFreeRevival`, `KarmaRevivalPrevention`, `ChoamFreeRevivalPrevention`, `RequestTleilaxuRevival`, `AcceptOrCancelTleilaxuRevival` and `SetIncreasedRevivalLimits`. The payloads are in `view/src/schema/revival.rs`. Each component changes its own inputs while it renders: an undrawn picker resets its field to 0, a number picker clamps into its range, a required image choice falls back to the first value, and the map pick drops a target that is disabled or no longer valid. `normalize` repeats those changes until the draft stops changing, both for the default draft and for every query. Each payload carries the adjusted draft (`draft`, in savegame JSON), so the client shows the values whose validation it shows. Two C# details are kept: `RaiseDeadComponent` keeps a chosen hero when no hero can be revived, and `AcceptOrCancelTleilaxuRevival` touches its hero and price only while there are requests.
- Web: `web/src/components/revival/` (`RevivalForms.tsx`, and `parts.tsx` with the card layout, `SelectForces`, the yes/no radio, the hero and faction choices and the draft hook). `ActionPanel.tsx` dispatches through `revivalForm(a)`. `RevivalForm` takes `inPopover` for the hand's card-play popover (Raise Dead, the two Karama revival events). `NumberPicker` gained `symbolStarred` and `symbolFill` for force symbols. Revival and Raise Dead use `useMapPicker` for the location, with the pinned `PinActions`.
- Tests: `view/tests/revival_forms.rs`. It covers every fixture and faction, and every state of every sample bot game where a revival event applies. For the default draft and for a sweep of drafts per form (every amount at its limits and beyond, every hero with a skill, every target, the secret ally on and off), it checks that the options equal the engine helpers, the validation equals the engine's validation of the adjusted draft, the draft stays within the widgets' ranges, and adjusting it again changes nothing. It also checks that the server's query path returns the builder's answer.

Package 7b, battle forms (2026-11-10): done.

- Rust: `view/src/schema/battle.rs` (payloads, one `ActionOptions` variant per event class of the inventory's "Action panel: battle"; names start with `Battle`/the event so they don't collide in `web/src/generated`), `view/src/actions/battle.rs` (`TYPES`, default drafts, options, dispatch lines in `actions.rs`). As in 7d/7e, each payload carries `draft`: the event with the inputs as the component shows them. Enum choices are `BattleFormEnum {value, name, label}` (the savegame number, the member name for images, the skin's label), so the forms need no enum tables.
- The battle plan (`BattleComponent`): `normalize_battle` repeats the component's render-time adjustments in its order (ally spice clamped, invalid leader dropped and the first one taken when a leader is required, weapons from the defense and leader, then defenses from the kept weapon, Messiah, banker clamp); a query answers for the adjusted draft and validates it. `mixes` is the component's `Mixes` list (every affordable `[fs, hs, fn, hn, Battle.Dial, Battle.Cost]`), so the client steps dial and spice and moves tokens exactly as C#, Emperor cunning included, with no engine. The notes above the plan (Ecaz contribution, no opponent leaders, Voice, prescience/Karama sightings) are `BattleNote`s computed for the viewer's side only. The previous plan prefills the form when no plan is in yet (`OnInitialized`).
- Web: `web/src/components/battle/` (`parts.tsx`: layout, draft hook, Radio, SelectForces, MapForceStepper, PinActions; `BattleForms.tsx`: 24 forms and the `battleForm(action, inPopover)` dispatch; `BattleComponent.tsx`). `ActionPanel.tsx` tries `battleForm` first. `Retreat` and `BeneGesseritBattleAnnouncement` pick on the map with `useMapPicker` and their pins. `BattleInitiated` with one fight confirms after 500 ms and `TreacheryCalled` passes for a player who can't call, as in C#; their refusals only go to the console. Card plays from the hand (`ClairVoyancePlayed`, `JuicePlayed`, ...) are drawn by `CardPlayPopover.tsx` (OtherComponents/CardPlayPopover.razor: `ActionPanel` with a `filter` and `inPopover`, mounted in `GameScreen` as in GameColumns.razor). Faction choices (BattleInitiated, Clairvoyance's target) carry the faction name as tooltip, as Image.razor does.
- One dropdown: `web/src/components/Select.tsx` is SelectComponent.razor with `js/listbox.js` (top-layer list placed under or above the trigger, closes on outside click, scroll or resize; arrows, Page Up/Down, Home/End, Enter/Space/Tab to pick, Escape, type-ahead with one-letter cycling; group headings). The battle and other-event forms both use it; their own `Select`s are gone. `lobby/widgets.tsx` (`SelectComponent`, package 8) and `shipmove/PickForms.tsx` (`SimpleSelect`, 7c) still have their own copies and could switch to it.
- Tests, `view/tests/battle_forms.rs`: every fixture state and every state of the sample bot traces, every player, every applicable battle form: options equal the engine helpers, the draft is event JSON of the form's type that survives the engine's JSON, the validation is the draft's; dependent options: the battle plan per leader (including none) and per weapon (defenses), the mixes against `Battle.Dial`/`Cost`, Clairvoyance's questions per target. `battle_forms_in_rare_traces` runs the same checks over `tests/data/battle` (about 130 KB): six oracle bot games (`bots --seed 7701`, games 1, 2, 4, 9, 18, 49) cut after the event that reaches their last rare form, and `fuzz --seed 7700` game 48 as its savegame would hold it (undos applied; the only one of 270 bot and fuzz games with a battle in the Hidden Mobile Stronghold fought by its owner). Together they reach all twelve forms no fixture or sample game reached: AuditCancelled, Audited, CardsDetermined, ClairVoyanceAnswered, Diplomacy, FaceDanced, FaceDancerReplaced, HMSAdvantageChosen, JuicePlayed, Planetology, RockWasMelted and TakeLosses, and the test asserts each. With `BATTLE_FORMS_CORPUS_DIR=<dir>` it scans another directory of traces (bot or fuzz, up to the first undo) and prints where each form is first reached. A scan of 120 fuzz games ran for two hours without finishing (every leader and weapon is queried in every battle state), so the HMS game was found by grepping the traces instead.
- Fixtures: `take-losses` (Fremen decides storm losses, `FremenStormLosses`), `clairvoyance-answer` (a player answers Clairvoyance) and `face-danced` (Tleilaxu reveals a face dancer), from new specs in `fixture-specs.json`. `battle_forms_in_fixtures` asserts that they reach TakeLosses, ClairVoyanceAnswered and FaceDanced.
- Scripted comparisons (desktop): `comparisons/battle-plan.py` (battle-special/Emperor: a Sardaukar token out, half, full; dial +; spice −; a typed dial; ArrowUp in the dial; a leader; a defense; Battle!; then battle/Guild: a weapon and a token) and `comparisons/battle-clairvoyance.py` (karma-prescience/Emperor: the Clairvoyance card opens CardPlayPopover; the question select by keyboard: ArrowDown opens, arrows move, type-ahead, Enter picks; the parameter select by mouse; End and Escape). Both open the web side on the Rust server (`c.scene(..., server=True)`, `scripts/ui/scene_server.py`), because token clicks, leader picks and question changes query it and Battle! submits. New harness region `card-play` (`.card-play-pop`). All 19 steps match.
- Harness: `just compare --server` opens every web scene on the Rust server (one per worker, because opening a scene closes the scene user's other games; observers stay on the saved views, since `/dev/scene` seats a faction only). The first page that opens a scene on a fresh server used to be shown the game unseated, and `SceneServer.warm` worked around it; fixed in `dev.rs` (package 2 notes), and the workaround is gone.
- Comparison (2026-11-10, `--server`, the 15 battle fixtures plus both scripts): `prompt` matches in 122/122 desktop and 103/103 phone comparisons. In the saved-view run before it, the one miss was `mc-treachery/Emperor`, whose TreacheryCalled pass C# makes by itself; it matches on the server, and `scene_views.rs` now applies that pass too (package 5). The new fixtures: `take-losses` and `face-danced` match in `prompt`; `clairvoyance-answer` differs in `prompt` on the phone (and in `board` on the desktop) for every seat, because the ImageFlash text over the board (the Clairvoyance question, "Is Master Bewt one of your traitors?") is drawn heavier in the web client than in C#. The DOM is equal; the text border style differs (packages 3/4: `ImageFlash.tsx`, `RichText`'s `textBorder` vs ExpressionComponent's `TextBorder`). The other regions that differ (log, side-column) are package 6's.

Left for 7b: `HMSAdvantageChosen` images have no popover (`Skin.GetPopup(StrongholdAdvantage)` is not in `popups.ts`). The Retreat and BG-announcement pins and the FaceDanced pickers have no scripted comparison yet.

Package 7c, ship and move forms (2026-11-10): the 25 event types of the inventory's "Action panel: ship and move" (27 files: `PlacementComponent` and `StagedPlacementComponent` are base classes).

- Rust: `view/src/schema/ship_move.rs` (payloads; one `ActionOptions` variant per event class, sharing `PlacementOptions` for Move, Caravan and FremenRidesMonster, `LocationPickOptions` for the plain picks, `HmsMovementOptions`, `NoOptions` for FlightUsed and TestingStationUsed), `view/src/actions/ship_move.rs` (default drafts and options; two dispatch lines in `actions.rs`). Options are read from the draft, so a query answers the dependent helpers: a move's destinations for the selected forces, a shipment's destinations, maxima, cost, smuggling and Karama for its type, source, destination and No-Field, Discovery entry sources, CHOAM's factions and force choices per location, Tleilaxu ambassador skill per hero, the Bene Gesserit extra advisor per location. A territory pick lists every sector of each territory. A builder that throws leaves the options `null`.
- Unlike 7b/7d/7e, the payloads carry no adjusted `draft`: the C# components' render-time adjustments (dropping No-Field, smuggling, Karama, ally payment or homeworld origins that no longer apply; the Bene Gesserit advisor flag; CHOAM's target and force type) are made by the React form from the flags in the options, and the form queries again when they change something.
- Web: `web/src/components/shipmove/` (`common.tsx`: `EventLayout` with the collapse button, `useDraft`, `MapForceStepper`, `PinActions`, `PinChoice`; `ShipmentForm.tsx`; `PlacementForm.tsx`; `PickForms.tsx`; `OtherForms.tsx`; `index.tsx` with `shipMoveForm(action)`, which `ActionPanel.tsx` calls). The map is driven through package 4's API (see Map interaction above). `ts_bindings.rs` gained the values of `Ambassador`, `TerrorType`, `ShipmentType` and `ShipmentPermission` for the drafts, and exports `LobbyData` (the generator deletes files it doesn't produce, which had removed package 8's `LobbyData.ts`).
- Test: `view/tests/ship_move.rs`, over every fixture and every state of the sample bot games: options equal the engine helpers for the default draft and for drafts per source, destination, shipment type and No-Field value, Tleilaxu hero, CHOAM location; and the JSON query path on `ship` and `move`. Reached: Shipment, Move, Caravan, BeneGesseritAccompanies, PerformFremenSetup, FremenRidesMonster, FremenSentMonster (fixtures); AmbassadorActivated, AmbassadorPlaced, BeneGesseritFlip, FlightUsed, KarmaHmsMovement, PerformBeneGesseritPlacement, PerformHmsMovement, PerformHmsPlacement, PerformMoritaniSetup, SetShipmentPermission, TerrorPlanted (bot games). Not reached anywhere: ChoamMovePrevention, ChoamRemoveForce, DiscoveryEntered, DiscoveryRevealed, KarmaShipmentPrevention, TerrorRevealed, TestingStationUsed.
- Not done: scripted comparisons that click through a shipment or move on the map (needs the dev scene endpoint, since saved views can't answer queries), and the pinned panels have only been checked by type, not by screenshot.

Package 8, lobby and setup (2026-11-10):

- Rust: `view/src/lobby.rs` writes `web/public/static/lobby.json` (`examples/lobby_data.rs`; `tests/lobby_data.rs` keeps it current: `UPDATE_LOBBY_DATA=1`). The lobby protocol carries enums as numbers, so the file has, per lobby enum (`Faction`, `Ruleset`, `Rule`, `RuleGroup`, `MainPhase`, `Phase`, `ErrorType`, `UserStatus`, `SubscriptionType`), every member's name, number and `DefaultSkin` description, plus the static tables the C# lobby calls (`AvailableFactions`, maximum players and turns, `GetAvailableRulesets`, `RulesetDefinition`, `AvailableRuleGroups`, `GetRulesInGroup`, `GetRuleGroup`, `LatestVersion`).
- `view/src/actions/setup.rs` (payloads in `schema/setup.rs`): `EstablishPlayers` and `PerformSetup`. Both are host actions; a host without a seat acts as `Faction.None` for `EstablishPlayers` (`seatless_host_may_act`; `action_view` and `query` in `actions.rs` use it). `PerformSetup` gives the next faction to set up, its reserves, whether it may stage special forces, and the locations. Test: `view/tests/setup.rs`.
- Web, `web/src/lobby/`: `client.ts` is the lobby half of `Client.cs` (login, saved session token under the C# local-storage key, heartbeats, server status, the current game's participation, settings and phase control from the notifications, every `Request*` with the C# error texts, the re-login on `UserNotFound`). It uses `state/connection.ts` and follows the game through a `WebSocketSource`. `participation.ts` keeps .NET `Dictionary` order (free-slot reuse) for the seat lists.
- Web, `web/src/components/lobby/`: `MainPage` (Main.razor's branches: not connected, login, lobby, `EstablishPlayers`/`AwaitingGame`, `SeatPlayers`, the game screen), `LoginComponent`, `RunningGamesComponent`, `OwnGamesComponent` (lists, create/load, schedule, account), `SubscribeComponent`/`UserStatusComponent`, `AwaitingGameComponent`, `EstablishPlayersComponent`, `JoinedPlayersPanel` (also used by `MenuPanels`), `SeatPlayersComponent`, `AdminPage`, `PerformSetupForm` (dispatched from `ActionPanel.tsx`; map staging as 7c's `PerformFremenSetupForm`), and `widgets.tsx` with the generic pieces these need without a game context (`SelectComponent` with the listbox behaviour, `SelectNumber`, faction image choices, `CommandButton`, `DateTimePicker`, a Popper-placed Bootstrap dropdown). Routes in `main.tsx`: `/` and `/:userToken/:gameId` are `MainPage`, `/admin` is `AdminPage`; the skeleton's scene list moved to `/scenes` and the bare game route to `/game/:userToken/:gameId`.
- Bots (updated 2026-11-11, when the server got bots): the form is the C# one. "AI Players" lists `RuleGroup.Bots` (only `BotsCannotAlly`; the rules that add bots are in no group, in C# too), and seats no user takes become bots at the start. A refused start is logged to the console, as C# does; the Rust-only error line under the buttons is gone. The menu's bot speed (pause, 1x, 2x, 4x) was already there. `comparisons/lobby.py` now ticks the AI Players rule (`establish-players-bot-rule`) and starts with three players for two users (`establish-players-three`), so both servers start a game with a bot.
- Harness: `scripts/ui/lobby_servers.py` builds and starts the C# server (serving the C# client) and the Rust server (serving `/tmp/treachery-compare/web`) on empty databases; `compare.py` gained `Script.lobby()` (users as page pairs, `fill`, `wait_text`, `wait`) and the region `start-screen`. `scripts/ui/comparisons/lobby.py` goes through the login tabs and errors, account creation, the admin page, the lobby (empty, create, schedule, account edit, a scheduled game, subscribing), creating a game, the host's settings (basic and advanced), joining, the guest's waiting screen, and starting the game, which must reach the game screen on both servers. `just compare lobby` (about 2 minutes).
- Known limits: `JSON.parse` orders integer keys ascending, so a participation or subscriber list received whole is in user-id order, not the server's dictionary order (they agree unless a lower user id joined later). Seating (`SeatPlayersComponent`) is not in the comparison: reaching it needs the in-game players panel ("Re-seat players") and the factions after a start come from a random seed, so the shots can't agree. `Client.AdminPersistState` refuses outside a game, so the admin page's "Persist data" never works; kept as in C#.
- Fixed during comparison: fresh engine events have id fields 0, which are real objects (the Lasgun is card 0), so every default draft now sets the members the C# `ConfirmedResult` sets to null (`Battle` hero/weapon/defense, `LoserConcluded` kept card, `BattleConcluded` traitors and discards, `Retreat` location, `BeneGesseritBattleAnnouncement` territory). `battle_forms.rs` asserts the plan starts without cards when there is no previous plan, and that no card is kept by default.

Fixes after package 3 and 7c/8 (2026-11-10):

- Flash text border: `ExpressionComponent` leaves `TextBorder` off the parts of a nested `Message` or `MessagePart` (lists keep it). `FlashView.bordered` now marks, per message part, whether the border applies (`expr::message_with_borders`), and `RichText` takes it as `bordered`. This replaces package 3's note that the client borders every part. Only clairvoyance-answer's flash has a nested message among the fixtures; `just compare clairvoyance-answer` matches in every region, and the other fixtures with a flash still match on the board.
- One dropdown: the lobby's `SelectComponent` (lobby/widgets.tsx) and 7c's `SimpleSelect` (shipmove/PickForms.tsx) are gone; the ruleset selects, `TerrorPlanted`, `TerrorRevealed` and `DiscoveryRevealed` use `components/Select.tsx` with `required` (C#'s `Required` default).

Cleanup B, missing UI pieces (2026-11-11): the faction sheets, rule names in the turn popover, the disconnection banner, milestone sounds and the HMS advantage popover listed in the package notes above are done. Those notes (packages 4, 5, 7b) are left as written and are out of date on these points.

- View: `rust/view/src/skin_texts.rs` adds three `GameView` fields (protocol.md lists them). `factionSheets` holds `Skin.GetFactionSheet` and `GetFactionInfo_HTML(game, f, "")` per player. The summary line's placeholders are filled as `FactionPopupHtml` does, with places left as location or territory ids, because the client draws the place chips. `ruleLabels` holds the ruleset, rule groups and rules as the skin describes them. `recentMilestones` is `Game.RecentMilestones`. All three are the same for every viewer and read only public data: the sheets depend only on the rules and which factions play, and every C# client played the milestone sounds. To reach the skin, `Describer` gained `skin()`, which returns `None` by default. Test: `view/tests/skin_texts.rs` runs every fixture and checks the fields against `rust/skin` and the engine for every viewer, and checks the milestones after each event of four fixtures. `ts_bindings.rs` exports `Milestone`.
- Size: the sheets add about 14 KB to a 6-player view (mc-discard: 27 KB → 41 KB), mostly the full sheets. They are fixed for the whole game, so if view size matters (open question 5), they could be sent once per game instead.
- Web: `board/popups.ts` `faction` is now `Support.FactionPopupHtml` in full, and `turn` uses `ruleLabels`. Its dates are now printed in UTC, as C# prints `Game.Started`; before, they were converted to the browser's time zone. The Shift-held class from `wwwroot/index.html` is in `src/popover.ts`. `popups.ts` adds `strongholdAdvantagePopup` (`Skin.GetPopup(StrongholdAdvantage)`), which the `HMSAdvantageChosen` images now show. `state/connection.ts` adds `status` (`connecting`, `connected`, `reconnecting`, `lost`) and `onStatus`. Reconnection follows the C# `RetryPolicy` delays and gives up after about an hour. `ViewSource` gains the optional `connectionStatus`/`onConnectionStatus`, implemented by `WebSocketSource`. `GameBoard.tsx` draws `Disconnection` between the tanks and the battle plans, as GameBoard.razor does. `components/milestoneSounds.ts` plays `Skin.GetSound(m)` at the effect volume through `sound.ts`. It plays on the first view and on every view with a new event number, because C# plays them in `PerformPostEventTasks` after events, undo and loads, but not on views resent for the same event. It is called from `GameScreen`.
- Harness: `compare.py` has a new `popover` region (`body > .popover.show`) and `Pair.show_popover(selector, index, shift)`, which opens a Bootstrap popover in both clients without hovering. `comparisons/map-popovers.py` covers every seat's faction popover, the full sheet with Shift held, and the turn popover, on mc-discard and bid. All 16 shots match. `just compare mc-discard bid tanks-face-down` matches (21 scenes).
- Not compared against C#: the disconnection banner, because the C# scene page is always connected (`SceneGameService.IsConnected`). A web-only Playwright check opened `/scene/bid/Atreides` on the Rust server, stopped the server and saw both banner lines on the board. The markup uses the razor file's `MapText`s. Also not compared: the HMS advantage popover, because no fixture reaches `HMSAdvantageChosen` (only `tests/data/battle`'s fuzz game). The sounds have nothing to compare.

Cleanup A, form adjustments and lobby order (2026-11-11). This replaces two notes above: package 7c's bullet that the ship-and-move forms adjust their drafts in the browser, and package 8's known limit about `JSON.parse` reordering lists.

- Ship and move forms now work like 7b, 7d and 7e. `view/src/actions/ship_move.rs` has `normalize`, which makes the C# components' render-time adjustments to a draft. Every payload in `schema/ship_move.rs` carries the adjusted `draft`, and the validation is the adjusted draft's. The adjustments, each from its razor file:
  - Shipment, while the pinned panel shows: homeworld origins, No-Field, smuggled forces, Karama and ally payment are dropped when they no longer apply.
  - Move and Caravan: `AsAdvisors` follows the destination.
  - Bene Gesserit accompaniment: the extra advisor is dropped where it isn't allowed.
  - CHOAM's target faction and force type follow the pinned location.
  - Discovery entry: the forces are the sources next to the pinned stronghold, all of them when first pinned.
  - Ambassador activation: the actual ambassador, Ecaz checkboxes, Guild amount, the first Ixian card, Tleilaxu forces and skill.
  - Terror tokens: the planted type, and the revealed type's robbery choice, sabotage card and sneak attack forces.
  - Required choices fall back to their first value (placed ambassador, Karama shipment target, discovery to reveal, Testing Station).
- Web: `shipmove/common.tsx`'s `useDraft` takes the server's draft from each answer, and the forms read their inputs from it. The browser-side adjustment code is gone. Click handlers that need the new draft's options pass a step to `set`, which runs when the answer arrives. Shipment uses this for `InitializeAmounts`, the type change and the No-Field clamp. Before, these used the previous draft's options, which were wrong after a type or source change.
- Test: `view/tests/ship_move.rs` now checks that every default draft is stable. It also sends drafts with inputs that should be dropped and compares the answer with the adjustments written out from the razor files, along with the validation. A broken Shipment or `AsAdvisors` adjustment fails the test. Ecaz's checkboxes are not reached in the fixtures or sample bot games.
- `just compare --require prompt` on ship, ship-fremen, ship-guild, move, move-reinforce, move-within, worm-ride, worm-ride-reserves, worm-ride-ssw and bene-gesserit-accompanies: the prompt matches in every scene at both sizes. The log region differs in ship/BeneGesserit, ship-fremen and move-within. That is outside these changes, which don't touch the log.
- Lobby order: integer-keyed dictionaries (`Participation.PlayerNames`/`SeatedPlayers`, `GameInfo.SeatedPlayers`, `ScheduledGameInfo.Subscribers`/`SubscriberNames`, the `AssignSeats` notification and argument) now go over the wire as `[[key, value], ...]` in the server's order (protocol.md, Encoding). `server/src/json.rs` `wire()` switches `write_int_dict` to pairs. Only `transport/mod.rs` uses it, so the database and the equivalence transcripts keep the C# object form, and `server-equiv` still passes. Decoding `RequestAssignSeats` accepts both forms. Web: `lobby/types.ts` (`IntDict`, `dictGet`), `participation.ts`, `client.ts` and the lobby components read the pairs. `just compare lobby` (against the C# server) matches in 19 of 20 shots. The subscribe dropdown differs by 0.05% of pixels, as it did in the runs of 2026-11-10 before this change. One limit remains: a dictionary received whole doesn't show the server's free slot from an earlier removal, so a user who joins after that is listed last in the client and in the freed slot on the server.

Cleanup C, fixtures, scripted comparisons and a full check (2026-11-11). This replaces package 10's note that the version 187 fixtures can't be compared, and package 7b's note that the Retreat and BG-announcement pins and the FaceDanced pickers have no scripted comparison.

- Version 187 fixtures: `alliance-before` and `alliance-allied-before` are `alliance` and `alliance-allied` at version 187, for the `--before` shots of `scripts/ui/scenarios/alliance.py`. They stay in `scripts/fixtures`, where the Rust view tests read every fixture. `scene_views.rs` lists a fixture at an older version as `skipped` with no scenes, so `just compare` no longer reports them as errors. Regenerating them at 188 would only duplicate `alliance` and `alliance-allied`. (Deleted 2026-11-11 with old game versions, native-plan step 2.)
- New fixtures, with specs in `fixture-specs.json`: `bg-announcement` (Bene Gesserit may flip advisors in Carthag, Tuek's Sietch or Habbanya Sietch), `karma-revival` (the Emperor holds Karama and may play `KarmaFreeRevival`) and `revival-planet` (Tleilaxu at high threshold under Homeworlds may revive straight to the planet). No earlier fixture reached any of these three.
- New scripted comparisons in `scripts/ui/comparisons/`, all run with the web side on the Rust server:
  - `battle-pins`: Retreat on `retreat`/Harkonnen and the announcement on `bg-announcement`/BeneGesserit. Each one picks, re-picks and confirms from the pin.
  - `face-danced`: the reserve, source and target pickers, a typed value over the limit, then Confirm.
  - `revival-map`: Tleilaxu's forces, the planet amount, the pick on the map, then Revive.
  - `card-revival`: Raise Dead (`face-danced`/Emperor) and the Karama revival (`karma-revival`/Emperor), played from the hand's card panel.
  - `deal-offer`: the Deals popover's targets, payment, deal type, end phase and text, then Offer Deal.
- Client fixes:
  - `ImageFlash`: C# sets the flash's time only on its first render, so a flash shows only when the game screen opens with one, for that flash's time. It shows the latest flash meanwhile, and a later event never flashes by itself. The web client did the same until 2026-11-11. It now also flashes after each new event that brings a flash (protocol.md, deviation 6; `web/src/flashTrigger.ts`, checked by `npm run check:flash`). The harness masks such a flash (`flash-after-event`); the flash shown on opening is still compared.
  - `DealOfferedForm` keeps its inputs (targets, payment, type, end phase, text) across events, as the C# component does, and clears only the text after an offer. Before, every new view reset them.
  - FaceDanced's `ForceSelection` steps the placement with the pickers' buttons (C#'s `OnUp`/`OnDown`). A typed number only changes what that picker shows, as in C#. `NumberPicker` gained an optional `onStep` for this.
- Harness fixes in `compare.py`: before each screenshot, both logs (`.report .card-body`) are scrolled to the bottom, because the C# follow-log script sometimes stops short of the newest line. The DOM digest also reads a textarea's text from its value, because React writes a controlled value into the text node and Blazor doesn't.

Full check, 2026-11-11, `rust/tools/heavy.sh just compare --workers 2`, then the same with `--server`. There were 706 desktop comparisons (fixture scenes plus scripted steps) and 376 phone ones. No fixture failed to load.

| Region | Saved views, desktop | `--server`, desktop | Phone (both runs) |
|---|---|---|---|
| page | 699/706 | 700/706 | 376/376 |
| turn-banner | 701/706 | 701/706 | 376/376 |
| board | 705/706 | 706/706 | 376/376 |
| side-column | 700/706 | 705/706 | 376/376 |
| player-panel | 704/706 | 706/706 | 376/376 |
| prompt | 705/706 | 705/706 | 376/376 |
| log | 701/706 | 706/706 | 376/376 |
| winner, story, card-play, popover | 706/706 | 706/706 | 376/376 |
| start-screen | 705/706 | 705/706 | 376/376 |

Remaining mismatches:

- The turn banner's clock after a scripted step submits to the Rust server (`battle-pins:announce-confirmed`, `card-revival:karama-confirmed`, `deal-offer:offered`, `face-danced:confirmed`, `revival-map:confirmed`). The pages run on a fixed clock of 2027-01-01. The C# scene page stamps its events with that page time, so its clock shows 0:00. The Rust server stamps events with the real time, so the web clock shows about 1215 hours. Owner: package 2 and the server. The dev scene server needs a clock that matches the page's, such as a `ManualClock` set from the harness. This is a harness/server fix, not a client one.
- `revival-map:confirmed`, prompt and side column: after Revive the form stays open. C# keeps the inputs the component holds, so the cost reads 2, while the web form starts over from the new view's draft, so it reads 0. Owner: 7d (`useRevivalDraft`). Every web draft hook resets on a new view, while C# components keep their fields as long as they stay mounted. Only this case shows up in the comparisons. A fix should be decided for all form packages, not just this one.
- Leader portrait resampling, only in the saved-views run, with equal DOMs and 0.06–0.16% of pixels: `face-danced`/Guild, `mc-concluded`/BeneGesserit, `mc-treachery`/Atreides, Emperor and Guild. The difference sits in one leader disc in the player panel and in the battle log. All of these matched in the `--server` run, so it is rendering noise. Owner: none (5/6 if it becomes steady).
- `lobby:lobby-subscribe-menu`, start-screen: 0.05% of pixels with equal DOMs, in both runs. Owner: 8.
- `card-revival:ghola-leader-only`, board, saved-views run only: the fixture's flash hid while the shot was taken. Fixed: the script now clicks the flash away first, and the step matched in the `--server` run.
- Log: none remain. The scroll fix above resolved the earlier log mismatches.

Forms keep their inputs, and the dev scene clock (2026-11-11). This fixes the first two remaining mismatches of cleanup C.

- Forms keep their inputs across views, as C# components keep their fields. A C# component without parameters doesn't get `OnParametersSet` on a game update, and the client applies events to the same `Game` object, so a field changes only when the player changes it. `web/src/components/formDraft.ts` is the shared mechanism:
  - `useViewChanges` reports each later view to a mounted form as `event`, `replaced` (the event number went back: undo or load) or `seat` (the viewer's faction changed).
  - `useServerDraft` holds a form's options and draft. On a new view it sends the kept draft back as a draft query, so the server adjusts and validates it for the new game. It starts over from the view's options only on a seat change, or when the answer is for another form. `useBattleDraft`, `useRevivalDraft` and `useOtherDraft` are now thin wrappers around it.
  - `shipmove/common.tsx`'s `useDraft` already kept its draft. It now uses `useViewChanges` and also resets on a seat change.
  - React keys each form by event type, so a form unmounts, and drops its draft, when its action leaves the screen.
- Per-component differences from before, each checked against the C# component:
  - Shipment and Placement (Move, Caravan, worm ride): the map selection is reset only on `replaced`. C#'s `ResetSelection` in `OnParametersSet` runs only when the `Game` object is replaced. The web forms used to reset on every view.
  - BattleComponent: the token rows are built from the draft once (C#'s `OnInitialized`) and keep their order afterwards. They follow the draft only when a server answer changes the counts.
  - SimpleForm: Dismiss stays dismissed, and the 5-second auto-confirm is armed once, on mount (C#'s `OnInitialized`). It checks at fire time that the box is still ticked and the event is valid (`AutoPass`).
  - AllyPermission: `edited` is cleared only by Confirm Changes (`ConfirmAndSetUnedited`), not by a new view.
  - DealOffered: the effect that re-sent the fields on every view is gone, since the draft is kept. After an offer, the cleared text goes into the draft too.
  - StormDialled is no longer remounted on each view. It and AllianceOffered keep their input and validate it again on a new view.
  - Bidding already matched C#'s `OnParametersSetAsync`: outside silent auctions the amounts restart at the lowest winning bid on each event. It now also validates the kept draft again, and a view without a new event (a participation change) leaves the amounts alone.
- Dev scene clock: `DevSceneClock` (an ISO 8601 time) stops the server's clock at that time (`server/src/dev.rs`: `fixed_time`, `clock`, using `ManualClock`). It is read only when `DevScene` is on, and an unreadable value stops the server at startup. Production never sets `DevScene`, so it keeps `SystemClock`. `scene_server.py`'s `SceneServer` takes a `clock` argument, and `compare.py` passes `FIXED_TIME`, so events submitted in scripted steps carry the pages' fixed time. Test: `dev::tests`.
- Check, `rust/tools/heavy.sh just compare --workers 2 --server` with all 15 scripted comparisons: 330 desktop comparisons. The turn banner and prompt match in all 330, including the five steps whose clock differed (`battle-pins:announce-confirmed`, `card-revival:karama-confirmed`, `deal-offer:offered`, `face-danced:confirmed`, `revival-map:confirmed`). `revival-map:confirmed` now matches in every region. What remains: `card-revival:ghola-open`, `ghola-leader`, `ghola-leader-only` and `ghola-forces-2` differ by 0.06% (side column) and 0.11% (log) with equal DOMs, in the leader disc of the battle log. This is the leader portrait resampling noted above, on `face-danced`. `lobby:lobby-subscribe-menu` differs by 0.05%, as before.
