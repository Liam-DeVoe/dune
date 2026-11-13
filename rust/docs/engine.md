# Engine and bots layout

How `rust/engine` and `rust/bots` are organized. Most engine code starts with `use crate::prelude::*;`, which brings in the game, every event, the log records and the common data types.

## rust/engine/src

| Module | What it holds |
|---|---|
| `game/` | The `Game` struct (`mod.rs`) and its rules, one file per phase or topic: `setup`, `storm`, `blow`, `charity`, `bidding`, `revival`, `movement`, `battle`, `collection`, `contemplate` (mentat), `alliances`, `deals`, `leaders`, `rules`, `other`. Each file holds the `impl Game` block for its topic and the small state types it uses (`Fight` and `LossToTake` in `battle`, `Intrusion` in `movement`, `RevivalCost` in `revival`, and so on). Also `player`, `player_sequence`, `participation`, `settings`, `state` (savegames), `status` (what each player is asked to do), `statistics`, `info`, `counters`, `moment`, and `applicable_events` (which event types a player may submit now). |
| `events/` | Every event a player or host can submit, one file per event, grouped into `host`, `setup`, `storm`, `blow`, `charity`, `bidding`, `revival`, `movement`, `battle`, `collection`, `alliances` (alliances, deals and transfers), `cards` (cards played outside a phase's own flow) and `factions` (CHOAM, terror tokens, ambassadors, bureaucracy, audits). `mod.rs` has the `GameEvent` enum and its dispatch; `game_event.rs` and `passable_game_event.rs` the shared event behavior. |
| `log/` | Structured records attached to log messages, one file per phase. |
| `map/` | The board: `Map`, locations, territories, strongholds, homeworlds and discoveries. `data.rs` holds the static tables. |
| `cards/` | Treachery cards, spice cards and `Deck`. |
| `heroes/` | Leaders, the Messiah and the `Hero` handle. |
| `messages/` | `Message`, `Expression`, `Payment` and `Report`: the text of log entries and validation errors. |
| `enumerations.rs` | Factions, phases, rules, card types and the other enums. |
| `identity.rs` | The `IIdentifiable` and `IFetcher` traits for looking objects up by id. |
| `iter.rs` | Iterator helpers std lacks (`unique`). |
| `random.rs` | `LoggedRandom`, the game's random number generator. |
| `json/` | Reading and writing events and savegames, byte for byte in the established JSON format. |
| `canon.rs` | The canonical state dump the differ compares. |
| `error.rs` | `GameError`: an accepted event that failed partway (see engine-conventions.md, "Errors"). |
| `compat/` | .NET behavior the engine still imitates: null references and collection operators that panic like .NET throws (`cs`), integer-backed enums, `DateTimeOffset`, random selection helpers. Later cleanup steps shrink it. |

Inside `game/` and `events/`, the submodules are private and their items are re-exported, so a type is reached as `treachery_engine::game::Player` or `treachery_engine::events::Shipment`, wherever its file is. Three traits are the exception: `events::bid_trait::IBid`, `events::location_event::ILocationEvent` and `events::placement::IPlacement`. Their method names clash with the `GameEventMethods` ones, so they are not glob-exported, and `GameEvent` has a dispatching method for each of their members.

## rust/bots/src

| Module | What it holds |
|---|---|
| `bot/` | `ClassicBot` (`mod.rs`) and its decisions. `ClassicBot::decide(game, priority, allowed)` is what the server, differ and observe harness call: it asks the decision methods of that priority round in order and returns the first valid event. One file per topic: `alliance`, `battle`, `bidding`, `deals`, `factions` (faction-specific powers), `knowledge` (what the bot knows about the board and its opponents, and the `Place` trait for questions asked about a location or a territory), `movement`, `shipment`, `other`. |
| `parameters.rs` | `BotParameters`, the tuning values of a bot. |
| `support.rs` | The casts the bots apply to the untyped parameters of a Clairvoyance question. |
