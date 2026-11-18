# The web client and the view layer

The server holds each game and runs the engine. After every change it sends each participant a view: what that user may see, the actions they can take now, and the valid choices for each. The client in `web/` is a React and TypeScript app with no engine. It draws views, asks the server about a form's inputs while a player fills it in (draft queries), and submits finished events. The wire format is in [protocol.md](protocol.md), whose [Game view](protocol.md#game-view) section covers the view.

## Pieces

| Piece | Where | What |
|---|---|---|
| View crate | `rust/view` (`treachery-view`) | Builds views and answers draft queries. `src/schema.rs` and `src/schema/` are the wire types |
| View types | `web/src/generated/` | Generated from the view's schema and the engine's enums by `rust/view/tests/ts_bindings.rs` (ts-rs). Do not edit |
| Event types | `web/src/generated/Event.ts` | Every event, `HistoryEntry` and `GameSettings`, generated from the engine by `rust/protocol/tests/ts_bindings.rs` |
| Protocol types | `web/src/protocol/` | The calls, replies, notifications and lobby objects, generated from `rust/protocol` by the same test. They import the view's and the events' types from `src/generated/` |
| Static data | `web/public/static/data.json` | Locations, territories, leaders, cards, homeworlds, force types, rule groups. Written by `rust/view/examples/static_data.rs`; `rust/view/tests/static_data.rs` keeps it current |
| Lobby data | `web/public/static/lobby.json` | What the lobby and setup screens need outside a game: each lobby enum's members and texts, the rulesets, the game version. Written by `rust/view/src/lobby.rs`; `tests/lobby_data.rs` keeps it current (`UPDATE_LOBBY_DATA=1`) |
| Skin | `rust/skin`, `web/public/skin/default.json` | Text is rendered on the server with the game's skin. The client uses the same skin as JSON for images, colours, map geometry and popover texts |
| Web client | `web/` | React, TypeScript, Vite |

Each generated file is checked in, and its test fails when it is stale. After changing a Rust type that the client sees, rewrite them with `UPDATE_TS_BINDINGS=1` ([engine-conventions.md](engine-conventions.md#json) has the commands) and let the TypeScript compiler (`npm run build`) show what the client must change.

## Views and the server's game

Views are built from the server's own game. Every engine query that a view or draft query makes takes `&Game` ([Queries](engine-conventions.md#queries)), so building a view can't change the game, and views of different viewers can be built in any order.

What a game's views keep between builds is a `ViewCache` (`rust/view/src/cache.rs`): the skin, and the parts of the last view made only from the rules, the factions and the skin (the faction sheets, the rule labels, the rule names), which later views reuse while what they read stays the same. Its methods are the view calls: `view`, `query` (a draft query, refused unless the draft's type is one of the viewer's current actions), `report`, `undo_list` and `preview` (the view at an earlier point, from a replay into a new game). The server keeps one cache per followed game and starts it over when the game is replaced (undo, restore, dismiss, load) or a view fails to build (`server/src/views.rs`). Tools and tests that hold a game of their own use `ViewedGame`, a game together with its cache, seating and timer.

`rust/view/src/build.rs` builds a view from the game, a `Viewer` (the user's seat, and whether they host or observe; `resolve` finds their player and faction in the game), a `Seating` (player names and the bot speed, which the server knows and the engine doesn't) and a skin. `tests/slice.rs` builds every viewer's view in different orders and requires the same views.

### Shared view parts

The server and the harness build the views of every viewer after each event, and much of each view is the same for all of them. A `ViewBatch` (`ViewCache::batch`) builds the views of one game state, with one seating and timer. It borrows the game, so the game can't change while it lives, and its views share the parts that don't read the viewer as `Arc`s. serde's `rc` feature serializes an `Arc` like the value it holds, so the JSON doesn't change.

| Shared | For how long | Why it's the same for every viewer |
|---|---|---|
| `players`, `timing`, `mentat`, `ending`, `log.reports` | the batch | built by functions that don't get the viewer |
| the lines of `log.latest` | the batch | a line doesn't read the viewer; each viewer still gets only the lines they may read |
| `board`: `forces`, `spice`, `strongholds`, `ambassadors`, `attached`, `leaders`, `monsters`, `specialStrongholds`, `protectedFromStorm`, `recentMoves` | the batch | the rest of the board reads the viewer (terror tokens, discoveries, No-Field, storm prescience) or their status (highlighted territories), and stays per viewer |
| `factionSheets`, `ruleLabels`, `rules`, `rulesForBots` | while what they read stays the same | made from the rules, the factions and the skin; the `ViewCache` keeps each with a key of what it reads, and clears them when the skin changes |

A shared part is built when the first view of the batch needs it, at the point where that view would build it on its own, and kept only when it was built without a panic. So an engine panic happens in the same view and place as without sharing. `tests/shared_parts.rs` checks that views built together have the JSON of views built alone, after events, an undo and a restore, a rejected event and an event that failed after changing the game. `piles`, `tanks` and the rest of the view read the viewer and are built for each one.

## Visibility

A view holds what its viewer may see and nothing the rules hide from them. [protocol.md](protocol.md#visibility) has the table. The builder takes visibility from the engine's own knowledge (`Player::known_cards`, `Game::known_cards`, revealed traitors and face dancers, the prescience predicates) and invents no rules of its own. Action options belong to the acting player, so they may hold that player's hidden data (traitor choices, Ixian's starting cards, CHOAM's audited cards). `tests/corpus.rs` checks every view of every step of a corpus of bot games against the visibility rules, derived from the engine state independently of the builder.

## Action forms

For each applicable event type, `actions::action_view` builds the form's options and the validation of the event the form would send with its default inputs. The builders are split by area: `actions/bidding.rs`, `battle.rs`, `ship_move.rs`, `revival.rs`, `setup.rs`, `other.rs` and `other_nexus.rs`. Each builder:

- makes the default draft: the event as the form starts, after its widgets have adjusted the inputs (a number picker clamps into its range, a required choice falls back to the first value, a hidden picker resets to 0);
- normalizes a client's draft the same way, so the options carry the adjusted draft back and the validation is that of the draft the form shows;
- lists the choices, computed from the draft where one input depends on another (the weapons for the chosen leader, the maximum forces from the chosen source).

Validation and form building for a draft run inside `build::draft_check`, because a draft may lack values the engine expects. A panic there becomes "Validation failed" instead of reaching the server.

To give a new event type a form, add its options to `ActionOptions` in `schema.rs` (or use `Simple` for a confirm button), build them in the builder of its area, and draw them in the client's forms under `web/src/components/`.

The tests per area (`battle_forms.rs`, `ship_move.rs`, `revival_forms.rs`, `bidding.rs`, `other_forms.rs`, `setup.rs`) check, in every state of the fixtures and bot traces where a form is applicable, that its options equal the engine's helpers and that its default draft validates as the options say. The behavior harness ([native-harness.md](native-harness.md)) compares every player's view after every event against a reference build, and its fuzzer fills events from these forms' options.

## The web app

```
web/
  index.html          loads the compiled CSS, then src/main.tsx
  vite.config.ts      serves the art, CSS and fonts in dev and preview
  public/skin/        the default skin as JSON
  public/static/      the static data and the lobby data
  public/views/       saved views for /scene without a server (rust/view/examples/scene_views.rs)
  src/generated/      the view's and the events' TypeScript types (generated)
  src/protocol/       the protocol's TypeScript types (generated)
  src/state/          the WebSocket connection, view sources (WebSocket, or saved views) and the game context
  src/lobby/          the lobby protocol client and the lobby data
  src/components/     the game screen: board/, log/, battle/, shipmove/, revival/, other/, lobby/
  tools/              check-rich-text.ts, check-flash.ts
```

- **Types.** The client writes no wire type by hand. `Connection.call(name, args)` (`src/state/connection.ts`) takes a call's name and arguments and resolves with its reply, typed from `src/protocol/` (`Call`, `CallReplies`), and `on(name, handler)` receives notifications typed from `Notification`. Forms build events as the generated `Event` union. `src/components/formDraft.ts` keeps a form's inputs as the fields of its event type, so a misspelt field is a type error, and builds the event with `eventOf(type, fields)`: the inputs go in `play` for an event that can be passed and beside `type` otherwise. The client sends only the event. The server sets the submitter and the time.
- **Session.** The server knows the user per WebSocket ([protocol.md](protocol.md#sessions)). `Connection` authenticates each new socket with the session token (`authenticate`) before any other call. The page keeps the token in `sessionStorage` (`storeSession`), and in `localStorage` when the user asks to stay signed in. Tokens never go in URLs. When the server ends the session (`sessionEnded`, or a call refused with `notLoggedIn`), the client forgets it and shows the login.
- **State.** One connection per tab. The latest `GameView` replaces the previous one whole. Components derive what they draw from it and the skin, and keep only UI state: a form's draft, the log phase shown, collapsed panels, the map selection. A form keeps its draft across views and sends it back as a draft query after each one, so the server adjusts it to the new game. The draft is dropped when the form's event type leaves `actions` or the viewer's seat changes. Draft query answers older than the form's latest query are dropped. Preferences (sound volumes, auto-pass, collapse states, the Atreides tracker's notes) are in local storage.
- **Map interaction.** `components/board/mapInteraction.ts` holds the page's one map interaction. One form owns it at a time. The owner sets what is selectable, selected and targeted, and the board's layers only read it. Every location click goes to listeners, and a form acts on clicks only while it owns the map. Valid locations come from the action's options.
- **Routes.** `/` (login, then the lobby), `/game/:gameId` (reconnect to a game), `/link/:linkToken` (the single-use login link of an async-play mail, which then shows the game), `/scenes` and `/scene/:fixture/:faction` (a fixture from `scripts/fixtures` seen by one faction: played on a server started with `TREACHERY_DEV_SCENE`, or drawn from the saved views without one), and `/admin`. Inside a game, the screens follow the game's phase. `TREACHERY_BASE=/prefix/ npx vite build` serves the app under a path prefix, and every absolute URL goes through `withBase` (`src/base.ts`).
- **CSS, fonts, art.** The stylesheet (SCSS in `web/styles`, compiled by `npm run build:css`), fonts and art live in `web/assets`, which Vite (dev, preview) and the Rust server (production) serve under `/art`, `/css`, `/fonts`, `/js` and `/docs`. Design work follows `design/design-language.html` at the repository root, which is kept out of git.
- **Text.** Every rich-text part arrives with its `label`, rendered on the server, so the client needs no text rules of its own. Popovers are HTML built in `src/popups.ts` from the skin JSON and shown on hover by `src/popover.ts`.

### CSS

The stylesheet is our own, with no CSS framework. `web/styles/treachery.scss` loads three layers in order: `foundation` (the reboot, fonts, design tokens, page rules, animations), `components` (one file per component) and `app` (rules for this app's screens). A few components keep a plain `.css` file next to their TSX, which Vite bundles after `treachery.css`.

Class names follow BEM:

- A block is a component: `.chip`, `.panel`, `.battle-plan`. Its file is `components/_<block>.scss`.
- A part of a block is `block__element`: `.chip__name`, `.panel__header`. A part gets its own class rather than being styled by tag or by its position inside the block.
- A variant or state is `block--modifier` or `block__element--modifier`, next to the base class in the markup: `class="chip chip--place"`, `seg__button seg__button--on`. A state word like `on` or `you` is never a class of its own.
- In SCSS, elements and modifiers nest with `&__x` and `&--y`. Where a rule needs more weight than one class to keep winning over another rule, it says why in a comment.
- There are no utility classes. Spacing, alignment and text styles that a component needs are elements or modifiers of that component.
- A selector names one block. A parent never restyles another block's parts (`.panel .chip`). Instead the component that draws the child gives it a modifier, or an element of the parent as a second class: `<Button className="map-pin__action">`, `card--act`. Where the child is HTML from the server or `src/popups.ts` and can't take a class, the parent sets custom properties that the child reads with its usual look as the fallback. The parchment act card works this way. It sets `--control-bg`, `--control-text`, `--surface-link`, `--chip-bg` and the like (`app/_prompt-act.scss`), and inputs, buttons, chips and links read them: `background: var(--control-bg, var(--ink-0))`.
- The log's blocks (`_log-*.scss` and the battle blocks `log-faceoff`, `log-pair`, `log-plan`, `log-verdict`, `log-aftermath`) share one vocabulary of elements, listed at the top of `components/_log-kit.scss`: `__head` is the first row of a box, `__lines`/`__line` the lines under it, `__label` a small caps label, `__note` smaller text inside a line, `__rule` a hairline, `__sigil` a faction's sigil, `__img` a card's art, `__side` one side of a battle. The modifiers `--hot`, `--you`, `--won` and `--lost` mean the same in each.

#### Scales

Sizes and colours come from the tokens in `foundation/_tokens.scss`. A component never writes a literal for these:

- **Type:** `--fs-1` 11px (labels, captions), `--fs-2` 13px (secondary text, small controls), `--fs-3` 15px (UI text: chips, controls, panel rows), `--fs-body` 16px (the page's reading text, form fields and buttons), `--fs-4` 18px, `--fs-5` 22px, `--fs-6` 30px, `--fs-7` 40px (display). A size computed from a component's own size, like the spice digits or the force star, stays a `calc()`.
- **Spacing** (margin, padding, gap, and offsets that act as spacing): `--s-0` 2px for hairline gaps, `--s-1` 4px, `--s-2` 8px, `--s-3` 12px, `--s-4` 16px, `--s-5` 24px, `--s-6` 32px, and `--s-7` 48px and `--s-8` 64px for page layout. A negative margin is `calc(-1 * var(--s-2))`. A -1px that pulls a box over its neighbour's border stays -1px.
- **Radii:** `--r-1` 3px, `--r-2` 6px, `--r-3` 12px, `--r-pill` for pills and other round-ended shapes, `--r-round` for circles.
- **Colours:** the palette sampled from the map and the faction colours, plus named colours for everything else. A translucent colour is named by what it is and how opaque: `--wash-dark-55` is `--ink-0` at 55%, `--shade-35` is black at 35%, `--hairline` is the light line on ink surfaces.

Sizes that are geometry, not spacing, stay literal: widths and heights, icon and art sizes, positions of carets and markers worked out from art, and the checkbox's em-based hang.

Ids stay as hooks where scripts look for them (`#mapdiv`, `#tip`, `#printSection`). The server emits some of these classes in its HTML (`rust/skin/src/skin.rs`: chips, spice amounts, popover bodies), so renaming one of those changes the views the harness compares.

Behavior that a framework's script used to provide is our own TypeScript:

- **Popovers** (`src/popover.ts`): `usePopover` shows the HTML of `src/popups.ts` beside an element while the mouse is over it, after a short delay. The box is a native `popover="manual"` element, so it sits in the top layer, over an open modal too. It is placed on the preferred side, flips to another side when it doesn't fit, and shifts to stay inside the window. It is placed again when its size changes, since card art often loads after it opens. The map's popovers (`components/board/popover.ts`) use the same hook with their own frame.
- **Modals** (`components/ModalComponent.tsx`): a `<dialog>` opened with `openModal(id)` and closed with `closeModal(id)`, the close button, Escape or a click beside the box. The browser keeps focus inside while it is open and returns it to the opener when it closes.
- **Dropdowns** (`DropdownButton` in `components/lobby/widgets.tsx`, the game menu in `MenuComponent.tsx`): React state opens the menu, and a click outside or Escape closes it. In `DropdownButton` the arrow keys also move through the items.
- **Tooltips** (`web/assets/js/tooltip.js`): any element with a `title` shows it in the shared `.tip` box after a delay, in place of the browser's tooltip.

## Checks

- `rust/tools/check.sh` runs the view tests: forms against the engine, visibility over the bot-game corpus, build order independence, the log line snapshots (`tests/log_lines.rs`), and whether the generated types, the static data and the lobby data are current.
- In `web/`: `npm run build` (typecheck and bundle), `npm run check:rich-text` (the React rich text shows each snapshot line's expected text), `npm run check:flash`.
- `scripts/ui/e2e_server.py` starts the Rust server and drives the web client in Playwright: two players form an alliance by clicking, others join from the lobby, leave, are kicked and observe, the host undoes, a connection drops and comes back, and a user logs in through the form and out again. Its docstring lists every step.
- `observe live` ([native-harness.md](native-harness.md)) compares every view after every event with the reference build.
