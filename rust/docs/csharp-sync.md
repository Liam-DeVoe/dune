# Keeping the port in sync with the C#

The C# in `treachery-online/Shared` and `treachery-online/Bots` keeps changing after the port was generated from it. Many files in `rust/` are snapshots of that C#: the goldens, the generated modules, the coverage lists and the sample traces. When the C# changes, the snapshots go stale, and the tests built on them keep passing against the old C#.

`engine/tests/csharp_sync.rs` stops that. `engine/tests/golden/csharp_sync.sha256` holds the SHA-256 of every `.cs` file in Shared and Bots as of the last sync, and the test fails with the list of changed, added and removed files whenever the C# differs. A second test in the same file fails when a `Version <op> N` check in Shared has no entry in the oracle's `VersionGates` (`Oracle/GameConfig.cs`), so the fuzzer plays both sides of every version check.

## Procedure

Run every command with .NET 10 (`export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"`). Start from `git diff` of the C# against the commit the manifest was written at.

1. **Port the change.** Check every hunk line by line against the Rust, including hunks that look ported. Give each version check a test on both sides (as `engine/tests/sync_v188.rs` does for v188, with its C# side in `engine/csharp-ref/SyncV188.cs`). New `Game` fields need the canon attributes from [engine-conventions.md](engine-conventions.md#state-dump-names).
2. **Regenerate what csharp-gen writes.** The glue files (`events/mod.rs`, `log/mod.rs`, `prelude.rs`, `json_members.rs`):
   ```sh
   cd rust/engine/csharp-gen && dotnet run -- --glue
   ```
   It never overwrites a ported module; a new C# file gets a new skeleton module. Then the four coverage lists, each read by a test that fails for a site without a row in its doc, or a row for a site that no longer exists:

   | Command (in `rust/engine/csharp-gen`) | Test | Rows in |
   |---|---|---|
   | `dotnet run -- --message-collections ../tests/coverage/message_collections.txt` | `tests/live_lists.rs` | [live-lists.md](live-lists.md) |
   | `dotnet run -- --lazy-queries ../tests/coverage/lazy_queries.txt` | `tests/lazy_queries.rs` | [lazy-queries.md](lazy-queries.md) |
   | `dotnet run -- --span-binding ../tests/coverage/span_binding.txt` | `tests/span_binding.rs` | [span-binding.md](span-binding.md) |
   | `dotnet run -- --stored-event-collections ../tests/coverage/stored_event_collections.txt` | `tests/span_binding.rs` | [span-binding.md](span-binding.md) |

   The lists name sites by file and line, so an edit above a site moves it, and the test then reports the old row as stale and the new line as unclassified. Update the row's line number. A site that is really new needs a classification, and possibly a fix in the port.
3. **Regenerate the csharp-ref goldens.**
   ```sh
   cd rust/engine/csharp-ref
   dotnet run -c Release -- ../tests/golden/static_data.json          # also boundary, random_helpers, linq, messages
   dotnet run -c Release -- --events ../tests/golden/events.jsonl      # event JSON corpus: every event type
   dotnet run -c Release -- --canon-schema ../../differ/tests/golden/canon_schema.json
   dotnet run -c Release -- sync-v188 ../tests/golden/sync_v188.txt
   cd .. && python3 tools/gen_enumerations.py && python3 tools/gen_map_data.py
   ```
   `boundary.json` records `PointD.GetHashCode()`, which .NET seeds per process. Those two numbers change on every run and are not checked (`not_ported.txt`); restore them from git to keep the diff clean.

   Some goldens hold whole C# state dumps or their SHA-256. A change to any field of `Game` or a type it reaches (a new field, a renamed one, a new log record) makes them all stale, even when the behaviour they test is unchanged. Rerun each probe and copy the values from its C# output, never from the Rust failure message:

   | Golden | Probe | Test |
   |---|---|---|
   | `differ/tests/golden/canon_schema.json` | `--canon-schema` | `differ/tests/field_mapping.rs` |
   | `engine/tests/golden/undo_identity.json` | `undo-identity ../tests/golden/undo_identity.json` | `undo_identity.rs` |
   | State hashes in `review_pkg_i.rs` | `review-i` (`<name>.hash = …`) | `review_pkg_i.rs` |
   | State hashes in `review_pkg_j.rs` | `review-j` | `review_pkg_j.rs` |
   | Dump fragments in `review_pkg_a.rs` and `review_pkg_f.rs` | `review-a`, `review-f` | same files |
   | `rust/traces/sample/*` | the oracle (step 5) | `differ/tests/sample_corpus.rs` |

   To find new ones, run `grep -lE '"[0-9a-f]{64}"' engine/tests/*.rs` for hashes and `grep -ln dump_game engine/tests/*.rs` for dumps. The other review probes record results but no state, and only need a rerun when the C# they probe changes.
4. **Add new event types and tables.** A new event type needs a module and a `GameEventType` variant (glue does both), a line in the port-assignments table, and a row in `ported_types.txt` only if it is a static-data type. Check that `events.jsonl` contains it.
5. **Rebuild the oracle and update its version gates.** Add every new `N` in `Version <op> N` to `VersionGates` in `Oracle/GameConfig.cs` (the second `csharp_sync` test lists the missing ones), then:
   ```sh
   dotnet build -c Release treachery-online/Oracle/Treachery.Oracle.csproj
   dotnet treachery-online/Oracle/bin/Release/net10.0/Treachery.Oracle.dll enums --out rust/traces/sample/enums.json
   dotnet treachery-online/Oracle/bin/Release/net10.0/Treachery.Oracle.dll verify --in rust/traces/sample
   ```
   If `verify` reports mismatches, the sample traces are stale. A new `Game` field changes every state hash, so every trace goes stale. Regenerate each sample trace with the seed, index and dump policy in its header (`--start I --games 1 --gzip true`, through `rust/tools/heavy.sh`), and `replay-battle` from `scripts/fixtures/battle.json`. Keep `rust/traces/sample` under 50 MB: use `--dump final` for long fuzz games.
6. **Note bot changes.** Bots are not ported yet. List each change in Bots in the "Bots" section of [port-assignments.md](port-assignments.md).
7. **Run the tests and the differ.** `cargo test -p treachery-engine`, `cargo test -p differ`, then `differ live` on fuzz and bot games, both through the CPU rules in PORTING.md.
8. **Update the manifest.**
   ```sh
   UPDATE_CSHARP_SYNC=1 cargo test -p treachery-engine --test csharp_sync
   ```
   Update it last. Its contents say the port matches this C#.

## Syncs

| Date | C# change | Notes |
|---|---|---|
| 2026-11-08 | Nov 7: v188 alliance phase (`AlliancePassed`), `PrescienceSeenPlans`, `Revival.Validate` return, `GameStatus` | First manifest. See the escape in [escapes.md](escapes.md#c-changes-after-the-port-was-generated-2026-11). |
| 2026-11-08 | `AllianceOffered`: since v188, offers to the two new allies lapse when an alliance forms (bots waited forever on them) | Oracle also passes for seated humans in the v188 alliance phase (`source: "human"`). Differ seed 71: 100/100 bots, 100/100 fuzz. |
| 2026-11-09 | Every faction identifier, event type, file and wire name now uses the faction's name instead of a colour (`Faction.Emperor`, `EmperorBidSupport`, `World.SalusaSecundus`). Also server-only `PhaseControlInfo` and the `IGameHub`/`IGameClient` phase-control methods. |
