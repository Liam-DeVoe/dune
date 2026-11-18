# Engine conventions

How `rust/engine` represents a game, and the rules to follow when you change it. [engine.md](engine.md) lists the crates and the modules.

## Names and comments

- Most engine code starts with `use crate::prelude::*;` (`prelude.rs`), which brings in the game, every event, the log records and the common types.
- The rules were ported from a C# engine, and much of the code keeps that engine's names (in snake_case) and order. Where a C# overload had one name, the Rust names say how the versions differ: `Game::discard_card(card, log)` and `discard_card_of(player, card, log)`, `GameStatus::for_player` and `for_players`.
- Comments describe the Rust code. Two kinds of comment mention the old engine, and you will meet both: `// Not in C#: ...` marks a deliberate difference, listed in [behavior-changes.md](behavior-changes.md), and `// C# parity: <why>` marks a bug that is kept on purpose. Mark a kept bug the same way.
- A name that is a Rust keyword gets a trailing underscore: `card.type_()`.
- `map/data.rs` and the enum declarations in `enumerations.rs` are edited by hand. `tests/static_data.rs` checks them against a snapshot.

## Game state

`Game` (`game/mod.rs`) holds a few fields of its own (`seed`, `rng`, `players`, `human_seats`) and groups the rest by topic, one struct per topic. Each struct is defined in the file under `game/` that holds the rules of its topic:

| Field | Type | File | Holds |
|---|---|---|---|
| `rules` | `Rules` | `rules.rs` | version, ruleset, the rules in effect (`active`), settings, factions in play, automation rules, hidden secrets |
| `board` | `Board` | `board.rs` | the map, the storm and battle wheels, spice, the latest spice cards, worms and sandtrout, terror tokens, ambassadors, discoveries, stronghold ownership, homeworld occupation |
| `decks` | `Decks` | `decks.rs` | every deck and discard pile, the Richese cache, recently discarded cards |
| `leaders` | `Leaders` | `leaders.rs` | leader state, captured and assassinated leaders, Vidal |
| `alliances` | `Alliances` | `alliances.rs` | alliance offers, what each faction allows its ally, shipment permissions |
| `deals` | `Deals` | `deals.rs` | `offers`, `responses` and the deals in force (`agreed`) |
| `turn` | `TurnState` | `turn.rs` | turn and main phase, and what lasts for a turn or longer: payments, milestones, prevented advantages, cards and Nexus cards in effect, the intrusion queue, storm losses to take |
| `phase` | `PhaseState` | `phase.rs` | the phase the game is in, with that phase's own state ([Phases](#phases)) |
| `interrupts` | `Vec<Interrupt>` | `phase.rs` | what suspends the phase, the latest last ([Phases](#phases)) |
| `history` | `History` | `history.rs` | the played events (`entries`), undone events, the game before an undo, moments and timers |
| `log` | `Log` | `log.rs` | the reports |
| `counters` | `Counters` | `counters.rs` | numbers handed out in order, such as the leader deaths' `time_of_death` |

A new field goes into the state of its phase if only that phase reads it ([Phases](#phases)). Otherwise it goes into the struct of its topic, or into `turn` if it lasts a turn.

`human_seats` is all the engine knows about who sits at the table: the seats users play. Every other seat is a bot's (`Game::is_bot`), which a few rules ask (Clairvoyance questions, card trades, deals with a bot). The server owns everything else about seating (names, hosts, observers, open seats, bot speed; `server/src/participation.rs`) and updates `human_seats` whenever seats change. The seats aren't part of the history: `Game::load` takes them, `Game::new_like` copies them (and the deal-id numbering, `Deals::continue_ids`) into a game that replays the history, and `restore_recently_undone` keeps the current ones.

Game state is plain owned data. `Game` holds no `Rc`, `RefCell` or `Cell`, so it is `Send`; a compile-time check under the struct in `game/mod.rs` keeps it that way. `Game` is not `Clone`: to get a second game, replay the history into `game.new_like()`. No game state lives outside `Game`, so games on one thread don't affect each other. The two statics that look like exceptions aren't game state: `EntryId::fresh` (`events/rules.rs`) hands out process-unique ids, and the thread-local `QUIET` in `error.rs` only tells the panic hook whether to print.

## Phases

The game is always in one phase state, and on top of it may sit a stack of interrupts (`game/phase.rs`).

- `game.phase` is a `PhaseState`: `AwaitingPlayers`, `Setup(SetupState)`, `Storm(StormState)`, `Blow(BlowState)`, `Charity`, `Bidding`, `Revival`, `ShipMove`, `Battle(BattlePhase)`, `Collection`, `Contemplate` (the Mentat Pause), or `Ended(Ending)`. Each state struct holds a `step` and the fields only that phase uses. They are dropped when the phase ends, so nothing has to reset them.
- `game.interrupts` is a `Vec<Interrupt>`: things that suspend the phase until they are resolved, each with the data only it uses. Examples are `Discarding { factions }`, `CardTrade { offer }`, `StormLosses`, `Intrusion { kind, cause }` and `AllianceByAmbassador { .. }`.
- `game.phase()` is the flat `Phase` enum: the top interrupt's (`Interrupt::phase`), or else the phase state's (`PhaseState::phase`, which asks the step). Views, the client, the bots and the server's pause rules all key on it, so its names are part of the wire format.

Each phase file under `game/` follows one pattern. Take `charity.rs`:

- `CharityState { step, .. }` and `CharityStep`, whose `phase()` maps steps to flat phases. Phases whose code needs the reverse also have `of(phase)` (blow, battle, ship and move, contemplate).
- `game.charity()` and `charity_mut()` return the state while the game is in the phase (under any interrupts), else `None`. Some phases also have a crate-only `storm_state()`-style accessor that panics outside the phase, for code that only runs inside it.
- `begin_charity()` starts the phase, and `enter_charity(step)` moves to a step, starting the state if the game isn't in the phase yet. Both drop every interrupt and end the deals that last until the new phase (`Game::remove_ended_deals`), through `Game::enter_state` or by hand.
- `GameStatus::charity_status(game, me, phase)` gives the prompt, and `Game::add_charity_actions(player, is_host, result)` adds the event types a player may submit. Both match on `game.phase()`, because an interrupt on top changes what they show. `GameStatus::determine_status` (`status.rs`) and `Game::add_player_actions` (`applicable_events.rs`) call each phase's function in turn.
- The host's `EndPhase` (`events/host/end_phase.rs`) moves the game on from each step that waits for it.

Interrupts work like a stack:

- `game.interrupt(i)` pushes one. Push it where its data is known, as the last thing the caller does.
- The event that answers it calls `game.resume(i, cleanup_deals)`, or `resume_phase(phase, ..)` for the latest interrupt showing `phase` whatever its data. That drops it and anything pushed after it, and the game shows what is underneath again.
- Accessors read the data of the latest interrupt of a kind: `factions_that_must_discard()`, `card_trade_offer()`, `traitors_to_discard()`, `target_of_bureaucracy()`, with `_mut` versions.
- `interrupt_status` and `add_interrupt_actions` in `phase.rs` give the prompt and actions of the interrupts that aren't tied to one phase. Intrusions and alliance offers have theirs in `movement.rs`.

To change a phase, change its state struct, its step enum and the functions above in its own file. To add a step or an interrupt:

1. Add the variant (with its data) to the phase's step enum or to `Interrupt`, and its arms in `phase()`, and in `of()` if the phase has one.
2. If it needs a new flat phase, add it to `Phase` in `enumerations.rs`. That changes `web/src/generated/enums.ts` (regenerate it, see [JSON](#json)), and the skin needs to name it.
3. Add its prompt and actions in the phase's status and actions functions, and its `EndPhase` arm if the host moves it on.
4. Check who reads the flat phase: the bots (`rust/bots`), the views (`rust/view`), and `server/src/pause_rules.rs`, which decides what each pause between phases waits for.

Some state outlives its phase on purpose and lives elsewhere: in `turn` (the intrusion queue `turn.intrusions`, storm losses still to take) or in `board` (the latest spice cards, worms). Before you move such a field into a phase state, check every reader. The bots read some of them long after the phase.

## Ids and newtypes

Numbers that name something get their own type:

| Type | What | Notes |
|---|---|---|
| `PlayerId(u8)` | a player: its index in `game.players` (`game/player.rs`) | Players are only appended, so the index never changes. `game.player(p)`, `game.player_mut(p)`, `game.player_at(i)`, `game.player_indices()` |
| `Seat(u8)` | a place at the table, from 0 (`game/seats.rs`) | `Player.seat` is `Option<Seat>`, but `EstablishPlayers` seats every player, so game code reads `player.seated()`. Where JSON needs an absent seat it writes -1 (`seat_or_minus_one`) |
| `Turn(u8)` | a turn, from 1 (`game/turn.rs`) | `Turn::BEFORE_START` (0) is the setup, a real moment with its own report, so it isn't `Option<Turn>` |
| `Sector(u8)` | one of the 18 sectors, 0 to 17 (`map/sector.rs`) | Homeworlds and unplaced attached locations have none, so `Location::sector` is `Option<Sector>`. The storm's arithmetic modulo 18 is only in `Sector::moved`. Views and the log write -1 for none |
| `DealId(usize)` | a deal offer, numbered in the order offers are made. An undo keeps the ids of the offers it replays, and no id is given twice in a game (`game/deals.rs`) | Keys `deals.responses`; the deals view's `id` |
| `EntryId` | one submission of an event (`events/rules.rs`) | See [State instead of events](#state-instead-of-events) |
| `HalfPoints` | a battle strength or dial | See [Battle strength](#battle-strength) |
| `UserId(i64)` | a user | In `treachery-protocol` only. The engine knows seats, not users |

`Event` fields still hold some of these as plain numbers where the client sends any number: `FactionSelected.seat` is an `i32`, and a number that is no player's seat selects nobody.

### Static game data: handles

Territories, locations, leaders, the Messiah, treachery cards and resource cards are small `Copy` handles into static tables, compared by id: `Location`, `Territory`, `Leader`, `TreacheryCard`, `ResourceCard`, and `Hero` (`enum Hero { Leader(Leader), Messiah, TreacheryCard(TreacheryCard) }`). `Homeworld`, `DiscoveredLocation`, `HiddenMobileStronghold` and `AttachedLocation` deref to `Location`, and `.into()` gives the plain `Location` that messages, lists and comparisons take. Type tests are methods: `l.as_homeworld()`, `hero.as_leader()`.

```rust
LeaderManager::get_leaders(Faction::Emperor)
TreacheryCardManager::items()
game.board.map.locations(false)
game.board.map.strongholds()
game.board.map.arrakeen()
```

A `ResourceCard`'s identity is its position in the full list of resource cards, so the six Shai-Hulud cards stay distinct.

An event field that names one object holds the handle: `Shipment.to` is an `Option<Location>`, `Battle.hero` an `Option<Hero>`. Some event structs default such a field to the object with id 0 rather than `None` (`Move::default()` names location 0, and a default `Shipment` names card 0 as its Karama card), because the old engine left the id at 0. Leaders start at id 1001, so a leader field defaults to `None`.

### Absent values

A handle or id always names a real object. Where a value can be absent, the type is `Option<_>`, and collections never hold an absent value. What to do when a value is missing:

- Absence is part of the rules (no ally, no current battle, a field the client left out): keep the `Option` and handle `None`.
- An executed event needs the value, because it passed validation: `.required("what")?` (`error::Required`), which fails the event with a Game Error before it changes anything more.
- Anywhere else, a valid game always has the value: `.expect("what")`. Reaching it is a bug, and the panic says what was missing.

### Map state

Most of the map never changes. Two things do, and they live in the game's `Map` (`game.board.map`): the neighbour lists, because the Hidden Mobile Stronghold and discovered locations get attached to other locations, and what each attached location is attached to. So the location properties that depend on them take the map: `l.sector(&game.board.map)`, `l.neighbours(&game.board.map)`, `t.is_visible(&game.board.map)`. Everything else (`territory()`, `is_stronghold()`, `spice_blow_amount()`) is static.

`Map` reads the game through the `map::MapGame` trait, because Rust can't pass `&Game` while `game.board.map` is borrowed through it. Pathfinding is `Map` methods, and keeps no cache:

```rust
game.board.map.find_neighbours(from, max, false, faction, &game, true)
game.board.map.find_paths(from, to, dist, ignore_storm, faction, &game)
```

## Amounts of spice and forces

Amounts the game holds are `u16`: a player's spice (`resources` and the other spice fields of `Player`), spice on the board (`board.resources_on_planet`), the forces in a `Battalion`, and the forces in the tanks (`forces_killed`, `special_forces_killed`, `total_forces_killed_in_battle`). The rules compute with `i32`, so reads widen: `i32::from(p.resources)`, `battalion.forces()`, `game.resources_in(l)`, `player.forces_in(Some(l))`.

Every change goes through `game/amounts.rs`:

```rust
amounts::spend(&mut game.player_mut(me).resources, cost, "spice")?;     // a Game Error if short
amounts::gain(&mut game.player_mut(p).resources, income, "spice").expect("income");  // outside GameResult
Player::move_forces(game, me, from, to, n)?;      // Battalion::change_forces, then amounts::change
game.change_resources_on_planet(l, -collected)?;
```

A change that would go below zero is a Game Error, never a negative amount. So every `Player` function that moves, kills or revives forces returns `GameResult`. If a player can cause the error, refuse the event in `validate` instead. Where the rules say "as much as possible", take `min(amount, have)` first, as `Player::some_forces_in_territory_to_reserves` and spice collection do. Never write `+=` or `-=` on these fields: in a release build a `u16` wraps silently.

Amounts in events are `u16` too, and serde refuses one outside 0 to 65535, so validation never sees a negative amount. Code that computes an amount as `i32` for an event, as the bots and the view's forms do, sets it with `amounts::clamp_u16`. A few event fields stay `i32` because a negative value means something, such as a shipment back to reserves ([behavior-changes.md](behavior-changes.md), "Turns, sectors and amounts out of range").

## Battle strength

The engine holds no floats. A battle strength or dial is `HalfPoints` (`game/half_points.rs`), a whole number of halves, because a force without spice support counts half. It is signed: a Bureaucrat's penalty can take a total below zero, and -1 loses to 0. It prints as `13` or `2.5`.

```rust
let dial: HalfPoints = plan.dial(game, opponent);
Battle::determine_normal_force_strength(game, f) * forces   // HalfPoints * i32
HalfPoints::whole(hero_strength) + dial
dial.floor()                                                 // for Sietch Tabr's spice
half_rounded_up(n)                                           // for costs
```

Views write strengths as JSON numbers (`expr::strength_f64` in `rust/view`). The bots estimate with `f32` and convert with `support::f32_of`.

## Enums

Every game enum is a closed Rust `enum` declared with `game_enum!` (`enum_type.rs`, declarations in `enumerations.rs`), with fixed discriminants. A name that names no member doesn't decode, so event JSON holding `"Nobody"` for a faction is refused.

Where an enum needs a "no value", the type is `Option<X>`, and the declaration marks the old member as `absent None = 0;` so that texts can still name it:

- event JSON writes `None` as `null`;
- a message logs it as the `None` member (its `Part` variant holds an `Option<X>`), so the skin names it. An `Option<Option<X>>` logs its outer `None` as nothing (`Part::Null`);
- views, the lobby data and skin keys name it "None" (`enum_type::name_or_absent`).

```rust
let skill: Option<LeaderSkill> = game.skill(Some(hero));
if skill == Some(LeaderSkill::Bureaucrat) { /* ... */ }
let n: i32 = LeaderSkill::Bureaucrat as i32;        // or x.value()
let s = LeaderSkill::from_value(n);                 // Option<LeaderSkill>: None if undefined
name_or_absent(skill);                              // "Bureaucrat", or "None"
for w in World::ALL { /* ... */ }
```

- `ALL` is sorted by value and leaves the absent member out. `Rule` and `Ruleset` are declared out of value order, so never rebuild that list from declaration order.
- `Ord` compares values. `Option<X>` sorts `None` first.
- `Display` prints the member name. Use `name_or_absent` for an `Option<X>`.
- A `None` member that is a real choice stays a member: `DealType::None` is a custom deal, and the lobby enums (`UserStatus`, `SubscriptionType`) keep theirs. Where "no value" is offered as a choice, the list says so: `ClairVoyancePlayed::valid_questions` starts with `None`, "any question".
- `ShipmentPermission` is a struct of three bools (`cross`, `to_homeworld`, `guild_rate`) that implements `GameEnum` by hand.
- To declare an enum outside `enumerations.rs`, use `game_enum!` with `closed` (see `BattleFactKind` in `messages/message.rs`), and list `get_values` in value order.

### Factions

A player has no faction until faction selection (`Player.faction` is `Option<Faction>`), and neither has the host. So a submitter is `Option<Faction>`, and so are `Player.ally`, `Player.nexus` and most event fields that name a faction. Helpers for that:

```rust
game.is_playing(Faction::Fremen);  game.is_playing(by.faction);   // impl Into<Option<Faction>>
player.is(Faction::Atreides);                                     // false without a faction
game.turn.has_acted_or_passed.contains_opt(by.faction);           // iter::ContainsOpt
game.bidding().and_then(|b| b.bids.get_opt(faction));             // iter::MapOpt
let me = by.required_faction()?;                                  // an executed event has one
```

## Players

A player is a `PlayerId`, and `Game.players: Vec<Player>` holds them. Hold the id, never a `&mut Player` across a call that needs the game:

```rust
let me = by.player();
amounts::spend(&mut game.player_mut(me).resources, cost, "spice")?;
game.stone(Milestone::Bid);
```

`Player`'s methods take the form their work needs:

| The method | Signature | Call |
|---|---|---|
| reads only the player | `fn has_ally(&self) -> bool` | `game.player(p).has_ally()` |
| changes only the player | `fn f(&mut self, ...)` | `game.player_mut(p).f(...)` |
| reads the game | `fn allied_player(&self, game: &Game) -> Option<PlayerId>` | `game.player(p).allied_player(game)` |
| changes the game | `fn revive_forces(game: &mut Game, me: PlayerId, amount: i32) -> GameResult` | `Player::revive_forces(game, p, 3)?` |

Inside a `&self` method that needs its own id, use `game.index_of(self)`. Other objects inside `Game` that change with the game in hand work the same way. `PlayerSequence::next_player(&mut self, game: &Game)` is called by taking the sequence out and putting it back:

```rust
let mut seq = game.battle().required("the battle phase")?.order.clone();
seq.next_player(game);
game.battle_mut().required("the battle phase")?.order = seq;
```

## Events

Events are plain data. The `events!` list in `events/mod.rs` names every event type once.

### What every event has

- `events!` (`events/macros.rs`) declares `Event`, an enum with one variant per event type, and `EventType` (with `ALL`, `name()`, `from_name()`, `default_event()` and `is_passable()`). It also gives `From<X> for Event` for every event struct, `Event::event_type()`, the dispatch to each event's rules, serde and the TypeScript union.
- An event struct derives `Clone, Debug, PartialEq, Serialize, Deserialize, TS` and implements `Default`. It holds neither its submitter nor a time.
- A played event is a `HistoryEntry { by, at, event, id }` (`events/rules.rs`). `by` is the submitter's faction (`None` for the host, or a player before faction selection), `at` the time, and `id` an `EntryId`. `game.history.entries` holds them.
- The rules see the submitter as a `By { faction, player }`: the faction and that faction's player when the event was submitted. `By::new(game, faction)` looks the player up once, before validation, so it stays the submitter even when the event changes who plays which faction.

### Playing an event

```rust
game.submit(by, event, at, is_host)?;               // Result<(), SubmitError>
game.play(entry, validate, is_host)                  // the same with a whole HistoryEntry, validated or not
game.play_for_message(entry, validate, is_host)      // the error as the message the player sees
game.history.stamp_last(now);                        // set the time after the event is applied
```

`Game::play` (`game/history.rs`) normalizes a pass ([JSON](#json)), checks that the submitter may play this type now (`get_applicable_events(by.player, is_host)`), runs `try_validate`, clears the recent milestones, runs the pre-event tasks, applies the event, runs the post-event tasks, adds the entry to the history and settles the battle sequence ([Queries](#queries)). `EstablishPlayers` skips the applicability check and keeps the milestones. Unvalidated play is for replaying a history and for tests.

`SubmitError` is `NotApplicable(Message)`, `Rejected(Rejection)` or `Failed(GameError)`, and `message()` gives the text the player sees: "Event '...' is not applicable", the rejection, or "Game Error: ...". A `Failed` event isn't added to the history, and the game is rolled back to how it was before the event ([Errors](#errors)). A refused event changes nothing.

The event sees `at` while it is applied, and the timers read it. The server submits with no time and stamps the entry afterwards, so the timers only count on a replay.

### An event

An event implements `EventRules`:

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

`try_validate`, `try_describe` and `describe_short` have defaults. An event overrides the first two when it can be submitted without a value its validation or message needs ([Errors](#errors)). The submitter's faction is `by.faction`, its player `by.player()` (or `by.player` where it may be absent), and `by.is(f)` asks whether `f` submitted it. An event logs itself with `let m = self.describe(game, by); game.log_event(m);`. Static helpers such as `can_be_played` and `valid_targets` are inherent functions.

To add an event type:

1. Write the struct and its `EventRules` in the file of its topic under `events/`, and re-export it from that folder's `mod.rs`.
2. Add it to the `events!` list, with `: pass` if it can be passed (its struct then has a `#[serde(skip)] passed: bool` field).
3. Make it applicable: push its `EventType` in the right `add_*_actions` function.
4. Regenerate the TypeScript ([JSON](#json)) and give it a form in `rust/view` ([client.md](client.md#action-forms)). If the bots should play it, add a decision in `rust/bots` and its type to `event_kind!` in `bots/src/bot/mod.rs`.

Build an event with a struct literal or `X::default()`. `Event::from(x)` wraps any event struct, and `HistoryEntry::new(faction, x)` makes the entry.

### State instead of events

Game state never holds an event. Where the rules need an event's data later, `apply` builds a state struct with only what they read, and the submitter if they read it. Examples: `DealOffer` in `deals.offers`, `CardTradeOffer` in `Interrupt::CardTrade`, `BattlePlan` in the current battle, and the collection state's `current_division_proposal`, a `DivisionProposal { portion_to_first_player }`. Later changes go to the state struct: when the plans are revealed and a Mirror Weapon takes on the card it copies, the `BattlePlan`'s `weapon` changes and `original_weapon` keeps the card it replaced.

To tell two stored states apart, give them an id or compare their values. If anything outside the game keeps that id across an undo (a bot's plan does), use the entry's `EntryId` (`game.history.playing`, set while `apply` runs), not the event's place in the history. An undo replays the same entries, so their ids stay, while an event submitted after the undo gets a new id even at the same place in the history.

### Interfaces

`LocationEvent` (`events/movement/location_event.rs`) is the interface of the events that put forces in a location. It is not in the prelude, and `Event` dispatches its members (`ev.location_event_to(game)`), which panic for an event that doesn't implement it. `PlacementEvent` holds the rules that the force-placing events share (`validate_move`, `force_locations_by`).

## Queries

Every question asked of a game takes `&Game`: `get_applicable_events`, `GameStatus::determine_status`, `next_player_to_battle`, pathfinding, the views and the bots. A query never writes state, so views are built from the server's own game, and asking twice or in another order gives the same answers. The game also has no generator a query could draw from ([Randomness](#randomness)).

State that depends on what a query would see is written by `play`. The one case is the battle sequence: `next_player_to_battle` reads a copy of it, moved past the players without a battle, and after each event `play` moves the real sequence on while a player is to initiate a battle or the battle report is shown (`settle_battle_sequence` in `history.rs`).

## Undo

`game.undo(n)` (`game/history.rs`) replays the first `n` entries of the history into `game.new_like()` and puts the new game in `game`'s place. The entries after them become `history.recently_undone_events`, and the replaced game is kept, unchanged, in `history.previous_game_state`. `restore_recently_undone()` puts that game back. Without one (a game loaded with undone events), it replays the history and the undone events. `dismiss_recently_undone()` replays the history alone. A negative `n` panics and leaves the game as it was.

A replay plays clones of the entries, so they keep their `EntryId`s. `Game::play` uses the same replay to roll a game back after an event that fails halfway ([Errors](#errors)), so after a Game Error the game still equals a replay of its history. Nothing in one game refers to another: an event keeps no game, and a `PlayerId` is only a position in `players`.

## Messages

A `Message` (`messages/message.rs`) is plain data: who may read it (`to: Audience`), its `parts: Vec<Part>`, and an optional `detail` with what the log draws besides the text (a heading, a route, battle plans, a battle fact, or a record from `log/`). A `Part` (`messages/part.rs`) is a text, a number, a game object, an enum value, a list or a nested phrase. Turning parts into words is the skin's job (`Skin::render` in `rust/skin`, [skin.md](skin.md)), and `Message::plain_text` prints one without a skin, for tests and debugging.

`express!` and friends build messages. Every argument goes through `Part::from`:

```rust
return Err(Rejection(express!(Concept::Resource, " transfers are disabled")));
express_to!(Faction::Atreides; "You see ", card);
express_part_if!(amount > 0; " for ", Payment::of(amount));
Part::list(cards.iter().map(|c| express_part!(" ", *c, " ")));
express!(...).with_log(StormLog { /* ... */ }).as_heading(Concept::Monster);
```

What converts:

- `&'static str`, `String`, `bool`, `i32`, `u16`, `Turn`, `Sector`, `HalfPoints` and `PlayerId`. Pass any other `&str` as a `String`.
- Every enum in the `part!` list of `part.rs`. An enum declared with an absent `None` goes in its `optional` list.
- `Location` and its subtypes, `Territory`, `Leader`, `Hero`, `TreacheryCard`, `ResourceCard`.
- `Payment`, `Message`, `MessagePart` and `GameError`.
- `Option<T>`, where `None` is `Part::Null`, and `Vec<T>`, `&[T]` and `[T; N]`, which become a list.

A strength or dial goes in as `HalfPoints`, so it prints `2.5`. To put something new in a message, add a variant and a `From` impl in `part.rs`, and say how the skin (`Skin::describe`) and the view (`view/src/expr.rs`) draw it. A new log record goes in the `log_records!` list of `log/mod.rs`, and `view/src/log.rs` converts it for the client.

A message stores a copy of each collection passed to it, so the log keeps showing what the collection held when it was logged (`tests/logged_lists.rs`). The macros evaluate their arguments left to right, and `express_part_if!` evaluates them even when the condition is false.

## Errors

Validation doesn't rule out every event the rules can't handle, so an accepted event can fail while it is applied. That failure is a `GameError` (`error.rs`), and the code on the way to it returns `GameResult<T>`:

- `apply` returns `GameResult`, and so does every game method it calls that can fail: drawing from a deck that can run out (`Deck::try_draw`), changing an amount ([Amounts](#amounts-of-spice-and-forces)), using a phase state outside its phase (`game.revival_mut().required("the revival phase")?`), and the bid and battle resolution around them.
- `Game::play` returns the error as `SubmitError::Failed`, and the event isn't added to the history. The changes it made before the failure are undone: `play` replays the history into `new_like()`, as an undo does, and puts that game in place of the half-changed one (`roll_back` in `history.rs`). What the history doesn't hold is carried over: the human seats, the undo state (`recently_undone_events` and `previous_game_state`), the timers and `history.playing`. The timers are copied before an event only when it has a time, since only then can it change them. Every event in the history succeeded in the same sequence, so the replay rebuilds the game exactly. Errors are rare, so this costs nothing on the events that succeed. Replaying a 4,000-event, 12-player game takes about 20 ms (release build). A panic isn't rolled back: nothing in the engine catches it, and the server's crash barrier keeps the game as the panic left it.
- An event that can't build its message for an event about to be refused (a faction that isn't playing, no card) implements `try_describe`, which the "not applicable" refusal uses.
- An event that can't even be validated without a value the client left out implements `try_validate`, which `Game::play` calls, so it fails with a Game Error. Its `validate` turns the failure into a refusal for bots and views.

```rust
let card = self.in_bidding_mut().cards_on_auction.as_mut().required("cards on auction")?.try_draw()?;
let bene_gesserit = self.get_player(Some(Faction::BeneGesserit)).required("The Bene Gesserit player")?;
if self.turn.intrusions.is_empty() {
    game_error!("No intrusions to dequeue");
}
```

Everything else that can't happen in a valid game panics, with `expect("what")`, `panic!` or `assert!`. Nothing in the engine catches a panic. The crash barriers are outside it:

- the server catches a call or timer that panics, reports it and keeps the game (`server/src/server/mod.rs`), and builds each view inside `catch_unwind` (`server/src/views.rs`);
- the view validates drafts and builds their forms inside `treachery_view::build::draft_check`, because a draft may lack values the engine expects ("Validation failed").

The bots are not a barrier ([engine.md](engine.md#rustbotssrc)). Savegames that can't be read are errors, not panics: `GameState::load` returns `Result<GameState, String>`, and `Game::load` returns `Result<Game, Message>`. Tests that check a panic use `error::catch_panic`, which returns the panic message without printing it.

## Randomness

Every random draw of a game comes from one generator, `game.rng`: a ChaCha8 stream (`random::GameRng`, from `rand_chacha`) seeded with the `seed` of `EstablishPlayers`. A game is its seed plus its events, so replaying the events gives the same game. The server draws that seed, never the client ([protocol.md](protocol.md#playing)).

Nothing else holds the game's generator. Code that draws takes it as `&mut GameRng`, so a function that draws needs `&mut Game` or the generator itself, and a query can't draw. A deck has no generator: its `shuffle` takes one.

```rust
game.rng = random::new_rng(self.seed);                                   // EstablishPlayers
game.decks.treachery_deck.as_mut().expect("treachery deck").shuffle(&mut game.rng);   // the fields borrow separately
let roll = game.rng.random_range(1..=6);                                 // use crate::random::Rng
let card = cards.into_iter().random_item(&mut game.rng);                 // RandomPick, in the prelude
let hero = heroes.into_iter().random_highest(&self.random, |h| h.value());   // a bot's generator
```

`RandomPick` (`random.rs`) has `random_item`, `random_highest`, `random_lowest` and `random_of_lowest_n`. Each collects the items and computes every key once before it draws, and `random_item` draws only when there are two or more items. Keys implement `RankKey`: integers and `bool`. The bots rank floats with their own `support::FloatKey`. While the game's generator is borrowed for a pick, the items and the key function can't read the game, so collect the items first (`.collect::<Vec<_>>().into_iter()`). That changes no draw, because the pick collects anyway.

Each `ClassicBot` has its own generator, `bot.random`, a `BotRng` (`bots/src/bot/mod.rs`). The server seeds it from the operating system when it creates the bot, and the harness records the seed in its traces. Decision methods take `&self`, so `BotRng` lends the generator out for one draw at a time: `self.random.draw(|r| ..)`, or the bot's dice, `self.d(n, sides)`. A key function may draw from the bot's generator, because the pick has computed every key before it borrows the generator. Drawing inside `draw`'s own closure panics.

The generator is ChaCha8 rather than `rand::rngs::StdRng` because rand may change `StdRng`'s algorithm in any release, and a different stream turns every saved game into a different game. `rand_chacha` promises the same stream for a seed on every platform and version, and rand promises the same results from `random_range`, `shuffle` and `choose` within a minor version (0.9). A new minor version of rand, or turning on rand's `unbiased` feature anywhere in the workspace, can change the draws. `deck_shuffle_and_random_item` in `tests/static_data.rs` pins a few streams and catches that. Such a change needs a new game version (`Game::LATEST_VERSION`).

Tests that check how far a generator has drawn compare it (`game.rng`, or `bot.random.state()`) with a generator that made the expected draws, or read its `get_word_pos()`.

## JSON

Events, history entries, savegames and game settings derive serde's `Serialize` and `Deserialize` and ts-rs's `TS`. A savegame (`GameState` in `game/state.rs`) is the game version and the history:

```json
{"version":191,"events":[
  {"by":"Atreides","at":"2026-11-13T04:14:35.622405Z",
   "event":{"type":"Move","play":{"asAdvisors":false,"to":17,"forceLocations":[{"location":12,"forces":3,"specialForces":1}]}}},
  {"by":"Harkonnen","at":"2026-11-13T04:15:02Z","event":{"type":"Move","play":null}},
  {"by":null,"at":"2026-11-13T04:15:40Z","event":{"type":"EndPhase"}}
]}
```

- An event is tagged with `type`, its fields in camelCase. A field named `type_` is written as `dealType`, `terrorType`, `juiceType` or `cardType`, because `type` is the tag.
- An event marked `: pass` in the `events!` list holds its fields in `play`, and a pass is `"play": null`. Its struct still has a `passed` field, which serde skips. `Game::play` replaces a passed event with `Event::normalized()`, which keeps only `passed`, so a pass never depends on its other fields and a game read back from its savegame is the same game.
- Enums are written as member names, static objects as their id, and absence as `null`. An unknown name, id, event type or field doesn't decode. A missing field takes its default.
- Times are RFC 3339, UTC as `Z` (`DateTimeOffset` in `compat/date_time_offset.rs`, serde in `events/wire.rs`).
- Some fields are still kept as text inside the engine: force lists (`"12:3|1,14:2|0"`), card ids (`"3;17"`), player names (`"Alice>Bob"`) and factions in play. `events/wire.rs` writes each as a JSON list (`force_list`, `id_list`, `name_list`, `faction_list`), so the text can change later without changing the format. A text that isn't a well-formed list fails to serialize.
- `HistoryEntry.id` isn't written. A decoded entry gets a new one.

`GameState::load` reads a savegame and `GameState::get_state_as_string` writes one. `Game::load(state, human_seats, validate, is_host)` replays it. The engine plays only `Game::LATEST_VERSION` (191). `Game::check_version` refuses any other version.

Savegames of version 190 and before were in an older JSON format (`{"Version":190,"_events":[{"$type":..}]}`). Nothing reads them any more: `GameState::load` refuses them, so the server refuses such an upload and leaves such a stored game out at startup. The fixtures and recorded games of the tests were converted to 191 once.

The TypeScript types are generated and checked in. `protocol/tests/ts_bindings.rs` writes `web/src/generated/Event.ts` (every event, `HistoryEntry` and `GameSettings`) and `web/src/protocol/`; `view/tests/ts_bindings.rs` writes the rest of `web/src/generated/`, including `enums.ts`, whose list of enums is in that test. Both tests fail when the files are stale. To rewrite them:

```sh
UPDATE_TS_BINDINGS=1 cargo test --release -p treachery-view --test ts_bindings
UPDATE_TS_BINDINGS=1 cargo test --release -p treachery-protocol --test ts_bindings
```

## Iterators and laziness

Order of evaluation is observable: a predicate can draw a random number or panic on an element the rules never look at. So:

- Methods that return a sequence over static data return `impl Iterator<Item = T> + Clone + 'static` (see `Map::locations`).
- Methods that return a lazy query over game state return `impl Iterator<Item = T>` built from adapters with `move` closures. Callers stop as soon as they have the answer: `.next().is_some()`, `.any(..)`, `.find(..)`. Collecting into a `Vec` first would run the predicate on elements nobody asked about.
- Where a query must see the game after a mutation, run it again at that point, and write a comment saying so.
- `iter::IterExt::distinct()` keeps the first occurrence, by `==`.
- Sorting by a key computes each key once, in element order, before sorting. Use `sort_by_cached_key`, or compute the keys first, when a key has side effects such as random draws.

## Collections

Iteration order is observable in many places: the log, the views (spice, strongholds, forces, form options), the event a bot builds, and random draws over a collection. So:

- Maps and sets are `IndexMap` and `IndexSet` (insertion order). `std::collections::HashMap` and `HashSet` are only for lookups whose order can't leak. Remove with `shift_remove`, which keeps the order of the rest. `iter::MapExt::insert_new` inserts a key that must be new.
- `BTreeMap` where a sorted order reads better: spice, discoveries and stronghold owners on the board, each player's `forces_in_locations`, and `Game::forces_on_planet_excluding_empty_locations`, which the board view lists, are in map order (by location id).

## Snapshot tests

Several tests compare what the code computes with values recorded in `tests/golden/`: in the engine, the static data, pathfinding and the generator's streams (`static_data.rs`) and a few rule walkthroughs (`sync_v188.rs`, `review_pkg_b.rs`, `review_pkg_c.rs`); the bots (`rust/bots/tests`); and the skin (`rust/skin/tests`). A difference is a change in behavior. After a deliberate one, rewrite the snapshots of the crate concerned and review the diff:

```sh
UPDATE_GOLDENS=1 cargo test --release -p treachery-engine     # or -p treachery-bots, treachery-skin
```

When you add static data, add it to `static_data.rs` too.
