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

`rust/tools/check.sh` and 12 bot plus 12 fuzz games gate every merge. 50 + 50 games run once per wave, after its last merge, and for every wave 5 merge. Agents run 5-game batches while working and check.sh once before reporting, not on every commit (the owner's decision, 2026-11-15, to cut testing effort by four). A trace holds the reference's event JSON, which the working tree replays, so the harness needs three changes first.

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
| 1, 3a, 3b, 4a, 4c | No differences in 12 + 12 games per merge, and in 50 + 50 once the wave is merged | No |
| 5 | No differences in 50 + 50 games per merge | No |
| 2a, 2b, 2c | Every first difference attributed to the one change, by replaying with that change reverted (the method in native-harness.md) | Once, after 2d |
| 2d | In a sample of about 5 failing games out of 12 + 12, the first difference is at an event carrying an undefined, negative or out-of-range value | After 2d |
| 3c | Not covered by `observe`. Server protocol tests, `scripts/ui/e2e_server.py`, `npm run build`. | No |
| 4b | The three steps above | Yes, in step 3 |

From 2d on, a deliberate behavior change isn't attributed game by game (the owner's decision, 2026-11-15). For a sample of about 5 failing games, check that the first difference is the intended change. Any other first difference is a bug to find. Then the reference is rebuilt.

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

## Progress

- 2026-11-15: The ChaCha8 generator (game version 189) is merged (c44151a), and the shared reference is rebuilt from it. The old one is kept as `~/treachery-native-ref/observe-before-chacha`. Against the new reference, 200 of 200 bots games and 200 of 200 fuzz games (seed 1) pass, with the same endings as the branch's check (bots 197 ended, 3 stalled; fuzz 126 ended, 74 at the cap). Decisions on section 4: a notice is enough at deploy (1), log lines take a snapshot (2), and skins aren't stored in the database, so no stored skin can break, though 3b must still load a host's skin file that has `{0}` templates (3). Unreached phases get targeted unit tests and more fuzz reach before wave 5 (4), negative amounts are decided case by case as behavior changes (5), and the e2e script grows before 3c (6). Running: wave 0 and the e2e coverage.
- 2026-11-15 01:32: The e2e lobby coverage is merged (e503147), including a client fix for seats after a reconnect. Wave 0 has three commits on its branch and is running its final 200 + 200 harness check. Wave 1 waits for it, because the sweeps need a quiet tree.
- 2026-11-15 02:25: Wave 0 is merged (d18dc21). Events are compared as decoded values, and `observe compare-refs` exists. It passes 200 + 200 games against the shared reference, and check.sh passes after the merge. Running: sweep 1a (renames), alone in rust/.
- 2026-11-15 04:23: Sweep 1a has three commits on its branch, but its worktree has sat in an interactive rebase for an hour with nothing running. I asked the agent to abort the rebase and continue.
- 2026-11-15 10:30: Sweep 1a is rebased onto main, and its bots run is done. Its 200 fuzz games are running. Replaying the same games, its CPU time equals main's (36.4 s for bots game 45), so the slow harness runs today come from the machine, not the code.
- 2026-11-15 13:40: Sweep 1a is merged, in three commits on top of main 8c672f8. About 100 functions named after C# overload signatures are renamed, the five I-prefixed traits lose the prefix, 39 unused public functions and the Newtonsoft savegame reader are deleted, and about 380 comment lines that only described the C# code are gone (880 lines fewer in all). check.sh passes, and 200 bots and 200 fuzz games (seed 1) pass against the shared reference (bots 197 ended, 3 stalled; fuzz 126 ended, 74 at the cap). Left for later: about 210 comment lines that still name C# members, the C# names in doc comments of the view types (they are copied into web/src/generated, so 4b changes them with the format), and the wire names `Metheor`/`MetheorPlayed`/`MetheorUsed`/`MetheorAndStormSpell` and the "shufle" log text (2c).
  `GameState::load` still replaces "Treachery.online" with "Treachery" in a savegame's text, a leftover of the Newtonsoft path. It stays until 4b, whose new savegame format drops it.
- 2026-11-15: Sweep 1b is done on its branch, in three commits. `Game` keeps `seed`, `random`, `players` and `participation`, and the rest of its fields are in `rules`, `board`, `decks`, `leaders`, `alliances`, `deals`, `turn`, `history` and `log`, plus one struct per phase (`setup`, `storm`, `blow`, `charity`, `bidding`, `revival`, `ship_move`, `battle`, `collection`, `contemplate`). Each struct is defined in its topic's file under `game/`, and the third commit moves the matching methods out of `game/mod.rs` too. engine-conventions.md, "Game state", lists the structs. check.sh passes, and 200 bots and 200 fuzz games (seed 1) pass against the shared reference (bots 197 ended, 3 stalled; fuzz 126 ended, 74 at the cap). There is no `counters` struct yet, because the counters are still thread-locals, which 1c moves onto `Game`. Wave 5 should check which phase-struct fields outlive their phase (`ship_move.shipment_permissions`, `revival.factions_with_increased_revival_limits`, the alliance offers made through terror and ambassadors in `ship_move`, `contemplate.winners`), since a phase's state will be dropped when the phase ends.
- 2026-11-15 15:00: Sweep 1b is merged (2d358ab), and check.sh passes on main. Running: sweep 1c (thread-local counters onto Game, Participation and Shared out of the engine).
- 2026-11-15: Sweep 1c is done on its branch, in five commits. The engine has no thread-local game state left. Leader deaths are numbered in `game.counters`, and messages lost their numbers, which only `Message`'s unused equality read. `EstablishPlayers` gives the game a copy of its settings instead of sharing them. `Participation` is in the server (`server/src/participation.rs`). The engine keeps only `Game::human_seats`, which `is_bot` reads and the server updates on every change of seats. The server checks the seat count before it submits `EstablishPlayers` (`validate_for_table`), and the view gets names and the bot speed as a `Seating`. check.sh passes, 200 bots and 200 fuzz games (seed 1) pass against the shared reference (bots 197 ended, 3 stalled; fuzz 126 ended, 74 at the cap), `npm run build` leaves web/src/generated unchanged, and e2e_server.py passes. Two games on one thread did share both counters, but players couldn't see it: message numbers were never shown, and death numbers only order the tanks within a game. The per-game count starts at 1 to keep one order the views showed: a leader that is dead without having been killed (the fuzzer makes one by reviving a living leader) comes before killed ones. Fuzz games 77, 97, 113 and 126 failed with a count from 0. Left: `Shared<T>` stays for the random number generator the decks share with the game (`SharedRandom`). Taking it out means passing the game's generator to every deck shuffle and giving the bots their own, which fits better with the plain-data work of wave 3. `Game` still isn't `Send`, because of the `Rc` events and log records (3a, 3b) and `SharedRandom`.
- 2026-11-15 16:54: Sweep 1c is merged (30d0bf4), so wave 1 is done. check.sh passes on main. `Shared` stays only for the game's random number generator (`SharedRandom`), and `Game` isn't `Send` yet because events and log records are in `Rc` (3a, 3b). Passing the generator explicitly goes with 3a. Wave 2: 2a, 2b and 2c run in parallel, because they touch different code and each branch then differs from the reference by one change, which makes attribution simpler. 2b also fixes the revival of a living leader (known-issues.md). After all three merge, the reference is rebuilt before 2d, so 2d is compared only with its own changes.
- 2026-11-15: 2c is done on its branch, in four commits. The three messages that read a list from the game at render time (the hand the Atreides ambassador shows, CHOAM's audited cards, a bot's discarded traitors after a deal) now log a copy, and `LiveList` and the skin's and view's `_in` renderers are gone. About 35 texts are fixed: "shufle", "Harkonnen Markt", "A en B", "discovert", "put select a", "Treachery.Shared.Playerget", "top Spicecard", "loose the battle" and the rest, each listed in behavior-changes.md with its old and new text. check.sh and `npm run build` pass. Harness, 200 bots and 200 fuzz games (seed 1) against the shared reference: `observe live` was killed by heavy.sh at bots game 37, because every game fails from its first view (the faction sheets changed) and live records each failure again with full views, as native-harness.md warns. So each game was recorded with the reference and replayed by three builds: the branch, the branch without the snapshot, and the branch without the text fixes. 395 games differ (bots 195, fuzz 200), each at the same line without the snapshot and not at all without the text fixes, so all are text. Without the text fixes only fuzz games 127 and 196 differ, both in the ambassador's hand line, which is the snapshot. Nothing is unattributed. The allow rules for the text fixes are in docs/allow-2c-log-text.txt. They can't hide differences in views, which show the log and the faction sheets. Note for 3b: `Skin::load` accepts any string, `{0}`, `{x}` and a lone `{` included, and log, status and form texts print such labels as they are, because labels are only ever arguments of a template. 3b must keep loading those skins and printing the braces. The one exception is a bug 3b can drop: the faction sheets put advantage titles (labels of concepts, card types, factions, forces and phases) into the sheet template before formatting it, so `{N}` in such a label is replaced by sheet parameter N, and a malformed one makes the whole sheet come out unformatted. Also, a skin dictionary that lacks any of the default keys is replaced by the default one entirely (C#'s `FixDictionary`), so a skin with only a few names of its own loses them.
- 2026-11-15: Wave 2a is done on its branch, in four commits. The thirteen `NetDictionary`/`NetHashSet` fields are `BTreeMap` in map order (spice, discoveries, stronghold owners, each player's forces, and the board view's forces list) or `IndexMap`/`IndexSet` in insertion order (terror, ambassadors, occupied homeworlds, bids, shipment permissions, recently discarded cards, early revival offers, known cards, revealed traitors). The engine, bots and view no longer use them, and the prelude no longer exports them. `Location` derives `Ord` (one line in `map/location.rs`). check.sh passes, `npm run build` leaves web/src/generated unchanged, and every first difference in 200 bots and 200 fuzz games (seed 1) comes from 2a: all 400 games match with the fields put back, and each first difference moves later with one field put back (forces in all but 3 games, details in native-harness.md).
- 2026-11-15: Change 2b is done on its branch, in three commits with the fix for "A leader who is alive can be revived" (known-issues.md). `Map` has no cache and no `Cell` left. Pathfinding works from the current state on every call, and no caller needs a `PathCache`: recording 15 bots games takes the same CPU time either way. The cache could answer for an earlier state within one event or after an event that failed halfway (escapes.md, "Paths found for an earlier state"), but no game in 200 bots and 200 fuzz games (seed 1) got a wrong answer from it. `Revival` and `KarmaFreeRevival` refuse a living leader, and the leader death count starts at 0. check.sh passes. All 200 bots games pass `observe live` against the shared reference. The fuzz games were recorded with the reference and replayed one at a time, because `live` re-recorded its failures with full views and ran out of memory. 154 pass. The other 46 first differ at a revival of a living leader. All 46 pass with the revival fix reverted, and fail the same way with the cache or the old death count put back, so all 46 come from the revival fix.
- 2026-11-15 19:12: 2a, 2b and 2c are merged (caabede), and check.sh, check:rich-text and npm run build pass. Conflicting snapshots were regenerated on the merged code, and their other lines kept main's formatting. The shared reference is rebuilt from caabede (the previous one is `~/treachery-native-ref/observe-before-wave2`), and a 200 + 200 run against it is going. Running: 2d (types sweep, in milestones), and 3c (lobby protocol) in parallel with it, because 3c's files are the server protocol layer and the web lobby.
- 2026-11-15 19:41: Against the reference rebuilt from caabede, main passes 200 of 200 bots games (197 ended, 3 stalled) and 200 of 200 fuzz games (125 ended, 75 at the cap; before wave 2 it was 126 and 74. One game now ends at the cap instead, which fits 2b's refused revivals, but I haven't checked which game).
- 2026-11-15: 3c is done on its branch, in four commits. `rust/protocol` (treachery-protocol) holds every lobby message: `Call` (one variant per call, typed arguments, declared with its reply type in one `calls!` table, which also gives `replies::X` and the TypeScript `CallReplies`), `Notification`, `ServerMessage`, `ErrorCode` with default texts, and the lobby objects, camelCase with enums by name. Its `tests/ts_bindings.rs` generates `web/src/protocol/`, importing the view's types from `web/src/generated/`. The envelope is `{"id","call","args"}`, `{"id","ok"}` / `{"id","err":{"code","message"}}` and `{"notify","data"}`; `badCall` and `serverError` replace the old `error` string. A connection authenticates once with `authenticate` (the old `RequestResumeSession`) or a call that starts a session; the server's handlers return their call's reply type, checked by `reply::<replies::X>` in the dispatch. The web client calls through `Connection.call(name, args)`, typed from the generated files, and shows the server's error `message`; the hand-written lobby types are gone. Tables are unchanged, async-play login links work (`linkLogin`). check.sh passes (server tests rewritten to the new envelope with the same checks, plus one that a token in a call's arguments authenticates nothing), `npm run build`, check:rich-text and check:flash pass, and e2e_server.py passed three runs in a row. `observe` is unaffected: the only change outside the server and protocol is a `TS` derive on `UndoEntry` in `view/src/replica.rs`. Left: events and game settings stay in the savegame format (4b); the lobby's engine enums are strings converted in `server/src/protocol.rs` (`enum_name`, `enum_value`), which 2d's closed enums should replace with the real types; user ids are `i32` (2d's `UserId`); `ErrorCode`'s texts copy the skin's `describe_error_type`, and the engine's `ErrorType`, `UserStatus`, `SubscriptionType` and `GameListScope` and lobby.json's `ErrorType` table are no longer used by the server or client (delete in wave 6); `components/ChatComponent.tsx` isn't wired up and keeps its own `ChatEntry` type.
- 2026-11-15 19:57: 3c is merged (0f611bf): a treachery-protocol crate, the call/reply/notify envelope, authenticate once per connection, and generated TypeScript in web/src/protocol. check.sh, npm run build and e2e_server.py pass on main. Running: 2d (types), and a fuzz-reach agent that prepares wave 5 (risk 4). 3a and 3b wait for 2d, because all three touch most of the engine.
- 2026-11-15: 2d, first part of milestone 1 (closed enums), on branch `worktree-agent-a9063c6c8cdad42a3`: three commits on top of caabede, a merge of main after 3c, and two more (the last is `2ad151a`). 39 enums are closed: every enum except `Faction`, `Phase`, `MainPhase`, `ShipmentPermission` and the lobby enums. check.sh passes. engine-conventions.md ("Enums") describes the design and behavior-changes.md ("Event values that name no enum member") the change.
  - Design. `game_enum!` has a `closed` form: a Rust `enum` with the C# values as discriminants. A C# `None = 0` member meaning "no value" is declared `absent None = 0;`: the Rust type leaves it out, and absence is `Option<X>`. To keep event JSON, view JSON and log text identical, `Option<X>` of such an enum is written as 0 in event JSON (its own `ToJson`/`FromJson`; the generic `Option` impls need `json::JsonNullable`), logs as the `None` member (`From<Option<X>> for ExpressionElement`; the generic impl needs `NullElement`), and views and the lobby name it "None" (`name_or_absent`, `value_or_absent`). The skin's describers for these enums take `Option<X>`, and its dictionaries are keyed by `Option<X>` (`optional_enum_keys!`). `EnumTypeInfo::declared` still lists `None`, so web/src/generated/enums.ts doesn't change, and nothing in web/ changed. Where C# offered `None` as a choice, the list keeps it as `None` (`ClairVoyancePlayed::valid_questions`, `Karma::valid_faction_advantages`). `DealType::None` stays a member, because a custom deal is a real choice. No enum derives `TS` yet: views carry enums as strings with `#[ts(type = ..)]`, and a nullable TypeScript type would mean a view JSON change. I'd leave that to 4b.
  - Harness. Against the shared reference, every fuzz game now first differs at the first event with a number that names no member, almost always an early `AutomationConfigured` with an undefined `Action` (all 20 of fuzz seed 1 games 0-19, while bots games 0-11 pass). So the shared reference shows little past that point. Instead, `~/2d-ref` holds a copy of the reference sources whose reader refuses undefined values for the enums named in `TREACHERY_CLOSED_ENUMS` (patch `~/2d-ref/closed-enums.patch`, binary `~/2d-ref/bin/observe`, behavior-changes.md says how to run it). Against it, 20 bots and 20 fuzz games (seed 1) passed after the third and after the fifth commit. Its sources are the shared reference's (caabede), so it must be rebuilt if the shared reference is.
  - Left in milestone 1, in this order: `ShipmentPermission` (a struct of three bools, written as the flag number, refusing unknown bits); `MainPhase` and `Phase` (`turn.current_phase` starts as `Phase::None` in `TurnState::new`, and the "Unknown phase: None" status test sets it; `Report::new(MainPhase::None, 0)`); `Faction` last (3,578 mentions: `Player.faction`, `ally`, `predicted_faction` and the events' `Initiator` are `None` before faction selection or for the host, so they become `Option<Faction>`; `server/src/protocol.rs` `enum_name`/`enum_value` and `observe/src/session.rs` use it). Then remove the open arm of `game_enum!` and `get_values_except_default`, turn the doc test in enum_type.rs back on (it is `ignore` until `Faction` closes), and add a check to `engine/tests/event_json.rs` that every event type keeps an accepted case: each closed enum turns more of the snapshot's random values into refusals. The lobby enums (`UserStatus`, `SubscriptionType`, `GameListScope`, `ErrorType`) stay open: since 3c only the skin and the lobby data use them, and wave 6 deletes them.
  - Method that worked: close a batch of enums, run `~/2d-ref/tools/optify.sh X` per enum (rewrites `X::None` to `None` and `== X::Y` to `== Some(X::Y)`), change the struct and event fields that held `None` to `Option`, then loop `~/2d-ref/tools/unwrapsome.sh` and `wrapsome.sh` (they wrap or unwrap `Some(..)` at the columns of `cargo check`'s type errors) until both report 0, and fix the rest by hand. `dotzero.sh` turns `.0` on a closed enum into `.value()`. After each batch, look for lists built from `X::ALL` that C# built with `None` in them, and for `_ =>` arms that are now unreachable. Snapshots that change: `UPDATE_GOLDENS=1` for engine `static_data` and `event_json` (cases with undefined values become refusals) and skin `static_corpus`, whose calls that pass the `None` member to a function taking a real member come out as `!panic: [..] is not a ..` and are deleted from the file.
  - Milestones 2 to 4 (handles, newtypes, unsigned amounts) are not started.
- 2026-11-15: Fuzz reach for risk 4 is done on its branch. 30% of fuzz games now seat Bene Gesserit, Ecaz, Moritani, Guild, Fremen and Tleilaxu together, and the fuzzer steers shipments, moves, worm rides and revivals into territories where they intrude, and picks Diplomat leaders so lost battles lead to a retreat. Over 200 fuzz games (seed 1) it reaches 115 of the 123 phases instead of 105. Of the eight left, four can't be entered with the engine as it is, two are never a resting phase, and two are reached in only a few games per 200. native-harness.md, "Phases the fuzzer doesn't reach", lists them with the unit test wave 5 should write for each. One of them may be a rules bug for the owner: Bene Gesserit fighters accompanying a shipment never trigger an Ecaz ambassador. Fuzz games now hit the 3000-event cap more often (100 of 200 instead of 75) and take about a third longer. The shared reference still records the old fuzz games until it's rebuilt from the merge.
- 2026-11-15 21:22: Main (4d68cc7) has the other agent's next-card view and Messiah popover, the first 2d agent's closed enums (milestone 1 except ShipmentPermission, Phase and Faction), and the fuzz-reach work. Over 200 fuzz games the fuzzer now reaches 115 of 123 phases, up from 105. The unreached ones, with the tests to write instead, are in native-harness.md. Fuzz games hit the cap more often (100 of 200) and take about a third longer. The shared reference is rebuilt from 4d68cc7 (the previous one is `observe-before-2d-enums`), and a 50 + 50 check against it is running. Integration happens on `coord-integration` (worktree .claude/worktrees/coord) whenever the main checkout has someone's uncommitted work, and main fast-forwards to it. Running: the second 2d agent.
- 2026-11-15 21:33: Main passes 50 of 50 bots games (49 ended, 1 stalled) and 50 of 50 fuzz games (25 ended, 25 at the cap) against the reference from 4d68cc7. The bottleneck is agents, not CPU (about 3 of 8 cores busy), so 3b (messages as data) and the wave 5 phase tests now run beside 2d. 3a waits for 2d, because both rewrite the event files.
- 2026-11-15: Unit tests for risk 4 are done on branch `worktree-agent-a877d2e718aac3ead`, as new test files only. `engine/tests/intrusion_phases.rs` has two tests for each of the 24 intrusion phases reached today: one enters the phase, one reacts and checks where the game goes on. `engine/tests/phases_unreached.rs` covers the eight phases fuzz games never rest in, plus `Discarding` and `Auditing`. native-harness.md, "Phases the fuzzer doesn't reach", names the test for each. The games come from `engine/tests/intrusion_table/mod.rs`, which seats the six focus factions, does the rest of setup by hand, and submits events as JSON. Two findings: the doc had the terror threshold backwards (fewer than three forces trigger a token only while Moritani is *not* at its low threshold), and accompanying Bene Gesserit fighters never trigger an ambassador, which stays a question for the owner. Wave 5 still needs a second fuzz seed for the ship and move agent.
- 2026-11-15: 3b is done on its branch, in five commits on top of d265da1 plus a merge of main (85586b4).
  - A message is plain data: `Message { to: Audience, parts: Vec<Part>, detail: Option<Box<LogDetail>> }`, `Clone + PartialEq + Send + Sync` (checked at compile time in message.rs). `Part` (messages/part.rs) has a variant per kind of value and one per enum, declared by the `part!` list, so nothing in a message names a .NET type or holds an `EnumTypeInfo` to match on. An enum whose C# `None` member is absence in Rust is in the `optional` list and its variant holds an `Option`. When 2d closes `Faction`, `Phase`, `MainPhase` or `ShipmentPermission`, it moves the enum to that list. Lists hold their items only, and `Rc<dyn LogRecord>` is a closed `log::LogRecord` enum.
  - `LogDetail` is a struct, not the enum of section 1: a message can carry a record together with a route (moves), a battle fact (discards) or a heading (spice decks), and the view sends them as separate fields. Besides the variants of section 1, `Part` has `Null`, `Bool`, `Float` and `Double` (until 2d's `HalfPoints`), `Phrase` (an `express_part!`), `Message` (`Game::log` keeps the detail of a message logged on its own) and `Error` (the harness tells a Game Error from a refusal by it).
  - The skin renders with `Skin::render(&Message)` and `render_parts`, and its own texts use `parts![..]`. `Skin::format` and `dotnet_compat::fmt::string_format` are deleted. Faction sheets keep their `{N}` templates (`Skin::fill_sheet`), and a skin label used as an advantage title has its braces escaped, which drops the bug 2c found (behavior-changes.md, "Braces in a custom skin's advantage titles"; test `skin/tests/custom_labels.rs`). `Message::plain_text` replaces `to_cs_string` in tests. `nr` was already gone (1c).
  - `Rejection(Message)` exists, but no event returns it: every `validate` is in the event files, so 3a changes them to `Result<(), Rejection>` with `submit`.
  - Checks: check.sh passes on the merge. The skin's trace corpus renders all 41,758 texts as before. The static corpus lost only its 4,023 `string_format` lines and the 218 calls with enum numbers that closed enums can't hold. check:rich-text (9,499 of 9,499 lines), `npm run build` and the generated TypeScript are unchanged. Against the shared reference, 12 bots and 12 fuzz games (seed 1) pass.
  - Left for later: the `elems!`/`log!` macros in several `game/*.rs` and event files duplicate `parts!`; the enum variants no message ever logs (the lobby enums) could go when wave 6 deletes those enums; `Part::Payment`'s `by` becomes `Option<Faction>` with 2d.
- 2026-11-15 22:26: 3b is merged on coord-integration (67f2b4b), and check.sh passes there. Messages are typed parts, one skin renderer, `Rejection` exists, and no rendered text changed. Wiring `Rejection` into each event's validate is left for 3a. Main is behind coord-integration until the other session commits its edits to behavior-changes.md, bidding.rs and the skin static snapshot. Running: 2d (Faction, then milestones 2 to 4). 3a waits for 2d.
- 2026-11-16: 2d milestone 1 is done on branch `worktree-agent-a1507c1d1103c8091` (coord-integration 83e3940, then a merge of coord-integration at 4d68cc7). `5bffee8` closes `ShipmentPermission` (a struct of three bools, written as the flag number, refusing other bits), `MainPhase` and `Phase`. `411b641` closes `Faction` and the four lobby enums, removes the open form of `game_enum!` and `get_values_except_default`, and turns the doc test in enum_type.rs back on. The last commit fixes the two differences the harness found. `engine/tests/event_json.rs` now fails when an event type has no accepted case left. engine-conventions.md ("Enums", "Factions") and behavior-changes.md ("Event values that name no enum member") are updated. check.sh passes, `npm run build` isn't needed (web/ is unchanged), and against the second reference below 12 of 12 bots and 12 of 12 fuzz games (seed 1) pass.
  - Design. A player has no faction before faction selection and the host has none, so `Player.faction`, `ally`, `nexus`, `predicted_faction`, an event's initiator and the event fields that name a faction are `Option<Faction>`. Queries that C# called with `Faction.None` take `impl Into<Option<Faction>>` (`is_playing`, `Player::is`, `karma_prevented`, `has_deal`, `are_allies`, `find_neighbours`, the event constructors `X::new(game, faction)` and more). Lists and maps keyed by faction use `ContainsOpt::contains_opt` and `MapOpt::get_opt` (iter.rs). `history.timers` is keyed by `Option<Faction>`, because host events have a timer too. C#'s nullable `Faction?` fields that could also hold `None` (`battle_winner`, `battle_loser`, `current_ecaz_or_ally_fighter`, `owner_of_flight_discovery`) are `Option<Option<Faction>>`. An executed event that needs a faction uses `.required("faction")?` (87 places), and code that C# only ran with a faction uses `.expect("faction")` (about 170). Bots decide only for players with a faction, except to end a phase.
  - The harness found two changes, both fixed: a player without a faction got `viewer.faction: null` instead of "None" in the view, and a bot without a faction could no longer end a phase. The bot golden tests found a third: `pay_for_card` must return `Option<Faction>`, for payments to the bank.
  - Second reference. The shared reference refuses undefined values of the enums closed before 4d68cc7. `~/2d-ref/rust` is a fresh copy of `~/treachery-native-ref/rust` (4d68cc7) plus `~/2d-ref/closed-enums.patch`, which now also handles `[Flags]` enums, built into `~/2d-ref/bin/observe`. `~/2d-ref/closed-enums.txt` lists the enums this branch closed. The copy from caabede is kept as `~/2d-ref/rust-caabede` and `bin-caabede`.
  - Rewriting scripts are in `~/2d-ref/tools-a1507`: copies of `~/2d-ref/tools` with private temp files, and `fixopt.py`, `wrapexpr.py`, `require.py`, `containsopt.py`, `mapopt.py`, `viewfix.py`, `misc.py` and `widen.py`, which read `cargo check --message-format short` errors. Review what they write: `require.py` once put `.required(..)?` inside an `Ok(..)` where `None` was valid.
  - Not done: merging coord-integration b781d2c, which brings 3b (messages as plain data). It conflicts in 32 files, so I aborted it and stopped here. Per the coordinator: the enums this branch closed (`ShipmentPermission`, `MainPhase`, `Phase`, `Faction`) move from the `plain` to the `optional` list in `engine/src/messages/part.rs`. `Payment.by` is already `Option<Faction>` here. Tests that use `to_cs_string`, `ExpressionElement`, `m.target` or `m.expression` become `plain_text()`, `Part`, `m.to` and `m.parts`. One trap when resolving: before `Faction` closed, a logged `Option<Faction>` that was `None` printed nothing (a C# null) while `Faction::None` printed "None". Now both are `None`. `SetShipmentPermission` without factions logs `ExpressionElement::Null` explicitly to keep printing nothing, and the merge must keep that.
  - Next, in order: the merge, then check.sh and 5 + 5 games against `~/2d-ref/bin/observe` (rebuild it from `~/treachery-native-ref/rust` plus the patch if the shared reference has been rebuilt). Then milestone 2 (handles that fail to decode), 3 (newtypes) and 4 (unsigned amounts and `HalfPoints`, which also removes 3b's `Part::Float` and `Double`). No negative-amount decision (risk 5) has been made yet. They belong to milestone 4.
- 2026-11-16: 2d continued on branch `worktree-agent-ad9f4c6465c8df314`, from `a79cf64`. Three commits: the merge of coord-integration b781d2c (3b), milestone 2, and the first newtype of milestone 3. check.sh passes on the last one.
  - Merge (`5f3074c`). `Faction`, `MainPhase` and `Phase` are in `Part`'s `optional` list. `ShipmentPermission` stays `plain`: it has no absent member, and its empty set is a value. The trap was real in ten places, one log line and nine status or flash texts: a C# nullable faction (`battle_winner`, `battle_loser`, the clairvoyance asker, the winning bidder, an opponent that may not exist, the factions of a collection division not yet proposed) logged nothing when null in 3b, but printed "None" once the branch flattened it. Each now logs an `Option<Option<Faction>>`, and `From<Option<Option<T>>> for Part` turns the outer `None` into `Part::Null` (engine-conventions.md, "Enums"). To find them, I built coord-integration with `NullPart` removed from `Faction` and listed every call that stopped compiling. SetShipmentPermission keeps its explicit `Part::Null`. The skin's static snapshot keeps the branch's `!panic` lines for `Faction` 0. 5 bots and 5 fuzz games (seed 1) pass against the second reference.
  - Milestone 2 (`ee7ba5a`). Event fields that name one location, territory, leader, hero or treachery card hold the handle (`Shipment.to: Option<Location>`), and their accessors and setters are gone. JSON writes the id and -1 for none, and an unknown id doesn't decode (`json_handle!` in json/mod.rs). Defaults are the object with id 0, as C# left the id at 0, so event JSON doesn't change. The one exception is `SkillAssigned`'s leader (no leader has id 0), whose default now writes -1. engine-conventions.md ("Ids in events") and behavior-changes.md ("Event ids that name no game object") describe it. The id lists in one string (`_cardIds`, `_forceLocations`) are left for 4b.
  - Second reference. `~/2d-ref/rust` now also carries `~/2d-ref/handle-ids.patch`: with `TREACHERY_CHECKED_IDS` set, the 76 id members refuse an id that names nothing. `~/2d-ref/bin` is rebuilt from it; the previous binary and sources are `~/2d-ref/bin-m1` and `~/2d-ref/rust-m1-src-backup`. Run it as `TREACHERY_CLOSED_ENUMS=$(cat ~/2d-ref/closed-enums.txt) TREACHERY_CHECKED_IDS=1 observe live --reference ~/2d-ref/bin/observe`. Against it, 5 of 5 bots and 4 of 5 fuzz games (seed 1) pass after milestone 2 and after `PlayerId`. Fuzz game 0 first differs at trace line 6956, in the view replica, not the game: the replica, rebuilt from a game the fuzzer had already broken (a card in two hands), applies a `Discarded` that nothing paused. The reference enters phase `None` there, and milestone 1 made that a Game Error (behavior-changes.md, "Event values that name no enum member"). The merge commit, given the same id refusals, shows the same difference on the same trace, so it doesn't come from milestone 2. The game itself matches to the end.
  - Without the patch, the old code accepted an id that names nothing: fuzz game 0 plays `Karma` with card 132, which the merge commit accepts as a Karama without a card.
  - Milestone 3, `PlayerId` (`f199353`): `PlayerId(u8)` with `index()` replaces `PlayerIndex { index }`. No behavior change.
  - Left in milestone 3, each reaching past the engine: `Seat(u8)` (`Player.seat` is -1 until `determine_positions_at_table`, so `Option<Seat>` or a seat drawn at creation; `human_seats`, the view's `Viewer.seat` and about 190 server lines use `i32` seats, with -1 for none in the protocol); `Turn(u8)` (`turn.current_turn` starts at 0, `predicted_turn`, `Report.turn`, `Moment.turn`, `BeneGesseritPrediction.turn`, `maximum_turns`, and the view's turn fields); `Sector(u8)` (`board.sector_in_storm` is -1 before the first storm, homeworlds have no sector, and storm moves are arithmetic modulo the sector count); `DealId(u32)` has nothing to wrap until 3a gives deals an id; `UserId(i64)` in the server and protocol (`i32` today). Then milestone 4. No negative-amount decision (risk 5) is made yet.
  - Scripts for milestone 2 are in `~/2d-ref/tools-ad9f`: `m2_fields.py` (fields, defaults, members.rs), `m2_calls.py` (rewrites calls of removed accessors and renamed fields at `cargo check --message-format json` error spans), `idmembers.py` and `refpatch.py` (the reference patch), `fix_index.py`.
- 2026-11-16: main 51e5896 (Voice before Prescience, pause reasons, the Nexus card leak after auctions, Kaitain thresholds) is merged into coord-integration 04bc90c on branch `worktree-agent-a4937f4d43034659c`. Main's rule changes are rewritten in 2d's types (`Option<Faction>`, `PlayerId`, `Territory` in `BattleInitiated`). `VoiceDeclined` has cases in `events.jsonl`. Main left 124 log lines without their `dom` text and a stray conflict marker in behavior-changes.md; both are fixed here. check.sh, the web build and `check:rich-text` pass. The shared reference is at game version 189, so the harness ran a build of the merge with the version set back to 189 and empty `waitingActions` left out. All 5 bots and 5 fuzz games (seed 1) still differ from it, each first at one of these intended changes: the auction winner's `ReplacingCardJustWon` step (bots 0, 3, 4 and fuzz 3), the removed `voiceNote` (bots 1 and 2), or an `AutomationConfigured` with an undefined `Faction` that 2d refuses (fuzz 0, 1, 2 and 4).
- 2026-11-16 00:12: Main (efdef3e) has 3b, 2d milestones 1 and 2 with PlayerId, and the other session's game fixes (Voice before Prescience, pause reasons, the Nexus leak after auctions, Kaitain thresholds, game version 190), merged with every conflict resolved in the rewrite's types. check.sh, npm run build and check:rich-text pass. The shared reference is rebuilt from efdef3e (the previous one is `observe-before-v190`). Running: the rest of 2d in two agents, the engine half (Turn, Sector, u16 amounts, HalfPoints) and the server half (Seat, UserId). 3a starts after both.
- 2026-11-16: 2d's `Seat(u8)` and `UserId(i64)` are done on branch `worktree-agent-a28a8234f64b71c81` (from efdef3e). `Player.seat` is an `Option<Seat>`, `None` only between creating the players and drawing their seats in the same `EstablishPlayers`, so game code reads `Player::seated()` (engine-conventions.md, "Players"). `human_seats`, the view's `Viewer.seat` and `Seating`, and the server's participation, pauses and seating calls use `Seat`, with `Option<Seat>` for users without one. `UserId` is in `rust/protocol/src/ids.rs` and the server; the database already stores user ids as 64-bit SQLite integers, and the view's `Viewer.user_id` is a plain `i64` because the view can't depend on the protocol crate. The engine depends on ts-rs now, for `Seat`'s TypeScript type. The JSON is unchanged: views, lobby messages and stored participations still write -1 for no seat, and `npm run build` leaves web/src/protocol and web/src/generated unchanged. One deliberate protocol change: a call with a seat outside 0 to 255, or -1 where a seat is required, is a `badCall` (protocol.md). `FactionSelected.seat` stays `i32` so fuzzed events decode as before. check.sh, `npm run build`, `check:rich-text` and e2e_server.py pass, and 12 of 12 bots and 12 of 12 fuzz games (seed 1) pass against the shared reference.
- 2026-11-16: 2d continued on branch `worktree-agent-a9b3d867a1638d867` from efdef3e, with main d88fb00 (Seat, UserId) merged in. Commits: `2f240e4` `Turn` and `Sector`, `ebca7ae` the merge, `d0f9138` a player's spice as `u16`. check.sh passes. Against the shared reference, 12 of 12 bots games pass, and 7 of 12 fuzz games (seed 1, recorded with the reference and replayed one at a time). The other 5 first differ at a bid carrying an amount out of range: an Emperor contribution larger than the Emperor offered (games 2, 5, 7 and 8, refused now) and a pass whose unchecked amounts won a silent auction (game 4).
  - `Turn(u8)` (game/turn.rs): turn 0 is `Turn::BEFORE_START`, a value of the type, because the setup has its own report and moments. `Player.predicted_turn` and `alliances.last_turn_card_was_traded` are `Option<Turn>`. Event JSON refuses a turn outside 0 to 255, and `EstablishPlayers` refuses 0 turns. The view's turn fields are `Turn` with `#[ts(type = "number")]`, so web/ is unchanged.
  - `Sector(u8)` (map/sector.rs): `Location::sector` is `Option<Sector>` (homeworlds, the Polar Sink and unplaced attached locations have none), `board.sector_in_storm` is `None` before the first storm, and the modulo-18 arithmetic is `Sector::moved`. Nothing is in the storm before the first storm now, where C# compared -1 with -1. Views still write -1.
  - Spice: `Player.resources` and the five other spice fields are `u16`, changed only through `game::amounts::spend` and `gain`. The decisions are in behavior-changes.md ("Turns, sectors and amounts out of range"), the bugs in escapes.md ("Bids paid with spice nobody had").
  - Left for milestone 4, in this order: forces (`Battalion` amounts, `forces_killed`) and spice on the board (`board.resources_on_planet`) as `u16`, with the same helpers (`Battalion::change_forces` silently ignores a change below zero, a C# quirk to keep or decide on); event amount fields that are never negative as `u16` at decode, keeping `i32` where a negative value means something (a shipment back to reserves has negative force amounts); then `HalfPoints(u16)` for battle strength, which removes 3b's `Part::Float` and `Part::Double`. Floats left in the engine: `GameStatistics.time` (`f32`) and battle strength. Reads of the `u16` spice fields are `i32::from(..)`. `~/2d-ref/tools-a9b3/wrapreads.py FIELD FILES..` wraps every read of a field that way and lists the writes to decide by hand, and `probe.py` then unwraps the reads that turn out to be `i32` already (fields of the same name on events).
- 2026-11-16: 2d continued on branch `worktree-agent-a19fc5da557757cf8` from main 905c640, one commit: forces and spice on the board as `u16`. check.sh passes.
  - Battalion amounts, the tanks (`forces_killed`, `special_forces_killed`, `total_forces_killed_in_battle`) and `board.resources_on_planet` are `u16`. Reads widen (`battalion.forces()`, `i32::from(..)`, `game.resources_in(l)`), and changes go through `amounts::change`, so a change below zero is a Game Error where C# ignored it. Every `Player` function that moves, kills or revives forces returns `GameResult`, and so do about 60 game and phase functions that call them (`Step::Run` takes a fallible method, and `enter_or_else_try` exists beside `enter_or_else`). engine-conventions.md, "Amounts of spice and forces", has the rules.
  - Decisions and bugs are in behavior-changes.md ("Turns, sectors and amounts out of range") and escapes.md ("Forces below zero"). In short: validation refuses a passed Caravan, FaceDanced reserves and sources the player doesn't have, negative amounts in TakeLosses, Revival, KarmaFreeRevival, the Tleilaxu secret ally's NexusPlayed and smuggling, and gifts of spice a player still has to pay for a bid (`Game::spice_committed_to_bids`). The JSON reader refuses force lists out of range (`json::read_force_locations`). Two rule fixes: forces back to reserves from a territory take what each location holds, and Reinforcements kill reserves on any homeworld (`Player::kill_forces_in_reserves`). Everywhere else, a Game Error.
  - Harness, 12 bots and 12 fuzz games (seed 1), recorded with the shared reference into files and replayed one at a time (`observe live` runs out of memory re-recording failing fuzz games): 11 bots and 5 fuzz games pass. First differences: a passed Caravan, now refused (fuzz 0, 2, 11); the previous commit's refused Emperor bids and pass amounts (fuzz 4, 5, 7); and the deals panel during a silent Black Market auction (bots 9, fuzz 8), where deals that cost spice a bidder needs are no longer acceptable. That last one is a rule fix that players of well-formed games see. Recording 60 fuzz and 30 bots games with the branch found no Game Error from the new checks except the one now refused (fuzz 58, a gift during a silent auction).
  - Next, in order: event amounts that are never negative as `u16` at decode (67 `i32` fields on events; `Shipment.force_amount` and `special_force_amount` stay `i32` for shipments back to reserves, `FactionSelected.seat` stays `i32`, and `direction`, `seed` and the No-Field values need a look); then `HalfPoints(u16)` for battle strength, which removes `Part::Float` and `Part::Double`, and a representation for `GameStatistics.time` (`f32`); then the wave-level check of 50 bots and 50 fuzz games. Scripts for this commit are in `~/2d-ref/tools-a19f`: `widen.py` (widens u16 reads at cargo's type errors), `question.py` (adds `?`, or a suffix such as `.unwrap()`, at unused-Result warnings), `enclosing.py` and `convert.py` (turn the enclosing functions into `GameResult`), `loop.sh` (repeats them until nothing changes), and `record.sh` and `replay.sh` (the harness by files).
