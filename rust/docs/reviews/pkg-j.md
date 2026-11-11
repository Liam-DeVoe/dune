# Package J review: faction specials

Scope: the 21 files in table J of [port-assignments.md](../port-assignments.md) (Karma, Nexus, Moritani terror, Ecaz ambassadors, CHOAM, clairvoyance and the small card events), C# and Rust side by side, member by member. C# paths are relative to `treachery-online/Shared/`, Rust paths to `rust/engine/src/`.

Every version gate (`<150`, `<156`, `>=164`, `>=166`, `>=167`, `>=169`, `<171`, `>=174`, `<176`), the float/double split in Robbery (`0.5f`) and Amal (`0.5`), the deliberate C# bugs (Emperor's contribution refunded to Richese's ally, Atomics recording `MetheorUsed`, `KarmaChoamDiscard` playing the first Karama card instead of `CardToUse`), statement order before each throw, and the lazy `Where` in `MetheorPlayed` all match.

The differences I found all come from one situation: a hand that holds a null card. C# reaches it with validation on. `RicheseGaveCard.Validate` always returns null, and `_cardId = -1` makes `Card` null, so `target.TreacheryCards.Add(Card)` puts a null in the ally's hand (`Events/RicheseGaveCard.cs:42,62`). Rust copies that with `push(card.or_null())`. `TerrorRevealed` Robbery adds a null too when both treachery decks are empty (`Events/TerrorRevealed.cs:214`). From there the two engines handle the null card differently in two ways.

Tests are in `engine/tests/review_pkg_j.rs`. Expected values come from `csharp-ref review-j` (`engine/csharp-ref/ReviewJ.cs`). All four tests fail today.

## Confirmed

### 1. `RandomOrDefault` picks a null card: C# skips it, Rust discards it and throws

C# `RandomOrDefault` returns the null element, and the caller's `card != null` (or `Game.Discard(TreacheryCard?)`'s own `card is null` check) skips it. Rust's `random_or_default` returns `Some(TreacheryCard::NULL)`. `if let Some(card)` accepts it, and `Game::discard_treachery_card_boolean` treats it as a real card. It finds the owner with `has(Some(NULL))`, logs the discard, removes the null from the hand, and then `RecentlyDiscarded.Add(NULL, ...)` throws `ArgumentNullException: Value cannot be null. (Parameter 'key')`.

Tested site, CHOAM betrayal with Nexus:

- C#: `Events/NexusPlayed.cs:421-424`. The event succeeds and CHOAM keeps `[null]`.
- Rust: `events/nexus_played.rs:566-577`. The event returns "Game Error: Value cannot be null. (Parameter 'key')" after the Nexus card was already played.
- Tests:
  - `richese_gave_null_card_then_nexus_choam_betrayal` runs the whole path with validation on. Richese gives its ally CHOAM card id -1, then Atreides, holding CHOAM's Nexus card, betrays CHOAM. The state hash after `RicheseGaveCard` matches C# (`3d32...`). After `NexusPlayed`, C# returns null with hash `77a6...`, and Rust returns the Game Error. The phase is set directly to Contemplate on both sides; everything else goes through `Execute(true, true)`.
  - `nexus_choam_betrayal_skips_null_card` is the short form, in the TradingFactions phase.

Untested sites with the same code shape. Each takes a card from `random_or_default` over a hand and passes it on as `Some(NULL)`:

| Site | C# | Rust | C# behaviour | Rust behaviour |
|---|---|---|---|---|
| Sabotage terror | `Events/TerrorRevealed.cs:231` | `events/terror_revealed.rs:337-344` | `Discard(null)` returns | discards null, throws |
| Atomics hand limit, Moritani and ally | `Events/TerrorRevealed.cs:195,198` | `events/terror_revealed.rs:283-294` | `Discard(p, null)` does nothing, loop draws again | throws on the first null |
| CHOAM secret-ally audit | `Events/NexusPlayed.cs:646-652` | `events/nexus_played.rs:896-901` | skips the audit logs | `register_known` of a null card, logs "You see: null" |
| `MakeAlliance` hand limit (package K) | `Game_Alliances.cs:61,63` | `game_alliances.rs:32-40` | as Atomics | as Atomics |

In each, the null belongs to another player than the one whose applicability is checked, so a C# exception in the null holder's own `GetApplicableEvents` doesn't block the event. This is the one way the difference shows up in a fuzzed game.

Would the traces catch it? Bot games, no: bots never send an unknown card id. Fuzz games could, but rarely. A fuzzer has to send `RicheseGaveCard` with an id that is -1 or unknown while Richese is allied, and then a later event has to sample the ally's hand. The trace would show it as a result difference (`accepted` against `error`) plus a hash difference.

What would catch the class: this is the Null handles rule in [engine-conventions.md](../engine-conventions.md#null-handles) ("Never write `Some(h)` for such a handle") broken inside a helper, so a grep for `Some(` won't find it. Two systematic checks:

- Make `random_or_default` (and the `highest_or_default` family) on handle types return `h.opt()` instead of `Some(h)`, or make `discard_treachery_card_boolean`/`discard_player_treachery_card_boolean`/`register_known_*` treat `Some(NULL)` exactly like `None`. Then add a unit test that calls every `Option<Handle>` parameter of a `Game` method with `Some(NULL)` and `None` and requires identical state.
- Give the fuzzer a "null card" move: when Richese has an ally, send `RicheseGaveCard` with `_cardId = -1` with some fixed probability. Every later event that samples or scans that hand then runs on a null card in both engines.

### 2. Eager `Vec` instead of a lazy `Where` that callers stop early

`Karma.ValidKarmaCards` (`Events/Karma.cs:231-237`) is a lazy `Where`, and every caller stops at the first match: `Any()` in `CanKarma` (`:241`) and so `Player.HasKarma`, `FirstOrDefault()` in the Karama events, `FirstOrDefault(pred)` in `ClairVoyancePlayed.Card` (`Events/ClairVoyancePlayed.cs:221,227`), and `KarmaChoamDiscard` (`Events/KarmaChoamDiscard.cs:53,68`). Rust collects the whole hand (`events/karma.rs:291-308`). If a null card comes after the first match, C# never reads it, and Rust throws `NullReferenceException` on `c.type_()`.

The same holds for `ChoamDiscarded.ValidCards(p).Contains(Card)` and `.Any()` (C# `Events/ChoamDiscarded.cs:44,49`, Rust `events/choam_discarded.rs:60`) and `NexusPlayed.ValidChoamCards(p).Any()` (C# `Events/NexusPlayed.cs:229,270`, Rust `events/nexus_played.rs:390`).

- Tests: `karma_valid_cards_stop_at_first_match` (hand `[Karama, null]`: C# `CanKarma`, `HasKarma` are True and `FirstOrDefault` is card 23, Rust throws) and `choam_valid_cards_stop_at_first_match` (hand `[Useless, null]`: C# True, Rust throws).
- How reachable it is: the hand is the event initiator's own. Its `GetApplicableEvents` scans the whole hand in many places, for example `Player.Has(Amal)` in `Game_Events.cs:535` and `JuicePlayed.CanBePlayedBy` in `Events/JuicePlayed.cs:70`. Those throw in C# too unless an earlier card matches each scan. So `Execute(true, ...)` usually fails in both engines with the same `NullReferenceException`, before the Rust-only throw. A difference needs a hand whose earlier cards satisfy every scan, or a caller that skips applicability (bots, `Execute(false, ...)`). The [package B review](pkg-b.md) and [package D review](pkg-d.md) checked these same `Any()` sites and concluded the predicates can't throw on a later element. That's true only for hands without null cards.
- Would the traces catch it? Unlikely, for the reason above. The bot phase will, if a bot ever holds a null card, because bot code calls `CanKarma` and `ValidKarmaCards` without the applicability gate.
- What would catch the class: a test that runs every static `Valid*`/`Can*`/`May*` helper whose C# body is a lazy query, on hands and lists that have a null after the first match, and compares results with C#. The `boundary.json` golden test has the machinery for this; its generated inputs just never include a null element after a valid one.

## Suspected, not confirmed

Neither of these is reachable through validated events, as far as I can tell.

- **`HasAlly` true with `AlliedPlayer` null.** `TerrorRevealed.ValidSneakAttackTargets(...).Contains(SneakAttackTo)` (`Events/TerrorRevealed.cs:82,121-134`) and `NexusPlayed.ValidEcazTerritories(g).Contains(...)` (`Events/NexusPlayed.cs:155,279`) dereference `p.AlliedPlayer` inside a lazy `Where` that `Contains` stops early. Rust collects (`events/terror_revealed.rs:193-216`, `events/nexus_played.rs:395-413`). That differs only if the ally is a faction nobody plays. I found no event that sets such an ally.
- **Null `Player` during `Execute(false, ...)`.** A few Rust bodies call `self.player()` earlier than C# reads `Player`. `TerrorPlanted` reads it before `TerrorIn(Stronghold).Any()` (C# `Events/TerrorPlanted.cs:124`, Rust `events/terror_planted.rs:150`). `TerrorRevealed` Robbery draws a card before it reads `Player`, where C# reads `Player.TreacheryCards` first (C# `Events/TerrorRevealed.cs:214`, Rust `events/terror_revealed.rs:313-315`). `AmbassadorActivated` CHOAM passes `Some(self.player())` where C#'s `Discard(Player?, card)` accepts null (C# `Events/AmbassadorActivated.cs:349`, Rust `events/ambassador_activated.rs:473`). `GetApplicableEvents(null, ...)` only offers host events, so none of these runs with a null `Player` under validation. They matter only for `Game.TryLoad` with `performValidation: false`. The fuzz mode that the [package F review](pkg-f.md) suggests, executing some events without validation, would cover them.

## Checked and equivalent

So the next reviewer doesn't repeat them:

- `NexusPlayed` Moritani betrayal: `TerrorIn(...).RandomOrDefault` on a value type gives `default(TerrorType)` when empty, and Rust's `unwrap_or_default()` removes and re-adds the same value.
- `NexusPlayed` Tleilaxu cunning: the draw loop re-reads `RevealedFaceDancers.Count`, which nothing in the loop changes. `AddRange` and the removals follow the set's order.
- `AmbassadorActivated` Atreides logs the victim's live hand through `LiveList`. `Harkonnen` materializes the lazy face dancer query once before `RandomOrDefault`.
- `ClairVoyancePlayed.Parameter1`: a `TreacheryCard` found through `HeroLookup` is still an `IHero`, so `ClairVoyanceAnswered` adds it to `ToldTraitors` in both engines (`ExpressionElement::Hero(Hero::TreacheryCard(_))`). `IsAbout` checks the enum type, not just the value.
- `TerrorPlanted`/`AmbassadorPlaced` duplicate-key adds use the `Key: ...` text, and `ValidTerrorTypes` keeps `Union` order.
- `Karma.ValidFactionAdvantages`: every faction block, rule check and `In()` gate matches in order.

## Differential runs

DIFFERENTIAL_RUNS_PLACEHOLDER
