//! The event and savegame JSON of game version 190 and before: byte-exact System.Text.Json, as
//! the C# version configured it in `Model/Utilities.cs` (`new JsonSerializerOptions {
//! IncludeFields = true }`). The per-type member lists are in members.rs.
//!
//! Version 191 replaced this format with the engine's serde format (rewrite-plan.md, "Wire
//! format"). This crate reads the old one, to convert a savegame or a stored game once
//! ([`load_savegame`]), and the harness reads and writes it for traces of a reference that
//! records the old format (native-harness.md).
//!
//! Writing is hand-rolled so the output matches .NET byte for byte: members in
//! the order C# declares them, enums as numbers, nulls written, compact, and the
//! default `JavaScriptEncoder` escaping (all non-ASCII and `"&'+<>\`` as `\uXXXX`).
//!
//! Reading goes through `serde_json::Value` (order-preserving). Differences from
//! .NET that only matter for malformed input:
//! - a duplicate key keeps its first position and last value (.NET: last value wins too);
//!   a duplicate `$type` is not detected;
//! - nesting deeper than 64 is accepted up to serde_json's limit of 128;
//! - a `DateTimeOffset` without an offset is read as UTC, not machine-local time;
//! - `-0.0` is accepted as the integer 0 (`-0` is accepted by both).
//!
//! Error messages are not .NET's: callers only learn that reading failed. The
//! one exception is an unknown event `$type` (an old savegame after an event
//! was renamed, escapes.md): its message, path and position are .NET's, see
//! [`JsonError::unrecognized_discriminator`].
pub mod members;
pub mod utilities;

use std::fmt::Write as _;

use indexmap::IndexMap;

use treachery_engine::enum_type::GameEnum;
use treachery_engine::prelude::*;

pub type JsonValue = serde_json::Value;

/// Reading failed. The `kind` is the .NET exception the C# reader would throw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError {
    pub kind: &'static str,
    pub message: String,
    /// For an error whose message is .NET's: where in the document .NET's reader
    /// throws, outermost first. [`locate`] appends the path and position to the message.
    pub path: Option<Vec<JsonPathSegment>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonPathSegment {
    Member(&'static str),
    Index(usize),
}

impl JsonError {
    pub fn new(kind: &'static str, message: impl Into<String>) -> Self {
        JsonError { kind, message: message.into(), path: None }
    }

    /// `ThrowHelper.ThrowJsonException_UnrecognizedTypeDiscriminator`, thrown with the
    /// reader just past the token after the discriminator value.
    fn unrecognized_discriminator(id: &str) -> Self {
        JsonError {
            kind: "JsonException",
            message: format!("Read unrecognized type discriminator id '{id}'."),
            path: Some(Vec::new()),
        }
    }

    /// The error met inside the value at `segment` of its parent.
    pub fn within(mut self, segment: JsonPathSegment) -> Self {
        if let Some(path) = &mut self.path {
            path.insert(0, segment);
        }
        self
    }

    fn invalid(what: &str, v: &JsonValue) -> Self {
        JsonError::new("JsonException", format!("cannot convert {v} to {what}"))
    }
}

/// Compact JSON writer with .NET's escaping.
#[derive(Default)]
pub struct JsonWriter {
    out: String,
    /// Per open container: whether the next item is the first.
    first: Vec<bool>,
    /// A key was written and its value is next.
    pending_value: bool,
}

impl JsonWriter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn finish(self) -> String {
        self.out
    }

    fn sep(&mut self) {
        if let Some(first) = self.first.last_mut() {
            if !*first {
                self.out.push(',');
            }
            *first = false;
        }
    }

    pub fn end_object(&mut self) {
        self.first.pop();
        self.out.push('}');
    }

    pub fn end_array(&mut self) {
        self.first.pop();
        self.out.push(']');
    }

    /// Starts an object member; the value follows.
    pub fn key(&mut self, name: &str) {
        self.sep();
        write_escaped(&mut self.out, name);
        self.out.push(':');
        self.pending_value = true;
    }

    /// Writes `"name":value`.
    pub fn member<T: ToJson + ?Sized>(&mut self, name: &str, value: &T) {
        self.key(name);
        value.write_json(self);
    }

    /// Writes a raw value token (after `key` or inside an array).
    fn value(&mut self, token: impl FnOnce(&mut String)) {
        if self.pending_value {
            self.pending_value = false;
        } else {
            self.sep();
        }
        token(&mut self.out);
    }

    pub fn null(&mut self) {
        self.value(|o| o.push_str("null"));
    }

    pub fn bool(&mut self, b: bool) {
        self.value(|o| o.push_str(if b { "true" } else { "false" }));
    }

    pub fn int(&mut self, i: i64) {
        self.value(|o| {
            let _ = write!(o, "{i}");
        });
    }

    pub fn string(&mut self, s: &str) {
        self.value(|o| write_escaped(o, s));
    }

    /// A string .NET writes without the encoder (dates: `+` stays literal).
    pub fn unescaped_string(&mut self, s: &str) {
        self.value(|o| {
            o.push('"');
            o.push_str(s);
            o.push('"');
        });
    }

    /// Opens an object as a value (after `key` or inside an array).
    pub fn object_value(&mut self) {
        self.value(|o| o.push('{'));
        self.first.push(true);
    }

    /// Opens an array as a value (after `key` or inside an array).
    pub fn array_value(&mut self) {
        self.value(|o| o.push('['));
        self.first.push(true);
    }
}

/// `JavaScriptEncoder.Default` escaping.
pub fn write_escaped(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            '"' | '&' | '\'' | '+' | '<' | '>' | '`' | '\u{7f}' => {
                let _ = write!(out, "\\u{:04X}", c as u32);
            }
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04X}", c as u32);
            }
            c if c.is_ascii() => out.push(c),
            c => {
                let mut buf = [0u16; 2];
                for unit in c.encode_utf16(&mut buf) {
                    let _ = write!(out, "\\u{:04X}", unit);
                }
            }
        }
    }
    out.push('"');
}

pub trait ToJson {
    fn write_json(&self, w: &mut JsonWriter);
}

pub trait FromJson: Sized {
    fn from_json(v: &JsonValue) -> Result<Self, JsonError>;
}

/// A type whose `Option` is written as `null` when absent. An enum whose absent value is its C#
/// `None` member doesn't have it: its `Option` is written as 0 (`game_enum!`).
pub trait JsonNullable {}
impl JsonNullable for String {}
impl<T> JsonNullable for Vec<T> {}
impl<K, V> JsonNullable for IndexMap<K, V> {}
impl JsonNullable for HistoryEntry {}
impl JsonNullable for treachery_engine::game::GameSettings {}

/// A C# class serialized as a JSON object (events, `GameSettings`).
pub trait JsonObject: Default {
    fn write_members(&self, w: &mut JsonWriter);
    /// Assigns one member. `Ok(false)` for an unknown key (ignored, like .NET).
    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError>;
}

/// The members of a JSON object, as a trait object (`Event::json_members`).
pub trait JsonMembers {
    fn write_members(&self, w: &mut JsonWriter);
}

impl<T: JsonObject> JsonMembers for T {
    fn write_members(&self, w: &mut JsonWriter) {
        JsonObject::write_members(self, w)
    }
}

/// [`read_object`], passing the members `T` doesn't know to `other` first. A member neither
/// knows is ignored.
pub fn read_object_with<T: JsonObject>(
    members: &[(String, JsonValue)],
    other: &mut dyn FnMut(&str, &JsonValue) -> Result<bool, JsonError>,
) -> Result<T, JsonError> {
    let mut result = T::default();
    for (k, v) in members {
        if k.starts_with('$') {
            continue;
        }
        if !other(k, v)? {
            result.read_member(k, v)?;
        }
    }
    Ok(result)
}

/// Reads an object's members into `T::default()` (the C# parameterless
/// constructor and field initializers run first, so missing members keep them).
///
/// `$`-prefixed members are skipped as unknown members, which is what .NET does
/// for a non-polymorphic type (`GameSettings`). Events are polymorphic and
/// reject them; [`Event`]'s `from_json` checks that before calling this.
pub fn read_object<T: JsonObject>(members: &[(String, JsonValue)]) -> Result<T, JsonError> {
    let mut result = T::default();
    for (k, v) in members {
        if k.starts_with('$') {
            continue;
        }
        result.read_member(k, v)?;
    }
    Ok(result)
}

fn object_members(v: &JsonValue) -> Result<Vec<(String, JsonValue)>, JsonError> {
    match v {
        JsonValue::Object(map) => Ok(map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
        _ => Err(JsonError::invalid("object", v)),
    }
}

pub fn serialize<T: ToJson + ?Sized>(value: &T) -> String {
    let mut w = JsonWriter::new();
    value.write_json(&mut w);
    w.finish()
}

/// `JsonSerializer.Deserialize<T>(text, Options)` (not the legacy Newtonsoft path).
pub fn deserialize<T: FromJson>(text: &str) -> Result<T, JsonError> {
    let v: JsonValue = serde_json::from_str(text).map_err(|e| JsonError::new("JsonException", e.to_string()))?;
    T::from_json(&v).map_err(|e| locate(text, e))
}

/// Completes an error that has a `path` the way `JsonException` messages from
/// `JsonSerializer` end: `" Path: $._events[1] | LineNumber: 0 | BytePositionInLine: 96."`.
/// Only the unrecognized discriminator has a path, and .NET throws it after reading the
/// discriminator value and one more token: the next property name with its colon, or the
/// object's closing brace. The line number counts `\n`; the position is in UTF-8 bytes.
pub fn locate(text: &str, mut e: JsonError) -> JsonError {
    let Some(path) = e.path.take() else {
        return e;
    };
    let mut r = PositionReader { bytes: text.as_bytes(), at: 0 };
    let Some(()) = r.find_discriminator_end(&path) else {
        return e;
    };
    let consumed = &text.as_bytes()[..r.at];
    let line = consumed.iter().filter(|&&b| b == b'\n').count();
    let line_start = consumed.iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1);
    let mut path_text = String::from("$");
    for segment in &path {
        match segment {
            JsonPathSegment::Member(name) => {
                path_text.push('.');
                path_text.push_str(name);
            }
            JsonPathSegment::Index(i) => {
                let _ = write!(path_text, "[{i}]");
            }
        }
    }
    let _ = write!(e.message, " Path: {path_text} | LineNumber: {line} | BytePositionInLine: {}.", r.at - line_start);
    e
}

/// Walks a document serde_json already accepted, for [`locate`].
struct PositionReader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl PositionReader<'_> {
    fn skip_whitespace(&mut self) {
        while matches!(self.bytes.get(self.at), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    /// Skips whitespace and one expected byte.
    fn expect(&mut self, b: u8) -> Option<()> {
        self.skip_whitespace();
        (self.bytes.get(self.at) == Some(&b)).then(|| self.at += 1)
    }

    /// A string token; returns it decoded.
    fn string(&mut self) -> Option<String> {
        self.skip_whitespace();
        let start = self.at;
        self.expect(b'"')?;
        loop {
            match self.bytes.get(self.at)? {
                b'\\' => self.at += 2,
                b'"' => {
                    self.at += 1;
                    break;
                }
                _ => self.at += 1,
            }
        }
        serde_json::from_slice(&self.bytes[start..self.at]).ok()
    }

    fn skip_value(&mut self) -> Option<()> {
        self.skip_whitespace();
        match self.bytes.get(self.at)? {
            b'"' => self.string().map(|_| ()),
            b'{' | b'[' => {
                let mut depth = 0usize;
                loop {
                    match self.bytes.get(self.at)? {
                        b'"' => {
                            self.string()?;
                            continue;
                        }
                        b'{' | b'[' => depth += 1,
                        b'}' | b']' => {
                            depth -= 1;
                            if depth == 0 {
                                self.at += 1;
                                return Some(());
                            }
                        }
                        _ => {}
                    }
                    self.at += 1;
                }
            }
            _ => {
                while !matches!(self.bytes.get(self.at)?, b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r') {
                    self.at += 1;
                }
                Some(())
            }
        }
    }

    /// Moves to the start of the value at `path`, then past the discriminator value
    /// and the token after it.
    fn find_discriminator_end(&mut self, path: &[JsonPathSegment]) -> Option<()> {
        for segment in path {
            match segment {
                // The first member of that name: .NET reads in document order.
                JsonPathSegment::Member(name) => {
                    self.expect(b'{')?;
                    loop {
                        let key = self.string()?;
                        self.expect(b':')?;
                        if key == *name {
                            break;
                        }
                        self.skip_value()?;
                        self.expect(b',')?;
                    }
                }
                JsonPathSegment::Index(i) => {
                    self.expect(b'[')?;
                    for _ in 0..*i {
                        self.skip_value()?;
                        self.expect(b',')?;
                    }
                }
            }
        }
        self.expect(b'{')?;
        self.string()?;
        self.expect(b':')?;
        self.skip_value()?;
        self.skip_whitespace();
        match self.bytes.get(self.at)? {
            b'}' => self.at += 1,
            _ => {
                self.expect(b',')?;
                self.string()?;
                self.expect(b':')?;
            }
        }
        Some(())
    }
}

// ---- primitives ----

impl ToJson for bool {
    fn write_json(&self, w: &mut JsonWriter) {
        w.bool(*self)
    }
}

impl FromJson for bool {
    fn from_json(v: &JsonValue) -> Result<Self, JsonError> {
        v.as_bool().ok_or_else(|| JsonError::invalid("bool", v))
    }
}

impl ToJson for i32 {
    fn write_json(&self, w: &mut JsonWriter) {
        w.int(*self as i64)
    }
}

impl FromJson for i32 {
    fn from_json(v: &JsonValue) -> Result<Self, JsonError> {
        match v {
            // serde_json reads `-0` as the float -0.0; .NET accepts it as 0. (Known gap:
            // `-0.0` is accepted too, which .NET rejects.)
            JsonValue::Number(n) if n.as_f64().is_some_and(|f| f == 0.0 && f.is_sign_negative()) && !n.is_i64() => Ok(0),
            JsonValue::Number(n) => n.as_i64().and_then(|i| i32::try_from(i).ok()).ok_or_else(|| JsonError::invalid("int", v)),
            _ => Err(JsonError::invalid("int", v)),
        }
    }
}

impl ToJson for u16 {
    fn write_json(&self, w: &mut JsonWriter) {
        w.int(i64::from(*self))
    }
}

/// An amount in an event (spice, forces, a dial): 0 to 65535. Anything else is refused when
/// decoded: "-1 is not a valid amount".
impl FromJson for u16 {
    fn from_json(v: &JsonValue) -> Result<Self, JsonError> {
        read_bounded::<u16>(v, "amount")
    }
}

/// Reads an integer that must fit `T`, refusing one that doesn't with "N is not a valid NAME".
fn read_bounded<T: TryFrom<i32>>(v: &JsonValue, name: &str) -> Result<T, JsonError> {
    let n = i32::from_json(v)?;
    T::try_from(n).map_err(|_| JsonError::new("JsonException", format!("{n} is not a valid {name}")))
}

/// Reads a force list (`_forceLocations` and the like: `location:forces|special,...`) and refuses
/// one with an amount outside 0 to 65535. Text that isn't a list of numbers is left for the
/// event to fail on, as before.
pub fn read_force_locations(v: &JsonValue) -> Result<Option<String>, JsonError> {
    let s: Option<String> = FromJson::from_json(v)?;
    for pair in s.as_deref().unwrap_or("").split(',') {
        let Some((_, amounts)) = pair.split_once(':') else { continue };
        for amount in amounts.split('|') {
            if let Ok(n) = treachery_engine::parse::int(amount) {
                if u16::try_from(n).is_err() {
                    return Err(JsonError::new("JsonException", format!("{n} is not a valid amount of forces")));
                }
            }
        }
    }
    Ok(s)
}

impl ToJson for treachery_engine::game::Turn {
    fn write_json(&self, w: &mut JsonWriter) {
        w.int(self.number() as i64)
    }
}

impl FromJson for treachery_engine::game::Turn {
    fn from_json(v: &JsonValue) -> Result<Self, JsonError> {
        read_bounded::<u8>(v, "turn").map(treachery_engine::game::Turn::new)
    }
}

impl ToJson for str {
    fn write_json(&self, w: &mut JsonWriter) {
        w.string(self)
    }
}

impl ToJson for String {
    fn write_json(&self, w: &mut JsonWriter) {
        w.string(self)
    }
}

impl FromJson for String {
    fn from_json(v: &JsonValue) -> Result<Self, JsonError> {
        v.as_str().map(str::to_string).ok_or_else(|| JsonError::invalid("string", v))
    }
}

/// An enum is written as its number. `Option` of an enum whose C# type had a `None` member
/// writes absence as 0, as C# did; `Option` of any other enum writes `null`.
macro_rules! json_enum {
    (absent: $($t:ty),*; nullable: $($n:ty),* $(,)?) => {
        $(json_enum!(@one $t);
        impl ToJson for Option<$t> {
            fn write_json(&self, w: &mut JsonWriter) {
                w.int(treachery_engine::enum_type::value_or_absent(*self) as i64)
            }
        }
        impl FromJson for Option<$t> {
            fn from_json(v: &JsonValue) -> Result<Self, JsonError> {
                let n = i32::from_json(v)?;
                treachery_engine::enum_type::from_value_or_absent(n).map_err(|e| JsonError::new("JsonException", e))
            }
        }
        impl EnumKey for $t {})*
        $(json_enum!(@one $n);
        impl JsonNullable for $n {}
        impl EnumKey for $n {})*
    };
    (@one $t:ty) => {
        impl ToJson for $t {
            fn write_json(&self, w: &mut JsonWriter) {
                w.int(GameEnum::value(*self) as i64)
            }
        }
        impl FromJson for $t {
            fn from_json(v: &JsonValue) -> Result<Self, JsonError> {
                let n = i32::from_json(v)?;
                <$t as GameEnum>::from_value(n).ok_or_else(|| JsonError::new("JsonException", treachery_engine::enum_type::undefined_value::<$t>(n)))
            }
        }
    };
}

/// An enum that keys a dictionary.
pub trait EnumKey: GameEnum {}

json_enum! {
    absent: Faction, Nexus, Ambassador, TechToken, MainPhase, MainPhaseMoment, ChoamEconomicsStatus, Phase, Milestone, FactionAdvantage,
        FactionForce, FactionSpecialForce, Ruleset, DiscoveryToken, DiscoveryTokenType, Rule, RuleGroup, TreacheryCardType, Concept,
        WinMethod, HeroType, AuctionType, JuiceType, LeaderSkill, StrongholdAdvantage, World, CaptureDecision, TerrorType, IntrusionType,
        VidalMoment, ClairVoyanceAnswer, ClairvoyanceQuestion, PrescienceAspect;
    nullable: DealType, ShipmentType, AutomationRuleType, ItemAction, ShipmentPermission,
}

impl ToJson for DateTimeOffset {
    fn write_json(&self, w: &mut JsonWriter) {
        w.unescaped_string(&self.to_json_string())
    }
}

impl FromJson for DateTimeOffset {
    fn from_json(v: &JsonValue) -> Result<Self, JsonError> {
        v.as_str().and_then(DateTimeOffset::parse_json).ok_or_else(|| JsonError::invalid("DateTimeOffset", v))
    }
}

// ---- wrappers ----

impl<T: ToJson + JsonNullable> ToJson for Option<T> {
    fn write_json(&self, w: &mut JsonWriter) {
        match self {
            Some(v) => v.write_json(w),
            None => w.null(),
        }
    }
}

impl<T: FromJson + JsonNullable> FromJson for Option<T> {
    fn from_json(v: &JsonValue) -> Result<Self, JsonError> {
        if v.is_null() { Ok(None) } else { T::from_json(v).map(Some) }
    }
}

/// Event JSON names a static game object by its id, and none by -1. An id that names no object
/// of the type doesn't decode.
macro_rules! json_handle {
    ($($t:ty => $name:literal, $find:expr;)*) => {$(
        impl ToJson for Option<$t> {
            fn write_json(&self, w: &mut JsonWriter) {
                w.int(self.map_or(-1, |h| h.id()) as i64)
            }
        }

        impl FromJson for Option<$t> {
            fn from_json(v: &JsonValue) -> Result<Self, JsonError> {
                let id = i32::from_json(v)?;
                if id == -1 {
                    return Ok(None);
                }
                let find: fn(i32) -> Option<$t> = $find;
                find(id).map(Some).ok_or_else(|| JsonError::new("JsonException", format!("{id} is not a valid {}", $name)))
            }
        }
    )*};
}

json_handle! {
    treachery_engine::map::location::Location => "Location", treachery_engine::map::location::Location::from_id;
    treachery_engine::map::territory::Territory => "Territory", treachery_engine::map::territory::Territory::from_id;
    treachery_engine::heroes::leader::Leader => "Leader", |id| treachery_engine::identity::Fetcher::find(&treachery_engine::heroes::leader_manager::LeaderManager::leader_lookup(), id);
    treachery_engine::heroes::hero::Hero => "Hero", |id| treachery_engine::identity::Fetcher::find(&treachery_engine::heroes::leader_manager::LeaderManager::hero_lookup(), id);
    treachery_engine::cards::treachery_card::TreacheryCard => "TreacheryCard", |id| treachery_engine::identity::Fetcher::find(&treachery_engine::cards::treachery_card_manager::TreacheryCardManager::lookup(), id);
}

impl<T: ToJson> ToJson for Vec<T> {
    fn write_json(&self, w: &mut JsonWriter) {
        w.array_value();
        for item in self {
            item.write_json(w);
        }
        w.end_array();
    }
}

impl<T: FromJson> FromJson for Vec<T> {
    fn from_json(v: &JsonValue) -> Result<Self, JsonError> {
        match v {
            JsonValue::Array(items) => items.iter().map(T::from_json).collect(),
            _ => Err(JsonError::invalid("array", v)),
        }
    }
}

/// `Dictionary<TEnum, V>`: keys are written as enum names (`ToString()`), and
/// read leniently: trimmed, case-insensitive names or integers.
impl<K: EnumKey, V: ToJson> ToJson for IndexMap<K, V> {
    fn write_json(&self, w: &mut JsonWriter) {
        w.object_value();
        for (k, v) in self.iter() {
            w.key(&K::INFO.format(k.value()));
            v.write_json(w);
        }
        w.end_object();
    }
}

impl<K: EnumKey, V: FromJson> FromJson for IndexMap<K, V> {
    fn from_json(v: &JsonValue) -> Result<Self, JsonError> {
        let mut result = IndexMap::new();
        for (key, value) in object_members(v)? {
            let k = parse_enum_key::<K>(&key).ok_or_else(|| JsonError::new("JsonException", format!("invalid dictionary key {key}")))?;
            // Duplicate keys throw in .NET (Dictionary.Add); serde_json already merged them.
            result.insert(k, V::from_json(&value)?);
        }
        Ok(result)
    }
}

fn parse_enum_key<K: GameEnum>(s: &str) -> Option<K> {
    let t = s.trim();
    if let Ok(i) = t.parse::<i32>() {
        return K::from_value(i);
    }
    K::INFO.declared.iter().find(|(n, _)| n.eq_ignore_ascii_case(t)).and_then(|(_, v)| K::from_value(*v))
}

// ---- objects ----

impl ToJson for treachery_engine::game::GameSettings {
    fn write_json(&self, w: &mut JsonWriter) {
        w.object_value();
        JsonObject::write_members(self, w);
        w.end_object();
    }
}

impl FromJson for treachery_engine::game::GameSettings {
    fn from_json(v: &JsonValue) -> Result<Self, JsonError> {
        read_object(&object_members(v)?)
    }
}

/// A history entry written as an event of the C# base type `GameEvent`: `$type` first, then the
/// event's members, then `Initiator` and `Time`.
impl ToJson for HistoryEntry {
    fn write_json(&self, w: &mut JsonWriter) {
        w.object_value();
        w.member("$type", self.event.event_type().name());
        write_event_members(&self.event, w);
        w.member("Initiator", &self.by);
        w.member("Time", &self.at);
        w.end_object();
    }
}

/// Reads a polymorphic `GameEvent` into a history entry: `$type` must be the first member and a
/// known discriminator; no other `$` member may follow. `Initiator` and `Time` go to the entry.
impl FromJson for HistoryEntry {
    fn from_json(v: &JsonValue) -> Result<Self, JsonError> {
        let members = object_members(v)?;
        let Some((first, discriminator)) = members.first() else {
            return Err(JsonError::new("NotSupportedException", "must specify a type discriminator"));
        };
        if first != "$type" {
            return Err(JsonError::new("NotSupportedException", "must specify a type discriminator"));
        }
        let name = discriminator.as_str().ok_or_else(|| JsonError::invalid("type discriminator", discriminator))?;
        // .NET reads the member after `$type` before it resolves the discriminator, and
        // throws for a metadata name there first.
        if let Some((k, _)) = members.get(1).filter(|(k, _)| matches!(k.as_str(), "$id" | "$ref" | "$type" | "$values")) {
            return Err(JsonError::new("JsonException", format!("unexpected metadata property {k}")));
        }
        let t = EventType::from_name(name).ok_or_else(|| JsonError::unrecognized_discriminator(name))?;
        // A polymorphic type rejects any other metadata member.
        if let Some((k, _)) = members[1..].iter().find(|(k, _)| k.starts_with('$')) {
            return Err(JsonError::new("JsonException", format!("unexpected metadata property {k}")));
        }
        let mut by: Option<Faction> = None;
        let mut at = DateTimeOffset::default();
        let event = read_event(t, &members[1..], &mut |k, v| {
            match k {
                "Initiator" => by = FromJson::from_json(v)?,
                "Time" => at = FromJson::from_json(v)?,
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(HistoryEntry { by, at, event, id: treachery_engine::events::EntryId::fresh() })
    }
}

/// A savegame in the old format: `{"Version":...,"_events":[...]}`.
#[derive(Clone, Debug, Default)]
pub struct LegacyGameState {
    pub version: i32,
    pub events: Option<Vec<HistoryEntry>>,
}

impl ToJson for LegacyGameState {
    fn write_json(&self, w: &mut JsonWriter) {
        w.object_value();
        w.member("Version", &self.version);
        w.member("_events", &self.events);
        w.end_object();
    }
}

impl FromJson for LegacyGameState {
    fn from_json(v: &JsonValue) -> Result<Self, JsonError> {
        let mut result = LegacyGameState { version: 0, events: Some(Vec::new()) };
        for (k, v) in object_members(v)? {
            match k.as_str() {
                "Version" => result.version = FromJson::from_json(&v)?,
                "_events" => {
                    // Vec<HistoryEntry>'s reader, keeping the index for an error's path.
                    result.events = match &v {
                        JsonValue::Array(items) => Some(
                            items
                                .iter()
                                .enumerate()
                                .map(|(i, item)| {
                                    HistoryEntry::from_json(item).map_err(|e| e.within(JsonPathSegment::Index(i)).within(JsonPathSegment::Member("_events")))
                                })
                                .collect::<Result<Vec<HistoryEntry>, _>>()?,
                        ),
                        _ => FromJson::from_json(&v)?,
                    }
                }
                // GameState is not polymorphic: .NET skips `$` members as unknown ones.
                _ => {}
            }
        }
        Ok(result)
    }
}

/// The last game version whose savegames are in this format. Its rules are those of the
/// version after it, which changed only the format.
pub const LAST_LEGACY_VERSION: i32 = 190;

/// Whether `text` looks like a savegame in the old format (`Version` and `_events`), rather
/// than the engine's (`version` and `events`). It only looks at the first member name.
pub fn is_legacy_savegame(text: &str) -> bool {
    let rest = text.trim_start().strip_prefix('{').unwrap_or("").trim_start();
    rest.starts_with("\"Version\"") || rest.starts_with("\"_events\"") || rest.starts_with("\"$")
}

/// Reads a savegame in the old format as it was written.
pub fn read_savegame(data: &str) -> Result<LegacyGameState, JsonError> {
    // Old savegames named their types "Treachery.online.*". The replacement also changes
    // any text in the savegame that contains the string, which players could see.
    let data = data.replace("Treachery.online", "Treachery");
    let v: JsonValue = serde_json::from_str(&data).map_err(|e| JsonError::new("JsonException", e.to_string()))?;
    if v.is_null() {
        return Err(JsonError::new("JsonException", "Cannot load game state"));
    }
    LegacyGameState::from_json(&v).map_err(|e| locate(&data, e))
}

/// Reads a savegame in the old format and converts it: a game of version
/// [`LAST_LEGACY_VERSION`] becomes a game of the next version, which has the same rules. Other
/// versions keep theirs, and the engine refuses them as before.
pub fn load_savegame(data: &str) -> Result<treachery_engine::game::GameState, JsonError> {
    let state = read_savegame(data)?;
    let events = state.events.ok_or_else(|| JsonError::new("JsonException", "A savegame without events"))?;
    let version = if state.version == LAST_LEGACY_VERSION { LAST_LEGACY_VERSION + 1 } else { state.version };
    Ok(treachery_engine::game::GameState { version, events })
}

/// Reads a savegame in either format, converting an old one ([`load_savegame`]).
pub fn load_any(data: &str) -> Result<treachery_engine::game::GameState, String> {
    if is_legacy_savegame(data) { load_savegame(data).map_err(|e| e.message) } else { treachery_engine::game::GameState::load(data) }
}

/// Writes a savegame in the old format, with the version it had there.
pub fn savegame_text(events: &[HistoryEntry]) -> String {
    serialize(&LegacyGameState { version: LAST_LEGACY_VERSION, events: Some(events.to_vec()) })
}

/// Declares the members of every event: [`write_event_members`] and [`read_event`]. A missing
/// or extra event type is a compile error, because both match exhaustively.
macro_rules! legacy_events {
    ($($name:ident),* $(,)?) => {
        /// The event's members, without `$type`, `Initiator` and `Time`.
        pub fn write_event_members(e: &Event, w: &mut JsonWriter) {
            match e {
                $( Event::$name(e) => JsonObject::write_members(e, w), )*
            }
        }

        /// Builds an event of type `t` from its JSON members. `other` reads the members that
        /// aren't the event's own (`Initiator`, `Time`), in document order.
        pub fn read_event(
            t: EventType,
            members: &[(String, JsonValue)],
            other: &mut dyn FnMut(&str, &JsonValue) -> Result<bool, JsonError>,
        ) -> Result<Event, JsonError> {
            Ok(match t {
                $( EventType::$name => Event::$name(read_object_with::<$name>(members, other)?), )*
            })
        }
    };
}

legacy_events! {
    AcceptOrCancelTleilaxuRevival, AllianceBroken, AllianceByAmbassador, AllianceByTerror, AllianceOffered, AlliancePassed, AllyPermission,
    AmalPlayed, AmbassadorActivated, AmbassadorPlaced, AuditCancelled, Audited, AutomationConfigured, Battle, BattleClaimed,
    BattleConcluded, BattleInitiated, BattleRevision, BeneGesseritAccompanies, BeneGesseritBattleAnnouncement, BeneGesseritFlip,
    BeneGesseritPrediction, Bid, BlackMarketBid, Bureaucracy, Captured, Caravan, CardGiven, CardTraded, CardsDetermined, CharityClaimed,
    ChoamDiscarded, ChoamEconomics, ChoamExtraMove, ChoamFreeRevivalPrevention, ChoamKarmaPrevention, ChoamMovePrevention,
    ChoamRemoveForce, ClairVoyanceAnswered, ClairVoyancePlayed, DealAccepted, DealOffered, DealRejected, Diplomacy, Discarded,
    DiscardedSearched, DiscardedSearchedAnnounced, DiscardedTaken, DiscoveryEntered, DiscoveryRevealed, DistransUsed, DivideResources,
    DivideResourcesAccepted, Donated, EmperorBidSupport, EmperorDiscarded, EndPhase, EstablishPlayers, ExtortionPrevented, FaceDanced,
    FaceDancerReplaced, FaceDancerRevealed, FactionSelected, FactionTradeOffered, FlightDiscoveryUsed, FlightUsed, FremenRidesMonster,
    FremenSentMonster, GuildDelay, HMSAdvantageChosen, HarvesterPlayed, HideSecrets, IxianRemovedCardFromAuction,
    IxianSelectedStartingCard, IxianSwappedCardOnBid, JuicePlayed, Karma, KarmaChoamDiscard, KarmaEcazDial, KarmaFreeRevival,
    KarmaHandSwap, KarmaHandSwapInitiated, KarmaHmsMovement, KarmaMonster, KarmaPrescience, KarmaRevivalPrevention, KarmaRicheseBuy,
    KarmaShipmentPrevention, LoserConcluded, MetheorPlayed, Move, MulliganPerformed, NexusCardDrawn, NexusPlayed, NexusVoted,
    PerformBeneGesseritPlacement, PerformFremenSetup, PerformHmsMovement, PerformHmsPlacement, PerformMoritaniSetup, PerformSetup,
    Planetology, PlayerReplaced, PoisonToothCancelled, PortableAntidoteUsed, Prescience, RaiseDeadPlayed, RecruitsPlayed,
    ReplacedCardWon, RequestTleilaxuRevival, ResidualPlayed, ResourcesAudited, ResourcesTransferred, Retreat, Revival,
    RicheseAnnouncesAuction, RicheseAnnouncesBlackMarket, RicheseGaveCard, RicheseKeepsUnsoldCard, RicheseRevealedNoField,
    RicheseSpecifiesAuction, RockWasMelted, SetIncreasedRevivalLimits, SetShipmentPermission, Shipment, SkillAssigned, StormDialled,
    StormSpellPlayed, SwitchedSkilledLeader, TakeLosses, TerrorPlanted, TerrorRevealed, TestingStationUsed, Thought, ThoughtAnswered,
    ThumperPlayed, TraitorDiscarded, TraitorsSelected, TreacheryCalled, Voice, VoiceDeclined,
}
