# The web client and the view layer

The server holds each game and runs the engine. After every change it sends each participant a view: what that user may see, the actions they can take now, and the valid choices for each. The client in `web/` is a React + TypeScript app with no engine. It draws views, asks the server about a form's inputs while a player fills it in (draft queries), and submits finished events. The wire format is the Game view section of [protocol.md](protocol.md#game-view).

## Pieces

| Piece | Where | What |
|---|---|---|
| View crate | `rust/view` (`treachery-view`) | Builds views and answers draft queries. `src/schema.rs` and `src/schema/` are the wire types |
| TypeScript types | `web/src/generated/` | Generated from the schema by `rust/view/tests/ts_bindings.rs` (ts-rs) and checked in. The test fails when they are stale; `UPDATE_TS_BINDINGS=1` rewrites them |
| Static data | `web/public/static/data.json` | Locations, territories, leaders, cards, homeworlds, force types, rule groups. Written by `rust/view/examples/static_data.rs` from `src/static_data.rs`; `tests/static_data.rs` keeps it current |
| Skin | `rust/skin`, `web/public/skin/default.json` | Text is rendered on the server with the game's skin; the client uses the same skin as JSON for images, colours, map geometry and popover texts |
| Web client | `web/` | React, TypeScript, Vite |

## The view replica

Building a view and answering a draft query call into the engine in ways that write game state: working out whose battle is next advances the battle sequence, and pathfinding fills a neighbour cache. The server's game must never see those writes, so views are never built from it.

Each game on the server has a `ViewReplica` (`rust/view/src/replica.rs`): a second `Game` that only the view code touches.

- It executes each event the server accepted, without validating it again.
- Every view is built from it after each event, and draft queries run on it.
- Undo, restore and dismiss replace it with a replay of the server's new history.
- Nothing flows back from it, and it can be rebuilt from the history at any time.

One replica serves every viewer of a game. That works because the battle-sequence write stops once it finds a player with a battle, so a second viewer's build in the same state reads without writing. `tests/corpus.rs` and `tests/slice.rs` build all viewers' views in different orders on two replicas and require equal views.

## Visibility

A view holds what its viewer may see and nothing the rules hide from them; [protocol.md](protocol.md#visibility) has the table. The builder takes visibility from the engine's own knowledge (`Player::known_cards`, `Game::known_cards`, revealed traitors and face dancers, the prescience predicates) and invents no rules of its own. Action options belong to the acting player, so they may hold that player's hidden data (traitor choices, Ixian's starting cards, CHOAM's audited cards). `tests/corpus.rs` checks every view of every bot-game step against the visibility rules, derived from the engine state independently of the builder.

## Action forms

For each applicable event type, `actions::action_view` builds the form's options and the validation of the event the form would send with its default inputs. The builders are split by area: `actions/bidding.rs`, `battle.rs`, `ship_move.rs`, `revival.rs`, `setup.rs`, `other.rs` and `other_nexus.rs`. Each builder:

- makes the default draft: the event as the form starts, after its widgets have adjusted the inputs (a number picker clamps into its range, a required choice falls back to the first value, a hidden picker resets to 0);
- normalizes a client's draft the same way, so the options carry the adjusted draft back and the validation is that of the draft the form shows;
- lists the choices, computed from the draft where one input depends on another (the weapons for the chosen leader, the maximum forces from the chosen source).

The tests per area (`battle_forms.rs`, `ship_move.rs`, `revival_forms.rs`, `bidding.rs`, `other_forms.rs`, `setup.rs`) check, in every state of the fixtures and bot traces where a form is applicable, that its options equal the engine's helpers and that its default draft validates as the options say.

The observable-behavior harness ([native-harness.md](native-harness.md)) compares every player's view after every event against a frozen reference build, and its fuzzer fills events from these forms' options.

## The web app

```
web/
  index.html          loads the compiled CSS, then src/main.tsx
  vite.config.ts      serves the art, CSS and fonts (see below) in dev and preview
  public/skin/        the default skin as JSON
  public/static/      the static data
  public/views/       saved views for /scene without a server (rust/view/examples/scene_views.rs)
  src/generated/      the view's TypeScript types (generated, do not edit)
  src/state/          view sources (WebSocket, or saved views) and the game context
  src/lobby/          login, lobby, seating and the lobby protocol client
  src/components/     the game screen: board/, log/, battle/, shipmove/, revival/, other/, lobby/
  tools/              check-rich-text.ts, check-flash.ts
```

- **State.** One connection per tab. The latest `GameView` replaces the previous one whole; components derive what they draw from it and the skin, and keep only UI state (a form's draft, the log phase shown, collapsed panels, the map selection). A form's draft lives in the form and is dropped when its event type leaves `actions`. Draft query answers older than the form's latest query are dropped. Preferences (sound volumes, auto-pass, collapse states, the Atreides tracker's notes) are in local storage.
- **Map interaction.** `components/board/mapInteraction.ts` holds the page's one map interaction. One form owns it at a time; the owner sets what is selectable, selected and targeted, and the board's layers only read it. Every location click goes to listeners, and a form acts on clicks only while it owns the map. Valid locations come from the action's options.
- **Routes.** `/` (login, then the lobby), `/:userToken/:gameId` (reconnect to a game), `/scene/:fixture/:faction` (a fixture from `scripts/fixtures` seen by one faction: played on a server started with `TREACHERY_DEV_SCENE`, or drawn from the saved views without one), `/admin`. Inside a game, the screens follow the game's phase. `TREACHERY_BASE=/prefix/ npx vite build` serves the app under a path prefix; every absolute URL goes through `withBase` (`src/base.ts`).
- **CSS, fonts, art.** The stylesheet (SCSS in `web/styles`, compiled by `npm run build:css`), Bootstrap, fonts and art live in `web/assets`, which Vite (dev, preview) and the Rust server (production) serve under `/art`, `/css`, `/fonts`, `/js` and `/docs`. Design work follows `design/design-language.html`.
- **Text.** Every rich-text part arrives with its `label`, rendered on the server, so the client needs no skin `Describe`. Popovers are HTML built in `src/popups.ts` from the skin JSON and shown with Bootstrap popovers (`src/popover.ts`).

## Checks

- `rust/tools/check.sh` runs the view tests: forms against the engine, visibility over the bot-game corpus, replica order independence, the log line snapshots (`tests/log_lines.rs`), the generated types and the static data.
- In `web/`: `npm run build` (typecheck and bundle), `npm run check:rich-text` (the React rich text shows each snapshot line's expected text), `npm run check:flash`.
- `scripts/ui/e2e_server.py` starts the Rust server, logs in two users in Playwright, creates a game from a fixture and plays a few events over the WebSocket.
- `observe live` (native-harness.md) compares every view after every event with the reference build.
