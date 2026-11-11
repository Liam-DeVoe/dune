# Review: package I (battle actions around the fight)

Scope: the 20 files in table I of [port-assignments.md](../port-assignments.md): Voice, Prescience, Thought, ThoughtAnswered, Diplomacy, PoisonToothCancelled, PortableAntidoteUsed, RockWasMelted, AuditCancelled, Audited, FaceDanced, FaceDancerReplaced, FaceDancerRevealed, BeneGesseritBattleAnnouncement, ResidualPlayed, Planetology, JuicePlayed, KarmaEcazDial, KarmaPrescience and TraitorDiscarded.

## Result

No confirmed differences, and none suspected.

Every member was read side by side with its C# original. Every case that looked risky was then run in both engines (see "Probes" below), and all of them matched. The things I checked for:

- statement order before a throw, and which exception is thrown
- `Faction?` null against `Faction.None`
- receiver-null and argument evaluation order
- live lists placed in messages
- lazy queries
- random draw order
- `List.Remove`/`First` semantics
- wrapping arithmetic and checked `Sum`
- every version gate: Voice 108/178/140, Diplomacy 179, RockWasMelted 146, FaceDanced 150/157, JuicePlayed 117

Places that looked like candidates and turned out equivalent:

- `Prescience.ValidAspects`, `Voice.MayUseVoice` and `FaceDanced.MaximumNumberOfForces` evaluate a `CurrentBattle.Territory` argument before the null receiver throws. The Rust code dereferences the receiver first. This can't differ, because `TerritoryLookup.Find` uses `SingleOrDefault` and never throws.
- `BeneGesseritBattleAnnouncement.ValidTerritories` is a lazy `Where` that `Game_Events.cs:640` reads with `.Any()`. Rust builds the whole list. The only clause that can throw (`p.AlliedPlayer.AnyForcesIn`) throws on the first territory or never, so stopping early changes nothing.
- `JuicePlayed`'s `check_current_player` takes the `PlayerSequence` out of the game while it runs. `PlayerSequence.CheckCurrentPlayer` never reads the game's own sequence fields, so nothing can see the empty slot.
- `FaceDanced.DetermineSourceLocations` puts a lazy query in the log. The query's source is a freshly parsed dictionary that nothing else holds, so the snapshot plus the C# runtime type name (`SOURCE_LOCATIONS_QUERY_TYPE`) is equivalent.

## Probes

The bots and fuzz traces hardly reach the costly parts of this package. [coverage.md](../coverage.md) lists the following as uncovered or barely covered:

- the paying branch of `AuditCancelled`
- most of `KarmaEcazDial`
- the plan swap in `JuicePlayed`
- the removal of source forces in `FaceDanced.ReplaceForces`

So I wrote a probe harness that reaches them on purpose:

- `rust/engine/csharp-ref/ReviewI.cs` (mode `review-i`)
- `rust/engine/tests/review_pkg_i.rs`

Both sides replay steps 0 to 127 or 0 to 129 of `traces/sample/bots-2026-00042` (stored as `engine/tests/review_pkg_i/bots42.jsonl`). The replay includes the oracle's queries and the after-step `GetApplicableEvents` calls. Both sides then run hand-made events, mostly with `performValidation: false`, and compare:

- the returned message or exception, in canonical dump form
- the SHA-256 of the full state dump

The two prefix hashes equal the trace's own hashes, which checks the harness. A few `HOOK:` steps change state directly so a branch becomes reachable. For example, Tleilaxu's first face dancer is set to the winning leader so that `FaceDanced` runs to the end.

What the 12 tests cover, all passing:

| Test | What it runs |
|---|---|
| `karma_ecaz_dial` | with and without a Karma card; leader difference written to `EcazKarmaBonus` |
| `juice_played`, `juice_then_the_rest_of_the_trace` | Aggressor swap of plans and traitor actions, GoFirst and GoLast before a battle, then the remaining 20-odd trace steps on the changed game |
| `residual_played` | random leader kill for both sides, plan revoked when the killed leader is in it |
| `plan_modifiers` | RockWasMelted, PortableAntidoteUsed, Diplomacy, KarmaPrescience, Voice, PoisonToothCancelled, BeneGesseritBattleAnnouncement, TraitorDiscarded |
| `thought` | Thought, then ThoughtAnswered; ThoughtAnswered without a Thought (NullReferenceException) |
| `audit` | AuditCancelled paying and not paying, during and after the fight; Audited |
| `face_dancers` | FaceDancerReplaced (shuffle and draw); FaceDanced with reserves, with planet sources, and when the last unrevealed dancer triggers ReplaceFacedancers; FaceDanced with no matching dancer |
| `exceptions_keep_partial_state` | KarmaEcazDial and ResidualPlayed outside a battle: the state changes made before the throw stay, as in C# |
| `face_danced_then_the_rest_of_the_trace` | the rest of the trace after a face dance |
| `validation_messages` | 18 `Validate()` calls: FaceDanced count messages, a FormatException, a null `_targetForceLocations`, `int` wrap in `ForcesFromReserve + Sum`, FaceDancerReplaced, TraitorDiscarded, Voice, AuditCancelled, BeneGesseritBattleAnnouncement (wrong faction, unknown territory id), Thought, ThoughtAnswered |

Regenerate the expected values with `cd rust/engine/csharp-ref && dotnet run -c Release -- review-i`. Set `REVIEW_I_DUMPS=DIR` on both sides to write the dumps when a hash differs.

I also replayed the sample corpus (17 traces, all match) and ran `differ live --mode fuzz --seed 9191`.

## Would the differential traces catch this class?

Mostly no. The code itself is simple. The risk sits in branches the bots never take and the fuzzer seldom reaches. A fuzzed event of the right type has to arrive while a battle has both plans in, and as the right player. Exceptions there would show up in traces once reached, because each step dumps the full state and the exception. Reaching them is the hard part.

Two changes would cover this whole class without hand-written probes:

1. **Steer the fuzzer into battle actions.** Give `FuzzerSteering` a rule: when `CurrentBattle` has both plans, or the phase is `RevealingFacedancer` or `AvoidingAudit`, submit one of the battle-action events of this package for an involved player. Fill its fields from the battle: the plan's cards, the opponent's leaders, and face dancer and target strings built from the battle territory. `KarmaEcazDial`, `JuicePlayed` with Aggressor, `ResidualPlayed` and `AuditCancelled` with `Cancelled` are the four with uncovered lines today.
2. **Allow a share of unvalidated steps.** Let the fuzzer submit, say, 2% of its events with `performValidation: false` as host. That is how `Undo` re-executes history anyway. Execution code then runs in states its applicability check would have kept it out of, which is where the partial-state-on-exception paths live. The probe harness above does exactly this by hand.

Either change shows up in [coverage.md](../coverage.md) as soon as it works. The coverage lines listed there for this package are the acceptance test.
