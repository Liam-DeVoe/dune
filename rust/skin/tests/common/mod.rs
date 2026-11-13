//! Shared helpers of the skin's snapshot tests. Each test compares what the skin renders now with
//! a file in `tests/golden`; with `UPDATE_GOLDENS=1` it rewrites the file instead.

#![allow(dead_code)]

use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;

use serde_json::Value;
use treachery_engine::error::catch_panic;
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

/// Whether the tests rewrite their snapshots (`UPDATE_GOLDENS=1`) instead of comparing.
pub fn updating() -> bool {
    std::env::var("UPDATE_GOLDENS").is_ok()
}

/// Writes JSON lines, gzipped when the path ends in `.gz`.
pub fn write_lines(path: &Path, lines: &[Value]) {
    let file = std::fs::File::create(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut w: Box<dyn Write> = if path.extension().is_some_and(|e| e == "gz") {
        Box::new(flate2::write::GzEncoder::new(file, flate2::Compression::best()))
    } else {
        Box::new(file)
    };
    for l in lines {
        writeln!(w, "{}", serde_json::to_string(l).unwrap()).unwrap();
    }
    w.flush().unwrap();
}

/// The result of a call as the snapshots hold it: the text, or `!Type: message` for a panic that
/// names a .NET exception (as the skin's own errors do), `!panic: message` for any other panic.
pub fn outcome(f: impl FnOnce() -> String) -> String {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| catch_panic(f))) {
        Ok(Ok(s)) => s,
        Ok(Err(message)) if is_dotnet_exception(&message) => format!("!{message}"),
        Ok(Err(message)) => format!("!panic: {message}"),
        Err(p) => format!("!panic: {}", treachery_engine::error::panic_message(p.as_ref())),
    }
}

/// Whether a panic message names a .NET exception, as the skin's own .NET-style errors do.
fn is_dotnet_exception(message: &str) -> bool {
    message.split_once(": ").is_some_and(|(t, _)| t.ends_with("Exception") && t.chars().all(|c| c.is_ascii_alphabetic()))
}

macro_rules! enum_by_name {
    ($name:expr, $v:expr; $($t:ident),* $(,)?) => {
        match $name {
            $(stringify!($t) => Some(EnumValue::of($t($v))),)*
            _ => None,
        }
    };
}

/// An enum value from its type name and number.
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

/// An argument token (`["Faction", 20]`, `["Location", 5]`, `["string", "x"]`, ...) as
/// the element it stands for.
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
pub fn enum_arg<T: treachery_engine::enum_type::GameEnum>(tok: &Value) -> T {
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
            self.failures.push(format!("{key}\n    snapshot: {expected:?}\n    now:      {actual:?}"));
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
