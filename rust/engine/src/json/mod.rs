//! Byte-exact System.Text.Json for events and savegames, as configured in
//! `Model/Utilities.cs` (`new JsonSerializerOptions { IncludeFields = true }`).
//! Support module; the per-type member lists are in
//! members.rs.
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

use std::cell::{Cell, RefCell};
use std::fmt::Write as _;

use indexmap::IndexMap;

use crate::shared::Shared;
use crate::compat::date_time_offset::DateTimeOffset;
use crate::events::{GameEvent, GameEventType};
use crate::enum_type::GameEnum;

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

/// A C# class serialized as a JSON object (events, `GameSettings`).
pub trait JsonObject: Default {
    fn write_members(&self, w: &mut JsonWriter);
    /// Assigns one member. `Ok(false)` for an unknown key (ignored, like .NET).
    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError>;
}

/// Reads an object's members into `T::default()` (the C# parameterless
/// constructor and field initializers run first, so missing members keep them).
///
/// `$`-prefixed members are skipped as unknown members, which is what .NET does
/// for a non-polymorphic type (`GameSettings`). Events are polymorphic and
/// reject them; [`GameEvent`]'s `from_json` checks that before calling this.
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

/// `JsonSerializer.Serialize(value, Options)`
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

impl<T: GameEnum> ToJson for T {
    fn write_json(&self, w: &mut JsonWriter) {
        w.int(self.value() as i64)
    }
}

impl<T: GameEnum> FromJson for T {
    fn from_json(v: &JsonValue) -> Result<Self, JsonError> {
        i32::from_json(v).map(T::from_value)
    }
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

impl<T: ToJson> ToJson for Option<T> {
    fn write_json(&self, w: &mut JsonWriter) {
        match self {
            Some(v) => v.write_json(w),
            None => w.null(),
        }
    }
}

impl<T: FromJson> FromJson for Option<T> {
    fn from_json(v: &JsonValue) -> Result<Self, JsonError> {
        if v.is_null() { Ok(None) } else { T::from_json(v).map(Some) }
    }
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

impl<T: ToJson + Copy> ToJson for Cell<T> {
    fn write_json(&self, w: &mut JsonWriter) {
        self.get().write_json(w)
    }
}

impl<T: FromJson> FromJson for Cell<T> {
    fn from_json(v: &JsonValue) -> Result<Self, JsonError> {
        T::from_json(v).map(Cell::new)
    }
}

impl<T: ToJson> ToJson for RefCell<T> {
    fn write_json(&self, w: &mut JsonWriter) {
        self.borrow().write_json(w)
    }
}

impl<T: FromJson> FromJson for RefCell<T> {
    fn from_json(v: &JsonValue) -> Result<Self, JsonError> {
        T::from_json(v).map(RefCell::new)
    }
}

impl<T: ToJson> ToJson for Shared<T> {
    fn write_json(&self, w: &mut JsonWriter) {
        self.borrow().write_json(w)
    }
}

impl<T: FromJson> FromJson for Shared<T> {
    fn from_json(v: &JsonValue) -> Result<Self, JsonError> {
        T::from_json(v).map(Shared::new)
    }
}

/// `Dictionary<TEnum, V>`: keys are written as enum names (`ToString()`), and
/// read leniently: trimmed, case-insensitive names or integers.
impl<K: GameEnum, V: ToJson> ToJson for IndexMap<K, V> {
    fn write_json(&self, w: &mut JsonWriter) {
        w.object_value();
        for (k, v) in self.iter() {
            w.key(&K::INFO.format(k.value()));
            v.write_json(w);
        }
        w.end_object();
    }
}

impl<K: GameEnum, V: FromJson> FromJson for IndexMap<K, V> {
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
        return Some(K::from_value(i));
    }
    K::INFO.declared.iter().find(|(n, _)| n.eq_ignore_ascii_case(t)).map(|(_, v)| K::from_value(*v))
}

// ---- objects ----

impl ToJson for crate::game::GameSettings {
    fn write_json(&self, w: &mut JsonWriter) {
        w.object_value();
        self.write_members(w);
        w.end_object();
    }
}

impl FromJson for crate::game::GameSettings {
    fn from_json(v: &JsonValue) -> Result<Self, JsonError> {
        read_object(&object_members(v)?)
    }
}

/// An event written as its base type `GameEvent`: `$type` first, then the members.
impl ToJson for GameEvent {
    fn write_json(&self, w: &mut JsonWriter) {
        let t = self.event_type();
        let Some(discriminator) = t.json_discriminator() else {
            panic!("Runtime type '{}' is not supported by polymorphic type 'Treachery.Shared.GameEvent'.", t.name());
        };
        w.object_value();
        w.member("$type", discriminator);
        self.write_json_members(w);
        w.end_object();
    }
}

/// Reads a polymorphic `GameEvent`: `$type` must be the first
/// member and a known discriminator; no other `$` member may follow.
impl FromJson for GameEvent {
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
        let t = GameEventType::from_json_discriminator(name).ok_or_else(|| JsonError::unrecognized_discriminator(name))?;
        // A polymorphic type rejects any other metadata member.
        if let Some((k, _)) = members[1..].iter().find(|(k, _)| k.starts_with('$')) {
            return Err(JsonError::new("JsonException", format!("unexpected metadata property {k}")));
        }
        GameEvent::read_json(t, &members[1..])
    }
}

/// `GameState`: `{"Version":...,"_events":[...]}`.
impl ToJson for crate::game::GameState {
    fn write_json(&self, w: &mut JsonWriter) {
        w.object_value();
        w.member("Version", &self.version);
        w.member("_events", &self.events);
        w.end_object();
    }
}

impl FromJson for crate::game::GameState {
    fn from_json(v: &JsonValue) -> Result<Self, JsonError> {
        let mut result = crate::game::GameState::default();
        for (k, v) in object_members(v)? {
            match k.as_str() {
                "Version" => result.version = FromJson::from_json(&v)?,
                "_events" => {
                    // Vec<GameEvent>'s reader, keeping the index for an error's path.
                    result.events = match &v {
                        JsonValue::Array(items) => Some(
                            items
                                .iter()
                                .enumerate()
                                .map(|(i, item)| {
                                    GameEvent::from_json(item).map_err(|e| e.within(JsonPathSegment::Index(i)).within(JsonPathSegment::Member("_events")))
                                })
                                .collect::<Result<Vec<GameEvent>, _>>()?,
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
