# Crates and modules

The Rust workspace (`rust/Cargo.toml`) and how the engine and bots are laid out. [engine-conventions.md](engine-conventions.md) says how the engine represents a game and what to follow when you change it.

## Crates

| Crate | Directory | Holds | Uses |
|---|---|---|---|
| `treachery-engine` | `engine` | The rules: game state, events, validation, log messages. No I/O, no lobby data | serde, rand, rand_chacha, indexmap, ts-rs |
| `treachery-skin` | `skin` | Names, images and texts, and the renderer from a `Message` to text ([skin.md](skin.md)) | engine |
| `treachery-view` | `view` | Each viewer's `GameView`, the action forms and draft queries, all from `&Game` ([client.md](client.md)) | engine, skin |
| `treachery-bots` | `bots` | The bots: `ClassicBot::decide(&Game, ..)` returns the event a bot wants to submit, if any | engine |
| `treachery-protocol` | `protocol` | Every message between client and server, and their TypeScript types ([protocol.md](protocol.md)) | engine, view |
| `treachery-server` | `server` | Accounts, lobby, seating, running games, bots, storage ([server.md](server.md)) | all of the above |
| `treachery-legacy` | `legacy` | Reading the JSON of savegames of game version 190 and before ([engine-conventions.md](engine-conventions.md#json)) | engine |
| `observe` | `observe` | The behavior harness, which replays recorded games against a reference build ([native-harness.md](native-harness.md)) | engine, legacy, bots, view, skin |

`rust/tools/check.sh` runs every test of the workspace. [DEVELOPMENT.md](../../DEVELOPMENT.md) says how to build and check a change.

## rust/engine/src

| Module | What it holds |
|---|---|
| `game/` | `Game` (`mod.rs`) and the rules, one file per phase or topic: `setup`, `storm`, `blow`, `charity`, `bidding`, `revival`, `movement` (shipment and move, intrusions), `battle` and `battle_outcome`, `collection`, `contemplate` (the Mentat Pause and the game's end), `alliances`, `deals`, `leaders`, `rules`, `other`. Each file holds the struct with its topic's part of the game state, the `impl Game` block for its topic, and the small types it uses. A phase's file also holds its `PhaseState` variant's struct and step enum, its prompt (`GameStatus::<phase>_status`) and its actions (`Game::add_<phase>_actions`). |
| `game/` (support) | `phase` (`PhaseState`, `Interrupt` and the computed `Phase`), `turn` (`TurnState`, `Turn`, main-phase start and end), `history` (playing events, undo, timers), `state` (savegames), `status` (what each player is asked to do), `applicable_events` (which event types a player may submit now), `board`, `decks`, `log`, `player`, `player_sequence`, `battalion`, `seats` (`Seat`, which seats are bots), `amounts` (changing `u16` amounts), `half_points` (`HalfPoints`), `settings`, `statistics`, `counters`, `moment` |
| `events/` | Every event a player or host can submit, one file per event, in folders `host`, `setup`, `storm`, `blow`, `charity`, `bidding`, `revival`, `movement`, `battle`, `collection`, `alliances` (alliances, deals and transfers), `cards` (cards played outside a phase's own flow) and `factions` (CHOAM, terror, ambassadors, Bureaucracy, audits). `mod.rs` has the `events!` list, `macros.rs` the macro, `rules.rs` `EventRules`, `By`, `HistoryEntry`, `EntryId` and `SubmitError`, and `wire.rs` the parts of the JSON format serde doesn't derive. |
| `log/` | The structured records attached to log messages, one file per phase, and the `log_records!` list. |
| `messages/` | `Message`, `Part`, `Payment` and `Report`: the content of log entries and refusals. |
| `map/` | The board: `Map`, locations, territories, sectors, strongholds, homeworlds and discoveries. `data.rs` holds the static tables. |
| `cards/` | Treachery cards, resource (spice) cards and `Deck`. |
| `heroes/` | Leaders, the Messiah and the `Hero` handle. |
| `enumerations.rs` | Factions, phases, rules, card types and the other enums. |
| `enum_type.rs` | `game_enum!` and the `GameEnum` trait, which every enum implements. |
| `random.rs` | `GameRng` and the selection helpers (`RandomPick`). |
| `identity.rs` | The `Identifiable` and `Fetcher` traits for looking objects up by id. |
| `iter.rs` | Iterator and map helpers std lacks (`distinct`, `remove_first`, `contains_opt`, `get_opt`, `insert_new`). |
| `error.rs` | `GameError`, `GameResult`, `Required` ([Errors](engine-conventions.md#errors)). |
| `compat/` | `DateTimeOffset` and `TimeSpan`, the time of history entries and the timers. |
| `prelude.rs` | The common import, `use crate::prelude::*;`. |

Inside `game/` and `events/`, the submodules are private and their items are re-exported, so a type is reached as `treachery_engine::game::Player` or `treachery_engine::events::Shipment`, wherever its file is. One trait is the exception: `events::location_event::LocationEvent` isn't glob-exported, and `Event` has a dispatching method for each of its members.

## rust/bots/src

| Module | What it holds |
|---|---|
| `bot/mod.rs` | `ClassicBot`, its generator `BotRng`, and `decide` |
| `bot/` | The decision methods, one file per topic: `alliance`, `battle`, `bidding`, `deals`, `factions` (faction powers), `knowledge` (what the bot knows about the board and its opponents, and the `Place` trait for questions about a location or a territory), `movement`, `shipment`, `other` |
| `parameters.rs` | `BotParameters`, the tuning values of a bot |
| `support.rs` | Casting the free-form parameters of a Clairvoyance question, and `f32_of` and `FloatKey` for the bots' float estimates |

`ClassicBot::decide(game, priority, allowed)` is what the server and the harness call. It asks the decision methods of one priority round (`Priority::Highest` down to `Low`, and `End` when bots act as host) in order, and returns the first valid event as a `HistoryEntry` by the bot's faction, or `None`. `allowed` is the list of event types the player may submit now.

Deciding never changes the game: `decide` takes `&Game`. It may change the bot's own fields (plans carried from one decision to the next) and draw from its generator ([Randomness](engine-conventions.md#randomness)).

A decision method returns its event by value: `X`, `Option<X>`, or a `Resubmitted<X>`, an entry with its own submitter (a bot withdraws another faction's outdated deal offer with a copy of that offer's entry, `DealOffer::cancellation`). `try_decide` wraps each one: it skips a method whose event type isn't allowed, validates the event, and passes over a method whose event the game rejects. A new method also needs its event type in `event_kind!`.

Bots don't catch panics. A decision method that meets a state it can't handle returns `None` at that point: a Clairvoyance question whose parameter isn't of the type the question needs, a missing battle or opponent, an empty list to pick from. The clairvoyance answer gives "unknown" for a question it can't work out (`clairvoyance_answer` in `bot/other.rs`). Where a valid game always has a value, the bots use `expect`, and a panic there is a bug. A bot turn runs inside a server call or timer, so such a panic reaches the server's crash barrier, which reports it and keeps the game.

The bots' tests (`rust/bots/tests`) write a decision with `common::decision(bot, game, ..)` and compare decisions and generator positions with snapshots.
