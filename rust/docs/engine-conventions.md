# Engine porting conventions

How C# constructs in `Shared` map to Rust in `rust/engine`. Read [PORTING.md](../../PORTING.md) and the summary of [hazards.md](hazards.md) first. This file covers the patterns the foundation layer set up, so every `Game_*.cs` and `Events/*.cs` port uses the same ones.

## Modules and names

- One module per C# file, snake_case: `Model/TreacheryCard.cs` becomes `src/treachery_card.rs`, `IHero.cs` becomes `i_hero.rs`. `Events/X.cs` goes in `src/events/`, `Model/Log/X.cs` in `src/log/`, everything else in `src/`. The `Game` partials are `game.rs`, `game_battle.rs` and so on, each with its own `impl Game` block; the `Game` struct with all its fields is in `game.rs`, grouped by the partial that declares them.
- Members keep their C# names in snake_case and their C# order. Properties become methods: `card.IsWeapon` is `card.is_weapon()`.
- A name that is a Rust keyword gets a trailing underscore: `card.Type` is `card.type_()`.
- Support modules have no C# counterpart. `cs.rs` holds exceptions, the throwing LINQ operators, `Shared<T>` and `RcKey<T>`. `net_enum.rs` defines how enums work. `date_time_offset.rs` is `DateTimeOffset`/`TimeSpan`, `json.rs` and `json_members.rs` are the event JSON, and `prelude.rs` is the common import.
- Generated files say so at the top. `map_data.rs` comes from `tools/gen_map_data.py` and `enumerations.rs` from `tools/gen_enumerations.py` (which also emits the enums declared in `Model/Log` and `Model/Result.cs`). Edit the generator, not the output. The engine skeleton came from `csharp-gen`; see [The skeleton generator](#the-skeleton-generator).
- Mark deliberate copies of C# bugs with `// C# parity: <why>`.

## Enums

A C# enum is an `int` with names, and any `int` is a legal value. JSON accepts `13` for a `Faction`, and the fuzzer sends such values. So every enum is an open newtype over `i32`, with the members as associated constants that keep their C# names:

```rust
use crate::enumerations::*;

let f = Faction::Fremen;                 // Faction(10)
if f == Faction::Atreides { /* ... */ }
match phase {
    Phase::BlowA | Phase::BlowB => { /* ... */ }
    _ => {}                               // always needed: the type is open
}
let n: i32 = f.0;                         // (int)f
let g = Faction(n);                       // (Faction)n
f.to_string();                            // f.ToString(): "Fremen", or "13" if undefined
for w in World::ALL { /* ... */ }         // Enum.GetValues<World>(): sorted by value
get_values_except_default(Faction::None); // Enumerations.GetValuesExceptDefault
```

- `Ord` compares values, like C#.
- `ALL` is in `Enum.GetValues` order, which is sorted by value. `Rule` and `Ruleset` are declared out of order, so never rebuild that list from declaration order.
- `Display` is `Enum.ToString()`. That includes `[Flags]` combinations such as `"Cross, ToHomeworld"` for `ShipmentPermission`. Use it for exception messages and `$"{x}"` interpolation.
- serde (de)serializes the number.
- To port an enum declared outside `Enumerations.cs`, use the `net_enum!` macro (see `BattleFactKind` in `message.rs`). Give `get_values` in value order.

## Static game data: handles

Territories, locations, leaders, the Messiah, treachery cards and resource cards are created once per process or once per game, and C# compares them by reference or by `Id`. Within one game those agree (hazards §7.1). In Rust each is a small `Copy` handle into a static table. Its `==` matches the C# `Equals`/`==`.

| C# | Rust handle | Notes |
|---|---|---|
| `Location` | `location::Location` | Equality by id, like C# |
| `Homeworld`, `DiscoveredLocation`, `HiddenMobileStronghold`, `AttachedLocation` | same-named handles | `Deref` to `Location`. `.location()` converts |
| `Territory` | `territory::Territory` | |
| `Leader` | `leader::Leader` | |
| `IHero` | `i_hero::Hero` | `enum Hero { Leader(Leader), Messiah, TreacheryCard(TreacheryCard) }` |
| `Messiah` | `Hero::Messiah` | `LeaderManager::messiah()` returns it |
| `TreacheryCard` | `treachery_card::TreacheryCard` | |
| `ResourceCard` | `resource_card::ResourceCard` | Identity is the position in `GetResourceCardsInAndOutsidePlay`, so the six Shai-Hulud cards stay distinct |

Type tests and casts become methods:

```rust
// if (l is Homeworld hw && hw.World == World.Emperor)
if let Some(hw) = l.as_homeworld() { if hw.world() == World::Emperor { /* ... */ } }

// hero as Leader / hero is Messiah
hero.as_leader();
hero.is_messiah();

// Messages, lists and comparisons take the plain Location
let l: Location = hw.into();
```

Static collections and lookups:

```rust
LeaderManager::leaders()                // &'static [Leader], in C# order
LeaderManager::get_leaders(Faction::Emperor)
TreacheryCardManager::items()
game.map.locations(false)               // Map.Locations(false)
game.map.territories(true)
game.map.strongholds()
game.map.arrakeen()                     // Map.Arrakeen
game.map.shield_wall()                  // Map.ShieldWall (a Territory)
```

### Ids in events

Events store integer ids, as in C#, and resolve them through the fetchers. `Find` throws where C# throws: `LocationLookup.Find(200)` is ArgumentOutOfRangeException.

```rust
use crate::i_fetcher::IFetcher;
let to = game.map.location_lookup().find(self.to_id);        // Option<Location>
let hero = LeaderManager::hero_lookup().find(self.hero_id);  // Option<Hero>
let card = TreacheryCardManager::lookup().find(self.card_id);
```

### Null handles

C# can store `null` in any collection of game objects: `Player.TreacheryCards.Add(Card!)` with an unknown card id puts null in the hand, and `Winners.Add(AlliedPlayer)` can add a null player. Code then passes such elements along until a member access throws. So every handle has a null value: `TreacheryCard::NULL`, `Leader::NULL`, `Hero::NULL`, `Location::NULL`, `Territory::NULL`, `ResourceCard::NULL` and `PlayerIndex::NULL`. The trait `cs::CsNull` gives them `is_null()`, `opt()` and `nn()`.

The null handle behaves like a C# null reference:

- It fits in any `Vec`, `NetHashSet` or `Deck`, and `==` is true only against another null. `Hero::Leader(Leader::NULL)` is the same null as `Hero::NULL`.
- Every member (`id()`, `type_()`, `territory()`, `data()`, ...) throws NullReferenceException. `Game::player(PlayerIndex::NULL)` does too.
- As a `NetDictionary` key, every method throws ArgumentNullException "Value cannot be null. (Parameter 'key')". `cs::install_null_key_check` registers this with dotnet-compat. `Map::new` and `cs::catch` call it.
- It dumps as `null`, becomes `ExpressionElement::Null` in messages, and `IFetcher::get_id` gives `-1` for it.
- `as_leader()`, `as_treachery_card()` and `is_messiah()` are false for it, like C# `is`.

The rule for choosing between the two forms of null:

- **Fields, parameters and returns that C# declares nullable stay `Option<Handle>`**, as the skeleton generated them. `None` is null.
- **Collection elements are plain handles**, and the null handle is null there. So is a non-nullable parameter that C# passes possibly-null values to (`MoveForces(from, To)`, `KillAllForcesIn(territory)`).
- **Crossing between the two:** to store an `Option` in a collection or pass it to a handle parameter, use `opt.or_null()`, never `opt.nre()`. `x!` in C# does not throw. Only the member access after it does. To make an `Option` from a handle that may be null, use `h.opt()`. Never write `Some(h)` for such a handle. `Some(Handle::NULL)` would not compare equal to `None`, and functions that test `x is null` would treat it as non-null. `draw_treachery_card` and `kill_all_forces_in` show the pattern.
- **`Some(NULL)` is still handled, in two places.** Every function with an `Option<Handle>` parameter starts with `let x = x.and_then(crate::cs::CsNull::opt);`, so a null handle that arrives as `Some` takes the `None` path. `tests/null_option_params.rs` scans the sources and fails for any function without that line. The "OrDefault" helpers (`random_or_default` and the `highest`/`lowest` family built on it, `cs_first_or_default_where`, `cs_single_or_default`, `cs_last_or_default_where`, `last_or_default_where_list`) return `None` for a null element, through `cs::null_to_none`. So `if let Some(card) = hand.random_or_default(..)` skips a null card the way C#'s `if (card != null)` does. Code that takes an element from a collection some other way (`hand[0]`, `.first()`, `.next()`) and wraps it in `Some` must call `.opt()`.

```rust
// Player.TreacheryCards.Add(Card!);      Card may be null
game.player_mut(me).treachery_cards.push(self.card().or_null());
// Player.TreacheryCards.Remove(Card);    List.Remove(null) removes the first null
cs::list_remove(&mut game.player_mut(me).treachery_cards, self.card());
// Game.KillAllForcesIn(territory, false);  territory may be null; AnyForcesIn(null) is 0
game.kill_all_forces_in(territory.or_null(), false);
// x.Where(...) / x.Sum() / x.ToList() on a possibly null x
let amounts = cs::linq_source(self.amounts.as_ref());   // ArgumentNullException (Parameter 'source')
```

Use `x.nre()` only where C# accesses a member of `x` right away (`x.Foo`, `x!.Foo`). Where C# stores or passes `x` on, keep it as a possibly null value. The throw comes later, from the member access, the dictionary or the LINQ operator that C# throws in. Statements in between can change state, as in `MoveForces`, which removes forces from `from` before `ChangeForces(null)` throws ArgumentNullException.

### Map state

Most of the map never changes. Three things do, and they live in `Map` (one per game, like `Game.Map`):

- the `Neighbours` lists, because `PointAt` moves the attached locations;
- `AttachedToLocation` of the hidden mobile stronghold and the discovered locations;
- the `FindNeighbours` cache.

So the location properties that depend on them take the map:

```rust
l.sector(&game.map)        // l.Sector
l.visible(&game.map)       // l.Visible
l.neighbours(&game.map)    // l.Neighbours
t.is_visible(&game.map)    // t.IsVisible
hms.attached_to_location(&game.map)

// Game.Map.HiddenMobileStronghold.PointAt(Game, Target);
game.map.hidden_mobile_stronghold().point_at(&mut game.map, game.version, target);
```

Everything else (`territory()`, `is_stronghold()`, `spice_blow_amount()`, `orientation()` ...) is static and needs no map.

`Map` reads the game through the `map::MapGame` trait, because Rust cannot pass `&Game` while `game.map` is borrowed through it. `Game` implements that trait (and through it `treachery_card_manager::CardsGame`). The C# static pathfinding methods are `Map` methods here:

```rust
game.map.find_neighbours(from, max, false, faction, &game, true)   // Map.FindNeighbours
game.map.find_paths(from, to, dist, ignore_storm, faction, &game)  // Map.FindPaths
game.map.find_neighbours_for_hms_movement(start, 1, false, game.sector_in_storm)
```

`find_neighbours` keeps the C# cache, including its stale results (hazards §6.1). C# returns the cached list itself. Rust returns a copy, which is equivalent because no caller mutates the result (`grep -rn "FindNeighbours(" Shared Bots`).

## Objects the game owns

- Plain ownership is the default. `Player`, `LeaderState`, `Battalion` and the like are fields of `Game`.
- Events stored in game state are `Rc<Event>`, shared with `History` (see [Events](#events)). The few event fields that C# writes after the event is stored are `Cell`s.
- When C# keeps two references to one mutable object that is not an event and mutates through either, use `cs::Shared<T>`, an `Rc<RefCell<T>>` whose `==` is reference equality. The skeleton uses it for the lists that `EstablishPlayers` shares with the game (hazards §5.1):
  - `Game.settings: Shared<GameSettings>` and `EstablishPlayers.settings: Option<Shared<GameSettings>>`;
  - `Game.factions_in_play: Shared<Vec<Faction>>` and `GameSettings.allowed_factions_in_play: Option<Shared<Vec<Faction>>>`.

  ```rust
  // Game.Settings = Settings;  Game.FactionsInPlay = Game.Settings.AllowedFactionsInPlay;
  game.settings = self.settings.clone().nre();
  game.factions_in_play = game.settings.borrow().allowed_factions_in_play.clone().nre();
  game.factions_in_play.borrow_mut().retain(|f| *f != faction);   // also changes the event's settings
  ```

  `AllRules`/`InitialRules`, `PermittedUseOfEmperorSpice`/`Amounts` and `FactionsWithIncreasedRevivalLimits`/`Factions` are also aliased in C#, but nothing mutates them afterwards, so they are plain copies.

  `Game.participation` is a `Shared<Participation>` too. `new Game(version, participation)` keeps the caller's object, and the games that `Undo`, `RestoreRecentlyUndone` and `DismissRecentlyUndone` make share it, so `Game::new_int32_participation` takes a `&Shared<Participation>`.
- When C# uses an object as an identity token, key by a stable handle: `PlayerIndex` for players (`LegacyNames`), `cs::RcKey<Event>` for events (`DealResponses` is keyed by `RcKey<DealOffered>`). Don't compare structurally.
- `LoggedRandom` is shared between `Game.Random` and every deck built from it. That is `logged_random::SharedRandom`, a `Shared<LoggedRandom>`.
- C# objects that hold their owning game (`Player.Game`, `PlayerSequence._game`, `GameEvent.Game`) have a `GameRef` field in its place: the id of that game, or null. Their methods take the game as a parameter. After an undo the field can name another game; see [Game identity and undo](#game-identity-and-undo).

## Players

A C# `Player` is a reference into some game's `Players`. In Rust a player is addressed by `PlayerIndex { game, index }`: the id of the game the player object belongs to, and its position in that game's `players`. Players are only ever appended, so the index never changes, and `==` on `PlayerIndex` is C# reference equality, also between games. Every `Player` in a C# signature or field is a `PlayerIndex` in Rust, except `Game.players: Vec<Player>` itself.

Make one with `game.player_at(i)`, or iterate with `game.player_indices()`. Never build one from a bare position: it would point into whichever game you name.

```rust
let p: PlayerIndex = game.get_player(Some(Faction::Emperor)).nre();   // Player p = Game.GetPlayer(Faction.Emperor)!  (C# takes Faction?)
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

`Player`'s own methods come in three forms. The generator picked one per method from what the C# body does:

| C# body | Rust signature | Call |
|---|---|---|
| reads only the player | `fn has_ally(&self) -> bool` | `game.player(p).has_ally()` |
| changes only the player | `fn f(&mut self, ...)` | `game.player_mut(p).f(...)` |
| reads the game | `fn allied_player(&self, game: &Game) -> Option<PlayerIndex>` | `game.player(p).allied_player(game)` |
| changes the game, or changes itself and reads the game | `fn revive_forces(game: &mut Game, me: PlayerIndex, amount: i32)` | `Player::revive_forces(game, p, 3)` |

Inside a `&self` method that needs its own index (C# `this` passed to the game, as in `Game.GetPlayerName(this)`), use `game.index_of(self)`, which compares addresses like C# `ReferenceEquals`.

Other objects that live inside `Game` and are mutated with the game in hand use the same idea. `PlayerSequence` methods are `fn next_player(&mut self, game: &Game)`; call them by taking the sequence out of the game and putting it back:

```rust
let mut seq = game.battle_sequence.take().nre();
seq.next_player(game);
game.battle_sequence = Some(seq);
```

`LeaderState.Kill(Game)` and `LeaderState.Assassinate(Game)` change the leader state and the game; port them the same way, or as `Game` methods taking the hero.

## Game identity and undo

`Game.Undo`, `RestoreRecentlyUndone` and `DismissRecentlyUndone` (`Game.cs:150-224`) build a new `Game` and replay the old game's history into it. The event objects are shared between the two games, and C# never copies them:

- `Initialize(newGame)` sets an event's `Game` to the new game, but sets `Player` only if the initiator is seated in the new game. Otherwise the event keeps the old game's `Player` object, and executing it changes that player.
- The undone events (`RecentlyUndoneEvents`) are not replayed, so they keep the old game and its players.
- `RestoreRecentlyUndone` returns the old game object itself. Its history events were re-initialized by `Undo`, so they now belong to the undo game. So do the events its state holds (`CurrentBid`, `CurrentBattle`, `AggressorPlan`, `DealOffers`, ...).
- C# keeps every game alive that any of these references reaches.

The Rust model:

- Every `Game` has a process-unique `id: GameId`. `GameRef` and `PlayerIndex` carry that id, so `==` is reference equality across games.
- `Game.other_games` owns every game this one was made from by undo, restore or dismiss that C# would still reach, flattened (a game in `other_games` has none of its own). `previous_game_state` is the id of a game in `other_games`. After each undo, restore and dismiss, `drop_unreachable_games` drops the games that no `PreviousGameState`, event `Game` or `Player` reference reaches any more (found with the dump's walk, `canon::referenced_games`), as C#'s garbage collector would. Without it a long game with many undos keeps every earlier game in memory.
- `game.undo(n)`, `game.restore_recently_undone()` and `game.dismiss_recently_undone()` are C#'s `game = game.Undo(n)` and so on: they put the resulting game in `game`'s place and move the old one into its `other_games`. If an exception escapes (`Undo(-1)`), `game` is the old game again, as the C# caller still holds it, and the half-built game stays reachable from it.
- `game.player(p)` and `game.player_mut(p)` resolve a player of another game through `other_games`. A write to an event's `Player` therefore changes the other game's player, as in C#, and a comparison with a player of this game is false.
- A C# member that runs on an object holding its own game reads that game, not the one the caller has:
  - `Player` methods that take `&Game` start with `game.enter_own_game(self.game)`.
  - `Player` methods that take `&mut Game` call themselves through `game.with_game_mut(me.game, ...)`, which puts that game in this place for the call, with every other game held by it.
  - Event members that game code calls on events it holds do the same with `self.game_ref()`: the `BattleInitiated` members, `Battle.GetBattlePlanMessage`, `GetPlanSummary` and `ActivateDynamicWeapons`, `TreacheryCalled.Succeeded`, `ClairVoyancePlayed.Parameter1`, `DealOffered.Acceptance`/`Rejection`, and `GameStatus`'s flash messages. `ILocationEvent.To` of a held event (`LastShipmentOrMovement`, `Intrusion.TriggeringEvent`) is `e.to_location(game)`, not `e.i_location_event_to(game)`. When you port such a member, start it with:

  ```rust
  let own = game.enter_own_game(self.game_ref()); // GameEvent.Game: another game after Undo
  let game: &Game = &own;
  // or, for a member that changes its game:
  if let Some(id) = game.other_game_id(self.game_ref()) {
      return game.with_game_mut(id, |own| self.member(own, args));
  }
  ```

  `enter_own_game` (rather than `own_game`) lets the other game resolve players of every game this one holds, for example an event of the undo game that kept a player of this game.
- Each game's map has its own `Location` and `Territory` objects. A handle carries the id of the game whose map it came from (`map_game`), or 0 for the map of the game in this place. When a game moves into `other_games`, its players' locations and homeworlds get its id; when it comes back, they lose it. The `&mut Game` `Player` methods tag the map objects they pass to a player of another game, `Location::territory` keeps the location's map, and members that read another game's map tag what they return. C# compares territories by reference, so `Territory`'s `==` compares maps (a territory of the game entered with `enter_own_game` counts as untagged). C# compares locations by id, so `Location`'s `==` ignores the map.
- The state dump writes a `GameRef` to another game as `{"$ref":"OtherGame"}`, a player of another game in full, and a map object of another game's map with `"foreign":true`.

## Events

### Representation

- Each C# event class is a struct in `src/events/<name>.rs` with its fields in C# order, the fields inherited from `PassableGameEvent`/`PlacementEvent` inlined, and the `GameEvent` fields in `base: GameEventBase` (`Initiator`, `Time`, `Game`, `Player`).
- A C# `GameEvent` reference is the enum `events::GameEvent`, with one variant per concrete event holding an `Rc`. Cloning shares the object; `==` and `Hash` are by address. A field typed as a concrete event (`Game.AggressorPlan`) is `Option<Rc<Battle>>`. A field typed as `IBid`, `ILocationEvent` or `IPlacement` is `GameEvent`.
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

`GameEvent` (the enum) has a dispatching method for every `GameEventMethods` member: `ev.validate(&game)`, `ev.execute(&mut game, true, false)`, `ev.get_message(&game)`. Interface members dispatch as `ev.i_bid_amount()` and `ev.i_location_event_to(&game)`, which throw `InvalidCastException` for an event that does not implement the interface. The interface traits (`IBid`, `ILocationEvent`, `IPlacement`) are not in the prelude, because their method names clash with the base traits.

### Signatures

- Every override of one virtual member, and every implementation of one interface member, has the same Rust signature. That is why `Validate` takes `&Game` even where an event doesn't read the game.
- `ExecuteConcreteEvent` and every method that stores or upcasts `this` take `self: &Rc<Self>`, so they can write `game.aggressor_plan = Some(self.clone())` or `game.last_shipment_or_movement = Some(self.clone().into_event())`.
- A parameter of a concrete event type is `&Battle`, so `this` can be passed as `self`. Where the callee stores the event, the parameter is `&Rc<Battle>`.
- Event methods only get a `game` parameter when the C# body reads `Game` or `Player`.

### Fields written after the event is shared

C# events are live objects. The analysis in the generator found these fields written after the event may be stored in game state or `History`; they are `Cell`s so `&self` methods can write them:

- `GameEventBase.initiator`, `GameEventBase.player` (`Initialize`)
- `GameEventBase.time` (set by the server's `ValidateAndExecute` after `Execute`; seeded by hand in the generator's analysis, which only sees `Shared`)
- `Battle.weapon_card_id`, `defense_card_id`, `original_weapon`, `original_defense` (`ActivateDynamicWeapons`, hazards §5.2)
- `AutomationConfigured.automation_rule_id`
- `Shipment.guild_income_prevented`

Setters of other fields take `&mut self`: call them before wrapping the event in `Rc`. If you find another C# write to a stored event, make that field a `Cell` and add it here.

When the event to call is itself stored in the game, clone the `Rc` first so the game is free to borrow mutably:

```rust
let plan = game.aggressor_plan.clone().nre();
plan.activate_dynamic_weapons(game, hero, weapon, defense);
```

## Signatures in the skeleton

The generator derived every signature from the C# declaration and a whole-program analysis of the bodies. The rules, so you can predict and check them:

- **`&self` or `&mut self`.** A `Game` method is `&mut self` when it, or anything it calls, writes game state. Reads that write (`NextPlayerToBattle`, hazards §4.3) count, so `next_player_to_battle(&mut self)` and `get_applicable_events(&mut self, ...)` are `&mut`. Writes to `Map`'s neighbour cache and draws from `Game.Random` do not count, because those have interior mutability in Rust.
- **Explicit `Game` parameters** become `&Game` or `&mut Game` by the same analysis. A method with a `Player` parameter and no other way to reach the game gets a `game: &Game` parameter, because a `PlayerIndex` is useless without it.
- **Nullability** follows the C# annotations: `Leader?` is `Option<Leader>`, `Leader` is `Leader`. A non-nullable `Leader` can still hold `Leader::NULL` (see [Null handles](#null-handles)). Exceptions: event fields of reference type are always `Option` (JSON can hold null); event members returning a handle, an event or a player are `Option` (they resolve ids that may not exist); and a method that contains `return null` returns `Option` whatever its annotation says.
- **Collections** are `Vec`, `NetHashSet` and `NetDictionary`, both for fields and for `IEnumerable` returns. The generator could not tell lazy returns apart; those are now `impl Iterator` (see [IEnumerable and laziness](#ienumerable-and-laziness)). Parameters are borrowed: `&[T]`, `&NetHashSet<T>`, `&NetDictionary<K, V>`, `&str`. `params T[]` is `Vec<T>`. `ref`/`out` parameters are `&mut T`. C# default parameter values are not represented; pass them explicitly.
- **Returned objects** of a model class (`Battalion`, `LeaderState`, `Report`, ...) are owned values. Where C# returns a live object that the caller mutates (`Game.CurrentReport`, `Player.GetAndCreateIfNeeded`, hazards §5.4), the porter of that method changes the return type to `&T`/`&mut T` (or adds a `_mut` variant) and fixes the callers.
- **Delegates.** `Action` is `fn(&mut Game)`, `Func<A, R>` is `&dyn Fn(A) -> R`, `object` is `ExpressionElement`.
- **Overloads.** The first overload keeps the name; later ones get the snake_case parameter types appended: `By(Faction)` is `by`, `By(Player)` is `by_player`. Every method's doc comment quotes the C# signature and `file:line`. C# `Clone()` is `cs_clone()`, so it never shadows `Clone::clone`.
- **Constants** with a literal value are associated `const`s; other static fields are functions with a `todo!`, because each is either a constant table or process-wide state (hazards §6.2) and needs a decision.

If a signature is wrong for the body you are porting, fix it and its callers in the same change. Callers in another package's files are fine to touch for this; keep the edit to the call site.

## Messages

`Message.Express(...)` and friends are macros. Every argument goes through `ExpressionElement::from`:

```rust
use crate::{express, express_to, express_part, express_part_if};

// return Message.Express(Concept.Resource, " transfers are disabled");
return Some(express!(Concept::Resource, " transfers are disabled"));

// Message.ExpressTo(Faction.Atreides, "You see ", card)
express_to!(Faction::Atreides; "You see ", card);

// MessagePart.ExpressIf(amount > 0, " for ", Payment.Of(amount))
express_part_if!(amount > 0; " for ", Payment::of(amount));

// Cards.Select(c => MessagePart.Express(" ", c, " "))
ExpressionElement::list(cards.iter().map(|c| express_part!(" ", *c, " ")));

// .WithLog(new StormLog(...)), .AsHeading(Concept.Monster)
express!(...).with_log(StormLog { /* ... */ }).as_heading(Concept::Monster);
```

What converts:

- `&str`, `String`, `bool`, `i32`, `i64`, `f32`, `f64`.
- Every enum.
- `Location` and its subtypes, `Territory`, `Leader`, `Hero`, `TreacheryCard`, `ResourceCard`.
- `Payment`, `Message`, `MessagePart` and `CsException`.
- `Option<T>`, where `None` is null.
- `Vec<T>`, `&[T]` and `[T; N]`, which become a list element.

Keep the C# numeric type: a `float` dial must go in as `f32`, not `i32`. If something new needs to go in a message, add a variant and a `From` impl in `expression.rs`.

### Collections in messages

A C# message keeps a collection argument by reference, and the oracle's state dump enumerates it at every step. A collection from game state that changes after it was logged shows its new contents in C#. A lazy query runs again at each dump. A Rust message stores a copy, so check each collection you pass:

- If C# passes a fresh collection (`ToList()`, a getter that builds a new list), or one that nothing changes later, a copy is equivalent. Pass a `Vec`, or `ExpressionElement::collection(type_name, items)` for a lazy query, with the runtime type name .NET prints.
- If C# passes a collection that lives in game state and changes later, use `ExpressionElement::live(game, LiveList::...)`. The variant names where the collection lives, such as `PlayerTreacheryCards(p)`. The state dump (`CanonWriter::for_game`) writes the game's current contents for it. To add a variant, give it a `type_name`, a `read` and an entry in `LiveList::all`. This is only correct while C# never reassigns that field, so check that first.

[live-lists.md](live-lists.md) lists every such C# site with its class. `tests/live_lists.rs` fails for a site without a row there. We chose named variants over storing a shared `Rc<RefCell<Vec<T>>>` that the game mutates. The shared version would have changed the type of every game field that a message might log, and three fields need it.

The macro evaluates its arguments left to right before it creates the message, so nested messages get lower `Nr` values, as in C#. `express_part_if!` evaluates its arguments even when the condition is false, as C# does. Messages are data. Turning them into text is `Skin.Describe`, in the `rust/skin` crate ([skin.md](skin.md)).

`Validate()` returns `Option<Message>`, where `None` means valid.

## Exceptions

A C# exception is a Rust panic whose payload is `cs::CsException { type_name, message }`. The replayer runs each event inside `cs::catch`, which turns that panic back into a value. Hazards §1 requires that a port stop at the same statement, with the same exception type and message, and keep every state change made before the throw. A panic at the C# throw site gives all three without touching any signature.

The alternative was `Result` plus `?` everywhere. Nearly every member access can throw a NullReferenceException, and many throws happen inside LINQ lambdas. With `Result`, most getters and closures would become fallible, and ports would stop reading like the C#.

```rust
use crate::throw;
use crate::cs::{self, NullRef, CsIterExt};

throw!(InvalidOperationException, "Cannot draw from an empty deck");   // throw new ...
throw!(Exception, "Could not find a player after {}", faction);        // $"... {faction}"

let ally = player.allied_player().nre();         // Player.AlliedPlayer!.X  -> NullReferenceException
let x = *cs::index(&dials, 1);                   // Dials[1]                -> ArgumentOutOfRangeException
let first = items.iter().cs_first();             // .First()
let m = items.iter().cs_first_where(|c| c.is_weapon());   // .First(pred) -> "no matching element"
let one = items.iter().cs_single_or_default();   // .SingleOrDefault()
let q = cs::div(total, players);                 // int / int               -> DivideByZeroException
let r = cs::rem(i, n);                           // int % int

// try { ... } catch (Exception) { ... }   (bots, Skin.Format)
match cs::catch(|| risky(game)) { Ok(v) => v, Err(_e) => fallback() }
```

Rules:

- Wherever C# can throw, implicitly or explicitly, call the helper that throws the same .NET exception at the same point. Never `unwrap()`, `expect()` or index a slice where C# could throw. Those produce a plain Rust panic, which the replayer treats as a porting bug and does not catch.
- `x!` does not throw in C#. Only a member access does. Port `list.Add(x!)` and `f(x!)` with `x.or_null()`, not `x.nre()` (see [Null handles](#null-handles)).
- Keep the statement order. The mutations before the throw are the specified behaviour.
- Exception messages are the exact .NET texts. `cs.rs` has the common ones. Hazards §1.4 and §1.5 list the reachable sites with their messages.
- The "Game Error" message of `GameEvent.Execute` puts the exception object in the message. That is `ExpressionElement::Exception`. Compare type and message only.
- `Dictionary` indexer misses must throw `KeyNotFoundException` (`cs::key_not_found`). Until `NetDictionary::index` throws a `CsException` itself, check with `get` and call `cs::key_not_found(&key_text)`.

## IEnumerable and laziness

Rust iterator adapters are lazy and pull one element at a time, like LINQ. `filter`, `map`, `take`, `find` and `any` interleave side effects in the same order as `Where`, `Select`, `Take`, `First` and `Any`. So:

- Methods returning `IEnumerable<T>` over static data return `impl Iterator<Item = T> + Clone + 'static` (see `Map::locations`).
- Methods returning a lazy query over game state return `impl Iterator<Item = T>` too, built from the same adapters with `move` closures. Callers stop where C# stops: `Any()` is `.next().is_some()`, `Contains(x)` is `.cs_contains(&x)`, `FirstOrDefault()` is `.next()`. A `Vec` would run the predicate on elements that C# never reaches, and those can throw (a null card after the match). What C# evaluates at the call (the receiver of the first operator) goes before the iterator; what it evaluates inside a lambda (`p.Faction` of a possibly null `p`) goes inside the closure. [lazy-queries.md](lazy-queries.md) has the full rule, the helpers in `lazy.rs`, and every site with its class; `tests/lazy_queries.rs` fails for an unclassified site or a short-circuited producer that returns a collection.
- Where C# enumerates the query later, after a mutation, or more than once (hazards §4), the Rust code must re-run the query at that point. Write a comment saying so.
- `ToList()` is `collect::<Vec<_>>()`. `Distinct()` is `cs_distinct()`, which keeps the first occurrence and uses `==`.
- `OrderBy`/`ThenBy` are stable sorts that compute each key once, in element order, before sorting. Use `sort_by_cached_key`, or compute the keys first, when a key has side effects such as random draws. `List.Sort` is not stable; use `dotnet_compat::sort::list_sort_by`.
- `Dictionary` and `HashSet` are `dotnet_compat::NetDictionary` and `NetHashSet`. `std::collections::HashMap` and `HashSet` are banned. A small lookup-only collection can be a `Vec` scan, as `Map`'s neighbour cache is.
- C# returns `default` for "OrDefault" operators: null for classes, but 0 or `false` for value types. Rust returns `Option`, so write `.unwrap_or_default()` at value-type call sites.

## Randomness

```rust
// Game.Random = new LoggedRandom(Seed);
game.random = Some(SharedRandom::from_seed(seed));

// new Deck<TreacheryCard>(cards, Game.Random)  (Random may still be null in C#)
let mut deck = Deck::new(cards, game.random.clone());
deck.shuffle();                                      // same draws as Deck<T>.Shuffle
let card = deck.draw();                              // throws on an empty deck

// player.TreacheryCards.RandomOrDefault(Game.Random!)
use crate::extension_methods::RandomExt;
let c = player.treachery_cards.iter().copied().random_or_default(game.random.as_ref().nre());
```

`RandomOrDefault` only draws when there are two or more items. The bots' overloads that use C#'s unseeded static `Random` are the `*_unseeded` methods. They need `set_static_random_seed` and a matching C# hook before bot traces can be compared.

## Process-wide state

`Message.Nr` comes from a counter that C# never resets. In Rust it is thread-local (`Message::reset_counter`, `Message::counter`), so parallel tests don't interfere. The oracle omits `Nr` from state dumps.

## The skeleton generator

`engine/csharp-gen` (Roslyn plus reflection over `Shared`) wrote the skeleton: every state type with every instance field, every event, and every member signature with a `todo!("port: File.cs:line")` body. [port-assignments.md](port-assignments.md) lists the files.

```sh
export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"
cd rust/engine/csharp-gen && dotnet run -- --glue
```

It never overwrites an existing module, because porters own those files now. `--glue` rewrites only the files nobody edits by hand: `events/mod.rs` (the `GameEvent` enum and dispatch), `log/mod.rs`, `prelude.rs` and `json_members.rs`. Run it after changing an event's fields. `--force` overwrites everything and must not be used any more. `out/` gets the warnings, the mutation reasons (`why.txt`: why a method was judged mutating) and the list of `Cell` fields.

Every generated module starts with `use crate::prelude::*;`, which brings in the enums, handles, collections, `Game`, `Player`, `PlayerIndex`, all events and all model types.

## JSON

`json.rs` writes and reads System.Text.Json exactly as `Utilities.Serialize`/`Deserialize` do (hazards §12): member order, escaping, number and date formats, the `$type` rules, missing members keeping their initializers, unknown members ignored. A `$`-prefixed member other than the leading `$type` is rejected in an event (a polymorphic type) and skipped as an unknown member anywhere else (`GameSettings`, `GameState`), as .NET does. `json_members.rs` holds each event's member list, generated from `JsonSerializerOptions.GetTypeInfo`.

```rust
let text = json::serialize(&event);                       // Utilities.Serialize<GameEvent>(e)
let event: GameEvent = json::deserialize(&text)?;         // Utilities.Deserialize<GameEvent>(text)
let state: GameState = json::deserialize(&savegame)?;     // without the legacy path, see below
```

`tests/event_json.rs` checks every event type with random member values, plus malformed variants, against the C# serializer: same accept/reject decision, byte-identical re-serialization. Regenerate its corpus after changing an event:

```sh
cd rust/engine/csharp-ref && dotnet run -c Release -- --events ../tests/golden/events.jsonl
```

Known gaps, all on malformed input: a duplicate `$type` is not detected, nesting is limited at 128 instead of 64, `-0.0` is read as an integer, and a date without an offset is read as UTC where .NET uses machine-local time. Error messages are not .NET's.

`Utilities::deserialize` includes the legacy Newtonsoft path (hazards §12.5), checked against .NET in `tests/pkg_a_core.rs`, and `GameState::load` does the `Treachery.online` replacement. One known gap: `JsonNode.Parse` throws on a duplicate key, serde_json keeps the last value.

## State dump names

The oracle's state dump (`Oracle/Canon/StateDumper.cs`) writes every instance field under its C# name. Every state struct derives `Canon` (from `rust/canon-derive`), and the field attributes say how each Rust field maps back:

- No attribute: the C# name is the Rust name in PascalCase, with each `_` starting a new word (`resources_after_bidding` is `ResourcesAfterBidding`).
- `#[canon(name = "_toId")]`: anything else, including private fields and acronyms.
- `#[canon(flatten)]`: `base: GameEventBase`, whose fields belong to the event.
- `#[canon(omit)]`: fields the dumper leaves out (`LeaderState.TimeOfDeath`, `LoggedRandom._supply`).
- `#[canon(skip)]`: Rust-only fields: `Game.id` and `Game.other_games` (see [Game identity and undo](#game-identity-and-undo)).
- `#[canon(with = "crate::canon::history_count")]`: the dumper's special cases on `Game` (`History` as its count, `PreviousGameState` as present or not, `Random` expanded).
- `#[canon(manual)]` on a type: no derived impl. References (`PlayerIndex`, `GameRef`), `LoggedRandom`, the static-data handles and `Map` are written by hand in `canon.rs`.

A field you add while porting needs the same attribute treatment. A log record passed to `with_log` must derive `Canon`, and a new log record type must be added to the `log_records!` list in `canon.rs`. `cargo test -p differ --test field_mapping` compares every field name with the C# dumper and fails on anything missing or misnamed; [differ.md](differ.md) has the details.

## Not ported

- `Skin` and `DefaultSkin` are not in the engine. Engine logic never reads them, and `Message.DefaultDescriber` stays null here. They are ported in the separate crate `rust/skin` ([skin.md](skin.md)).
- `Deck<T>.Randomize`, which uses an unseeded random and has no callers.
- `Game.Clone`. Only the client's story view and the C# tests call it. `Map` derives `Clone` as an independent copy, whereas a C# clone shares its map with the original.

## Checking static data against C#

`engine/csharp-ref` is a small C# program that references `Shared`. It dumps every static table and many pathfinding results to `engine/tests/golden/static_data.json`, and `engine/tests/static_data.rs` rebuilds the same JSON from Rust. To regenerate it after a C# change:

```sh
export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"
cd rust/engine/csharp-ref && dotnet run -c Release -- ../tests/golden/static_data.json
cd .. && python3 tools/gen_enumerations.py && python3 tools/gen_map_data.py
```

When you port something static, add it to both sides of this test.

## dotnet-compat helpers

The compat crate is finished; [compat.md](compat.md) has the full API. The points that are easiest to get wrong:

- `NetDictionary::index` and `add` now throw `CsException` themselves (`KeyNotFoundException`, duplicate key with `Key: …`). Ignore older advice to check with `get` first. Key types need a `CsToString` impl, or use `index_with`/`add_with`.
- `MaxBy`/`MinBy` are `linq::max_by_key_first`/`min_by_key_first`. Never use `Iterator::max_by_key`, which returns the last maximum. Wrap in `linq::or_no_elements(...)` when the C# source element is a value type, because .NET throws there.
- `First(pred)`, `Last(pred)`, `Single(pred)`, `SingleOrDefault(pred)` are the `_where` variants. `filter` followed by `first` gives a different exception message.
- `ElementAt` is `element_at` on a lazy source and `element_at_list` on a List or array.
- `new Dictionary(d)` and `ToDictionary` are `from_dictionary`. `NetDictionary::clone()` keeps freed slots, which a C# copy does not.
- `new HashSet(x)` is `from_set`, `from_collection` or `from_enumerable`, depending on the C# type of `x`. The same goes for the `intersect_with_*` and `symmetric_except_with*` variants.
- Floats: `math::cmp_f32`/`cmp_f64` for comparisons and sort keys, `math::math_max_f32`/`math_min_f32` for `Math.Max`/`Math.Min`, `math::equals_f32` for `Equals`, `fmt::f32_to_string`/`f64_to_string` for `ToString()`. Where hazards.md says `max_f32`, `min_f32`, `f32_compare` or `float_equals`, it means these.
- Parsing (`int.Parse`, `Convert.ToInt32`, `float.Parse`, `Enum.Parse`) is in `dotnet_compat::parse`.

## Helpers added after the foundation review

The [foundation review](reviews/foundation.md) found differences in the support code. Its fixes added these helpers. [escapes.md](escapes.md) explains each bug and the test that now guards it.

### Exceptions and indexing

```rust
throw!(NullReferenceException);                         // throw new NullReferenceException(): .NET default message
cs::argument_out_of_range_actual("faction", &f.to_string()); // new ArgumentOutOfRangeException(nameof(f), f, null)
cs::with_parameter("bad", "x");                         // message of new ArgumentException("bad", "x")
let x = *cs::index_array(&parts, 1);                    // array[1]: IndexOutOfRangeException
*cs::index_mut(&mut list, i) = v;                       // list[i] = v
*cs::index_array_mut(&mut array, i) = v;                // array[i] = v
```

`cs::index` is for `List<T>` only. Arrays, including `string.Split` results, throw a different exception.

### LINQ predicate overloads

A predicate overload has its own exception text ("no matching element", "more than one matching element"). Never port `Single(p)` as `filter(p).cs_single()`.

| C# | Rust |
|---|---|
| `First(p)` / `FirstOrDefault(p)` | `cs_first_where(p)` / `cs_first_or_default_where(p)` |
| `Single(p)` / `SingleOrDefault(p)` | `cs_single_where(p)` / `cs_single_or_default_where(p)` |
| `Last()` | `cs_last()` |
| `Last(p)` / `LastOrDefault(p)` on a lazy sequence | `cs_last_where(p)` / `cs_last_or_default_where(p)` |
| `Last(p)` / `LastOrDefault(p)` on a `List<T>` or array | `cs::last_where_list(&list, p)` / `cs::last_or_default_where_list(&list, p)` |
| `ElementAt(i)` on a lazy sequence | `cs_element_at(i)` |
| `ElementAt(i)` on a `List<T>` or array | `cs::element_at_list(&list, i)` |

.NET walks a list backwards for `Last(p)`, so the predicate runs fewer times than on a lazy sequence. That matters when the predicate has side effects.

### Keys of the `HighestOrDefault` family

Selectors return any `cs::CsComparable` key: `i32`, `i64`, `bool`, every enum, `f32`, `f64`, `String` and `&str`. It carries both halves of the C# behaviour. Ordering is `Comparer.Default`, where NaN is lowest and strings compare by culture. Ties are `Equals`, where NaN equals NaN and strings compare ordinally.

`cs::compare_invariant` is the culture comparison. It handles ASCII only and panics on anything else. Each helper calls the selector exactly as often as the C# query does.

`take_random_n` returns a lazy iterator. It draws nothing until iterated, and draws again for each clone that is iterated.

### Messages

- A collection element records its C# runtime type for `ToString()`:
  - `Vec<T>` and `&[T]` become `List<T>` (``System.Collections.Generic.List`1[T]``).
  - `[T; N]` becomes an array (`T[]`).
  - `ExpressionElement::collection(type_name, items)` covers anything else, such as a LINQ iterator.
  - Element types implement `expression::CsType`.
- A single reference-type array passed to `Message.Express` (`object[]`, `string[]`, `Location[]`) is the element list itself. Port it as `express!(@params array)`. `express_to!` and `express_part!` take `@params` too. A value-type array or a `List<T>` stays one element.
- Numbers print through `dotnet_compat::fmt` under the invariant culture. Exceptions print as `Exception.ToString()` without a stack trace (`cs::exception_to_string`).
- Record equality follows C#. `PlanSummary` and `BattleFact` treat NaN as equal to NaN. `Route.from` is a `Shared` list, compared by reference.
- `TreacheryCard::countered_by` takes `Option<TreacheryCard>` for the defense and throws only on the branches that read it.

### Golden tests that grow with the port

One `csharp-ref` run writes every golden file next to `static_data.json`:

```sh
cd rust/engine/csharp-ref && dotnet run -c Release -- ../tests/golden/static_data.json
```

| Golden file | Test | Checks |
|---|---|---|
| `boundary.json` | `tests/boundary.rs` | Every public member of every type in `tests/coverage/ported_types.txt`, called with boundary inputs |
| `random_helpers.json` | `tests/random_helpers.rs` | Results, selector calls and draws of every random helper |
| `linq.json` | `tests/linq_table.rs` | Every `cs.rs` helper against the .NET operator, plus exception default texts |
| `messages.json` | `tests/message_corpus.rs` | `Message.ToString()` of every element kind, and `params` spreading |

When you port a C# type, add it to `ported_types.txt` and regenerate. `boundary.rs` then lists every member that needs an entry in its table, or a line in `tests/coverage/not_ported.txt`. A `Covered("test name")` entry is accepted only for members that never throw on the generated inputs.

The tests loop over the C# entries. A new enum, operator or helper on the C# side fails the Rust test until it is handled there.

## When the C# changes

`tests/csharp_sync.rs` fails as soon as any `.cs` file in `Shared` or `Bots` differs from the version the port is synced to, and it names the files. Port the change, regenerate every derived file and update the manifest as described in [csharp-sync.md](csharp-sync.md).
