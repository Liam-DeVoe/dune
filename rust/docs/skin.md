# The skin crate

`rust/skin` (`treachery-skin`) turns messages, game objects and enums into text. It also holds the skin's art references, HTML popups, faction rules sheets and its JSON form. The server (`server/src/render.rs`: error texts and mails) and the view layer (log and status text) use it. Read [CLAUDE.md](../../CLAUDE.md) and [engine-conventions.md](engine-conventions.md) first.

## Layout

| File | What |
|---|---|
| `src/skin.rs` | The `Skin` struct: names, descriptions, art references and popups of game values, and the renderer of messages (`Skin::render`) |
| `src/faction_manual.rs` | The faction rules sheets and faction info popups (`get_faction_info_html`, `get_faction_sheet`, the twelve templates) |
| `src/skin_json.rs` | Loading, fixing and saving a skin as JSON (`Skin::load`) |
| `src/default_skin.rs` | The default skin, edited by hand; `tests/skin_json.rs` checks its JSON form against `tests/golden/default_skin.json` |
| `src/text.rs` | HTML encoding and the regular expression that `with_spice_icons` uses |

`treachery_skin::default_skin()` is the default skin, built once per process.

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

Functions that render one type of value are named after it: `describe_rule`, `get_popup_terror_type`. Where two take the same type, the name says what the other parameter adds (`get_popup_hero_in_game`, `get_popup_stronghold_advantage_for`). There are no optional parameters: `with_spice_icons(text, 18, true)` names all three.

**Panics.** `describe` panics on a resource card without a location and on a skin missing a label it reads. Neither happens with the default skin in a valid game. `Skin::load` returns an error instead of panicking.

## Limits

- An `Exception` element prints its type and message, without a stack trace (the engine has none). Only "Game Error" messages contain one.
- Numbers are formatted the same everywhere, with no locale.
- `with_spice_icons`' `\b`, `\w` and `\d` and the upper-casing of sheet titles use Rust's Unicode tables. The default skin only holds ASCII and Latin-1 text, where that makes no difference.
- `Skin::load`'s JSON errors carry serde's messages.

## Snapshot tests

The tests compare the rendered texts with snapshots in `tests/golden/`. A difference is a change in what players read.

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
