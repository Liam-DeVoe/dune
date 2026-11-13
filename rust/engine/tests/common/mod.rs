//! Shared helpers for the golden tests that compare values one by one with
//! .NET (boundary.rs, random_helpers.rs, linq_table.rs, message_corpus.rs).
//!
//! Values use the encoding of csharp-ref/Enc.cs:
//!
//! | value                  | encoding                         |
//! |------------------------|----------------------------------|
//! | null                   | JSON null                        |
//! | bool, int, long        | JSON bool / number               |
//! | float / double         | `"f:1E-05"` / `"d:0.1"` (.NET invariant ToString) |
//! | string                 | `"s:text"`                       |
//! | enum                   | `"e:Faction:20"`                 |
//! | IIdentifiable          | `"o:Homeworld#105"` (runtime type and Id) |
//! | ResourceCard           | `"rc:3"` (index in GetResourceCardsInAndOutsidePlay) |
//! | PointD                 | `"pt:1.5,-2"`                    |
//! | Message, MessagePart   | `"m:"` + ToString()              |
//! | IEnumerable            | JSON array                       |
//! | anything else          | `"?:TypeName"`                   |
//! | exception              | `"x:Type:Message"`               |

#![allow(dead_code)]

use std::sync::OnceLock;

use serde_json::{json, Value};
use treachery_engine::compat::cs::{self, CsException};
use treachery_engine::enumerations::*;
use treachery_engine::identity::IFetcher;
use treachery_engine::heroes::hero::Hero;
use treachery_engine::heroes::leader::Leader;
use treachery_engine::heroes::leader_manager::LeaderManager;
use treachery_engine::map::location::Location;
use treachery_engine::map::Map;
use treachery_engine::messages::message::{BattleFactKind, Message, MessagePart};
use treachery_engine::compat::net_enum::NetEnum;
use treachery_engine::map::point::PointD;
use treachery_engine::cards::resource_card::ResourceCard;
use treachery_engine::map::territory::Territory;
use treachery_engine::cards::treachery_card::TreacheryCard;
use treachery_engine::cards::treachery_card_manager::TreacheryCardManager;

/// Loads `tests/golden/<name>` once per test binary.
pub fn golden(name: &'static str) -> &'static Value {
    static FILES: OnceLock<std::sync::Mutex<Vec<(&'static str, &'static Value)>>> = OnceLock::new();
    let files = FILES.get_or_init(Default::default);
    let mut files = files.lock().unwrap();
    if let Some((_, v)) = files.iter().find(|(n, _)| *n == name) {
        return v;
    }
    let path = format!("{}/tests/golden/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}. Regenerate with csharp-ref."));
    let v: &'static Value = Box::leak(Box::new(serde_json::from_str(&text).unwrap()));
    files.push((name, v));
    v
}

/// Reads a list file from tests/coverage: one entry per line, `#` comments.
pub fn coverage_list(name: &str) -> Vec<String> {
    let path = format!("{}/tests/coverage/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{path}: {e}"))
        .lines()
        .map(|l| l.split('#').next().unwrap().trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

pub fn exception(e: &CsException) -> Value {
    json!(format!("x:{}:{}", e.type_name, e.message))
}

/// Runs `f`; its encoded result, or the encoded C# exception.
pub fn try_enc<R: Enc>(f: impl FnOnce() -> R) -> Value {
    match cs::catch(f) {
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
        if cs::CsNull::is_null(*self) {
            return Value::Null;
        }
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
        if cs::CsNull::is_null(*self) {
            return Value::Null;
        }
        json!(format!("o:Territory#{}", self.id()))
    }
}
impl Enc for Leader {
    fn enc(&self) -> Value {
        if cs::CsNull::is_null(*self) {
            return Value::Null;
        }
        json!(format!("o:Leader#{}", self.id()))
    }
}
impl Enc for TreacheryCard {
    fn enc(&self) -> Value {
        if cs::CsNull::is_null(*self) {
            return Value::Null;
        }
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
        if cs::CsNull::is_null(*self) {
            return Value::Null;
        }
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
pub fn enc_enum<T: NetEnum>(v: T) -> Value {
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
pub fn en<T: NetEnum>(v: &Value) -> T {
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
    // null: the null handle (engine-conventions.md, "Null handles").
    if v.is_null() {
        return Location::NULL;
    }
    let (_, id) = object(v);
    Map::new().location_lookup().find(id).unwrap()
}

pub fn opt<T>(v: &Value, f: impl FnOnce(&Value) -> T) -> Option<T> {
    if v.is_null() { None } else { Some(f(v)) }
}

pub fn territory(v: &Value) -> Territory {
    // null: the null handle (engine-conventions.md, "Null handles").
    if v.is_null() {
        return Territory::NULL;
    }
    let (t, id) = object(v);
    assert_eq!(t, "Territory");
    Map::new().territory_lookup().find(id).unwrap()
}

pub fn leader(v: &Value) -> Leader {
    // null: the null handle (engine-conventions.md, "Null handles").
    if v.is_null() {
        return Leader::NULL;
    }
    let (t, id) = object(v);
    assert_eq!(t, "Leader");
    LeaderManager::leader_lookup().find(id).unwrap()
}

pub fn card(v: &Value) -> TreacheryCard {
    // null: the null handle (engine-conventions.md, "Null handles").
    if v.is_null() {
        return TreacheryCard::NULL;
    }
    let (t, id) = object(v);
    assert_eq!(t, "TreacheryCard");
    TreacheryCardManager::get(id).unwrap()
}

pub fn hero(v: &Value) -> Hero {
    // null: the null handle (engine-conventions.md, "Null handles").
    if v.is_null() {
        return Hero::NULL;
    }
    let (_, id) = object(v);
    LeaderManager::hero_lookup().find(id).unwrap()
}

pub fn resource_card(v: &Value) -> ResourceCard {
    if v.is_null() {
        return ResourceCard::NULL;
    }
    let i: usize = tagged(v, "rc:").parse().unwrap();
    ResourceCard::get_resource_cards_in_and_outside_play()[i]
}

pub fn point(v: &Value) -> PointD {
    let s = tagged(v, "pt:");
    let (x, y) = s.split_once(',').unwrap();
    PointD::new(parse_net_float(x), parse_net_float(y))
}

/// Compares two encoded values; floats compare by their .NET text, so this is
/// plain JSON equality.
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
            self.0.push(format!("{}\n    C#:   {expected}\n    Rust: {actual}", what()));
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

/// Every ported C# enum, by C# name. Tests that iterate golden data look
/// enums up here and fail on a name that is missing, so add each newly
/// ported enum.
pub fn enum_info(name: &str) -> Option<&'static treachery_engine::compat::net_enum::EnumTypeInfo> {
    macro_rules! all {
        ($($t:ident),*) => { [$((stringify!($t), <$t as NetEnum>::INFO)),*] };
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
