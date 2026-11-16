# The skin crate

`rust/skin` (`treachery-skin`) is the port of `Shared/Model/Skin.cs` and `Shared/DefaultSkin.cs`: how messages, game objects and enums become text, plus the skin's art references, HTML popups, faction rules sheets and its JSON form. The server (`server/src/render.rs`: error texts and mails) and the view layer (log and status text) use it. Read [DEVELOPMENT.md](../../DEVELOPMENT.md) and [engine-conventions.md](engine-conventions.md) first.

## Layout

| Rust | C# |
|---|---|
| `src/skin.rs` | `Skin.cs`, regions Attributes, Descriptions and NamesAndImages, and the renderer of messages (`Skin::render`) |
| `src/faction_manual.rs` | `Skin.cs`, region FactionManual (`GetFactionInfo_HTML`, `GetFactionSheet`, the twelve templates) |
| `src/skin_json.rs` | `Skin.cs`, region LoadingAndSaving (`Load`, `Fix`, `SkinToString`) |
| `src/default_skin.rs` | `DefaultSkin.cs`. Generated from the C# skin once and edited by hand since; `tests/skin_json.rs` checks its JSON form against `tests/golden/default_skin.json` |
| `src/text.rs` | `WebUtility.HtmlEncode` and the regular expression of `WithSpiceIcons` |

`treachery_skin::default_skin()` is `DefaultSkin.Default`, built once per process.

## Using it

```rust
use treachery_engine::parts;
use treachery_skin::default_skin;
let skin = default_skin();
skin.render(&message);                                      // a message's text
skin.describe(&Part::from(Faction::Atreides));              // one part
skin.describe_faction(Faction::Atreides);                   // one value of a known type
skin.render_parts(&parts![Faction::Emperor, " pays ", 3]);  // any text, in the syntax of express!
```

The skin's own texts that name game values are built the same way, with `parts!`, so a custom skin's label is always printed as it is, braces included. The faction sheets are the exception: they are HTML templates with `{N}` placeholders for the sheet parameters (`Skin::fill_sheet`). An advantage title that comes from a skin label goes into the template with its braces escaped.

Functions that render one type of value are named after it: `describe_rule`, `get_popup_terror_type`. Where two take the same type, the name says what the other parameter adds (`get_popup_hero_in_game`, `get_popup_stronghold_advantage_for`). Optional C# parameters are explicit (`with_spice_icons(text, 18, true)`).

**Exceptions.** `describe` panics where C# throws (a resource card without a location, a skin missing a label that an indexer reads). Neither happens with the default skin in a valid game. `Skin::load` returns an error instead of panicking.

## Known differences

- An `Exception` element prints as `Exception.ToString()` without the stack trace (the engine has none). C# appends the trace. Only "Game Error" messages contain one.
- Everything assumes the invariant culture. Under en-US with ICU, C# would print infinity as `∞`.
- `\b`, `\w` and `\d` of `WithSpiceIcons` and `ToUpper` of the sheet titles use Rust's Unicode tables, which differ from .NET's outside the ASCII and Latin-1 text the default skin holds.
- `Skin::load`'s JSON errors carry serde's messages, not .NET's. Malformed JSON that System.Text.Json reads differently (duplicate property names, comments, numbers in strings) is not covered.

## Snapshot tests

The tests compare the rendered texts with snapshots in `tests/golden/`. They were first recorded from the C# skin and have matched the Rust output since, so a difference is a change in what players read.

| Test | Snapshot | What |
|---|---|---|
| `tests/static_corpus.rs` | `static.jsonl.gz` | Every describe, image URL and popup function on every value of every enum (plus -1 and 12345 for the enums that are still open), every location, territory, leader, the Messiah, every treachery and resource card, numbers and other values, the center of attached locations, and `with_spice_icons` edge cases |
| `tests/trace_corpus.rs` | `traces/*.skin.jsonl.gz` | Replays the games in `rust/testdata/games` and renders every validation result, the message and short message of every event, every report message, the `GameStatus` texts of status queries, and at the end every faction sheet and leader popup |
| `tests/skin_json.rs` | `default_skin.json`, `load.jsonl.gz` | The default skin's JSON form, and `Skin::load` with fixes on edited skins |

After a deliberate change, rewrite the snapshots and review the diff:

```sh
UPDATE_GOLDENS=1 cargo test --release -p treachery-skin
```

A call added to the static corpus input fails `static_corpus.rs` until the test handles it, so new functions can't go unchecked silently.
