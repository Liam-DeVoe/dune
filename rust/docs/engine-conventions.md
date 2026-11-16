# Engine conventions

How `rust/engine` represents the game: enums, static data, players, events, messages, errors, randomness and collections. The engine began as a line-by-line port of the C# engine (`Shared`, still in git history before the C# tree was deleted), so many names and comments refer to the C# members they came from. [engine.md](engine.md) describes the module layout.

## Modules and names

- Modules are organized by concern; [engine.md](engine.md) describes the layout. (Until 2026-11-12 there was one module per C# file, named after it.)
- Members keep their C# names in snake_case and their C# order. Properties become methods: `card.IsWeapon` is `card.is_weapon()`.
- Where C# overloads one name, the Rust names say how the versions differ, not which parameter types they take: `Game::discard_card(card, log)` and `discard_card_of(player, card, log)`, `GameStatus::for_player` and `for_players`, `Player::kill_forces` and `kill_forces_in_territory`.
- Comments describe the Rust code. A comment about the C# engine is only for a deliberate difference (`// Not in C#: ...`, listed in [behavior-changes.md](behavior-changes.md)) or a copied C# bug (`// C# parity: ...`).
- A name that is a Rust keyword gets a trailing underscore: `card.Type` is `card.type_()`.
- Support modules have no C# counterpart. `enum_type.rs` defines how enums work, `iter.rs` the iterator and map helpers std lacks, `compat/date_time_offset.rs` is `DateTimeOffset`/`TimeSpan`, `random.rs` the random number generator and selection helpers, `events/wire.rs` holds the parts of the event JSON serde doesn't derive, and `prelude.rs` is the common import.
- `map/data.rs` and the enum declarations in `enumerations.rs` were generated from the C# source once and are edited by hand now. `tests/static_data.rs` checks them against a snapshot.
- Mark deliberate copies of C# bugs with `// C# parity: <why>`.

## Game state

`Game` holds a few fields of its own (`seed`, `rng`, `players`, `human_seats`) and groups the rest of its state by topic, one struct per topic. Each struct is defined in the file under `game/` that holds the rules of its topic, so work on one topic stays in one file:

| Field | Type | File | Holds |
|---|---|---|---|
| `rules` | `Rules` | `rules.rs` | version, ruleset, the rules in effect (`active`), settings, factions in play, automation rules, hidden secrets |
| `board` | `Board` | `board.rs` | the map, the storm sector, spice, terror tokens, ambassadors, discoveries, stronghold ownership, homeworld occupation |
| `decks` | `Decks` | `decks.rs` | every deck and discard pile, the Richese cache, recently discarded cards |
| `leaders` | `Leaders` | `leaders.rs` | leader state, captured and assassinated leaders, Vidal |
| `alliances` | `Alliances` | `alliances.rs` | alliance offers, card trades, what each faction allows its ally |
| `deals` | `Deals` | `deals.rs` | `offers`, `responses` and the deals in force (`agreed`) |
| `turn` | `TurnState` | `turn.rs` | turn, main phase and phase, and what lasts for one turn or phase: payments, milestones, prevented advantages, cards and Nexus cards in effect, phases paused by Clairvoyance, discarding and Bureaucracy |
| `history` | `History` | `history.rs` | the events (`entries`), undone events, the game before an undo, moments and timers |
| `log` | `Log` | `log.rs` | the reports |
| `counters` | `Counters` | `counters.rs` | numbers handed out in order: the leader deaths' `time_of_death` |
| `setup`, `storm`, `blow`, `charity`, `bidding`, `revival`, `ship_move`, `battle`, `collection`, `contemplate` | `SetupState` ... `ContemplateState` | the phase's file (`movement.rs` for `ship_move`) | what the rules of that phase use |

The phase structs are kept side by side whatever the current phase, and some of their fields last longer than the phase (`ship_move.shipment_permissions`, `revival.factions_with_increased_revival_limits`). Fields keep the names they had on `Game` (`game.battle.current_battle`), except where the old name would repeat the struct's: `rules.active`, `history.entries`, `deals.offers`, `deals.responses` and `deals.agreed`.

A new field goes into the struct of its topic, or into the phase struct if only that phase's rules use it.

`human_seats` is the one thing the engine knows about who sits at the table: the seats users play. Every other seat is a bot's (`Game::is_bot`), which a few rules ask (Clairvoyance questions, card trades, deals with a bot). The server owns the rest (names, hosts, observers, open seats, bot speed; `server/src/participation.rs`) and updates `human_seats` whenever seats change. The seats aren't in the history: `Game::load` takes them, `Game::new_like` copies them into a game that replays the history (undo, the view's ending), and `restore_recently_undone` keeps the current ones. Views get names and the bot speed from the caller (`treachery_view::build::Seating`).

No state lives outside `Game`: the engine has no thread-locals or statics that games share, so two games on one thread don't affect each other. The one thread-local, `error.rs`'s `QUIET`, only tells the panic hook whether to print.

## Enums

Every enum is a closed Rust `enum` with the C# values as discriminants (`enumerations.rs`), so a name that names no member doesn't decode: event JSON holding `"Nobody"` for a faction is refused. Where the C# enum had a `None = 0` member meaning "no value", the Rust enum leaves it out (`absent None = 0;`) and the game uses `Option<X>`. The C# `None` member doesn't disappear from what players see:

- event JSON writes `None` as `null` (savegames of version 190 wrote 0, which `treachery-legacy` reads as `None`);
- a message logs `None` as the `None` member (its `Part` variant holds an `Option<X>`), so the skin names it as before. A C# nullable of such an enum is `Option<Option<X>>` and logs its `None`, the C# `null`, as nothing (`Part::Null`);
- views, the lobby data and skin keys name it "None" (`enum_type::name_or_absent`, `value_or_absent`).

```rust
use crate::enum_type::{GameEnum, name_or_absent};

let skill: Option<LeaderSkill> = game.skill(Some(hero));   // C#: LeaderSkill.None when unskilled
if skill == Some(LeaderSkill::Bureaucrat) { /* ... */ }
match aspect {                                              // Option<PrescienceAspect>
    Some(PrescienceAspect::Dial) => { /* ... */ }
    None => { /* ... */ }                                   // C#'s PrescienceAspect.None
    _ => {}
}
let n: i32 = LeaderSkill::Bureaucrat as i32;                // (int)x, or x.value()
let s = LeaderSkill::from_value(n);                         // Option<LeaderSkill>: None if undefined
name_or_absent(skill);                                      // "Bureaucrat", or "None"
for w in World::ALL { /* ... */ }                           // Enum.GetValues<World>() without None
```

- `ALL` is in `Enum.GetValues` order, which is sorted by value, and leaves the absent member out. `Rule` and `Ruleset` are declared out of order, so never rebuild that list from declaration order. Where C# offered `None` as a choice, the list says so (`ClairVoyancePlayed::valid_questions` starts with `None`, "any question").
- A `None` member that is a real choice stays a member: `DealType::None` is a custom deal, and the lobby enums (`UserStatus`, `SubscriptionType`, `GameListScope`, `ErrorType`) keep theirs.
- `Ord` compares values, like C#. `Option<X>` sorts `None` first, where C# sorted the 0 member.
- `Display` is `Enum.ToString()`, the member name. Use `name_or_absent` for an `Option<X>`.
- `ShipmentPermission`, a C# `[Flags]` enum, is a struct of three bools (`cross`, `to_homeworld`, `guild_rate`) that implements `GameEnum` by hand. Event JSON writes the three bools (`{"cross":true,"toHomeworld":false,"guildRate":false}`); savegames of version 190 wrote the flag number.
- serde (de)serializes a closed enum as its member name, which is how events write it (see [JSON](#json)).
- To declare an enum outside `enumerations.rs`, use the `game_enum!` macro (`enum_type.rs`) with `closed` (see `BattleFactKind` in `message.rs`). Give `get_values` in value order.

### Factions

A player has no faction until faction selection (`Player.faction` is `Option<Faction>`), and neither has the host, so an event's initiator is `Option<Faction>` too, and so are `Player.ally`, `Player.nexus` and most event fields that name a faction. Some helpers take either:

```rust
// Queries that C# called with Faction.None take `impl Into<Option<Faction>>`:
game.is_playing(Faction::Fremen);  game.is_playing(e.initiator());
player.is(Faction::Atreides);      // false for a player without a faction
// Lists and maps keyed by Faction never hold an absent one:
game.turn.has_acted_or_passed.contains_opt(e.initiator());   // iter::ContainsOpt
game.bidding.bids.get_opt(game.player(p).faction);           // iter::MapOpt
// An executed event from a player always has a faction:
let me = self.initiator().required("faction")?;
```

`battle.battle_winner`, `battle_loser`, `current_ecaz_or_ally_fighter` and `collection.owner_of_flight_discovery` are `Option<Option<Faction>>`: `None` before the battle (or the discovery) is decided, `Some(None)` when it was decided and nobody won, as C#'s nullable `Faction?` holding `None`.

## Turns and sectors

A turn is a `Turn(u8)` (game/turn.rs). Turn 0 is `Turn::BEFORE_START`, the setup before the first storm: it has its own report and moments, so it's a value of the type and not `Option<Turn>`. Where a turn can really be missing, the field is `Option<Turn>`: `Player.predicted_turn` before the Bene Gesserit prediction, `alliances.last_turn_card_was_traded` before any trade. Event JSON reads a turn from 0 to 255 and refuses anything else ("256 is not a valid turn"), and `EstablishPlayers` refuses a game of 0 turns.

A sector is a `Sector(u8)` from 0 to 17 (map/sector.rs). Homeworlds have no sector, and neither has an attached location before it's placed, so `Location::sector` returns `Option<Sector>`. `board.sector_in_storm` is `None` before the first storm. The storm's arithmetic modulo 18 is in one place, `Sector::moved`, and `Sector::in_storm` says whether a location is in the storm: a location without a sector never is.

```rust
if game.turn.current_turn == Turn::FIRST { /* ... */ }
game.board.sector_in_storm = Some(sector.moved(1));
if game.is_in_storm(l) { /* ... */ }        // Sector::in_storm(l.sector(map), storm)
let n: i32 = Sector::number_or_none(l.sector(map));   // -1 for none, as views write it
```

Views and the log write both as numbers, and -1 for a missing sector, as before.

## Amounts of spice and forces

Amounts the game holds are `u16`: a player's spice (`resources` and the other spice fields of `Player`), spice on the board (`board.resources_on_planet`), the forces in a `Battalion`, and the forces in the tanks (`forces_killed`, `special_forces_killed`, `total_forces_killed_in_battle`). The rules still compute with `i32`, so reads widen: `i32::from(p.resources)`, `battalion.forces()`, `game.resources_in(l)`, `player.forces_in(Some(l))`. Changes go through `game::amounts`:

```rust
amounts::spend(&mut game.player_mut(me).resources, cost, "spice")?;     // Game Error if short
amounts::gain(&mut game.player_mut(p).resources, income, "spice").expect("income");  // outside GameResult
Player::move_forces(game, me, from, to, n)?;     // Battalion::change_forces, then amounts::change
game.change_resources_on_planet(l, -collected)?;
```

A change that would go below zero is a Game Error, never a negative amount, and C#'s habit of ignoring such a change silently is gone. So every `Player` function that moves, kills or revives forces returns `GameResult`. If a player can cause the error, refuse the event in `validate` instead. Where the rules take "as much as possible", take `min(amount, have)` first, as `some_forces_in_territory_to_reserves` and the collection of spice do. Don't write `+=` or `-=` on these fields: in a release build a `u16` wraps silently.

Amounts in events are `u16` as well, and serde refuses one outside 0 to 65535, so validation never sees a negative amount. Reads widen the same way: `i32::from(self.amount)`. Code that computes an amount as `i32` for an event, as bots and forms do, sets it with `amounts::clamp_u16`. The force lists (`_forceLocations` and the like) are strings, and the JSON reader checks their amounts too (`json::read_force_locations`). A few event fields stay `i32` because a negative value means something, such as a shipment back to reserves (behavior-changes.md, "Turns, sectors and amounts out of range").

## Battle strength

The engine holds no floats. A battle strength or dial is `HalfPoints` (`game/half_points.rs`), a whole number of halves, because a force without spice support counts half. It is signed: a Bureaucrat's penalty can take a total below zero, and -1 loses to 0. It prints as `13` or `2.5`, as C# printed the `float`.

```rust
let dial: HalfPoints = plan.dial(game, opponent);
Battle::determine_normal_force_strength(game, f) * forces   // HalfPoints * i32
HalfPoints::whole(hero_strength) + dial                      // whole points
dial.floor()                                                 // Math.Floor, for Sietch Tabr's spice
half_rounded_up(n)                                           // (int)Math.Ceiling(0.5 * n), for costs
```

Views write strengths as JSON numbers through `expr::strength_f64` and `strength_f32`, unchanged from the `float` they had. The bots estimate with `f32` and convert with `support::f32_of`. Both are exact.

## Static game data: handles

Territories, locations, leaders, the Messiah, treachery cards and resource cards are created once per process or once per game, and C# compares them by reference or by `Id`. Within one game those agree. In Rust each is a small `Copy` handle into a static table. Its `==` matches the C# `Equals`/`==`.

| C# | Rust handle | Notes |
|---|---|---|
| `Location` | `location::Location` | Equality by id, like C# |
| `Homeworld`, `DiscoveredLocation`, `HiddenMobileStronghold`, `AttachedLocation` | same-named handles | `Deref` to `Location`. `.location()` converts |
| `Territory` | `territory::Territory` | |
| `Leader` | `leader::Leader` | |
| `IHero` | `heroes::hero::Hero` | `enum Hero { Leader(Leader), Messiah, TreacheryCard(TreacheryCard) }` |
| `Messiah` | `Hero::Messiah` | `LeaderManager::messiah()` returns it |
| `TreacheryCard` | `treachery_card::TreacheryCard` | |
| `ResourceCard` | `resource_card::ResourceCard` | Identity is the position in `GetResourceCardsInAndOutsidePlay`, so the six Shai-Hulud cards stay distinct |

Type tests and casts become methods:

```rust
// if (l is Homeworld hw && hw.World == World.Emperor)
if let Some(hw) = l.as_homeworld() { if hw.world() == World::Emperor { /* ... */ } }

// hero as Leader / hero is Messiah
hero.as_leader();
hero == Hero::Messiah;

// Messages, lists and comparisons take the plain Location
let l: Location = hw.into();
```

Static collections and lookups:

```rust
LeaderManager::leaders()                // &'static [Leader], in C# order
LeaderManager::get_leaders(Faction::Emperor)
TreacheryCardManager::items()
game.board.map.locations(false)               // Map.Locations(false)
game.board.map.territories(true)
game.board.map.strongholds()
game.board.map.arrakeen()                     // Map.Arrakeen
game.board.map.shield_wall()                  // Map.ShieldWall (a Territory)
```

### Ids in events

An event field that names one location, territory, leader, hero or treachery card holds the handle: `Shipment.to` is an `Option<Location>`, `Battle.hero` an `Option<Hero>`. Event JSON writes the id, and `null` for `None` (`handle_wire!` in events/wire.rs). Reading an id that names no object of the field's type fails, so a decoded event never holds an unknown id: `"to":500` is refused with "500 is not a valid Location".

A field's default is the object with id 0 where there is one (`Location::from_id(0)`, `TreacheryCardManager::get(0)`), because C# left the id at 0. A default `Shipment` names card 0 as its Karama card, as it did in C#. Leaders start at id 1001, so a leader field defaults to `None`.

```rust
e.to = Some(location);                      // was e.set_to(game, Some(location))
if let Some(card) = e.karma_card { /* ... */ }
```

The lists of ids written as one string (`_cardIds`, `_forceLocations`) still go through the fetchers, `GameEvent::id_string_to_objects` and `PlacementEvent::parse_force_locations`. 4b replaces them with arrays.

### Absent values

A handle (`TreacheryCard`, `Leader`, `Hero`, `Location`, `Territory`, `ResourceCard`, `PlayerId`) always names a real object. Where a value can be absent, the type is `Option<Handle>`; collections never hold an absent value. (Until native-plan.md step 3c every handle had a null value that imitated a C# null reference.)

What to do when a value is missing:

- Absence is part of the rules (no ally, no current battle, an event field the client left out): keep the `Option` and handle `None`.
- An executed event needs the value (it passed validation, so it should be there): `.required("what")?`, which fails the event with a Game Error before it changes anything more.
- Anywhere else a valid game always has the value: `.expect("what")`. Reaching it is a bug, and the panic says what was missing.
- A C# method that took a possibly null argument and threw only on some paths keeps that shape where callers rely on it: `Voice::may_use_voice_opt` and `Prescience::may_use_prescience_opt` take an `Option<PlayerId>` and panic only where the answer depends on the player.

```rust
// Player.TreacheryCards.Add(Card)
let card = self.card().required("card")?;
game.player_mut(me).treachery_cards.push(card);
crate::iter::remove_first(&mut game.player_mut(other).treachery_cards, &card);
```

### Map state

Most of the map never changes. Two things do, and they live in `Map` (one per game, like `Game.Map`):

- the `Neighbours` lists, because `PointAt` moves the attached locations;
- `AttachedToLocation` of the hidden mobile stronghold and the discovered locations.

So the location properties that depend on them take the map:

```rust
l.sector(&game.board.map)        // l.Sector
l.visible(&game.board.map)       // l.Visible
l.neighbours(&game.board.map)    // l.Neighbours
t.is_visible(&game.board.map)    // t.IsVisible
hms.attached_to_location(&game.board.map)

// Game.Map.HiddenMobileStronghold.PointAt(Game, Target);
game.board.map.hidden_mobile_stronghold().point_at(&mut game.board.map, game.rules.version, target);
```

Everything else (`territory()`, `is_stronghold()`, `spice_blow_amount()`, `orientation()` ...) is static and needs no map.

`Map` reads the game through the `map::MapGame` trait, because Rust cannot pass `&Game` while `game.board.map` is borrowed through it. `Game` implements that trait (and through it `treachery_card_manager::CardsGame`). The C# static pathfinding methods are `Map` methods here:

```rust
game.board.map.find_neighbours(from, max, false, faction, &game, true)   // Map.FindNeighbours
game.board.map.find_paths(from, to, dist, ignore_storm, faction, &game)  // Map.FindPaths
game.board.map.find_neighbours_for_hms_movement(start, 1, false, game.board.sector_in_storm)
```

Pathfinding keeps no cache: every call works from the game as it is now.

## Objects the game owns

- Plain ownership is the default. `Player`, `LeaderState`, `Battalion` and the like are fields of `Game`. `Game` holds no `Rc`, `RefCell` or `Cell`, so it is `Send`, and a check in game/mod.rs keeps it that way.
- Game state holds no event: it holds a state struct that the event's `apply` builds ([Events](#events), "State instead of events").
- Where C# keeps two references to one mutable object, the game keeps its own copy if nothing mutates the object afterwards: `EstablishPlayers` copies its settings into `rules.settings`, and the allowed factions into `rules.factions_in_play`. The same goes for `AllRules`/`InitialRules`, `PermittedUseOfEmperorSpice`/`Amounts` and `FactionsWithIncreasedRevivalLimits`/`Factions`.
- When C# uses an object as an identity token, key by a stable handle: `PlayerId` for players. An event's state gets an id instead: `deals.responses` is keyed by `DealId`, the offer's place in the history.
- The game owns its random number generator (`Game.rng`), and the decks don't hold one: `shuffle` takes it ([Randomness](#randomness)).
- C# objects that hold their owning game (`Player.Game`, `PlayerSequence._game`, `GameEvent.Game`) don't keep it in Rust. Their methods take the game as a parameter.

## Players

A C# `Player` is a reference into some game's `Players`. In Rust a player is addressed by `PlayerId { index }`, its position in the game's `players`. Players are only ever appended, so the index never changes, and `==` on `PlayerId` is C# reference equality. Every `Player` in a C# signature or field is a `PlayerId` in Rust, except `Game.players: Vec<Player>` itself.

Make one with `game.player_at(i)`, or iterate with `game.player_indices()`.

A player's place at the table is a `Seat`, a number from 0. `Player.seat` is an `Option<Seat>` because a new `Player` has none, but `EstablishPlayers` draws the seats right after it creates the players, so game code reads `player.seated()`, which panics on a player without one. The server and the view use `Option<Seat>` for users, who may wait without a seat. Where the JSON of the lobby, the views or the stored participation needs a seat that may be absent, it writes -1 (`seat_or_minus_one`). Events still carry plain numbers: `FactionSelected.seat` is an `i32`, and a number that is no player's seat selects nobody.

```rust
let p: PlayerId = game.get_player(Some(Faction::Emperor)).expect("Emperor");
game.player(p).resources                                      // p.Resources
game.player_mut(p).resources -= 3;                            // p.Resources -= 3
```

This is what lets an event mutate the game and its own player in one method. Hold the index, never a `&mut Player` across a call that needs `game`:

```rust
// Player.Resources -= Resources; Game.Stone(Milestone.Payment); Log(...);
let me = self.player();                  // GameEvent.Player (throws NRE if unset)
game.player_mut(me).resources -= self.resources;
game.stone(Milestone::Payment);
```

`Player`'s own methods come in three forms, depending on what the method does:

| The method | Signature | Call |
|---|---|---|
| reads only the player | `fn has_ally(&self) -> bool` | `game.player(p).has_ally()` |
| changes only the player | `fn f(&mut self, ...)` | `game.player_mut(p).f(...)` |
| reads the game | `fn allied_player(&self, game: &Game) -> Option<PlayerId>` | `game.player(p).allied_player(game)` |
| changes the game, or changes itself and reads the game | `fn revive_forces(game: &mut Game, me: PlayerId, amount: i32)` | `Player::revive_forces(game, p, 3)` |

Inside a `&self` method that needs its own index (C# `this` passed to the game, as in `Game.GetPlayerName(this)`), use `game.index_of(self)`, which compares addresses like C# `ReferenceEquals`.

Other objects that live inside `Game` and are mutated with the game in hand use the same idea. `PlayerSequence` methods are `fn next_player(&mut self, game: &Game)`; call them by taking the sequence out of the game and putting it back:

```rust
let mut seq = game.battle.battle_sequence.take().expect("battle sequence");
seq.next_player(game);
game.battle.battle_sequence = Some(seq);
```

`LeaderState.Kill(Game)` and `LeaderState.Assassinate(Game)` change the leader state and the game; port them the same way, or as `Game` methods taking the hero.

## Undo

`game.undo(n)` replays the first `n` events of the history into a new game and puts it in `game`'s place. The events after them become `history.recently_undone_events`, and the game it replaced is kept, unchanged, in `history.previous_game_state`. `restore_recently_undone()` puts that game back. Without one (a game loaded with undone events), it replays the history and the undone events into a new game. `dismiss_recently_undone()` replays the history into a new game with neither. A negative `n` throws and leaves the game as it was.

Each replay plays copies of the history entries again in the new game. An event keeps no game of its own, and a `PlayerId` is only a position in `players`, so nothing in one game refers to another.

C# undo differs here. Its new game shares the event objects with the old one, and each event remembers the game it was last initialized against. After a restore, events that the restored game holds (`CurrentBattle`, `AggressorPlan`, deal offers) belong to the game made by the undo, and their members read that game. The Rust port imitated this until native-plan.md step 3. Snapshots drop it: a restore returns to the state before the undo, with nothing read from the discarded game.

The native harness sees the change in fuzz games that restore. In 200 fuzz games (seed 1), 4 differ from the reference, each after an undo that was restored. In game 18, for example, Ixian wins a Black Market card after the restore: the reference logs the win but puts the card in the hand of the discarded game's Ixian, so the live game's Ixian never gets it. The harness builds views from a separate replica, so such differences first show in a later query or bot decision.

## Events

Events are plain data (rewrite-plan.md, wave 3a). The `events!` list in `events/mod.rs` names every event once.

### What every event has

- `Event` is an enum with one variant per event type, declared by `events!` (`events/macros.rs`), which also gives `EventType` (with `ALL`, `name()` and the JSON discriminator), `From<X> for Event` for every event struct, `Event::event_type()`, `PartialEq`, and the dispatch.
- A played event is a `HistoryEntry { by, at, event }` (`events/rules.rs`): `by` is the submitter's faction, `None` for the host, and `at` the time. `game.history.entries` holds them. The event itself holds neither.
- The submitter reaches the rules as a `By { faction, player }`: the faction and that faction's player when the event was submitted. `By::new(game, faction)` looks the player up the way C#'s `Initialize` did (so the host, `None`, gets the first player without a faction during setup). The player is looked up once, before validation, so it stays the submitter even when the event changes who plays which faction.
- Event JSON reads into and writes from a `HistoryEntry`: `{"by","at","event"}` (see [JSON](#json)).

### Playing an event

```rust
game.submit(by, event, at, is_host)?;               // validated, as a submission: Result<(), SubmitError>
game.play(entry, validate, is_host)                  // the same with a whole entry, validated or not
game.play_for_message(entry, validate, is_host)      // Option<Message>, as C#'s Execute returned it
game.history.stamp_last(now);                        // the server sets the time after the event is applied
```

`SubmitError` is `NotApplicable(Message)`, `Rejected(Rejection)` or `Failed(GameError)`, and `message()` gives the text the player sees ("Event '...' is not applicable", the rejection, or "Game Error: ..."). `play` checks the applicable events (`get_applicable_events(by.player, is_host)`), runs `try_validate`, clears the recent milestones, runs the pre-event tasks, applies the event, runs the post-event tasks and adds the entry to the history. `EstablishPlayers` skips the applicability check and keeps the milestones, as C# did. An event sees `at` while it is applied; the server submits with no time and stamps the entry afterwards, so the timers only count on a replay, as in C#.

### Queries

Every question asked of a game takes `&Game`: `get_applicable_events`, `GameStatus::determine_status`, `next_player_to_battle`, pathfinding, the views and the bots' reads. A query never writes state, so views are built from the server's own game, and asking twice, or in another order, gives the same answers. State a query used to write as a side effect is written by `play` instead. The battle sequence is the one case: `next_player_to_battle` reads a copy of it, moved past the players without a battle, and `play` moves the sequence itself on after each event while a player is to initiate a battle or the battle report is shown (`settle_battle_sequence` in history.rs), as the views' queries did in C#.

### An event

An event is plain data: `#[derive(Clone, Debug, Default, PartialEq)]`, no `base`, no `Rc`, no `Cell`. It implements `EventRules`:

```rust
impl EventRules for CharityClaimed {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if i32::from(game.player(by.player()).resources) > 1 {
            return Err(Rejection(express!("You cannot claim charity")));
        }
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        game.claim_charity(by.player());
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " claim charity")
    }
}
```

`try_validate`, `try_describe` and `describe_short` have defaults; an event overrides them when it can be submitted without a value its validation or message needs. The submitter's faction is `by.faction`, its player `by.player()` (or `by.player` where it may be absent), and `by.is(f)` asks whether `f` submitted it. An event logs itself with `let m = self.describe(game, by); game.log_event(m);`. Static helpers (`can_be_played`, `valid_targets`) are inherent functions.

Build an event with a struct literal or `X::default()`: the submitter goes on the entry. `Event::from(x)` wraps any event struct, and `HistoryEntry::new(faction, x)` makes the entry. A bot's decision method returns an event by value (`X` or `Option<X>`). `ClassicBot::decide` returns the history entry, by the bot's faction. A method can also return a `Resubmitted<X>`, an entry with its own submitter and time. Bots use it to withdraw another faction's outdated deal offer with a copy of that offer's entry (`DealOffer::cancellation`). The bots' golden tests write a decision with `common::decision(bot, game, ..)`.

### State instead of events

Game state never holds an event. Where C# held one, it holds a state struct that `apply` builds, with only what the rules read later, and the submitter if they read it: `collection.current_division_proposal` is a `DivisionProposal { portion_to_first_player }`. Where C# compared stored events by reference, give the state an id (`DealId`) or compare the state's values. If anything outside the game keeps that state across an undo (a bot's plan does), the id must be the entry's `EntryId` (`game.history.playing` in `apply`), not the event's place in the history. An undo replays the same entries, so their ids stay, but an event submitted after the undo has a new id, as C#'s new event object was a new reference. A place in the history is reused by the next event after an undo. Event fields that C# wrote after storing the event (`Battle.weapon_card_id` and the other dynamic weapon fields, `AutomationConfigured.automation_rule_id`, `Shipment.guild_income_prevented`) are fields of the state struct that `apply` writes.

`Payment.reason` is the `EventType` the payment was for. Intrusions keep the intruder and the destination (`Intrusion { initiator, to, type_ }`), and `check_intrusion(e, initiator)` takes the initiator from the caller.

### Interfaces

`LocationEvent` (`events::location_event`) is not in the prelude. `Event` dispatches its members as `ev.location_event_to(&game)`, which panics for an event that doesn't implement it. It has no `initiator`: the caller passes it. `PlacementEvent` holds the rules the events that place forces share (`validate_move`, `force_locations_by`).

## Messages

A `Message` is plain data: who may read it (`to: Audience`), its `parts: Vec<Part>`, and an optional `detail` with what the log draws besides the text (a heading, a route, battle plans, a battle fact, and a record from `log/`). A `Part` is a text, a number, a game object, an enum value, a list or a nested phrase (`messages/part.rs`). Turning parts into words is the skin's job (`Skin::render`).

`express!` and friends build messages. Every argument goes through `Part::from`:

```rust
use crate::{express, express_to, express_part, express_part_if};

// return Message.Express(Concept.Resource, " transfers are disabled");
return Some(express!(Concept::Resource, " transfers are disabled"));

// Message.ExpressTo(Faction.Atreides, "You see ", card)
express_to!(Faction::Atreides; "You see ", card);

// MessagePart.ExpressIf(amount > 0, " for ", Payment.Of(amount))
express_part_if!(amount > 0; " for ", Payment::of(amount));

// Cards.Select(c => MessagePart.Express(" ", c, " "))
Part::list(cards.iter().map(|c| express_part!(" ", *c, " ")));

// .WithLog(new StormLog(...)), .AsHeading(Concept.Monster)
express!(...).with_log(StormLog { /* ... */ }).as_heading(Concept::Monster);
```

What converts:

- `&'static str`, `String`, `bool`, `i32`, `u16`, `Turn`, `Sector` and `HalfPoints`. Pass any other `&str` as a `String`.
- Every enum that has a variant in the `part!` list of part.rs. An enum whose C# `None` member is `Option::None` in Rust goes in the `optional` list.
- `Location` and its subtypes, `Territory`, `Leader`, `Hero`, `TreacheryCard`, `ResourceCard`.
- `Payment`, `Message`, `MessagePart` and `GameError`.
- `Option<T>`, where `None` is null.
- `Vec<T>`, `&[T]` and `[T; N]`, which become a list element.

A strength or dial goes in as `HalfPoints` (`Part::Strength`), not as a whole number, so it prints `2.5`. If something new needs to go in a message, add a variant and a `From` impl in part.rs, and say how the skin (`Skin::describe`) and the view (`view/src/expr.rs`) draw it.

`parts!` takes the same arguments and gives the `Vec<Part>`, for texts that aren't messages (the skin's own labels). A new log record goes in the `log_records!` list of `log/mod.rs`, and `view/src/log.rs` converts it for the client.

`Message::plain_text` prints a message without a skin, for tests and debugging.

### Collections in messages

A message stores a copy of each collection passed to it, taken when it is built, so the log keeps showing what the collection held then. `tests/logged_lists.rs` checks this for the three messages that log a list the game changes later: a hand, the audited cards and the discarded traitors.

The macro evaluates its arguments left to right. `express_part_if!` evaluates its arguments even when the condition is false, as C# does. Messages are data. Turning them into text is `Skin.Describe`, in the `rust/skin` crate ([skin.md](skin.md)).

An event's `validate` returns `Result<(), Rejection>`, where `Rejection(Message)` is what the player sees.

## Errors

Validation doesn't rule out every event the rules can't handle, so an accepted event can fail partway through executing. Such a failure is a `GameError` (`error.rs`), and the code on the way to it returns `GameResult<T>`:

- `apply` returns `GameResult`, and so do the game methods it calls that can fail: drawing from a deck that can run out (`Deck::try_draw`), discarding a card the player doesn't hold (`Game::discard`), moving on from a phase the intrusion queue doesn't expect (`determine_next_shipment_and_move_sub_phase`), and the bid and battle resolution around them.
- `Game::play` returns an `Err` as `SubmitError::Failed`, whose `message()` is the "Game Error: ..." text the server shows the player. The state changes made before the failure stay, and the event isn't added to the history.
- An event's message can't always be built for an event that is about to be refused (a faction that isn't playing, no card). Those events implement `try_describe`, which the "not applicable" refusal uses.
- A few events can't even be validated without a value the client left out. They implement `try_validate`, which `Game::play` calls, so such an event fails with a Game Error. Their `validate` turns the failure into a refusal for bots and views.

```rust
let card = game.bidding.cards_on_auction.as_mut().required("cards on auction")?.try_draw()?;
let bene_gesserit = self.get_player(Some(Faction::BeneGesserit)).required("The Bene Gesserit player")?;
if self.ship_move.intrusions.is_empty() {
    game_error!("No intrusions to dequeue");
}
```

Inside a function that returns `GameResult`, a value that must be there is `.required("what")?`.

Everything else that can't happen in a valid game panics, with `expect("what")`, `panic!` or `assert!`. Nothing in the engine catches a panic. The crash barriers are outside it:

- the server catches a call or timer that panics, reports it and keeps the game (`server/src/server/mod.rs`), and builds each view inside `catch_unwind` (`server/src/views.rs`);
- the bots are not a barrier. A bot round runs in a server call or timer, so a panic in a decision reaches the first barrier and is reported as a bug. A decision method that meets a state it can't handle returns `None` (no decision) at the point where C#'s `Bot.Do` caught the exception. Examples are a Clairvoyance question whose parameter isn't the type the question needs, a missing battle or opponent, and an empty list to pick from. The clairvoyance answer gives "unknown" for a question it can't work out (`clairvoyance_answer` in `bots/src/bot/other.rs`), as `Bot_Other.cs` did;
- the view validates and builds forms for drafts, which may be missing values the engine expects, inside `treachery_view::build::draft_check` ("Validation failed").

Savegames that can't be read are errors too: `GameState::load` returns `Result<GameState, JsonError>` and `Game::load` returns `Result<Game, Message>`.

Tests that check a panic use `error::catch_panic`, which returns the panic message without printing it.

## IEnumerable and laziness

Rust iterator adapters are lazy and pull one element at a time, like LINQ. `filter`, `map`, `take`, `find` and `any` interleave side effects in the same order as `Where`, `Select`, `Take`, `First` and `Any`. So:

- Methods returning `IEnumerable<T>` over static data return `impl Iterator<Item = T> + Clone + 'static` (see `Map::locations`).
- Methods returning a lazy query over game state return `impl Iterator<Item = T>` too, built from the same adapters with `move` closures. Callers stop where C# stops: `Any()` is `.next().is_some()`, `Contains(x)` is `.any(|v| v == x)`, `FirstOrDefault()` is `.next()` and `FirstOrDefault(p)` is `.find(p)`. A `Vec` would run the predicate on elements that C# never reaches, and those can throw (a null card after the match). What C# evaluates at the call (the receiver of the first operator) goes before the iterator; what it evaluates inside a lambda (`p.Faction` of a possibly null `p`) goes inside the closure.
- Where C# enumerates the query later, after a mutation, or more than once, the Rust code must re-run the query at that point. Write a comment saying so.
- `ToList()` is `collect::<Vec<_>>()`. `Distinct()` is `distinct()` (`iter::IterExt`), which keeps the first occurrence and uses `==`.
- `OrderBy`/`ThenBy` are stable sorts that compute each key once, in element order, before sorting. Use `sort_by_cached_key`, or compute the keys first, when a key has side effects such as random draws.
- Collections: see [Collections](#collections).
- C# returns `default` for "OrDefault" operators: null for classes, but 0 or `false` for value types. Rust returns `Option`, so write `.unwrap_or_default()` at value-type call sites.

## Randomness

Every random draw of a game comes from one generator, a ChaCha8 stream (`rand_chacha::ChaCha8Rng`, `random::GameRng`) seeded with the `Seed` of `EstablishPlayers`. A game is its seed plus its events, so replaying the events gives the same game. Each bot has its own generator of the same kind, seeded when the bot is created (the server draws the seed from the operating system; the harness records it in the trace).

The game owns its generator, `game.rng`, and nothing else holds it. Code that draws takes it as `&mut GameRng`, so a function that draws needs `&mut Game` (or the generator itself), and a query that takes `&Game` can't draw. A deck has no generator of its own: its `shuffle` takes one.

```rust
// EstablishPlayers
game.rng = random::new_rng(self.seed);

// A deck is shuffled with the game's generator. The two fields borrow separately.
game.decks.treachery_deck.as_mut().expect("treachery deck").shuffle(&mut game.rng);
let mut deck = Deck::new(cards);
deck.shuffle(&mut game.rng);                         // SliceRandom::shuffle
let card = deck.draw();                              // panics on an empty deck; try_draw fails the event

// rand's sampling on the generator
use crate::random::Rng;
let roll = self.rng.random_range(1..=6);

// A random item, or a random one of the best (prelude: RandomPick)
let cards: Vec<TreacheryCard> = game.player(p).treachery_cards.clone();
let card = cards.into_iter().random_item(&mut game.rng);
let hero = heroes.into_iter().random_highest(&self.random, |h| h.value());   // a bot
```

`RandomPick` (`random.rs`) has `random_item`, `random_highest`, `random_lowest` and `random_of_lowest_n`. Each collects the items and computes every key once before it draws, and `random_item` draws only when there are two or more items. Keys implement `RankKey`: integers, `bool` and the game enums. The bots rank floats with their own `support::FloatKey`.

A pick draws from a `RandomSource`: the game's `&mut GameRng`, or a bot's `&BotRng`. While the game's generator is borrowed for a pick, the items and the key function can't read the game, so collect the items first (`.collect::<Vec<_>>().into_iter()`) and compute the keys first when they read the game. Collecting changes no draw, because the pick collects anyway.

Each `ClassicBot` owns its generator (`bot.random`, a `BotRng`). Bot decision methods take `&self`, so `BotRng` lends the generator out for one draw at a time (`self.random.draw(|r| ..)`, and the bot's dice, `self.d(n, sides)`). That lets a bot's key function draw from the bot's own generator, as some do: the pick has computed every key before it borrows the generator. A draw inside `draw`'s closure panics. Nothing shares a generator: a game or bot that draws advances only its own stream.

Why ChaCha8 and not `rand::rngs::StdRng`: rand may change `StdRng`'s algorithm in any release, and a different stream turns every saved game into a different game. `rand_chacha` promises the same stream for a seed on every platform and version, and rand promises the same results from `random_range`, `shuffle` and `choose` within a minor version (0.9). A new minor version of rand, or turning on rand's `unbiased` feature anywhere in the workspace, can change the draws. `deck_shuffle_and_random_item` in tests/static_data.rs pins a few streams and catches that; such a change needs a new game version (`Game::LATEST_VERSION`), like the move to this generator in version 189 ([behavior-changes.md](behavior-changes.md#random-number-generator)).

Nothing counts draws any more. Tests that check how far a generator has drawn compare it (`game.rng`, or `bot.random.state()`) with a generator that made the expected draws, or read its `get_word_pos()`.

## JSON

Events, history entries, savegames and game settings derive serde's `Serialize` and `Deserialize`, and ts-rs's `TS`. The format is the one in rewrite-plan.md, "Wire format":

```json
{"version":191,"events":[
  {"by":"Atreides","at":"2026-11-13T04:14:35.622405Z",
   "event":{"type":"Move","play":{"asAdvisors":false,"to":17,"forceLocations":[{"location":12,"forces":3,"specialForces":1}]}}},
  {"by":"Harkonnen","at":"2026-11-13T04:15:02Z","event":{"type":"Move","play":null}}
]}
```

- An event is tagged with `type`. Its fields are camelCase. A field named `type_` is written as `dealType`, `terrorType`, `juiceType` or `cardType`, because `type` is the tag.
- An event that can be passed is marked `Name: pass` in the `events!` list. Its fields go in `play`, and a pass is `"play": null`. The struct still has its `passed` field, which serde skips.
- `Game::play` replaces a passed event with `Event::normalized()`, which keeps only `passed`. So a pass never depends on its other fields, and a game is the same game after its savegame is read back.
- Enums are written as member names, static objects as their id, and absence as `null`. An unknown name or id doesn't decode, and neither does an unknown field. A missing field takes its default.
- Some fields still hold the text C# kept: force lists (`"12:3|1,14:2|0"`), card ids (`"3;17"`), player names (`"Alice>Bob"`) and factions in play. events/wire.rs writes each as a JSON list, so the text can change later without changing the format. A text that isn't a well-formed list fails to serialize.
- `HistoryEntry.id` isn't written. A decoded entry gets a new one.
- `GameState::load` reads a savegame and `GameState::get_state_as_string` writes one.

`web/src/generated/Event.ts` is generated from these types by `protocol/tests/ts_bindings.rs` (`UPDATE_TS_BINDINGS=1 cargo test --release -p treachery-protocol --test ts_bindings`). The enums that events use must be in `enums.ts`, whose list is in `view/tests/ts_bindings.rs`.

Savegames of version 190 and before are in System.Text.Json's format (`{"Version":190,"_events":[{"$type":"Move","_toId":17,"Initiator":20,"Time":..}]}`). The `treachery-legacy` crate (rust/legacy) reads and writes that format, with the hand-written reader that was `engine/src/json`. `treachery_legacy::load_any` reads a savegame of either format and turns version 190 into 191, which has the same rules. The server uses it for uploaded savegames and for the database migration, the harness for traces of a reference that records version 190, and tests for fixtures and recorded games that are still in the old format. `legacy/tests/event_json.rs` keeps the old reader's snapshot and checks that every event it accepts converts and reads back as the same event.

## Not ported

- `Skin` and `DefaultSkin` are not in the engine. Engine logic never reads them, and `Message.DefaultDescriber` stays null here. They are ported in the separate crate `rust/skin` ([skin.md](skin.md)).
- `Deck<T>.Randomize`, which uses an unseeded random and has no callers.
- `Game.Clone`. Only the client's story view and the C# tests call it. `Map` derives `Clone` as an independent copy, whereas a C# clone shares its map with the original.

## Snapshot tests

Several engine tests compare what the engine computes with values recorded in `tests/golden/`: the static data and pathfinding (`static_data.rs`), the old event JSON (`legacy/tests/event_json.rs`), message texts (`message_corpus.rs`), the generator's streams (in `static_data.rs`), and a few rule walkthroughs (`sync_v188.rs`, `review_pkg_b.rs`, `review_pkg_c.rs`). The values were first recorded from the C# engine and have matched the Rust output since. A difference is a change in behavior. After a deliberate one, rewrite the snapshots and review the diff:

```sh
UPDATE_GOLDENS=1 cargo test --release -p treachery-engine
```

The bots (`rust/bots/tests`), the skin (`rust/skin/tests`) and dotnet-compat have snapshot tests that work the same way. When you add static data, add it to `static_data.rs` too.

## Collections

Iteration order is observable in many places: the log, the views (spice, strongholds, forces, form options), the event a bot builds, and random draws over a collection. So:

- Maps and sets are `IndexMap` and `IndexSet` (insertion order). `std::collections::HashMap` and `HashSet` are only for lookups whose order can't leak. Remove with `shift_remove`, which keeps the order of the rest. `iter::MapExt::insert_new` inserts a key that must be new.
- `BTreeMap` where a sorted order reads better: spice, discoveries and stronghold owners on the board, and each player's `forces_in_locations`, are in map order (by location id), and so is `Game::forces_on_planet_excluding_empty_locations`, which the board view lists. These were .NET `Dictionary`s until wave 2a of rewrite-plan.md, whose order after a removal (the next new key took the freed slot) players could see ([behavior-changes.md](behavior-changes.md#order-of-spice-forces-and-other-lists-on-the-board)).

## What is left of .NET

- `dotnet_compat::parse`: the parsing of the strings events carry (force locations, Clairvoyance parameters). A Clairvoyance dial is parsed by `try_halves_parse_invariant`, so the float stays in the compat crate.
- `compat/date_time_offset.rs`: the time of history entries (`at`).
- `enum_type.rs`: open enums that print like .NET's `Enum.ToString()`.
