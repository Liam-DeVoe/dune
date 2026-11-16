# Engine conventions

How `rust/engine` represents the game: enums, static data, players, events, messages, errors, randomness and collections. The engine began as a line-by-line port of the C# engine (`Shared`, still in git history before the C# tree was deleted), so many names and comments refer to the C# members they came from. [engine.md](engine.md) describes the module layout.

## Modules and names

- Modules are organized by concern; [engine.md](engine.md) describes the layout. (Until 2026-11-12 there was one module per C# file, named after it.)
- Members keep their C# names in snake_case and their C# order. Properties become methods: `card.IsWeapon` is `card.is_weapon()`.
- Where C# overloads one name, the Rust names say how the versions differ, not which parameter types they take: `Game::discard_card(card, log)` and `discard_card_of(player, card, log)`, `GameStatus::for_player` and `for_players`, `Player::kill_forces` and `kill_forces_in_territory`.
- Comments describe the Rust code. A comment about the C# engine is only for a deliberate difference (`// Not in C#: ...`, listed in [behavior-changes.md](behavior-changes.md)) or a copied C# bug (`// C# parity: ...`).
- A name that is a Rust keyword gets a trailing underscore: `card.Type` is `card.type_()`.
- Support modules have no C# counterpart. `shared.rs` holds `Shared<T>` (only the random number generator uses it) and `RcKey<T>`, `enum_type.rs` defines how enums work, `iter.rs` the iterator and map helpers std lacks, `compat/date_time_offset.rs` is `DateTimeOffset`/`TimeSpan`, `random.rs` the random number generator and selection helpers, `json/mod.rs` and `json/members.rs` are the event JSON, and `prelude.rs` is the common import.
- `map/data.rs` and the enum declarations in `enumerations.rs` were generated from the C# source once and are edited by hand now. `tests/static_data.rs` checks them against a snapshot.
- Mark deliberate copies of C# bugs with `// C# parity: <why>`.

## Game state

`Game` holds a few fields of its own (`seed`, `random`, `players`, `human_seats`) and groups the rest of its state by topic, one struct per topic. Each struct is defined in the file under `game/` that holds the rules of its topic, so work on one topic stays in one file:

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

Every enum is a closed Rust `enum` with the C# values as discriminants (`enumerations.rs`), so a number that names no member doesn't decode: event JSON holding `13` for a faction is refused with "13 is not a valid Faction". Where the C# enum had a `None = 0` member meaning "no value", the Rust enum leaves it out (`absent None = 0;`) and the game uses `Option<X>`. The C# `None` member doesn't disappear from what players see:

- event JSON writes `None` as 0 and reads 0 as `None` (`Option<X>` has its own `ToJson`/`FromJson`, see `json::JsonNullable`);
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
- `ShipmentPermission`, a C# `[Flags]` enum, is a struct of three bools (`cross`, `to_homeworld`, `guild_rate`) that implements `GameEnum` by hand. Event JSON writes it as the flag number, and a number with any other bit doesn't decode.
- serde (de)serializes a closed enum as its member name. Nothing serializes enums with serde yet; event JSON goes through `json/`.
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

An event field that names one location, territory, leader, hero or treachery card holds the handle: `Shipment.to` is an `Option<Location>`, `Battle.hero` an `Option<Hero>`. Event JSON still writes the id, and -1 for `None` (`json_handle!` in json/mod.rs). Reading an id that names no object of the field's type fails, so a decoded event never holds an unknown id: `"_toId":500` is refused with "500 is not a valid Location".

A field's default is the object with id 0 where there is one (`Location::from_id(0)`, `TreacheryCardManager::get(0)`), because C# left the id at 0 and event JSON must not change. A default `Shipment` names card 0 as its Karama card, as it did in C#. Leaders start at id 1001, so a leader field defaults to `None`.

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

Most of the map never changes. Three things do, and they live in `Map` (one per game, like `Game.Map`):

- the `Neighbours` lists, because `PointAt` moves the attached locations;
- `AttachedToLocation` of the hidden mobile stronghold and the discovered locations;
- the `FindNeighbours` cache.

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

`find_neighbours` keeps the C# cache, including its stale results. C# returns the cached list itself. Rust returns a copy, which is equivalent because no caller mutates the result (`grep -rn "FindNeighbours(" Shared Bots`).

## Objects the game owns

- Plain ownership is the default. `Player`, `LeaderState`, `Battalion` and the like are fields of `Game`.
- Events stored in game state are `Rc<Event>`, shared with `History` (see [Events](#events)). The few event fields that C# writes after the event is stored are `Cell`s.
- Where C# keeps two references to one mutable object, the game keeps its own copy if nothing mutates the object afterwards: `EstablishPlayers` copies its settings into `rules.settings`, and the allowed factions into `rules.factions_in_play`. The same goes for `AllRules`/`InitialRules`, `PermittedUseOfEmperorSpice`/`Amounts` and `FactionsWithIncreasedRevivalLimits`/`Factions`.
- When C# uses an object as an identity token, key by a stable handle: `PlayerId` for players, `shared::RcKey<Event>` for events (`deals.responses` is keyed by `RcKey<DealOffered>`). Don't compare structurally.
- The game's random number generator is shared between `Game.random` and every deck built from it. That is `random::SharedRandom`, a `Shared<GameRng>`, the last use of `Shared<T>` (an `Rc<RefCell<T>>` whose `==` is reference equality).
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

The event objects are shared between the games (`Rc`), and each replay initializes them again against the new game. An event keeps no game of its own, and a `PlayerId` is only a position in `players`, so nothing in one game refers to another.

C# undo differs here. Its new game shares the event objects with the old one, and each event remembers the game it was last initialized against. After a restore, events that the restored game holds (`CurrentBattle`, `AggressorPlan`, deal offers) belong to the game made by the undo, and their members read that game. The Rust port imitated this until native-plan.md step 3. Snapshots drop it: a restore returns to the state before the undo, with nothing read from the discarded game.

The native harness sees the change in fuzz games that restore. In 200 fuzz games (seed 1), 4 differ from the reference, each after an undo that was restored. In game 18, for example, Ixian wins a Black Market card after the restore: the reference logs the win but puts the card in the hand of the discarded game's Ixian, so the live game's Ixian never gets it. The harness builds views from a separate replica, so such differences first show in a later query or bot decision.

## Events

### Representation

- Each C# event class is a struct in `src/events/<name>.rs` with its fields in C# order, the fields inherited from `PassableGameEvent`/`PlacementEvent` inlined, and the `GameEvent` fields in `base: GameEventBase` (`Initiator`, `Time`, `Game`, `Player`).
- A C# `GameEvent` reference is the enum `events::GameEvent`, with one variant per concrete event holding an `Rc`. Cloning shares the object; `==` and `Hash` are by address. A field typed as a concrete event (`Game.AggressorPlan`) is `Option<Rc<Battle>>`. A field typed as `BidEvent`, `LocationEvent` or `Placement` is `GameEvent`.
- `events::GameEventType` is C# `Type` for events: `GetType()`, `typeof(Bid)`, the result of `GetApplicableEvents`. `name()` is `Type.Name`.
- `X::default()` is `new X()` with the C# field initializers. `X::new(game, initiator)` is `new X(game, initiator)`; it calls `initialize`.

### Behaviour

`GameEvent`'s instance members are the trait `GameEventMethods` (in `events/game_event.rs`); `PassableGameEvent` and `PlacementEvent` add `PassableGameEventMethods` and `PlacementEventMethods`. The non-virtual base members are default trait methods, ported once in those files. Each event implements the traits in its own module, and overridden members (`Validate`, `ExecuteConcreteEvent`, `GetMessage`, `Execute` in `EstablishPlayers`, `TotalAmountOfForcesAddedToLocation` in `FremenRidesMonster`) delegate to the event's inherent method of the same name. Port the inherent method; leave the trait impls alone.

```rust
self.initiator()           // Initiator
self.player()              // Player (NullReferenceException if it was never set)
self.player_opt()          // Player, possibly null
self.by(Faction::Emperor)      // By(Faction.Emperor)
self.passed                // Passed (a field on every passable event)
```

`GameEvent` (the enum) has a dispatching method for every `GameEventMethods` member: `ev.validate(&game)`, `ev.execute(&mut game, true, false)`, `ev.get_message(&game)`. Interface members dispatch as `ev.bid_amount()` and `ev.location_event_to(&game)`, which panic for an event that does not implement the trait. The interface traits (`BidEvent`, `LocationEvent`, `Placement`) are not in the prelude, because their method names clash with the base traits.

### Signatures

- Every override of one virtual member, and every implementation of one interface member, has the same Rust signature. That is why `Validate` takes `&Game` even where an event doesn't read the game.
- `ExecuteConcreteEvent` and every method that stores or upcasts `this` take `self: &Rc<Self>`, so they can write `game.battle.aggressor_plan = Some(self.clone())` or `game.ship_move.last_shipment_or_movement = Some(self.clone().into_event())`.
- A parameter of a concrete event type is `&Battle`, so `this` can be passed as `self`. Where the callee stores the event, the parameter is `&Rc<Battle>`.
- Event methods only get a `game` parameter when the C# body reads `Game` or `Player`.

### Fields written after the event is shared

These event fields are written after the event may be stored in game state or `History`, so they are `Cell`s that `&self` methods can write:

- `GameEventBase.initiator`, `GameEventBase.player` (`Initialize`)
- `GameEventBase.time` (set by the server after `execute`)
- `Battle.weapon_card_id`, `defense_card_id`, `original_weapon`, `original_defense` (`ActivateDynamicWeapons`)
- `AutomationConfigured.automation_rule_id`
- `Shipment.guild_income_prevented`

Setters of other fields take `&mut self`: call them before wrapping the event in `Rc`. If you find another C# write to a stored event, make that field a `Cell` and add it here.

When the event to call is itself stored in the game, clone the `Rc` first so the game is free to borrow mutably:

```rust
let plan = game.battle.aggressor_plan.clone().expect("aggressor plan");
plan.activate_dynamic_weapons(game, hero, weapon, defense);
```

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

- `&'static str`, `String`, `bool`, `i32`, `f32`, `f64`. Pass any other `&str` as a `String`.
- Every enum that has a variant in the `part!` list of part.rs. An enum whose C# `None` member is `Option::None` in Rust goes in the `optional` list.
- `Location` and its subtypes, `Territory`, `Leader`, `Hero`, `TreacheryCard`, `ResourceCard`.
- `Payment`, `Message`, `MessagePart` and `GameError`.
- `Option<T>`, where `None` is null.
- `Vec<T>`, `&[T]` and `[T; N]`, which become a list element.

Keep the C# numeric type: a `float` dial must go in as `f32`, not `i32`. If something new needs to go in a message, add a variant and a `From` impl in part.rs, and say how the skin (`Skin::describe`) and the view (`view/src/expr.rs`) draw it.

`parts!` takes the same arguments and gives the `Vec<Part>`, for texts that aren't messages (the skin's own labels). A new log record goes in the `log_records!` list of `log/mod.rs`, and `view/src/log.rs` converts it for the client.

`Message::plain_text` prints a message without a skin, for tests and debugging.

### Collections in messages

A message stores a copy of each collection passed to it, taken when it is built, so the log keeps showing what the collection held then. `tests/logged_lists.rs` checks this for the three messages that log a list the game changes later: a hand, the audited cards and the discarded traitors.

The macro evaluates its arguments left to right. `express_part_if!` evaluates its arguments even when the condition is false, as C# does. Messages are data. Turning them into text is `Skin.Describe`, in the `rust/skin` crate ([skin.md](skin.md)).

`Validate()` returns `Option<Message>`, where `None` means valid.

## Errors

Validation doesn't rule out every event the rules can't handle, so an accepted event can fail partway through executing. Such a failure is a `GameError` (`error.rs`), and the code on the way to it returns `GameResult<T>`:

- `execute_concrete_event` returns `GameResult`, and so do the game methods it calls that can fail: drawing from a deck that can run out (`Deck::try_draw`), discarding a card the player doesn't hold (`Game::discard`), moving on from a phase the intrusion queue doesn't expect (`determine_next_shipment_and_move_sub_phase`), and the bid and battle resolution around them.
- `GameEventMethods::execute` turns an `Err` into the "Game Error: ..." message the server shows the player. The state changes made before the failure stay, and the event isn't added to the history. `try_execute` returns the `GameResult` itself.
- An event's message can't always be built for an event that is about to be refused (a faction that isn't playing, no card). Those events implement `try_get_message`, which the "not applicable" refusal uses.
- A few events can't even be validated without a value the client left out. They implement `try_validate`, which `try_execute` calls, so such an event fails with a Game Error. Their `validate` turns the failure into a refusal for bots and views.

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
- a bot decision that panics counts as no decision (`ClassicBot::try_decide` in `bots/src/bot/mod.rs`), as C#'s `Bot.Do` catches every exception. The clairvoyance answer does the same for the question it can't work out (`determine_clair_voyance_answered`), as `Bot_Other.cs` does;
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

```rust
// EstablishPlayers
game.random = Some(SharedRandom::from_seed(self.seed));

// A deck shares the game's generator
let mut deck = Deck::new(cards, game.random.clone());
deck.shuffle();                                      // SliceRandom::shuffle
let card = deck.draw();                              // panics on an empty deck; try_draw fails the event

// rand's sampling on the generator
use crate::random::Rng;
let roll = game.random.as_ref().expect("random").lock().random_range(1..=6);

// A random item, or a random one of the best (prelude: RandomPick)
let card = player.treachery_cards.iter().copied().random_item(game.random.as_ref().expect("random"));
let hero = heroes.into_iter().random_highest(&self.random, |h| h.value());
```

`RandomPick` (`random.rs`) has `random_item`, `random_highest`, `random_lowest` and `random_of_lowest_n`. Each collects the items and computes every key once before it draws, so a key function may draw from the same generator, and `random_item` draws only when there are two or more items. Keys implement `RankKey`: integers, `bool`, the game enums, and floats (by `total_cmp`, with -0 equal to 0). Don't hold `SharedRandom::lock()` across a call that may draw from the same generator: the `RefCell` panics.

Why ChaCha8 and not `rand::rngs::StdRng`: rand may change `StdRng`'s algorithm in any release, and a different stream turns every saved game into a different game. `rand_chacha` promises the same stream for a seed on every platform and version, and rand promises the same results from `random_range`, `shuffle` and `choose` within a minor version (0.9). A new minor version of rand, or turning on rand's `unbiased` feature anywhere in the workspace, can change the draws. `deck_shuffle_and_random_item` in tests/static_data.rs pins a few streams and catches that; such a change needs a new game version (`Game::LATEST_VERSION`), like the move to this generator in version 189 ([behavior-changes.md](behavior-changes.md#random-number-generator)).

Nothing counts draws any more. Tests that check how far a generator has drawn compare `SharedRandom::state()` with a generator that made the expected draws, or read its `get_word_pos()`.

## JSON

`json.rs` writes and reads System.Text.Json exactly as `Utilities.Serialize`/`Deserialize` do: member order, escaping, number and date formats, the `$type` rules, missing members keeping their initializers, unknown members ignored. A `$`-prefixed member other than the leading `$type` is rejected in an event (a polymorphic type) and skipped as an unknown member anywhere else (`GameSettings`, `GameState`), as .NET does. `json_members.rs` holds each event's member list, generated from `JsonSerializerOptions.GetTypeInfo`.

```rust
let text = json::serialize(&event);                       // Utilities.Serialize<GameEvent>(e)
let event: GameEvent = json::deserialize(&text)?;         // Utilities.Deserialize<GameEvent>(text)
let state: GameState = json::deserialize(&savegame)?;     // without the legacy path, see below
```

`tests/event_json.rs` checks every event type with random member values, plus malformed variants, against a snapshot recorded from the C# serializer: same accept/reject decision, byte-identical re-serialization. A new event type needs cases added to `tests/golden/events.jsonl` by hand; a deliberate format change is recorded with `UPDATE_GOLDENS=1` (see [Snapshot tests](#snapshot-tests)).

Known gaps, all on malformed input: a duplicate `$type` is not detected, nesting is limited at 128 instead of 64, `-0.0` is read as an integer, and a date without an offset is read as UTC where .NET uses machine-local time. Error messages are not .NET's.

`Utilities::deserialize` reads only the System.Text.Json format. The Newtonsoft format of old savegames is no longer read, since those games are of versions the engine refuses anyway. `GameState::load` still replaces `Treachery.online` with `Treachery` in the text first.

## Not ported

- `Skin` and `DefaultSkin` are not in the engine. Engine logic never reads them, and `Message.DefaultDescriber` stays null here. They are ported in the separate crate `rust/skin` ([skin.md](skin.md)).
- `Deck<T>.Randomize`, which uses an unseeded random and has no callers.
- `Game.Clone`. Only the client's story view and the C# tests call it. `Map` derives `Clone` as an independent copy, whereas a C# clone shares its map with the original.

## Snapshot tests

Several engine tests compare what the engine computes with values recorded in `tests/golden/`: the static data and pathfinding (`static_data.rs`), event JSON (`event_json.rs`), message texts (`message_corpus.rs`), the generator's streams (in `static_data.rs`), and a few rule walkthroughs (`sync_v188.rs`, `review_pkg_b.rs`, `review_pkg_c.rs`). The values were first recorded from the C# engine and have matched the Rust output since. A difference is a change in behavior. After a deliberate one, rewrite the snapshots and review the diff:

```sh
UPDATE_GOLDENS=1 cargo test --release -p treachery-engine
```

The bots (`rust/bots/tests`), the skin (`rust/skin/tests`) and dotnet-compat have snapshot tests that work the same way. When you add static data, add it to `static_data.rs` too.

## Collections

Iteration order is observable in many places: the log, the views (spice, strongholds, forces, form options), the event a bot builds, and random draws over a collection. So:

- Maps and sets are `IndexMap` and `IndexSet` (insertion order). `std::collections::HashMap` and `HashSet` are only for lookups whose order can't leak. Remove with `shift_remove`, which keeps the order of the rest. `iter::MapExt::insert_new` inserts a key that must be new.
- `BTreeMap` where a sorted order reads better: spice, discoveries and stronghold owners on the board, and each player's `forces_in_locations`, are in map order (by location id), and so is `Game::forces_on_planet_excluding_empty_locations`, which the board view lists. These were .NET `Dictionary`s until wave 2a of rewrite-plan.md, whose order after a removal (the next new key took the freed slot) players could see ([behavior-changes.md](behavior-changes.md#order-of-spice-forces-and-other-lists-on-the-board)).

## What is left of .NET

- `dotnet_compat::fmt` (float `ToString()`) and `parse`: log texts and the parsing of the strings events carry (force locations, Clairvoyance parameters).
- `compat/date_time_offset.rs`: the `Time` of events in the JSON.
- `enum_type.rs`: open enums that print like .NET's `Enum.ToString()`.
