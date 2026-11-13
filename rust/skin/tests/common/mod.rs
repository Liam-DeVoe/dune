//! Shared helpers of the skin's differential tests.

#![allow(dead_code)]

use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use serde_json::Value;
use treachery_engine::compat::cs::{self, CsException};
use treachery_engine::enumerations::*;
use treachery_engine::messages::expression::{EnumValue, ExpressionElement};
use treachery_engine::identity::IFetcher;
use treachery_engine::heroes::hero::Hero;
use treachery_engine::heroes::leader_manager::LeaderManager;
use treachery_engine::map::location::Location;
use treachery_engine::messages::message::BattleFactKind;
use treachery_engine::messages::payment::Payment;
use treachery_engine::cards::resource_card::ResourceCard;
use treachery_engine::map::territory::Territory;
use treachery_engine::cards::treachery_card_manager::TreacheryCardManager;

/// The lines of a (possibly gzipped) JSON lines file.
pub fn read_lines(path: &Path) -> Vec<String> {
    let file = std::fs::File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let reader: Box<dyn Read> =
        if path.extension().is_some_and(|e| e == "gz") { Box::new(flate2::read::GzDecoder::new(file)) } else { Box::new(file) };
    BufReader::new(reader).lines().map(|l| l.unwrap()).filter(|l| !l.is_empty()).collect()
}

pub fn golden(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(name)
}

/// The result of a call as the probe writes it: the text, or `!Type: message` for an exception.
pub fn outcome(f: impl FnOnce() -> String) -> String {
    match cs::catch(f) {
        Ok(s) => s,
        Err(CsException { type_name, message }) => format!("!{type_name}: {message}"),
    }
}

macro_rules! enum_by_name {
    ($name:expr, $v:expr; $($t:ident),* $(,)?) => {
        match $name {
            $(stringify!($t) => Some(EnumValue::of($t($v))),)*
            _ => None,
        }
    };
}

/// An enum value from its C# type name and number.
pub fn enum_value(type_name: &str, v: i32) -> Option<EnumValue> {
    enum_by_name!(type_name, v;
        Ambassador, AsideKind, AuctionType, AutomationRuleType, BattleFactKind, BeneGesseritReactionKind, ChoamEconomicsStatus,
        CaptureDecision, ClairVoyanceAnswer, ClairvoyanceQuestion, Concept, DealType, DiscoveryToken, DiscoveryTokenType,
        ErrorType, Faction, FactionAdvantage, FactionForce, FactionSpecialForce, GameListScope, HeroType, IntrusionType,
        ItemAction, JuiceType, LeaderSkill, MainPhase, MainPhaseMoment, Milestone, Nexus, NexusAllianceChange, Phase,
        PrescienceAspect, Rule, RuleGroup, Ruleset, ShipKind, ShipmentPermission, ShipmentType, StrongholdAdvantage,
        SubscriptionType, TechToken, TerrorType, TreacheryCardType, UserStatus, VidalMoment, WinMethod, World,
    )
}

/// A probe argument token (`["Faction", 20]`, `["Location", 5]`, `["string", "x"]`, ...) as
/// the boxed C# object.
pub fn element(tok: &Value) -> ExpressionElement {
    let a = tok.as_array().expect("token is an array");
    let kind = a[0].as_str().unwrap();
    let int = || a[1].as_i64().unwrap();
    match kind {
        "null" => ExpressionElement::Null,
        "string" => ExpressionElement::String(a[1].as_str().unwrap().to_string()),
        "bool" => ExpressionElement::Bool(a[1].as_bool().unwrap()),
        "int" => ExpressionElement::Int(int() as i32),
        "long" => ExpressionElement::Long(int()),
        "float" => ExpressionElement::Float(f32::from_bits(int() as i32 as u32)),
        "double" => ExpressionElement::Double(f64::from_bits(int() as u64)),
        "Territory" => Territory::from_id(int() as i32).expect("territory").into(),
        "Location" => Location::from_id(int() as i32).expect("location").into(),
        "Leader" => LeaderManager::hero_lookup().find(int() as i32).expect("leader").into(),
        "Messiah" => Hero::Messiah.into(),
        "TreacheryCard" => TreacheryCardManager::lookup().find(int() as i32).expect("card").into(),
        "ResourceCard" => ResourceCard::get_resource_cards_in_and_outside_play()[int() as usize].into(),
        "Payment" => Payment::new(int() as i32).into(),
        t => ExpressionElement::Enum(enum_value(t, int() as i32).unwrap_or_else(|| panic!("unknown token type {t}"))),
    }
}

/// The enum value of a token, as `T`.
pub fn enum_arg<T: treachery_engine::compat::net_enum::NetEnum>(tok: &Value) -> T {
    match element(tok) {
        ExpressionElement::Enum(e) => e.get::<T>().unwrap_or_else(|| panic!("{tok} is not a {}", T::INFO.name)),
        _ => panic!("{tok} is not an enum"),
    }
}

pub fn str_arg(tok: &Value) -> String {
    tok[1].as_str().expect("string token").to_string()
}

pub fn int_arg(tok: &Value) -> i64 {
    tok[1].as_i64().expect("int token")
}

pub fn bool_arg(tok: &Value) -> bool {
    tok[1].as_bool().expect("bool token")
}

/// Compares `(key, expected, actual)` triples and panics with the first few differences.
pub struct Diffs {
    pub checked: usize,
    pub failures: Vec<String>,
}

impl Diffs {
    pub fn new() -> Self {
        Diffs { checked: 0, failures: Vec::new() }
    }

    pub fn check(&mut self, key: &str, expected: &str, actual: &str) {
        self.checked += 1;
        if expected != actual {
            self.failures.push(format!("{key}\n    C#:   {expected:?}\n    Rust: {actual:?}"));
        }
    }

    pub fn finish(self, what: &str) {
        if !self.failures.is_empty() {
            let shown: Vec<_> = self.failures.iter().take(25).cloned().collect();
            panic!("{what}: {} of {} differ:\n{}", self.failures.len(), self.checked, shown.join("\n"));
        }
        eprintln!("{what}: {} checked, all equal", self.checked);
    }
}
