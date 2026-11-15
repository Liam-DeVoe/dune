# Rewrite plan: from a cleaned-up port to a Rust design

The Rust code still has the shape of the C# engine it was ported from. This plan says what it should look like instead, in what order to get there, and how each step is checked. Saved games and the client protocol may break. Accounts must survive.

Counts were taken on main at 345bf6c with `grep` and `wc`. Sizes are estimates from those counts: S is under 1,000 changed lines, M up to 5,000, L up to 15,000, XL more.

## 1. Target design

### Crates

| Crate | Holds | Depends on |
|---|---|---|
| `treachery-engine` | Rules, state, events, messages. No JSON code of its own, no lobby data, no `Rc`, no `Cell`, no thread-locals. `Game` is `Clone + Send`. | serde, rand_chacha, indexmap |
| `treachery-skin` | Names, images and the renderer from `Message` to text | engine |
| `treachery-view` | `GameView`, action forms, draft queries, all from `&Game` | engine, skin |
| `treachery-bots` | `decide(&Game, ..) -> Option<Event>` | engine |
| `treachery-protocol` (new) | Every client/server message: calls, replies, notifications, lobby objects. Generates all TypeScript, events included. | engine, view |
| `treachery-server` | Accounts, lobby, seats (`Participation` moves here), games, storage | all of the above |
| `observe` | The behavior harness | engine, bots, view, skin |

`dotnet-compat`, `engine/src/json/`, `engine/src/compat/`, `enum_type.rs`, `shared.rs` and `view/src/replica.rs` are deleted.

### Ids, numbers, enums

- Enums are closed Rust enums deriving `Serialize`, `Deserialize` and `TS`. Decoding an unknown value fails. `Faction::None`, `Phase::None` and the like go away: absence is `Option`. There are 937 `X::None` uses today.
- `ShipmentPermission` (a `[Flags]` enum) becomes a struct of three bools.
- Static objects (`Location`, `Territory`, `Leader`, `TreacheryCard`, `ResourceCard`) stay `Copy` handles. They serialize as their id and fail to decode for an unknown id. Events hold the handle (`to: Location`), not `to_id: i32`.
- Newtypes: `PlayerId(u8)` replaces `PlayerIndex`, `Seat(u8)`, `Turn(u8)`, `Sector(u8)`, `DealId(u32)`, and `UserId(i64)` in the server only.
- Amounts of spice and forces are `u16`. A subtraction that can go below zero is written `checked_sub` or `saturating_sub` at the call site.
- Battle strength is `HalfPoints(u16)`, which prints as `2.5`. No float remains in the engine.

### Game

`Game` today has 259 public fields ([game/mod.rs](../engine/src/game/mod.rs)). The target:

```rust
pub struct Game {
    pub rules: Rules,              // version, ruleset, rule list, settings (owned, not Shared)
    pub rng: ChaCha8Rng,
    pub players: Vec<Player>,
    pub board: Board,              // map, storm, spice, forces, strongholds, terror, ambassadors, discoveries
    pub decks: Decks,              // treachery, spice, traitor, nexus, skill decks and discard piles
    pub leaders: Leaders,          // leader_state, captured, assassinated
    pub alliances: Alliances,      // offers and the fifteen ally-permission fields
    pub deals: Deals,
    pub turn: TurnState,           // turn number, per-turn counters, prevented advantages, played Nexus effects
    pub phase: PhaseState,
    pub interrupts: Vec<Interrupt>,
    pub history: History,
    pub log: Log,
    pub counters: Counters,        // leader death order, deal ids
}

pub enum PhaseState {
    AwaitingPlayers,
    Setup(SetupState), Storm(StormState), Blow(BlowState), Charity(CharityState),
    Bidding(BiddingState), Revival(RevivalState), ShipMove(ShipMoveState),
    Battle(BattlePhase), Collection(CollectionState), Contemplate(ContemplateState),
    Ended(Ending),
}

pub struct BattlePhase { pub order: PlayerSequence, pub fought: u8, pub current: Option<CurrentBattle>, pub step: BattleStep }
```

Each phase struct owns the fields that only mean something in that phase, plus a small `step` enum. `CurrentBattle` holds both plans, traitor calls, voice, prescience, the outcome and the battle's flags, which are dropped with the battle instead of being reset by hand.

`interrupts` replaces the 13 `phase_before_*`/`paused_*_phase` fields and the 28 `XTriggeredByY` and `BeneGesseritIntrudedByY` phases. An interrupt is something that suspends the phase until it is resolved: `Discarding`, `DiscardingTraitor`, `Clairvoyance`, `CardTrade`, `KarmaHandSwap`, `Bureaucracy`, `SkillAssignment`, `Thought`, `MeltingRock`, `SearchingDiscarded`, `Intrusion { kind, cause }`, `AllianceByTerror`, `AllianceByAmbassador`. The game is waiting on the top interrupt if there is one, otherwise on the phase step.

The flat `Phase` enum stays as a value computed by `game.phase()`, because views, the client and bot timing key on its names.

### Events and history

```rust
pub struct Move { pub to: Location, pub from: Vec<Battalion>, pub as_advisors: bool }

events! { Bid(Passable<Bid>), Move(Passable<Move>), Shipment(Passable<Shipment>), EndPhase, /* 138 in all */ }

pub trait EventRules {
    fn validate(&self, game: &Game, by: Option<PlayerId>) -> Result<(), Rejection>;
    fn apply(&self, game: &mut Game, by: Option<PlayerId>) -> Result<(), GameError>;
    fn describe(&self, game: &Game, by: Option<PlayerId>) -> Message;
}

pub struct HistoryEntry { pub by: Option<Faction>, pub at: Timestamp, pub event: Event }
pub struct History { pub entries: Vec<HistoryEntry>, pub undone: Vec<HistoryEntry> }
```

- `events!` is a declarative macro. It generates the `Event` and `EventType` enums, the dispatch and the `From` impls. With serde and ts-rs derives, that replaces the 2,318 lines of [events/mod.rs](../engine/src/events/mod.rs) and the 3,064 of [json/members.rs](../engine/src/json/members.rs).
- Events are plain data: `Clone + PartialEq + Serialize + Deserialize`. The initiator and time live on the history entry. `by` is `None` for host events.
- `Passable<T>` is `Pass` or `Play(T)`, so a pass has no leftover fields.
- Game state never holds an event. Where it holds `Rc<Battle>` or `Rc<DealOffered>` today, it holds a state struct (`BattlePlan`, `DealOffer { id: DealId, .. }`) that the event's `apply` builds. The six `Cell` fields become ordinary state fields written by `apply`.
- `Game::submit(by, event, at) -> Result<(), SubmitError>`, where `SubmitError` is `NotApplicable`, `Rejected(Rejection)` or `Failed(GameError)`.
- Every query takes `&Game`. `applicable_events`, `status` and `next_player_to_battle` no longer advance the battle sequence. `apply` advances it at the end of each event in the battle phase. The neighbour cache leaves `Map`. Pathfinding computes from the current state, and a caller that wants caching passes its own `PathCache`.

### Messages and the log

```rust
pub struct Message { pub to: Audience, pub parts: Vec<Part>, pub detail: Option<LogDetail> }
pub enum Audience { Everyone, Faction(Faction) }
pub enum Part {
    Text(Cow<'static, str>), Number(i32), Strength(HalfPoints), Payment { amount: u16, by: Option<Faction> },
    Faction(Faction), Force(Faction, ForceKind), Concept(Concept), Location(Location), Territory(Territory),
    Leader(Leader), Messiah, Card(TreacheryCard), CardType(TreacheryCardType), SpiceCard(ResourceCard),
    /* one variant per enum that is ever logged */ List(Vec<Part>),
}
pub enum LogDetail { Route(Route), Plans(BattlePlans), Fact(BattleFact), Storm(StormLog), /* the records in log/ */ }
pub struct Rejection(pub Message);
```

`express!` keeps its name and shape, so its 961 call sites change little. `Part` has no runtime type names and no live lists. `nr` goes: a message's identity is its position in `game.log`. The view's `RichText` parts map one to one from `Part`.

The skin has one renderer, `Skin::render(&Message) -> String`, and a typed name table per enum. The 112 `self.format("{0} ...", args![..])` calls in [skin.rs](../skin/src/skin.rs) and the faction sheets become `parts![..]` in the same macro syntax. `string_format` is deleted.

### Wire format

Field names are camelCase, enum values are variant names (what views and the skin JSON already use), events are tagged with `type`. A move by Atreides today:

```json
{"$type":"Move","AsAdvisors":false,"_toId":17,"_forceLocations":"12:3|1,14:2|0","Passed":false,"Initiator":20,"Time":"2026-11-13T04:14:35.622405+00:00"}
```

and in the new format, as a history entry:

```json
{"by":"Atreides","at":"2026-11-13T04:14:35.622405Z",
 "event":{"type":"Move","play":{"to":17,"from":[{"location":12,"forces":3,"specialForces":1},{"location":14,"forces":2,"specialForces":0}],"asAdvisors":false}}}
```

A pass is `{"type":"Move","play":null}`. The client sends only `event`. The server fills `by` from the caller's seat and `at` from its clock. A savegame is `{"version":189,"events":[HistoryEntry, ..]}`.

The lobby protocol becomes:

```json
→ {"id":7,"call":"joinGame","args":{"game":"g_ab12","password":null,"seat":3}}
← {"id":7,"ok":{"gameId":"g_ab12", "...": "..."}}
← {"id":7,"err":{"code":"seatTaken","message":"That seat was just taken"}}
← {"notify":"seatsChanged","data":{"...": "..."}}
```

`Call`, `Notification` and `ErrorCode` are serde enums in `treachery-protocol`. The token is sent once per connection in an `authenticate` call instead of in every call. The password hashing and the `users` and `sessions` tables don't change.

### How the other crates use this

- **View**: builds from the server's own `&Game`. `ViewReplica` and the `catch_unwind` in `draft_check` go, because a decoded draft is always safe to validate.
- **Bots**: decision methods return `Option<Event>`. `guarded` is deleted. A bot panic is a bug that reaches the server's crash barrier and gets reported.
- **Server**: owns `Participation`. `EstablishPlayers` no longer reads it. The server checks the seat count before it submits.
- **Web**: `web/src/generated/` gains `Event.ts` and the protocol types. Forms build typed events, replacing 60 untyped `$type` records.

## 2. Order of work

The rand_chacha change must be merged and the reference rebuilt before any of this starts.

### Wave 0: harness (one agent, S)

The `rust/observe` changes of section 3.

### Wave 1: whole-tree sweeps (one at a time, nobody else in `rust/`)

Each sweep touches most files, so they run serially on a quiet tree. All keep behavior the same.

| # | Sweep | Size |
|---|---|---|
| 1a | Rename the 439 functions named after C# overload signatures (`new_int32_participation`, `status_string_string_game_event`), drop `I` prefixes and .NET names, delete stubs, the Newtonsoft path and C# comments, fix typos in identifiers (`occured`, `Appearence`) | M |
| 1b | Move `Game`'s fields into the topic sub-structs above, as pure moves (`game.current_battle` to `game.battle.current_battle`). No `PhaseState` enum yet. | L |
| 1c | Thread-local counters onto `Game`. `Participation` and `Shared<T>` out of the engine. | S |

1b comes before everything else because later workstreams are split by topic, and after it each topic's state is in its own file.

### Wave 2: deliberate behavior changes (serial, small, one reference rebuild at the end)

| # | Change | Size |
|---|---|---|
| 2a | The 13 `NetDictionary`/`NetHashSet` fields become `IndexMap`/`IndexSet`, or `BTreeMap` where a sorted order reads better (spice and forces by location) | S |
| 2b | Neighbour cache always invalidated when state changes | S |
| 2c | Log typo fixes, live lists replaced by a snapshot | S |
| 2d | Closed enums, `Option` instead of `None` members, integer types and newtypes, `HalfPoints`. The legacy JSON reader converts and rejects out-of-range values. | XL |

2d is one workstream that goes enum by enum and merges daily. It changes behavior only where the fuzzer sends undefined or negative values.

### Wave 3: parallel, split by directory, behavior-neutral

| # | Workstream | Files | Size |
|---|---|---|---|
| 3a | Events as plain data: `events!`, `HistoryEntry`, `Rejection`, state structs instead of `Rc<Event>`. One agent lands the scaffolding, then one agent per event folder (`battle`, `bidding`, `movement`, the rest). | `engine/src/events/**`, the matching `game/<topic>.rs`, constructors in bots and view | XL |
| 3b | `Message`, `Part`, `LogDetail`, the skin renderer | `engine/src/messages`, `engine/src/log`, `rust/skin`, `view/src/expr.rs`, `view/src/log.rs` | L |
| 3c | Lobby protocol and the protocol crate | `rust/protocol`, `server/src/{protocol,wire}.rs`, `server/src/server/*`, `web/src/lobby`, `web/src/state/connection.ts` | L |

3a and 3b meet only at `express!` call sites, which keep their shape.

### Wave 4: parallel, after 3a

| # | Workstream | Files | Size |
|---|---|---|---|
| 4a | Pure queries, replica removed | `game/{applicable_events,status,battle,player_sequence}.rs`, `map/mod.rs`, `view/src/replica.rs`, `server/src/views.rs`, `observe/src/session.rs` | M |
| 4b | Wire format: serde on events, `json/` deleted, new savegame, event TypeScript, typed web forms, database migration, version 189 | `engine/src/json`, `rust/protocol`, `web/src/components/**` forms, `server/src/db.rs`, `observe/src/fuzz.rs` | L |
| 4c | Bots without panics | `rust/bots` | M |

### Wave 5: phase state (parallel by phase, after wave 4)

One agent first introduces `PhaseState`, `Interrupt` and the computed `Phase`. Then one agent each for setup/storm/blow, bidding/charity/revival, ship and move with intrusions, battle, and collection/contemplate. The same agents split the long functions as they go. `add_player_actions` (875 lines) and `determine_status` (559 lines) become one function per phase state. XL in total, M to L per agent.

### Wave 6: cleanup (S)

Delete `dotnet-compat`. Rewrite engine-conventions.md, engine.md, protocol.md and client.md.

## 3. Verification

`rust/tools/check.sh` and 200 bot plus 200 fuzz games gate every merge, as now. A trace holds the reference's event JSON, which the working tree replays, so the harness needs three changes first.

**Wave 0 changes to `observe`:**

1. Events and bot decisions are compared as decoded values, not as JSON text. Both sides decode to their own `Event` and the replay compares the working tree's re-encoding of each.
2. An event that fails to decode counts as rejected, and its message isn't compared, like a Game Error today.
3. A new command, `observe compare-refs A B`, records the same seeds with two reference binaries and compares only the observations (accepted or not, log, view hashes, applicable types). It ignores how events are encoded.

**The format change (4b) in three steps:**

1. The working tree gets the new serde format. The old hand-written reader in `engine/src/json` moves to `observe/src/legacy_events.rs` and is retargeted to produce the new typed `Event`. The existing reference still records old JSON, and the working tree replays it through that reader. This must pass with no differences. It proves the typed events behave like the old ones.
2. A candidate reference is built from that commit. Its recorder and fuzzer write the new format. `compare-refs old candidate` must show the same observations for the same seeds, which proves the ported fuzzer generates the same games. If the ported fuzzer can't be made to match, step 1 alone is the evidence and the owner signs off on the gap.
3. The candidate becomes the reference and `legacy_events.rs` is deleted.

**Per wave:**

| Wave | Check | Reference rebuild |
|---|---|---|
| 1, 3a, 3b, 4a, 4c, 5 | No differences in 200 + 200 games | No |
| 2a, 2b, 2c | Every first difference attributed to the one change, by replaying with that change reverted (the method in native-harness.md) | Once, after 2d |
| 2d | Differences only after an event carrying an undefined or negative value | Same rebuild |
| 3c | Not covered by `observe`. Server protocol tests, `scripts/ui/e2e_server.py`, `npm run build`. | No |
| 4b | The three steps above | Yes, in step 3 |

Changes players will see on purpose: the order of spice, forces, terror tokens, ambassadors and discoveries in views and logs (2a), moves that the stale cache wrongly allowed or refused (2b), corrected log text and card lists that no longer update after the fact (2c), and a different error text for malformed events (2d). Each gets an entry in behavior-changes.md.

In 4c, every panic that `guarded` swallows today must become an explicit "no decision" at the same point. Fixing one differently is a separate behavior change.

## 4. Risks and questions for the owner

1. **Running games end at deploy.** Savegames are event lists replayed from a seed, so the RNG change already makes every existing game unreplayable, and no converter can fix that. The migration should move old rows to a `games_legacy` table and keep `users`, `sessions` and `scheduled_games`. Should the deploy wait for running games to finish, or is a notice enough?
2. **Live lists.** A few log lines show a hand as it is now, not as it was when logged, as the C# client did. I recommend the snapshot (2c), because messages can't be plain data otherwise. Confirm.
3. **Custom skins.** Hosts can upload a skin. Its JSON keys are enum names and stay valid, but a skin holding a `{0}` template of its own would stop working. I haven't checked whether any stored skin does.
4. **Wave 5 is the riskiest step.** The interrupt stack replaces how every phase suspends and resumes. The harness covers it well for phases the fuzzer reaches (107 of the 123 phases, per the reach table in native-harness.md) and not at all for the rest. Budget time to extend fuzz reach first, or accept targeted unit tests for the unreached phases.
5. **Unsigned amounts.** The engine has about 960 lines mentioning `i32`. Where a value could go negative, the sweep has to choose between rejecting and saturating. Expect it to find real bugs, each a behavior change to approve.
6. **The harness doesn't see the lobby.** 3c rests on server tests and one Playwright run. The e2e script should grow to cover join, leave, kick, reconnect and undo before 3c merges.
7. **Wave 1 and 2d need a quiet tree.** Any other branch open during a sweep will conflict in hundreds of files, so everything else is merged or parked first.
