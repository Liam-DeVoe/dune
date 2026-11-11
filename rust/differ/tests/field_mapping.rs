//! Checks that the Rust state dump has the same fields as the C# one, before
//! any game logic runs.
//!
//! - Field names: `tests/golden/canon_schema.json` lists, for every C# type the
//!   oracle's dumper can expand, the fields it writes (regenerate with
//!   `cd rust/engine/csharp-ref && dotnet run -c Release -- --canon-schema ../../differ/tests/golden/canon_schema.json`).
//!   The Rust side is `Registry::full()`, built from the `Canon` derives.
//! - Value kinds: every value in the sample traces' dumps must be a kind the
//!   Rust field can produce (a reference where Rust writes a reference, a list
//!   where Rust writes a list, and so on).
//! - Reference kinds: for every C# field whose type the dumper writes as a
//!   reference (`Game`, `Player`, `Map`, `Location`, `Territory`), the Rust
//!   field can write every form the dumper can, including the ones only another
//!   game produces (`OtherGame`, an expanded `Player`, `"foreign":true`). This
//!   needs no trace, so it fails as soon as a type is generated.
//! - The map: the Rust dump of a fresh `Map` equals the map in a sample trace.

use std::collections::{BTreeMap, BTreeSet};
use std::io::BufRead;
use std::path::PathBuf;

use serde_json::Value;
use treachery_engine::canon::{self, CanonWriter, Registry, Shape};

fn rust_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn csharp_schema() -> BTreeMap<String, (BTreeSet<String>, bool)> {
    let text = std::fs::read_to_string(rust_dir().join("differ/tests/golden/canon_schema.json")).unwrap();
    let v: Value = serde_json::from_str(&text).unwrap();
    let mut out = BTreeMap::new();
    for (name, t) in v.as_object().unwrap() {
        let mut fields: BTreeSet<String> = t["fields"].as_object().unwrap().keys().cloned().collect();
        if name == "Map" {
            fields.insert("$territories".into());
        }
        out.insert(name.clone(), (fields, t["reachable"].as_bool().unwrap()));
    }
    out
}

/// Types the dumper writes by hand rather than by reflection.
const SPECIAL: &[&str] = &["LoggedRandom", "$Exception"];

#[test]
fn field_names_match_the_csharp_dumper() {
    let cs = csharp_schema();
    let reg = Registry::full();
    let mut report = Vec::new();
    for e in &reg.errors {
        report.push(format!("Rust schema error: {e}"));
    }
    for (name, fields) in &reg.types {
        if SPECIAL.contains(&name.as_str()) {
            continue;
        }
        let Some((cs_fields, _)) = cs.get(name) else {
            report.push(format!("{name}: Rust type has no C# counterpart (check #[canon(type_name)])"));
            continue;
        };
        let rust: BTreeSet<String> = fields.keys().cloned().collect();
        for f in cs_fields.difference(&rust) {
            report.push(format!("{name}.{f}: C# field with no Rust counterpart"));
        }
        for f in rust.difference(cs_fields) {
            report.push(format!("{name}.{f}: Rust field with no C# counterpart (misnamed? add #[canon(name)] or #[canon(skip)])"));
        }
    }
    for (name, (_, reachable)) in &cs {
        if *reachable && !reg.types.contains_key(name) {
            report.push(format!("{name}: C# type in the dump with no Rust Canon type"));
        }
    }
    let fields: usize = reg.types.values().map(|f| f.len()).sum();
    println!("compared {} Rust types ({} fields) with {} C# types", reg.types.len(), fields, cs.len());
    assert!(reg.types.len() > 150, "the Rust registry looks incomplete");
    if !report.is_empty() {
        panic!("{} field-mapping problems:\n{}", report.len(), report.join("\n"));
    }
}

/// The kind of a JSON value, in the vocabulary of `Shape`.
fn kind(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(_) => "bool".into(),
        Value::Number(_) => "num".into(),
        Value::String(_) => "str".into(),
        Value::Array(_) => "list".into(),
        Value::Object(o) => {
            if let Some(r) = o.get("$ref").and_then(|r| r.as_str()) {
                if o.get("foreign").and_then(|f| f.as_bool()) == Some(true) {
                    format!("ref:{r}+foreign")
                } else {
                    format!("ref:{r}")
                }
            } else if let Some(t) = o.get("$type").and_then(|t| t.as_str()) {
                format!("obj:{t}")
            } else {
                let keys: Vec<&str> = o.keys().map(|k| k.as_str()).collect();
                format!("obj?:{}", keys.join(","))
            }
        }
    }
}

fn check_value(path: &str, v: &Value, shape: &Shape, problems: &mut BTreeSet<String>) {
    let k = kind(v);
    if !shape.kinds.contains(&k) {
        problems.insert(format!("{path}: C# wrote {k}, Rust can write {:?}", shape.kinds));
        return;
    }
    if let (Value::Array(items), Some(elem)) = (v, &shape.elem) {
        for item in items {
            check_value(&format!("{path}[]"), item, elem, problems);
        }
    }
}

fn walk(v: &Value, reg: &Registry, problems: &mut BTreeSet<String>) {
    match v {
        Value::Array(items) => items.iter().for_each(|x| walk(x, reg, problems)),
        Value::Object(o) => {
            if let Some(t) = o.get("$type").and_then(|t| t.as_str()) {
                if let Some(fields) = reg.types.get(t) {
                    for (k, x) in o {
                        if k == "$type" {
                            continue;
                        }
                        if let Some(shape) = fields.get(k) {
                            check_value(&format!("{t}.{k}"), x, shape, problems);
                        }
                    }
                }
            }
            for x in o.values() {
                walk(x, reg, problems);
            }
        }
        _ => {}
    }
}

#[test]
fn value_kinds_in_sample_traces_match() {
    let reg = Registry::full();
    let mut problems = BTreeSet::new();
    let files = differ::trace::list(&rust_dir().join("traces/sample"));
    assert!(!files.is_empty(), "no sample traces");
    for f in files {
        let reader = differ::trace::open(&f).unwrap();
        for (i, line) in reader.lines().enumerate() {
            let line = line.unwrap();
            // Every fifth line is plenty; full dumps repeat most of the state.
            if i % 5 != 0 && !line.contains("\"kind\":\"end\"") {
                continue;
            }
            let v: Value = serde_json::from_str(&line).unwrap();
            for key in ["state", "validation", "participation"] {
                if let Some(x) = v.get(key) {
                    walk(x, &reg, &mut problems);
                }
            }
        }
    }
    if !problems.is_empty() {
        panic!("{} kind mismatches:\n{}", problems.len(), problems.join_lines());
    }
}

trait JoinLines {
    fn join_lines(&self) -> String;
}
impl JoinLines for BTreeSet<String> {
    fn join_lines(&self) -> String {
        self.iter().cloned().collect::<Vec<_>>().join("\n")
    }
}

/// What `StateDumper.Value` can write for a field of each C# type it treats as a
/// reference (trace-format.md 7.2). The forms after the first appear only when the
/// value belongs to another game: after `Undo`, an event keeps the old game and
/// its player, and that player's locations come from the old game's map.
const REFERENCE_FORMS: &[(&str, &[&str])] = &[
    ("Game", &["ref:Game", "ref:OtherGame"]),
    ("Player", &["ref:Player", "obj:Player"]),
    ("Map", &["ref:Map"]),
    ("Location", &["ref:Location", "ref:Location+foreign"]),
    ("Territory", &["ref:Territory", "ref:Territory+foreign"]),
];

/// The C# field types from `canon_schema.json`: type -> field -> C# type name.
fn csharp_field_types() -> BTreeMap<String, BTreeMap<String, String>> {
    let text = std::fs::read_to_string(rust_dir().join("differ/tests/golden/canon_schema.json")).unwrap();
    let v: Value = serde_json::from_str(&text).unwrap();
    let mut out = BTreeMap::new();
    for (name, t) in v.as_object().unwrap() {
        let fields = t["fields"].as_object().unwrap().iter().map(|(f, ty)| (f.clone(), ty.as_str().unwrap_or("").to_string())).collect();
        out.insert(name.clone(), fields);
    }
    out
}

/// Every Rust field whose C# type is a reference type of `REFERENCE_FORMS` must be
/// able to write all of the dumper's forms for it. This is a static check: it
/// does not depend on a trace reaching the state where the rarer form appears.
/// The kinds a reference type's writer actually produces, for values that cover every
/// case: this game, another game (after an undo), and null. Compared with what the
/// type's shape claims, so a shape can't promise a form the writer never writes.
#[test]
fn reference_types_write_every_form_they_claim() {
    use treachery_engine::canon::{Canon, CanonShape};
    use treachery_engine::cs::{CsNull, Shared};
    use treachery_engine::enumerations::Faction;
    use treachery_engine::game::{Game, GameRef};
    use treachery_engine::location::Location;
    use treachery_engine::participation::Participation;
    use treachery_engine::player::{Player, PlayerIndex};
    use treachery_engine::territory::Territory;

    // `old` becomes another game of `game`, as after `Undo`.
    let mut old = Game::new_int32_participation(187, &Shared::new(Participation::new()));
    let p = Player::new(&old, Faction::Atreides);
    old.players.push(p);
    let old_id = old.id;
    let old_ref = old.reference();
    let old_player = old.player_at(0);
    let mut game = old;
    game.undo(0);
    let p = Player::new(&game, Faction::Emperor);
    game.players.push(p);
    let own_player = game.player_at(0);

    fn written(game: &Game, values: Vec<&dyn Canon>) -> BTreeSet<String> {
        values
            .into_iter()
            .map(|v| {
                let mut w = CanonWriter::for_game(game);
                v.canon(&mut w);
                kind(&serde_json::from_slice(&w.out).unwrap())
            })
            .collect()
    }
    fn claimed<T: CanonShape>() -> BTreeSet<String> {
        T::canon_shape(&mut Registry::default()).kinds
    }

    let l = Location::from_id(1).unwrap();
    let t = Territory::from_id(1).unwrap();
    let cases: Vec<(&str, BTreeSet<String>, BTreeSet<String>)> = vec![
        ("GameRef", claimed::<GameRef>(), written(&game, vec![&game.reference(), &old_ref, &GameRef::NULL])),
        ("PlayerIndex", claimed::<PlayerIndex>(), written(&game, vec![&own_player, &old_player, &PlayerIndex::NULL])),
        ("Location", claimed::<Location>(), written(&game, vec![&l, &l.of_game_map(old_id), &Location::NULL])),
        ("Territory", claimed::<Territory>(), written(&game, vec![&t, &t.of_game_map(old_id), &Territory::NULL])),
    ];
    let mut problems = Vec::new();
    for (name, claimed, written) in cases {
        // A shape may leave out null where the Rust field can't hold one.
        let written: BTreeSet<String> = written.into_iter().filter(|k| k != "null" || claimed.contains("null")).collect();
        if claimed != written {
            problems.push(format!("{name}: shape claims {claimed:?}, writer produces {written:?}"));
        }
    }
    // Inside a player of another game, its own map objects are foreign.
    {
        let mut g = game;
        let player = Player::new(g.other_game(old_id), Faction::Fremen);
        g.other_game_mut(old_id).players.push(player);
        let fremen = g.other_game(old_id).player_at(1);
        Player::add_forces(g.other_game_mut(old_id), fremen, l, 1, false);
        let mut w2 = CanonWriter::for_game(&g);
        fremen.canon(&mut w2);
        let text = String::from_utf8(w2.out).unwrap();
        if !text.contains(r#"{"$ref":"Location","id":1,"foreign":true}"#) {
            problems.push(format!("a player of another game writes its locations without \"foreign\":true: {text}"));
        }
        // A location the current game passes to that player is the current game's object.
        Player::add_forces(&mut g, fremen, Location::from_id(2).unwrap(), 1, false);
        let mut w3 = CanonWriter::for_game(&g);
        fremen.canon(&mut w3);
        let text = String::from_utf8(w3.out).unwrap();
        if !text.contains(r#"{"$ref":"Location","id":2}"#) {
            problems.push(format!("a location passed from this game to another game's player is written as foreign: {text}"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn reference_fields_can_write_every_dumper_form() {
    let cs = csharp_field_types();
    let reg = Registry::full();
    let mut problems = BTreeSet::new();
    // The map is expanded once, from the dumped game; nothing inside it can belong to another game.
    const MAP_TYPES: &[&str] = &["Map", "LocationFetcher", "TerritoryFetcher", "Location", "Territory", "Homeworld", "DiscoveredLocation", "HiddenMobileStronghold", "AttachedLocation"];
    for (name, fields) in &reg.types {
        if MAP_TYPES.contains(&name.as_str()) {
            continue;
        }
        let Some(cs_fields) = cs.get(name) else { continue };
        for (field, shape) in fields {
            // The dumper's special cases on Game (trace-format.md 8): the map is expanded
            // at the root, PreviousGameState is written as presence only.
            if name == "Game" && (field == "Map" || field == "PreviousGameState") {
                continue;
            }
            let Some(cs_type) = cs_fields.get(field) else { continue };
            let Some((_, forms)) = REFERENCE_FORMS.iter().find(|(t, _)| t == cs_type) else { continue };
            for form in *forms {
                if !shape.kinds.contains(*form) {
                    problems.insert(format!("{name}.{field} ({cs_type}): the dumper can write {form}, Rust can write {:?}", shape.kinds));
                }
            }
        }
    }
    if !problems.is_empty() {
        panic!("{} reference fields miss a dumper form:\n{}", problems.len(), problems.join_lines());
    }
}

#[test]
fn fresh_map_dump_matches_oracle() {
    let path = rust_dir().join("traces/sample/bots-2026-00040.jsonl.gz");
    let reader = differ::trace::open(&path).unwrap();
    let line = reader.lines().nth(1).unwrap().unwrap();
    let v: Value = serde_json::from_str(&line).unwrap();
    let expected = &v["state"]["Map"];
    let map = treachery_engine::map::Map::new();
    let bytes = canon::dump_detached(&map, None);
    let actual: Value = serde_json::from_slice(&bytes).expect("Rust map dump is valid JSON");
    let diffs = differ::diff::diff(expected, &actual, 20);
    assert!(diffs.is_empty(), "map dump differs:\n{diffs:#?}");
    // Byte equality, through the same serializer on both sides.
    assert_eq!(serde_json::to_string(expected).unwrap(), serde_json::to_string(&actual).unwrap());
}

#[test]
fn writer_primitives() {
    let mut w = CanonWriter::new(None);
    w.string("a\"b\\c\u{8}\u{c}\n\r\t\u{1}\u{7f}/é");
    assert_eq!(String::from_utf8(w.out).unwrap(), "\"a\\\"b\\\\c\\b\\f\\n\\r\\t\\u0001\u{7f}/é\"");
    for (f, s) in [(2.5f32, "2.5"), (3.0, "3"), (1e-5, "1E-05"), (-0.0, "-0"), (f32::NAN, "\"NaN\""), (f32::INFINITY, "\"Infinity\"")] {
        let mut w = CanonWriter::new(None);
        w.f32(f);
        assert_eq!(String::from_utf8(w.out).unwrap(), s, "{f}");
    }
    let mut w = CanonWriter::new(None);
    w.f64(1e21);
    assert_eq!(String::from_utf8(w.out).unwrap(), "1E+21");
}

#[test]
fn participation_round_trips_through_the_header() {
    for f in differ::trace::list(&rust_dir().join("traces/sample")) {
        let header = differ::trace::open(&f).unwrap().lines().next().unwrap().unwrap();
        let v: Value = serde_json::from_str(&header).unwrap();
        let p = differ::replay::participation(v.get("participation")).unwrap_or_else(|_| panic!("{}", f.display()));
        let bytes = canon::dump_detached(&p, None);
        assert_eq!(String::from_utf8(bytes).unwrap(), serde_json::to_string(&v["participation"]).unwrap(), "{}", f.display());
    }
}
