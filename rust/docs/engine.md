# Engine and bots layout

How `rust/engine` and `rust/bots` are organized. Most engine code starts with `use crate::prelude::*;`, which brings in the game, every event, the log records and the common data types.

## rust/engine/src

| Module | What it holds |
|---|---|
| `game/` | The `Game` struct (`mod.rs`) and its rules, one file per phase or topic: `setup`, `storm`, `blow`, `charity`, `bidding`, `revival`, `movement`, `battle`, `collection`, `contemplate` (mentat), `alliances`, `deals`, `leaders`, `rules`, `other`. Each file holds the struct with its topic's part of the game state (`BiddingState` in `bidding`, held as `game.bidding`; a converted phase's state is a `PhaseState` variant, as `BattlePhase` in `battle`), the `impl Game` block for its topic and the small state types it uses (`Fight` and `LossToTake` in `battle`, `Intrusion` in `movement`, `RevivalCost` in `revival`, and so on). `turn` also has the phase flow (`enter`, `main_phase_start`, `Step`), `history` undo and the event queries, `log` the reports, `board` spice and occupation, and `decks` discarding. engine-conventions.md, "Game state", lists the structs. Also `player`, `player_sequence`, `seats` (players by faction and seat, and which are bots), `settings`, `state` (savegames), `status` (what each player is asked to do), `statistics`, `counters` (`Counters`, and the statistics' counters), `moment`, and `applicable_events` (which event types a player may submit now). |
| `events/` | Every event a player or host can submit, one file per event, grouped into `host`, `setup`, `storm`, `blow`, `charity`, `bidding`, `revival`, `movement`, `battle`, `collection`, `alliances` (alliances, deals and transfers), `cards` (cards played outside a phase's own flow) and `factions` (CHOAM, terror tokens, ambassadors, bureaucracy, audits). `mod.rs` has the `GameEvent` enum and its dispatch; `game_event.rs` and `passable_game_event.rs` the shared event behavior. |
| `log/` | Structured records attached to log messages, one file per phase. |
| `map/` | The board: `Map`, locations, territories, strongholds, homeworlds and discoveries. `data.rs` holds the static tables. |
| `cards/` | Treachery cards, spice cards and `Deck`. |
| `heroes/` | Leaders, the Messiah and the `Hero` handle. |
| `messages/` | `Message`, `Expression`, `Payment` and `Report`: the text of log entries and validation errors. |
| `enumerations.rs` | Factions, phases, rules, card types and the other enums. |
| `identity.rs` | The `Identifiable` and `Fetcher` traits for looking objects up by id. |
| `iter.rs` | Iterator and map helpers std lacks (`unique`, `distinct`, `remove_first`, `MapExt::insert_new`). |
| `enum_type.rs` | How the enums are made (`game_enum!`, `GameEnum`): open `i32` newtypes that print like .NET's `Enum.ToString()`. |
| `random.rs` | The game's random number generator (`GameRng`, a ChaCha8 stream, owned by `Game.rng`) and the selection helpers (`RandomPick`: `random_item`, `random_highest`, ...). |
| `json/` | Reading and writing events and savegames, byte for byte in the established JSON format. |
| `error.rs` | `GameError`: an accepted event that failed partway (see engine-conventions.md, "Errors"). |
| `compat/` | .NET behavior the engine still needs: `DateTimeOffset`/`TimeSpan`, whose text is part of the event JSON. |

Inside `game/` and `events/`, the submodules are private and their items are re-exported, so a type is reached as `treachery_engine::game::Player` or `treachery_engine::events::Shipment`, wherever its file is. One trait is the exception: `events::location_event::LocationEvent` is not glob-exported, and `Event` has a dispatching method for each of its members.

## rust/bots/src

| Module | What it holds |
|---|---|
| `bot/` | `ClassicBot` (`mod.rs`) and its decisions. `ClassicBot::decide(game, priority, allowed)` is what the server and the observe harness call: it asks the decision methods of that priority round in order and returns the first valid event. Bots are pure: `decide` takes `&Game`, so deciding never changes the game (the Rock Melter decision predicts the battle outcome on copies of the plans; tests/behavior_changes.rs checks it). One file per topic: `alliance`, `battle`, `bidding`, `deals`, `factions` (faction-specific powers), `knowledge` (what the bot knows about the board and its opponents, and the `Place` trait for questions asked about a location or a territory), `movement`, `shipment`, `other`. |
| `parameters.rs` | `BotParameters`, the tuning values of a bot. |
| `support.rs` | The casts the bots apply to the untyped parameters of a Clairvoyance question. |
