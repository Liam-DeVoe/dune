//! Shared helpers for the snapshot tests: tests that compare what the engine computes with values
//! recorded in `tests/golden/` (event_json.rs, message_corpus.rs, random_helpers.rs,
//! static_data.rs, review_pkg_c.rs, sync_v188.rs).
//!
//! The recorded values pin behavior that savegames, players and bots depend on, so a difference
//! is a change in behavior. After a deliberate change, rewrite the snapshots and review the diff:
//!
//! ```sh
//! UPDATE_GOLDENS=1 cargo test --release -p treachery-engine
//! ```
//!
//! Values are encoded like this:
//!
//! | value                  | encoding                         |
//! |------------------------|----------------------------------|
//! | null                   | JSON null                        |
//! | bool, int, long        | JSON bool / number               |
//! | float / double         | `"f:1E-05"` / `"d:0.1"` (as the engine prints them) |
//! | string                 | `"s:text"`                       |
//! | enum                   | `"e:Faction:20"`                 |
//! | location, card, leader | `"o:Homeworld#105"` (type and id) |
//! | ResourceCard           | `"rc:3"` (index in `get_resource_cards_in_and_outside_play`) |
//! | PointD                 | `"pt:1.5,-2"`                    |
//! | Message, MessagePart   | `"m:"` + `to_cs_string()`        |
//! | collection             | JSON array                       |
//! | anything else          | `"?:TypeName"`                   |
//! | panic                  | `"x:Type:Message"` or `"x:panic:<message>"` |

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use serde_json::{json, Value};
use treachery_engine::error::catch_panic;
use treachery_engine::enumerations::*;
use treachery_engine::identity::IFetcher;
use treachery_engine::heroes::hero::Hero;
use treachery_engine::heroes::leader::Leader;
use treachery_engine::heroes::leader_manager::LeaderManager;
use treachery_engine::map::location::Location;
use treachery_engine::map::Map;
use treachery_engine::messages::message::{BattleFactKind, Message, MessagePart};
use treachery_engine::enum_type::GameEnum;
use treachery_engine::map::point::PointD;
use treachery_engine::cards::resource_card::ResourceCard;
use treachery_engine::map::territory::Territory;
use treachery_engine::cards::treachery_card::TreacheryCard;
use treachery_engine::cards::treachery_card_manager::TreacheryCardManager;

/// Loads `tests/golden/<name>` once per test binary.
pub fn golden(name: &'static str) -> &'static Value {
    static FILES: OnceLock<Mutex<Vec<(&'static str, &'static Value)>>> = OnceLock::new();
    let files = FILES.get_or_init(Default::default);
    let mut files = files.lock().unwrap();
    if let Some((_, v)) = files.iter().find(|(n, _)| *n == name) {
        return v;
    }
    let v: &'static Value = Box::leak(Box::new(serde_json::from_str(&read_golden(name)).unwrap()));
    files.push((name, v));
    v
}

pub fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(name)
}

pub fn read_golden(name: &str) -> String {
    let path = golden_path(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Whether to write the snapshots instead of comparing with them (`UPDATE_GOLDENS=1`).
pub fn updating() -> bool {
    std::env::var_os("UPDATE_GOLDENS").is_some()
}

/// Serializes the read-modify-write of snapshot files, since several tests write sections of
/// the same file.
pub fn write_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Replaces section `section` of the JSON object in `tests/golden/<file>`.
pub fn write_section(file: &str, section: &str, value: Value) {
    let _guard = write_lock();
    let mut doc: Value = serde_json::from_str(&read_golden(file)).unwrap();
    doc[section] = value;
    std::fs::write(golden_path(file), serde_json::to_string(&doc).unwrap() + "\n").unwrap();
}

/// Compares `actual` with section `section` of `tests/golden/<file>` using `compare`, or writes
/// it there when updating.
pub fn snapshot_section(file: &'static str, section: &str, actual: Value, compare: impl FnOnce(&Value, &Value)) {
    if updating() {
        write_section(file, section, actual);
    } else {
        compare(&golden(file)[section], &actual);
    }
}

/// For each entry of the array in section `section` of `tests/golden/<file>`, `f` computes the
/// recorded fields from the entry's inputs, as an object. They must equal the entry's values for
/// the same keys. When updating, the entries get the computed values instead.
pub fn check_entries(file: &'static str, section: &str, what: &str, f: impl Fn(&Value) -> Value) {
    let entries = golden(file)[section].as_array().unwrap_or_else(|| panic!("{file} has no section {section}"));
    let mut failures = Failures::default();
    let mut updated = Vec::new();
    for e in entries {
        let actual = f(e);
        let fields = actual.as_object().expect("the computed fields are an object");
        if updating() {
            let mut entry = e.clone();
            for (k, v) in fields {
                entry[k] = v.clone();
            }
            updated.push(entry);
        } else {
            let expected: serde_json::Map<String, Value> = fields.keys().map(|k| (k.clone(), e[k].clone())).collect();
            failures.check(|| format!("{e}"), &Value::Object(expected), &actual);
        }
    }
    if updating() {
        write_section(file, section, Value::Array(updated));
    } else {
        failures.finish(what);
    }
}

/// Compares `lines` with the text snapshot `tests/golden/<file>` (one value per line), or writes
/// them there when updating. Reports every line that differs.
pub fn snapshot_lines(file: &str, lines: &[String]) {
    if updating() {
        let text: String = lines.iter().map(|l| format!("{l}\n")).collect();
        std::fs::write(golden_path(file), text).unwrap();
        return;
    }
    let text = read_golden(file);
    let expected: Vec<&str> = text.lines().collect();
    let mut failures = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        match expected.get(i) {
            Some(e) if e == line => {}
            e => failures.push(format!("line {}:\n  expected: {}\n  actual:   {line}", i + 1, e.unwrap_or(&"<missing>"))),
        }
    }
    if expected.len() != lines.len() {
        failures.push(format!("expected {} lines, got {}", expected.len(), lines.len()));
    }
    assert!(failures.is_empty(), "{file}: {} differences:\n{}", failures.len(), failures.join("\n"));
}

/// A panic message as an encoded exception: `"x:Type:Message"` when it names a .NET exception
/// (dotnet-compat's parsing and dictionaries still panic that way), `"x:panic:<message>"` otherwise.
pub fn exception(message: &str) -> Value {
    match message.split_once(": ") {
        Some((t, m)) if t.ends_with("Exception") && t.chars().all(|c| c.is_ascii_alphabetic()) => json!(format!("x:{t}:{m}")),
        _ => json!(format!("x:panic:{message}")),
    }
}

/// Runs `f`: its result, or the panic encoded as a value. A panic that names a .NET exception
/// (`"NullReferenceException: ..."`) is encoded as that exception (`"x:Type:Message"`), any
/// other panic as `"x:panic:<panic message>"`.
pub fn catch_any<R>(f: impl FnOnce() -> R) -> Result<R, Value> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| catch_panic(f))) {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(e)) => Err(exception(&e)),
        Err(p) => Err(json!(format!("x:panic:{}", treachery_engine::error::panic_message(p.as_ref())))),
    }
}

/// Runs `f`; its encoded result, or the encoded panic.
pub fn try_enc<R: Enc>(f: impl FnOnce() -> R) -> Value {
    match catch_panic(f) {
        Ok(v) => v.enc(),
        Err(e) => exception(&e),
    }
}

// ---------------------------------------------------------------- encoding

pub trait Enc {
    fn enc(&self) -> Value;
}

impl Enc for Value {
    fn enc(&self) -> Value {
        self.clone()
    }
}
impl Enc for () {
    fn enc(&self) -> Value {
        Value::Null
    }
}
impl Enc for i32 {
    fn enc(&self) -> Value {
        json!(self)
    }
}
impl Enc for i64 {
    fn enc(&self) -> Value {
        json!(self)
    }
}
impl Enc for bool {
    fn enc(&self) -> Value {
        json!(self)
    }
}
impl Enc for f32 {
    fn enc(&self) -> Value {
        json!(format!("f:{}", dotnet_compat::fmt::f32_to_string(*self)))
    }
}
impl Enc for f64 {
    fn enc(&self) -> Value {
        json!(format!("d:{}", dotnet_compat::fmt::f64_to_string(*self)))
    }
}
impl Enc for str {
    fn enc(&self) -> Value {
        json!(format!("s:{self}"))
    }
}
impl Enc for String {
    fn enc(&self) -> Value {
        self.as_str().enc()
    }
}
impl Enc for Location {
    fn enc(&self) -> Value {
        json!(format!("o:{}#{}", self.type_name(), self.id()))
    }
}
impl Enc for treachery_engine::map::homeworld::Homeworld {
    fn enc(&self) -> Value {
        self.location().enc()
    }
}
impl Enc for treachery_engine::map::discovery::DiscoveredLocation {
    fn enc(&self) -> Value {
        self.location().enc()
    }
}
impl Enc for treachery_engine::map::hidden_mobile_stronghold::HiddenMobileStronghold {
    fn enc(&self) -> Value {
        self.location().enc()
    }
}
impl Enc for Territory {
    fn enc(&self) -> Value {
        json!(format!("o:Territory#{}", self.id()))
    }
}
impl Enc for Leader {
    fn enc(&self) -> Value {
        json!(format!("o:Leader#{}", self.id()))
    }
}
impl Enc for TreacheryCard {
    fn enc(&self) -> Value {
        json!(format!("o:TreacheryCard#{}", self.id()))
    }
}
impl Enc for Hero {
    fn enc(&self) -> Value {
        match self {
            Hero::Leader(l) => l.enc(),
            Hero::Messiah => json!(format!("o:Messiah#{}", self.id())),
            Hero::TreacheryCard(c) => c.enc(),
        }
    }
}
impl Enc for ResourceCard {
    fn enc(&self) -> Value {
        json!(format!("rc:{}", self.index()))
    }
}
impl Enc for PointD {
    fn enc(&self) -> Value {
        json!(format!(
            "pt:{},{}",
            dotnet_compat::fmt::f64_to_string(self.x),
            dotnet_compat::fmt::f64_to_string(self.y)
        ))
    }
}
impl Enc for Message {
    fn enc(&self) -> Value {
        json!(format!("m:{}", self.to_cs_string()))
    }
}
impl Enc for MessagePart {
    fn enc(&self) -> Value {
        json!(format!("m:{}", self.to_cs_string()))
    }
}
impl<T: Enc> Enc for Option<T> {
    fn enc(&self) -> Value {
        match self {
            Some(v) => v.enc(),
            None => Value::Null,
        }
    }
}
impl<T: Enc> Enc for Vec<T> {
    fn enc(&self) -> Value {
        Value::Array(self.iter().map(Enc::enc).collect())
    }
}
impl<T: Enc> Enc for [T] {
    fn enc(&self) -> Value {
        Value::Array(self.iter().map(Enc::enc).collect())
    }
}
impl<T: Enc + ?Sized> Enc for &T {
    fn enc(&self) -> Value {
        (**self).enc()
    }
}

/// An enum value: `"e:Faction:20"`.
pub fn enc_enum<T: GameEnum>(v: T) -> Value {
    json!(format!("e:{}:{}", T::INFO.name, v.value()))
}

macro_rules! enc_enums {
    ($($t:ty),*) => {$(
        impl Enc for $t {
            fn enc(&self) -> Value { enc_enum(*self) }
        }
    )*};
}
enc_enums!(
    Faction, Nexus, World, HeroType, Rule, TreacheryCardType, DiscoveryToken, DiscoveryTokenType,
    StrongholdAdvantage, MainPhase, Concept, BattleFactKind
);

// ---------------------------------------------------------------- decoding

pub fn int(v: &Value) -> i32 {
    v.as_i64().unwrap_or_else(|| panic!("not an int: {v}")) as i32
}

pub fn boolean(v: &Value) -> bool {
    v.as_bool().unwrap_or_else(|| panic!("not a bool: {v}"))
}

fn tagged<'a>(v: &'a Value, tag: &str) -> &'a str {
    v.as_str().and_then(|s| s.strip_prefix(tag)).unwrap_or_else(|| panic!("expected {tag}...: {v}"))
}

/// Parses .NET invariant `ToString()` of a float ("NaN", "-Infinity", "1E-05").
fn parse_net_float(s: &str) -> f64 {
    match s {
        "NaN" => f64::NAN,
        "Infinity" => f64::INFINITY,
        "-Infinity" => f64::NEG_INFINITY,
        _ => s.parse().unwrap_or_else(|_| panic!("bad float {s}")),
    }
}

pub fn float(v: &Value) -> f32 {
    let s = tagged(v, "f:");
    match s {
        "NaN" => f32::NAN,
        "Infinity" => f32::INFINITY,
        "-Infinity" => f32::NEG_INFINITY,
        _ => s.parse().unwrap_or_else(|_| panic!("bad float {s}")),
    }
}

pub fn double(v: &Value) -> f64 {
    parse_net_float(tagged(v, "d:"))
}

pub fn string(v: &Value) -> String {
    tagged(v, "s:").to_string()
}

/// `"e:Faction:20"` as a `T`. Panics if the type name differs.
pub fn en<T: GameEnum>(v: &Value) -> T {
    let s = tagged(v, "e:");
    let (name, value) = s.rsplit_once(':').unwrap();
    assert_eq!(name, T::INFO.name, "enum type of {v}");
    T::from_value(value.parse().unwrap())
}

/// `"o:Type#id"`: the type name and id.
pub fn object(v: &Value) -> (&str, i32) {
    let s = tagged(v, "o:");
    let (t, id) = s.split_once('#').unwrap();
    (t, id.parse().unwrap())
}

pub fn location(v: &Value) -> Location {
    let (_, id) = object(v);
    Map::new().location_lookup().find(id).unwrap()
}

pub fn opt<T>(v: &Value, f: impl FnOnce(&Value) -> T) -> Option<T> {
    if v.is_null() { None } else { Some(f(v)) }
}

pub fn territory(v: &Value) -> Territory {
    let (t, id) = object(v);
    assert_eq!(t, "Territory");
    Map::new().territory_lookup().find(id).unwrap()
}

pub fn leader(v: &Value) -> Leader {
    let (t, id) = object(v);
    assert_eq!(t, "Leader");
    LeaderManager::leader_lookup().find(id).unwrap()
}

pub fn card(v: &Value) -> TreacheryCard {
    let (t, id) = object(v);
    assert_eq!(t, "TreacheryCard");
    TreacheryCardManager::get(id).unwrap()
}

pub fn hero(v: &Value) -> Hero {
    let (_, id) = object(v);
    LeaderManager::hero_lookup().find(id).unwrap()
}

pub fn resource_card(v: &Value) -> ResourceCard {
    let i: usize = tagged(v, "rc:").parse().unwrap();
    ResourceCard::get_resource_cards_in_and_outside_play()[i]
}

pub fn point(v: &Value) -> PointD {
    let s = tagged(v, "pt:");
    let (x, y) = s.split_once(',').unwrap();
    PointD::new(parse_net_float(x), parse_net_float(y))
}

/// Compares two encoded values. Floats are encoded as text, so this is plain JSON equality.
pub fn same(expected: &Value, actual: &Value) -> bool {
    expected == actual
}

/// Collects failures and reports them all at once (up to a limit), so one
/// run shows the whole extent of a difference.
#[derive(Default)]
pub struct Failures(pub Vec<String>);

impl Failures {
    pub fn check(&mut self, what: impl FnOnce() -> String, expected: &Value, actual: &Value) {
        if !same(expected, actual) {
            self.0.push(format!("{}\n    expected: {expected}\n    actual:   {actual}", what()));
        }
    }

    pub fn push(&mut self, s: impl Into<String>) {
        self.0.push(s.into());
    }

    pub fn finish(self, title: &str) {
        if self.0.is_empty() {
            return;
        }
        let shown: Vec<&String> = self.0.iter().take(60).collect();
        panic!(
            "{title}: {} failure(s)\n{}{}",
            self.0.len(),
            shown.iter().map(|s| format!("- {s}\n")).collect::<String>(),
            if self.0.len() > 60 { "...\n" } else { "" }
        );
    }
}

/// Every enum, by name. Tests that iterate snapshot data look enums up here and fail on a name
/// that is missing, so add each new enum.
pub fn enum_info(name: &str) -> Option<&'static treachery_engine::enum_type::EnumTypeInfo> {
    macro_rules! all {
        ($($t:ident),*) => { [$((stringify!($t), <$t as GameEnum>::INFO)),*] };
    }
    let all = all!(
        Faction, Nexus, Ambassador, TechToken, MainPhase, MainPhaseMoment, ChoamEconomicsStatus, Phase, Milestone,
        FactionAdvantage, FactionForce, FactionSpecialForce, Ruleset, DiscoveryToken, DiscoveryTokenType, Rule,
        RuleGroup, TreacheryCardType, Concept, WinMethod, HeroType, AuctionType, JuiceType, LeaderSkill,
        StrongholdAdvantage, World, CaptureDecision, DealType, TerrorType, IntrusionType, VidalMoment,
        ClairVoyanceAnswer, ClairvoyanceQuestion, PrescienceAspect, ShipmentPermission, UserStatus,
        SubscriptionType, GameListScope, ShipmentType, AutomationRuleType, ItemAction, BattleFactKind,
        AsideKind, NexusAllianceChange, ShipKind, BeneGesseritReactionKind, ErrorType
    );
    all.iter().find(|(n, _)| *n == name).map(|(_, i)| *i)
}
