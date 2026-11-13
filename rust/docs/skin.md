# The skin crate

`rust/skin` (`treachery-skin`) is the port of `Shared/Model/Skin.cs` and `Shared/DefaultSkin.cs`: how messages, game objects and enums become text, plus the skin's art references, HTML popups, faction rules sheets and its JSON form. The server (`server/src/render.rs`: error texts and mails) and the view layer (log and status text) use it. Read [PORTING.md](../../PORTING.md) and [engine-conventions.md](engine-conventions.md) first.

## Layout

| Rust | C# |
|---|---|
| `src/skin.rs` | `Skin.cs`, regions Attributes, Descriptions and NamesAndImages, plus the rendering half of `Message`/`MessagePart` (`ToString(IDescriber)`) |
| `src/faction_manual.rs` | `Skin.cs`, region FactionManual (`GetFactionInfo_HTML`, `GetFactionSheet`, the twelve templates) |
| `src/skin_json.rs` | `Skin.cs`, region LoadingAndSaving (`Load`, `Fix`, `SkinToString`) |
| `src/default_skin.rs` | `DefaultSkin.cs`. Generated from the C# skin once and edited by hand since; `tests/skin_json.rs` checks its JSON form against `tests/golden/default_skin.json` |
| `src/text.rs` | `WebUtility.HtmlEncode` and the regular expression of `WithSpiceIcons` |
| `dotnet-compat/src/fmt.rs` `string_format` | `string.Format` (composite formatting, .NET 10 parser) |

`treachery_skin::default_skin()` is `DefaultSkin.Default`, built once per process.

## Using it

```rust
use treachery_skin::{args, default_skin};
let skin = default_skin();
skin.message_to_string(&message);                        // message.ToString(DefaultSkin.Default)
skin.message_to_string_with_default_describer(&message); // message.ToString() on the C# server: "...*"
skin.describe(&ExpressionElement::from(Faction::Atreides)); // Describe(object)
skin.describe_faction(Faction::Atreides);                   // Describe(Faction)
skin.format("{0} pays {1}", &args![Faction::Emperor, 3]);    // Format(m, params object[])
```

Overloads of one C# name get the parameter type appended, as in the engine: `Describe(Rule)` is `describe_rule`, `GetPopup(TerrorType)` is `get_popup_terror_type`. Optional C# parameters are explicit (`with_spice_icons(text, 18, true)`).

**Live collections.** A message can refer to a game collection (`LiveList`), so it renders the collection's contents at render time. The `_in` variants (`describe_in`, `format_in`, `message_to_string_in`) take the game to read them from. Without a game they print the snapshot the engine took when the message was built, which is the same thing for a message rendered right after it was made (everything the server renders).

**Exceptions.** `Describe` panics where C# throws (a resource card without a location, a skin missing a label that an indexer reads). Neither happens with the default skin in a valid game, so `Format` doesn't catch these panics. It still returns the template unchanged when `string.Format` fails. `Skin::load` returns an error instead of panicking.

## Known differences

- An `Exception` element prints as `Exception.ToString()` without the stack trace (the engine has none). C# appends the trace. Only "Game Error" messages contain one.
- Everything assumes the invariant culture. Under en-US with ICU, C# would print infinity as `∞`.
- `\b`, `\w` and `\d` of `WithSpiceIcons` and `ToUpper` of the sheet titles use Rust's Unicode tables, which differ from .NET's outside the ASCII and Latin-1 text the default skin holds.
- `Skin::load`'s JSON errors carry serde's messages, not .NET's. Malformed JSON that System.Text.Json reads differently (duplicate property names, comments, numbers in strings) is not covered.

## Snapshot tests

The tests compare the rendered texts with snapshots in `tests/golden/`. They were first recorded from the C# skin and have matched the Rust output since, so a difference is a change in what players read.

| Test | Snapshot | What |
|---|---|---|
| `tests/static_corpus.rs` | `static.jsonl.gz` | Every describe, format, image URL and popup function on every value of every enum (plus -1 and 12345), every location, territory, leader, the Messiah, every treachery and resource card, numbers and other values, the center of attached locations, `with_spice_icons` edge cases, and 4,000 random `string_format` templates |
| `tests/trace_corpus.rs` | `traces/*.skin.jsonl.gz` | Replays the games in `rust/testdata/games` and renders every validation result, the message and short message of every event, every report message, the `GameStatus` texts of status queries, and at the end every faction sheet and leader popup |
| `tests/skin_json.rs` | `default_skin.json`, `load.jsonl.gz` | The default skin's JSON form, and `Skin::load` with fixes on edited skins |

After a deliberate change, rewrite the snapshots and review the diff:

```sh
UPDATE_GOLDENS=1 cargo test --release -p treachery-skin
```

A call added to the static corpus input fails `static_corpus.rs` until the test handles it, so new functions can't go unchecked silently.
