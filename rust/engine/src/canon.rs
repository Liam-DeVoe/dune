//! The canonical state dump: the Rust side of `Oracle/Canon/StateDumper.cs`.
//! Support module (no C# counterpart). The format is specified in
//! `rust/docs/trace-format.md` section 7; this module must produce the same
//! bytes as the oracle for the same state.
//!
//! Most types get their impls from `#[derive(Canon)]` (see `canon-derive`).
//! This module holds the traits, the JSON writer, the impls for primitives,
//! collections and handles, and the special cases the dumper has:
//! references to static data, the map, `Game.History` and friends.
//!
//! It also describes the dump's *shape* (`CanonShape`): for every object type,
//! its field names and the kinds of JSON values each field can hold. The
//! field-mapping test (`differ/tests/field_mapping.rs`) compares that with the
//! fields the C# dumper writes, so a field that is missing or misnamed in Rust
//! fails before any game logic runs.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::hash::Hash;
use std::rc::Rc;

use dotnet_compat::{NetDictionary, NetHashSet};

use crate::shared::{RcKey, Shared};
use crate::error::GameError;
use crate::compat::date_time_offset::{DateTimeOffset, TimeSpan};
use crate::enumerations::DiscoveryToken;
use crate::events::GameEvent;
use crate::messages::expression::ExpressionElement;
use crate::game::Game;
use crate::heroes::hero::Hero;
use crate::heroes::leader::Leader;
use crate::map::location::Location;
use crate::random::{LoggedRandom, SharedRandom};
use crate::map::{LocationKind, Map};
use crate::messages::message::LogRecord;
use crate::game::PlayerIndex;
use crate::cards::resource_card::ResourceCard;
use crate::map::territory::Territory;
use crate::cards::treachery_card::TreacheryCard;

// ---------------------------------------------------------------------------
// Writer
// ---------------------------------------------------------------------------

/// Appends canonical JSON (no whitespace) to a byte buffer, like `JsonOut.cs`.
pub struct CanonWriter<'g> {
    pub out: Vec<u8>,
    /// The dumped game (`StateDumper._root`).
    root: Option<&'g Game>,
    /// `Game.Random` of the dumped game: other places that hold the same
    /// object are written as `{"$ref":"GameRandom"}`.
    game_random: Option<SharedRandom>,
}

impl<'g> CanonWriter<'g> {
    pub fn new(game_random: Option<SharedRandom>) -> Self {
        CanonWriter { out: Vec::with_capacity(1 << 16), root: None, game_random }
    }

    /// A writer for values inside `game`: live message lists show the
    /// game's current contents.
    pub fn for_game(game: &'g Game) -> Self {
        let mut w = CanonWriter::new(game.random.clone());
        w.root = Some(game);
        w
    }

    pub fn raw(&mut self, s: &str) {
        self.out.extend_from_slice(s.as_bytes());
    }
    pub fn obj_start(&mut self) {
        self.out.push(b'{');
    }
    pub fn obj_end(&mut self) {
        self.out.push(b'}');
    }
    pub fn arr_start(&mut self) {
        self.out.push(b'[');
    }
    pub fn arr_end(&mut self) {
        self.out.push(b']');
    }
    pub fn comma(&mut self) {
        self.out.push(b',');
    }
    pub fn null(&mut self) {
        self.raw("null");
    }
    pub fn bool(&mut self, b: bool) {
        self.raw(if b { "true" } else { "false" });
    }
    pub fn int(&mut self, v: i64) {
        let mut buf = itoa_buf(v);
        self.out.append(&mut buf);
    }
    pub fn uint(&mut self, v: u64) {
        self.raw(&v.to_string());
    }
    /// `float` as `ToString("R")`; NaN and infinities as strings.
    pub fn f32(&mut self, v: f32) {
        let s = dotnet_compat::fmt::f32_to_string(v);
        if v.is_finite() { self.raw(&s) } else { self.string(&s) }
    }
    /// `double` as `ToString("R")`; NaN and infinities as strings.
    pub fn f64(&mut self, v: f64) {
        let s = dotnet_compat::fmt::f64_to_string(v);
        if v.is_finite() { self.raw(&s) } else { self.string(&s) }
    }
    pub fn key(&mut self, k: &str) {
        self.string(k);
        self.out.push(b':');
    }

    /// String escaping of `JsonOut.WriteString`: `"` and `\` escaped, the
    /// short forms for \b \f \n \r \t, other control characters as `\u00xx`,
    /// everything else raw UTF-8. (A Rust string has no lone surrogates.)
    pub fn string(&mut self, s: &str) {
        self.out.push(b'"');
        let bytes = s.as_bytes();
        let mut start = 0;
        for (i, &b) in bytes.iter().enumerate() {
            let esc: Option<&[u8]> = match b {
                b'"' => Some(b"\\\""),
                b'\\' => Some(b"\\\\"),
                0x08 => Some(b"\\b"),
                0x0c => Some(b"\\f"),
                b'\n' => Some(b"\\n"),
                b'\r' => Some(b"\\r"),
                b'\t' => Some(b"\\t"),
                _ => None,
            };
            if esc.is_none() && b >= 0x20 {
                continue;
            }
            self.out.extend_from_slice(&bytes[start..i]);
            start = i + 1;
            match esc {
                Some(e) => self.out.extend_from_slice(e),
                None => {
                    const HEX: &[u8; 16] = b"0123456789abcdef";
                    self.out.extend_from_slice(b"\\u00");
                    self.out.push(HEX[(b >> 4) as usize]);
                    self.out.push(HEX[(b & 0xf) as usize]);
                }
            }
        }
        self.out.extend_from_slice(&bytes[start..]);
        self.out.push(b'"');
    }

    /// `{"$ref":"<what>"}`
    pub fn reference(&mut self, what: &str) {
        self.obj_start();
        self.key("$ref");
        self.string(what);
        self.obj_end();
    }

    /// `{"$ref":"<type>","id":<id>}`
    pub fn entity_ref(&mut self, ty: &str, id: i32) {
        self.obj_start();
        self.key("$ref");
        self.string(ty);
        self.comma();
        self.key("id");
        self.int(id as i64);
        self.obj_end();
    }

    /// A `Location` or `Territory`: `{"$ref":<type>,"id":<id>}`.
    pub fn map_ref(&mut self, ty: &str, id: i32) {
        self.obj_start();
        self.key("$ref");
        self.string(ty);
        self.comma();
        self.key("id");
        self.int(id as i64);
        self.obj_end();
    }

    /// Writes `{"$type":<type_name>,<fields sorted by name>}`.
    pub fn object(&mut self, type_name: &str, mut fields: Vec<(&'static str, Field<'_>)>) {
        fields.sort_unstable_by(|a, b| a.0.cmp(b.0));
        self.obj_start();
        self.key("$type");
        self.string(type_name);
        for (name, f) in fields {
            self.comma();
            self.key(name);
            f.write(self);
        }
        self.obj_end();
    }
}

fn itoa_buf(v: i64) -> Vec<u8> {
    v.to_string().into_bytes()
}

// ---------------------------------------------------------------------------
// Traits
// ---------------------------------------------------------------------------

/// A value that has a canonical dump form.
pub trait Canon {
    fn canon(&self, w: &mut CanonWriter);
}

/// The dump shape of a type, for the field-mapping test.
pub trait CanonShape {
    fn canon_shape(reg: &mut Registry) -> Shape;
}

/// A type dumped as an object with fields (implemented by the derive).
pub trait CanonObject {
    /// The C# `$type`, e.g. `Battle` or `Deck<TreacheryCard>`.
    fn canon_type_name() -> String;
    /// The fields under their C# names, in any order.
    fn canon_fields<'a>(&'a self, out: &mut Vec<(&'static str, Field<'a>)>);
    /// The field names and shapes, for the field-mapping test.
    fn canon_schema_fields(reg: &mut Registry, out: &mut Vec<(&'static str, Shape)>);
}

/// One field of an object.
pub enum Field<'a> {
    Value(&'a dyn Canon),
    Custom(Box<dyn Fn(&mut CanonWriter) + 'a>),
}

impl Field<'_> {
    fn write(&self, w: &mut CanonWriter) {
        match self {
            Field::Value(v) => v.canon(w),
            Field::Custom(f) => f(w),
        }
    }
}

/// The body of every derived `Canon::canon`.
pub fn write_object<T: CanonObject>(v: &T, w: &mut CanonWriter) {
    let mut fields = Vec::new();
    v.canon_fields(&mut fields);
    w.object(&T::canon_type_name(), fields);
}

/// `Treachery.Shared.Faction` -> `Faction` (the dumper's `Type.Name`).
pub fn short_cs_type_name(full: &str) -> String {
    full.rsplit('.').next().unwrap_or(full).to_string()
}

// ---------------------------------------------------------------------------
// Shapes
// ---------------------------------------------------------------------------

/// The JSON kinds a value can take: `null`, `bool`, `num`, `str`, `list`,
/// `ref:<what>`, `obj:<type>`, or `obj?:<keys>` for untyped objects such as
/// `{"$count":n}`. `elem` is the shape of list elements.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shape {
    pub kinds: BTreeSet<String>,
    pub elem: Option<Box<Shape>>,
}

impl Shape {
    pub fn kind(k: &str) -> Shape {
        let mut s = Shape::default();
        s.kinds.insert(k.to_string());
        s
    }
    pub fn list(elem: Shape) -> Shape {
        let mut s = Shape::kind("list");
        s.elem = Some(Box::new(elem));
        s
    }
    pub fn with(mut self, k: &str) -> Shape {
        self.kinds.insert(k.to_string());
        self
    }
    pub fn union(mut self, other: Shape) -> Shape {
        self.kinds.extend(other.kinds);
        self.elem = match (self.elem.take(), other.elem) {
            (Some(a), Some(b)) => Some(Box::new(a.union(*b))),
            (a, b) => a.or(b),
        };
        self
    }
}

/// Every object type reachable in the dump, with its fields' shapes.
#[derive(Default)]
pub struct Registry {
    pub types: BTreeMap<String, BTreeMap<String, Shape>>,
    in_progress: BTreeSet<String>,
    /// Duplicate field names after flattening (C# would throw).
    pub errors: Vec<String>,
}

impl Registry {
    pub fn object<T: CanonObject>(&mut self) -> Shape {
        let name = T::canon_type_name();
        if !self.types.contains_key(&name) && self.in_progress.insert(name.clone()) {
            let mut fields = Vec::new();
            T::canon_schema_fields(self, &mut fields);
            self.insert(&name, fields);
        }
        Shape::kind(&format!("obj:{name}"))
    }

    /// Registers a hand-written object type.
    pub fn manual(&mut self, name: &str, fields: impl FnOnce(&mut Registry) -> Vec<(&'static str, Shape)>) -> Shape {
        if !self.types.contains_key(name) && self.in_progress.insert(name.to_string()) {
            let fields = fields(self);
            self.insert(name, fields);
        }
        Shape::kind(&format!("obj:{name}"))
    }

    fn insert(&mut self, name: &str, fields: Vec<(&'static str, Shape)>) {
        let mut map = BTreeMap::new();
        for (f, s) in fields {
            if map.insert(f.to_string(), s).is_some() {
                self.errors.push(format!("{name}.{f}: duplicate field name"));
            }
        }
        self.in_progress.remove(name);
        self.types.insert(name.to_string(), map);
    }

    /// The shapes of everything a game dump, a validation message and a step's
    /// exception can contain: `Game` and every event type and log record.
    pub fn full() -> Registry {
        let mut reg = Registry::default();
        <Game as CanonShape>::canon_shape(&mut reg);
        <GameEvent as CanonShape>::canon_shape(&mut reg);
        <dyn LogRecord as CanonShapeUnsized>::canon_shape(&mut reg);
        <crate::messages::message::Message as CanonShape>::canon_shape(&mut reg);
        reg
    }
}

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

/// The canonical dump of a game (`StateDumper.DumpGame`).
pub fn dump_game(game: &Game) -> Vec<u8> {
    let mut w = CanonWriter::for_game(game);
    game.canon(&mut w);
    w.out
}

/// The canonical rendering of a value outside the game state, such as a
/// validation message (`StateDumper.DumpDetached`).
pub fn dump_detached(v: &dyn Canon, game: Option<&Game>) -> Vec<u8> {
    let mut w = match game {
        Some(g) => CanonWriter::for_game(g),
        None => CanonWriter::new(None),
    };
    v.canon(&mut w);
    w.out
}

// ---------------------------------------------------------------------------
// Primitives
// ---------------------------------------------------------------------------

macro_rules! num_impl {
    ($method:ident as $cast:ty: $($t:ty),*) => {$(
        impl Canon for $t {
            fn canon(&self, w: &mut CanonWriter) { w.$method(*self as $cast) }
        }
        impl CanonShape for $t {
            fn canon_shape(_: &mut Registry) -> Shape { Shape::kind("num") }
        }
    )*};
}
num_impl!(int as i64: i8, i16, i32, i64);
num_impl!(uint as u64: u8, u16, u32, u64, usize);

impl Canon for f32 {
    fn canon(&self, w: &mut CanonWriter) {
        w.f32(*self)
    }
}
impl CanonShape for f32 {
    fn canon_shape(_: &mut Registry) -> Shape {
        Shape::kind("num").with("str")
    }
}
impl Canon for f64 {
    fn canon(&self, w: &mut CanonWriter) {
        w.f64(*self)
    }
}
impl CanonShape for f64 {
    fn canon_shape(_: &mut Registry) -> Shape {
        Shape::kind("num").with("str")
    }
}

impl Canon for bool {
    fn canon(&self, w: &mut CanonWriter) {
        w.bool(*self)
    }
}
impl CanonShape for bool {
    fn canon_shape(_: &mut Registry) -> Shape {
        Shape::kind("bool")
    }
}

impl Canon for str {
    fn canon(&self, w: &mut CanonWriter) {
        w.string(self)
    }
}
impl Canon for String {
    fn canon(&self, w: &mut CanonWriter) {
        w.string(self)
    }
}
impl CanonShape for String {
    fn canon_shape(_: &mut Registry) -> Shape {
        // A C# string field can be null; Rust models those as Option<String>.
        Shape::kind("str")
    }
}
impl Canon for &'static str {
    fn canon(&self, w: &mut CanonWriter) {
        w.string(self)
    }
}
impl CanonShape for &'static str {
    fn canon_shape(_: &mut Registry) -> Shape {
        Shape::kind("str")
    }
}
impl Canon for char {
    fn canon(&self, w: &mut CanonWriter) {
        w.string(&self.to_string())
    }
}
impl CanonShape for char {
    fn canon_shape(_: &mut Registry) -> Shape {
        Shape::kind("str")
    }
}

/// Placeholder for C# types the skeleton generator left as `()`; never in game state.
impl Canon for () {
    fn canon(&self, w: &mut CanonWriter) {
        w.null()
    }
}
impl CanonShape for () {
    fn canon_shape(_: &mut Registry) -> Shape {
        Shape::kind("null")
    }
}

impl Canon for DateTimeOffset {
    fn canon(&self, w: &mut CanonWriter) {
        w.string(&self.to_round_trip_string())
    }
}
impl CanonShape for DateTimeOffset {
    fn canon_shape(_: &mut Registry) -> Shape {
        Shape::kind("str")
    }
}
impl Canon for TimeSpan {
    fn canon(&self, w: &mut CanonWriter) {
        w.int(self.ticks)
    }
}
impl CanonShape for TimeSpan {
    fn canon_shape(_: &mut Registry) -> Shape {
        Shape::kind("num")
    }
}

/// Enum values: `"<Enum.ToString()>(<number>)"`. Used by `game_enum!`.
pub fn write_enum(w: &mut CanonWriter, text: &str, value: i32) {
    w.string(&format!("{text}({value})"));
}

// ---------------------------------------------------------------------------
// Wrappers and collections
// ---------------------------------------------------------------------------

impl<T: Canon> Canon for Option<T> {
    fn canon(&self, w: &mut CanonWriter) {
        match self {
            Some(v) => v.canon(w),
            None => w.null(),
        }
    }
}
impl<T: CanonShape> CanonShape for Option<T> {
    fn canon_shape(reg: &mut Registry) -> Shape {
        T::canon_shape(reg).with("null")
    }
}

impl<T: Canon + ?Sized> Canon for Box<T> {
    fn canon(&self, w: &mut CanonWriter) {
        (**self).canon(w)
    }
}
impl<T: CanonShape> CanonShape for Box<T> {
    fn canon_shape(reg: &mut Registry) -> Shape {
        T::canon_shape(reg)
    }
}

impl<T: Canon + ?Sized> Canon for Rc<T> {
    fn canon(&self, w: &mut CanonWriter) {
        (**self).canon(w)
    }
}
impl<T: CanonShape> CanonShape for Rc<T> {
    fn canon_shape(reg: &mut Registry) -> Shape {
        T::canon_shape(reg)
    }
}

impl<T: Canon + Copy> Canon for Cell<T> {
    fn canon(&self, w: &mut CanonWriter) {
        self.get().canon(w)
    }
}
impl<T: CanonShape + Copy> CanonShape for Cell<T> {
    fn canon_shape(reg: &mut Registry) -> Shape {
        T::canon_shape(reg)
    }
}

impl<T: Canon> Canon for RefCell<T> {
    fn canon(&self, w: &mut CanonWriter) {
        self.borrow().canon(w)
    }
}
impl<T: CanonShape> CanonShape for RefCell<T> {
    fn canon_shape(reg: &mut Registry) -> Shape {
        T::canon_shape(reg)
    }
}

impl<T: Canon> Canon for Shared<T> {
    fn canon(&self, w: &mut CanonWriter) {
        self.borrow().canon(w)
    }
}
impl<T: CanonShape> CanonShape for Shared<T> {
    fn canon_shape(reg: &mut Registry) -> Shape {
        T::canon_shape(reg)
    }
}

impl<T: Canon> Canon for RcKey<T> {
    fn canon(&self, w: &mut CanonWriter) {
        self.0.canon(w)
    }
}
impl<T: CanonShape> CanonShape for RcKey<T> {
    fn canon_shape(reg: &mut Registry) -> Shape {
        T::canon_shape(reg)
    }
}

fn write_seq<'a, T: Canon + 'a>(w: &mut CanonWriter, items: impl Iterator<Item = &'a T>) {
    w.arr_start();
    for (i, item) in items.enumerate() {
        if i > 0 {
            w.comma();
        }
        item.canon(w);
    }
    w.arr_end();
}

impl<T: Canon> Canon for [T] {
    fn canon(&self, w: &mut CanonWriter) {
        write_seq(w, self.iter())
    }
}
impl<T: Canon> Canon for Vec<T> {
    fn canon(&self, w: &mut CanonWriter) {
        write_seq(w, self.iter())
    }
}
impl<T: CanonShape> CanonShape for Vec<T> {
    fn canon_shape(reg: &mut Registry) -> Shape {
        Shape::list(T::canon_shape(reg))
    }
}
impl<T: Canon> Canon for VecDeque<T> {
    fn canon(&self, w: &mut CanonWriter) {
        write_seq(w, self.iter())
    }
}
impl<T: CanonShape> CanonShape for VecDeque<T> {
    fn canon_shape(reg: &mut Registry) -> Shape {
        Shape::list(T::canon_shape(reg))
    }
}
impl<T: Canon, const N: usize> Canon for [T; N] {
    fn canon(&self, w: &mut CanonWriter) {
        write_seq(w, self.iter())
    }
}
impl<T: CanonShape, const N: usize> CanonShape for [T; N] {
    fn canon_shape(reg: &mut Registry) -> Shape {
        Shape::list(T::canon_shape(reg))
    }
}

impl<T: Canon + Eq + Hash + Clone> Canon for NetHashSet<T> {
    fn canon(&self, w: &mut CanonWriter) {
        write_seq(w, self.iter())
    }
}
impl<T: CanonShape + Eq + Hash + Clone> CanonShape for NetHashSet<T> {
    fn canon_shape(reg: &mut Registry) -> Shape {
        Shape::list(T::canon_shape(reg))
    }
}

/// Dictionaries: `[[key, value], ...]` in enumeration order.
impl<K: Canon + Eq + Hash + Clone + 'static, V: Canon> Canon for NetDictionary<K, V> {
    fn canon(&self, w: &mut CanonWriter) {
        w.arr_start();
        for (i, (k, v)) in self.iter().enumerate() {
            if i > 0 {
                w.comma();
            }
            w.arr_start();
            k.canon(w);
            w.comma();
            v.canon(w);
            w.arr_end();
        }
        w.arr_end();
    }
}
impl<K: CanonShape + Eq + Hash + Clone, V: CanonShape> CanonShape for NetDictionary<K, V> {
    fn canon_shape(reg: &mut Registry) -> Shape {
        let k = K::canon_shape(reg);
        let v = V::canon_shape(reg);
        Shape::list(Shape::list(k.union(v)))
    }
}

impl<T: Canon + Eq + Hash> Canon for indexmap::IndexSet<T> {
    fn canon(&self, w: &mut CanonWriter) {
        write_seq(w, self.iter())
    }
}
impl<T: CanonShape + Eq + Hash> CanonShape for indexmap::IndexSet<T> {
    fn canon_shape(reg: &mut Registry) -> Shape {
        Shape::list(T::canon_shape(reg))
    }
}

impl<K: Canon + Eq + Hash, V: Canon> Canon for indexmap::IndexMap<K, V> {
    fn canon(&self, w: &mut CanonWriter) {
        w.arr_start();
        for (i, (k, v)) in self.iter().enumerate() {
            if i > 0 {
                w.comma();
            }
            w.arr_start();
            k.canon(w);
            w.comma();
            v.canon(w);
            w.arr_end();
        }
        w.arr_end();
    }
}
impl<K: CanonShape + Eq + Hash, V: CanonShape> CanonShape for indexmap::IndexMap<K, V> {
    fn canon_shape(reg: &mut Registry) -> Shape {
        let k = K::canon_shape(reg);
        let v = V::canon_shape(reg);
        Shape::list(Shape::list(k.union(v)))
    }
}

// ---------------------------------------------------------------------------
// References
// ---------------------------------------------------------------------------

impl Canon for PlayerIndex {
    fn canon(&self, w: &mut CanonWriter) {
        w.obj_start();
        w.key("$ref");
        w.string("Player");
        w.comma();
        w.key("index");
        w.int(self.index as i64);
        w.obj_end();
    }
}
impl CanonShape for PlayerIndex {
    fn canon_shape(_: &mut Registry) -> Shape {
        Shape::kind("ref:Player")
    }
}

/// `GameEvent.Game`: `{"$ref":"Game"}` once the event was initialized, else null.
pub mod game_link {
    use super::*;
    pub fn write(initialized: &std::cell::Cell<bool>, w: &mut CanonWriter) {
        if initialized.get() { w.reference("Game") } else { w.null() }
    }
    pub fn shape(_: &mut Registry) -> Shape {
        Shape::kind("ref:Game").with("null")
    }
}

macro_rules! location_ref {
    ($($t:ty),*) => {$(
        impl Canon for $t {
            fn canon(&self, w: &mut CanonWriter) {
                let l: Location = (*self).into();
                w.map_ref("Location", l.id())
            }
        }
        impl CanonShape for $t {
            fn canon_shape(_: &mut Registry) -> Shape { Shape::kind("ref:Location") }
        }
    )*};
}
location_ref!(
    Location,
    crate::map::homeworld::Homeworld,
    crate::map::discovery::DiscoveredLocation,
    crate::map::hidden_mobile_stronghold::HiddenMobileStronghold,
    crate::map::attached_location::AttachedLocation
);

impl Canon for Territory {
    fn canon(&self, w: &mut CanonWriter) {
        w.map_ref("Territory", self.id())
    }
}
impl CanonShape for Territory {
    fn canon_shape(_: &mut Registry) -> Shape {
        Shape::kind("ref:Territory")
    }
}

impl Canon for Leader {
    fn canon(&self, w: &mut CanonWriter) {
        w.entity_ref("Leader", self.id())
    }
}
impl CanonShape for Leader {
    fn canon_shape(_: &mut Registry) -> Shape {
        Shape::kind("ref:Leader")
    }
}

impl Canon for TreacheryCard {
    fn canon(&self, w: &mut CanonWriter) {
        w.entity_ref("TreacheryCard", self.id())
    }
}
impl CanonShape for TreacheryCard {
    fn canon_shape(_: &mut Registry) -> Shape {
        Shape::kind("ref:TreacheryCard")
    }
}

impl Canon for Hero {
    fn canon(&self, w: &mut CanonWriter) {
        match self {
            Hero::Leader(l) => l.canon(w),
            Hero::Messiah => w.entity_ref("Messiah", crate::heroes::messiah::Messiah::ID),
            Hero::TreacheryCard(c) => c.canon(w),
        }
    }
}
impl CanonShape for Hero {
    fn canon_shape(_: &mut Registry) -> Shape {
        Shape::kind("ref:Leader").with("ref:Messiah").with("ref:TreacheryCard")
    }
}

/// `ResourceCard` is an ordinary object in C# (not static data), so it is expanded.
impl Canon for ResourceCard {
    fn canon(&self, w: &mut CanonWriter) {
        let c = *self;
        w.object(
            "ResourceCard",
            vec![
                ("DiscoveryLocation", Field::Custom(Box::new(move |w| c.discovery_location().canon(w)))),
                ("IsGreatMaker", Field::Custom(Box::new(move |w| w.bool(c.is_great_maker())))),
                ("IsSandTrout", Field::Custom(Box::new(move |w| w.bool(c.is_sand_trout())))),
                ("Location", Field::Custom(Box::new(move |w| c.location().canon(w)))),
                ("SkinId", Field::Custom(Box::new(move |w| w.int(c.skin_id() as i64)))),
            ],
        )
    }
}
impl CanonShape for ResourceCard {
    fn canon_shape(reg: &mut Registry) -> Shape {
        reg.manual("ResourceCard", |reg| {
            let loc = <Option<Location>>::canon_shape(reg);
            vec![
                ("DiscoveryLocation", loc.clone()),
                ("IsGreatMaker", Shape::kind("bool")),
                ("IsSandTrout", Shape::kind("bool")),
                ("Location", loc),
                ("SkinId", Shape::kind("num")),
            ]
        })
    }
}

fn write_logged_random(w: &mut CanonWriter, count: i32) {
    w.obj_start();
    w.key("$type");
    w.string("LoggedRandom");
    w.comma();
    w.key("count");
    w.int(count as i64);
    w.obj_end();
}

fn logged_random_shape(reg: &mut Registry) -> Shape {
    reg.manual("LoggedRandom", |_| vec![("count", Shape::kind("num"))])
}

impl Canon for LoggedRandom {
    fn canon(&self, w: &mut CanonWriter) {
        write_logged_random(w, self.get_count())
    }
}
impl CanonShape for LoggedRandom {
    fn canon_shape(reg: &mut Registry) -> Shape {
        logged_random_shape(reg)
    }
}

/// `Game.Random` itself is `{"$ref":"GameRandom"}` wherever else it is held
/// (decks); any other `LoggedRandom` is written with its count.
impl Canon for SharedRandom {
    fn canon(&self, w: &mut CanonWriter) {
        if w.game_random.as_ref().is_some_and(|r| r.ptr_eq(self)) {
            w.reference("GameRandom")
        } else {
            let count = self.get_count();
            write_logged_random(w, count)
        }
    }
}
impl CanonShape for SharedRandom {
    fn canon_shape(reg: &mut Registry) -> Shape {
        logged_random_shape(reg).with("ref:GameRandom")
    }
}

/// Game errors: `{"$type":"$Exception","message":...,"type":"GameError"}`.
impl Canon for GameError {
    fn canon(&self, w: &mut CanonWriter) {
        w.obj_start();
        w.key("$type");
        w.string("$Exception");
        w.comma();
        w.key("message");
        w.string(self.message());
        w.comma();
        w.key("type");
        w.string("GameError");
        w.obj_end();
    }
}
impl CanonShape for GameError {
    fn canon_shape(reg: &mut Registry) -> Shape {
        reg.manual("$Exception", |_| vec![("message", Shape::kind("str")), ("type", Shape::kind("str"))])
    }
}

// ---------------------------------------------------------------------------
// Polymorphic values
// ---------------------------------------------------------------------------

impl Canon for GameEvent {
    fn canon(&self, w: &mut CanonWriter) {
        crate::events::dispatch!(self, e => e.canon(w))
    }
}

fn shape_of_rc<T: CanonShape>(_: &Rc<T>, reg: &mut Registry) -> Shape {
    T::canon_shape(reg)
}

impl CanonShape for GameEvent {
    /// Every event type. Built from a default instance of each type (the
    /// JSON reader with only `$type`), because the variants are generated.
    fn canon_shape(reg: &mut Registry) -> Shape {
        let mut shape = Shape::default();
        for t in crate::events::GameEventType::ALL {
            let text = format!("{{\"$type\":\"{}\"}}", t.name());
            match crate::json::deserialize::<GameEvent>(&text) {
                Ok(e) => {
                    let s = crate::events::dispatch!(&e, x => shape_of_rc(x, reg));
                    shape = shape.union(s);
                }
                Err(err) => reg.errors.push(format!("cannot build a default {}: {:?}", t.name(), err)),
            }
        }
        shape
    }
}

impl Canon for ExpressionElement {
    fn canon(&self, w: &mut CanonWriter) {
        use ExpressionElement as E;
        match self {
            E::Null => w.null(),
            E::String(s) => w.string(s),
            E::Bool(b) => w.bool(*b),
            E::Int(i) => w.int(*i as i64),
            E::Long(i) => w.int(*i),
            E::Float(f) => w.f32(*f),
            E::Double(f) => w.f64(*f),
            E::Enum(e) => write_enum(w, &e.ty.format(e.value), e.value),
            E::Location(l) => l.canon(w),
            E::Territory(t) => t.canon(w),
            E::Hero(h) => h.canon(w),
            E::ResourceCard(c) => c.canon(w),
            E::Payment(p) => p.canon(w),
            E::Message(m) => m.canon(w),
            E::MessagePart(m) => m.canon(w),
            E::Error(e) => e.canon(w),
            E::Player(p) => p.canon(w),
            E::List { items, live, .. } => {
                // A live list shows the game's contents now. Without a game
                // (`CanonWriter::new`) the snapshot taken at log time is written.
                let current = live.zip(w.root).and_then(|(l, g)| l.read(g));
                match current {
                    Some(items) => write_seq(w, items.iter()),
                    None => write_seq(w, items.iter()),
                }
            }
        }
    }
}
impl CanonShape for ExpressionElement {
    fn canon_shape(reg: &mut Registry) -> Shape {
        let mut s = Shape::kind("null").with("str").with("bool").with("num");
        s = s.union(Hero::canon_shape(reg));
        s = s.with("ref:Location").with("ref:Territory");
        s = s.union(ResourceCard::canon_shape(reg));
        s = s.union(<crate::messages::payment::Payment as CanonShape>::canon_shape(reg));
        s = s.union(<crate::messages::message::Message as CanonShape>::canon_shape(reg));
        s = s.union(<crate::messages::message::MessagePart as CanonShape>::canon_shape(reg));
        s = s.union(GameError::canon_shape(reg));
        s = s.with("ref:Player");
        // A list holds elements again; one level is enough for the test.
        let mut inner = Shape::kind("null").with("str").with("bool").with("num");
        inner = inner.union(Hero::canon_shape(reg)).with("ref:Location").with("ref:Territory");
        inner = inner.with("obj:MessagePart").with("obj:Message").with("obj:Payment").with("obj:ResourceCard").with("list");
        s.union(Shape::list(inner))
    }
}

/// Shape of an unsized type (`dyn LogRecord`).
pub trait CanonShapeUnsized {
    fn canon_shape(reg: &mut Registry) -> Shape;
}

/// The log records (`Model/Log`). New record types must be added here, or the
/// field-mapping test reports them as missing.
macro_rules! log_records {
    ($($t:ident),* $(,)?) => {
        impl CanonShapeUnsized for dyn LogRecord {
            fn canon_shape(reg: &mut Registry) -> Shape {
                let mut s = Shape::default();
                $( s = s.union(<crate::log::$t as CanonShape>::canon_shape(reg)); )*
                s
            }
        }
    };
}
log_records!(
    AsideLog,
    BiddingDrawn,
    BiddingPassedByAll,
    BiddingSkipped,
    BlowCardLog,
    BlowDeckLog,
    BlowReshuffleLog,
    BeneGesseritFlipped,
    BeneGesseritReaction,
    BribesCollected,
    CardSold,
    CardWonShown,
    CharityClaim,
    ExtraCardReceived,
    ExtraCardShown,
    FamilyAtomicsKilled,
    FamilyAtomicsUsed,
    GameEnded,
    GameWinner,
    GuildDelayed,
    MoveLogged,
    MovePassed,
    NexusAlliance,
    NexusLog,
    NexusRide,
    RevivalRow,
    ShipLogged,
    ShipMoveLoss,
    SpiceCollected,
    StormDestroyedSpice,
    StormDials,
    StormForecast,
    StormKilled,
    StormMoved,
    StormPlaced,
    TreacheryReshuffled,
    WinReason,
    WormAppearedLog,
    WormDevouredForcesLog,
    WormDevouredSpiceLog,
    WormIgnoredLog,
    WormSentLog,
    WormSurvivedLog,
);

impl CanonShape for Rc<dyn LogRecord> {
    fn canon_shape(reg: &mut Registry) -> Shape {
        <dyn LogRecord as CanonShapeUnsized>::canon_shape(reg)
    }
}

// ---------------------------------------------------------------------------
// Game special cases (`#[canon(with = ...)]` on Game fields)
// ---------------------------------------------------------------------------

/// `Game.History`: `{"$count":n}`.
pub mod history_count {
    use super::*;
    pub fn write(v: &Vec<GameEvent>, w: &mut CanonWriter) {
        w.obj_start();
        w.key("$count");
        w.int(v.len() as i64);
        w.obj_end();
    }
    pub fn shape(_: &mut Registry) -> Shape {
        Shape::kind("obj?:$count")
    }
}

/// `Game.PreviousGameState`: `{"$present":bool}`.
pub mod presence {
    use super::*;
    pub fn write<T>(v: &Option<T>, w: &mut CanonWriter) {
        w.obj_start();
        w.key("$present");
        w.bool(v.is_some());
        w.obj_end();
    }
    pub fn shape(_: &mut Registry) -> Shape {
        Shape::kind("obj?:$present")
    }
}

/// `Game.Random` at the root: expanded as `{"$type":"LoggedRandom","count":n}`.
pub mod game_random {
    use super::*;
    pub fn write(v: &Option<SharedRandom>, w: &mut CanonWriter) {
        match v {
            Some(r) => write_logged_random(w, r.get_count()),
            None => w.null(),
        }
    }
    pub fn shape(reg: &mut Registry) -> Shape {
        logged_random_shape(reg).with("null")
    }
}

// ---------------------------------------------------------------------------
// Map
// ---------------------------------------------------------------------------

/// The map is expanded once, at `Game.Map`: every location in `_locations`
/// order, every territory in `$territories` (first appearance order), and the
/// named properties as references. See trace-format.md 7.2.
impl Canon for Map {
    fn canon(&self, w: &mut CanonWriter) {
        let mut fields: Vec<(&'static str, Field<'_>)> = Vec::new();
        for (name, l) in named_locations(self) {
            fields.push((name, Field::Custom(Box::new(move |w| l.canon(w)))));
        }
        for (name, t) in named_territories(self) {
            fields.push((name, Field::Custom(Box::new(move |w| t.canon(w)))));
        }
        fields.push(("LocationLookup", Field::Custom(Box::new(|w| fetcher(w, "LocationFetcher")))));
        fields.push(("TerritoryLookup", Field::Custom(Box::new(|w| fetcher(w, "TerritoryFetcher")))));
        fields.push((
            "_locations",
            Field::Custom(Box::new(move |w| {
                w.arr_start();
                for (i, l) in Location::all().iter().enumerate() {
                    if i > 0 {
                        w.comma();
                    }
                    write_location(self, *l, w);
                }
                w.arr_end();
            })),
        ));
        fields.push((
            "$territories",
            Field::Custom(Box::new(|w| {
                w.arr_start();
                let mut seen: Vec<Territory> = Vec::new();
                for l in Location::all() {
                    let t = l.territory();
                    if seen.contains(&t) {
                        continue;
                    }
                    if !seen.is_empty() {
                        w.comma();
                    }
                    seen.push(t);
                    write_territory(t, w);
                }
                w.arr_end();
            })),
        ));
        w.object("Map", fields);
    }
}

fn fetcher(w: &mut CanonWriter, ty: &str) {
    w.obj_start();
    w.key("$type");
    w.string(ty);
    w.comma();
    w.key("map");
    w.reference("Map");
    w.obj_end();
}

fn named_locations(m: &Map) -> Vec<(&'static str, Location)> {
    vec![
        ("PolarSink", m.polar_sink()),
        ("Carthag", m.carthag()),
        ("Arrakeen", m.arrakeen()),
        ("TueksSietch", m.tueks_sietch()),
        ("SietchTabr", m.sietch_tabr()),
        ("HabbanyaSietch", m.habbanya_sietch()),
        ("FuneralPlain", m.funeral_plain()),
        ("TheGreatFlat", m.the_great_flat()),
        ("TheGreaterFlat", m.the_greater_flat()),
        ("HiddenMobileStronghold", m.hidden_mobile_stronghold().location()),
        ("Jacurutu", m.jacurutu().location()),
        ("Cistern", m.cistern().location()),
        ("TestingStation", m.testing_station().location()),
        ("Shrine", m.shrine().location()),
        ("ProcessingStation", m.processing_station().location()),
    ]
}

fn named_territories(m: &Map) -> Vec<(&'static str, Territory)> {
    vec![
        ("ImperialBasin", m.imperial_basin()),
        ("Meridan", m.meridan()),
        ("CielagoEast", m.cielago_east()),
        ("FalseWallSouth", m.false_wall_south()),
        ("FalseWallEast", m.false_wall_east()),
        ("TheMinorErg", m.the_minor_erg()),
        ("PastyMesa", m.pasty_mesa()),
        ("HoleInTheRock", m.hole_in_the_rock()),
        ("SihayaRidge", m.sihaya_ridge()),
        ("ShieldWall", m.shield_wall()),
        ("GaraKulon", m.gara_kulon()),
        ("OldGap", m.old_gap()),
        ("BrokenLand", m.broken_land()),
        ("Tsimpo", m.tsimpo()),
        ("RockOutcroppings", m.rock_outcroppings()),
        ("PlasticBasin", m.plastic_basin()),
        ("HaggaBasin", m.hagga_basin()),
        ("BightOfTheCliff", m.bight_of_the_cliff()),
        ("WindPass", m.wind_pass()),
        ("HabbanyaErg", m.habbanya_erg()),
        ("FalseWallWest", m.false_wall_west()),
        ("WindPassNorth", m.wind_pass_north()),
        ("CielagoWest", m.cielago_west()),
    ]
}

/// One entry of `Map._locations`, with the fields of its C# runtime type.
///
/// The dump reads backing fields, not the getters. Homeworlds override the
/// `Sector` getter (always -1) and attached locations (hidden mobile
/// stronghold, discovered strongholds) override `Sector` and `Neighbours`; the
/// map never sets the base backing fields of either, so their `Sector` stays 0
/// and the attached locations' `Neighbours` stays empty.
fn write_location(map: &Map, l: Location, w: &mut CanonWriter) {
    let d = l.data();
    let attached = matches!(d.kind, LocationKind::Discovered { .. } | LocationKind::HiddenMobileStronghold);
    let sector_overridden = attached || matches!(d.kind, LocationKind::Homeworld(_));
    let mut f: Vec<(&'static str, Field<'_>)> = vec![
        ("DiscoveryTokenType", Field::Custom(Box::new(move |w| d.discovery_token_type.canon(w)))),
        ("Id", Field::Custom(Box::new(move |w| w.int(d.id as i64)))),
        (
            "Neighbours",
            Field::Custom(Box::new(move |w| {
                if attached {
                    w.arr_start();
                    w.arr_end();
                } else {
                    l.neighbours(map).canon(w)
                }
            })),
        ),
        ("Orientation", Field::Custom(Box::new(move |w| w.string(d.orientation)))),
        ("Sector", Field::Custom(Box::new(move |w| w.int(if sector_overridden { 0 } else { d.sector as i64 })))),
        ("SpiceBlowAmount", Field::Custom(Box::new(move |w| w.int(d.spice_blow_amount as i64)))),
        ("Territory", Field::Custom(Box::new(move |w| d.territory.canon(w)))),
    ];
    match &d.kind {
        LocationKind::Normal => {}
        LocationKind::Homeworld(h) => {
            f.push(("BattleBonusAndLasgunShieldLimitAtHighThreshold", Field::Custom(Box::new(move |w| w.int(h.battle_bonus_and_lasgun_shield_limit_at_high_threshold as i64)))));
            f.push(("BattleBonusAndLasgunShieldLimitAtLowThreshold", Field::Custom(Box::new(move |w| w.int(h.battle_bonus_and_lasgun_shield_limit_at_low_threshold as i64)))));
            f.push(("Faction", Field::Custom(Box::new(move |w| h.faction.canon(w)))));
            f.push(("IsHomeOfNormalForces", Field::Custom(Box::new(move |w| w.bool(h.is_home_of_normal_forces)))));
            f.push(("IsHomeOfSpecialForces", Field::Custom(Box::new(move |w| w.bool(h.is_home_of_special_forces)))));
            f.push(("ResourceAmount", Field::Custom(Box::new(move |w| w.int(h.resource_amount as i64)))));
            f.push(("Threshold", Field::Custom(Box::new(move |w| w.int(h.threshold as i64)))));
            f.push(("World", Field::Custom(Box::new(move |w| h.world.canon(w)))));
        }
        LocationKind::Discovered { discovery } => {
            let discovery: DiscoveryToken = *discovery;
            push_attached(map, l, &mut f);
            f.push(("Discovery", Field::Custom(Box::new(move |w| discovery.canon(w)))));
        }
        LocationKind::HiddenMobileStronghold => push_attached(map, l, &mut f),
    }
    w.object(l.type_name(), f);
}

fn push_attached<'a>(map: &'a Map, l: Location, f: &mut Vec<(&'static str, Field<'a>)>) {
    let a = l.as_attached_location().expect("attached location");
    f.push(("AttachedToLocation", Field::Custom(Box::new(move |w| a.attached_to_location(map).canon(w)))));
    // `AttachedLocation.Game` is set by `PointAt`.
    f.push((
        "Game",
        Field::Custom(Box::new(move |w| match map.attached_to_game(l) {
            true => w.reference("Game"),
            false => w.null(),
        })),
    ));
}

fn write_territory(t: Territory, w: &mut CanonWriter) {
    let d = t.data();
    w.object(
        "Territory",
        vec![
            ("Advantage", Field::Custom(Box::new(move |w| d.advantage.canon(w)))),
            ("HasReducedShippingCost", Field::Custom(Box::new(move |w| w.bool(d.has_reduced_shipping_cost)))),
            ("Id", Field::Custom(Box::new(move |w| w.int(d.id as i64)))),
            ("IsDiscovery", Field::Custom(Box::new(move |w| w.bool(d.is_discovery)))),
            ("IsHomeworld", Field::Custom(Box::new(move |w| w.bool(d.is_homeworld)))),
            ("IsProtectedFromStorm", Field::Custom(Box::new(move |w| w.bool(d.is_protected_from_storm)))),
            ("IsProtectedFromWorm", Field::Custom(Box::new(move |w| w.bool(d.is_protected_from_worm)))),
            ("IsStronghold", Field::Custom(Box::new(move |w| w.bool(d.is_stronghold)))),
            ("Locations", Field::Custom(Box::new(move |w| t.locations().canon(w)))),
        ],
    )
}

impl CanonShape for Map {
    fn canon_shape(reg: &mut Registry) -> Shape {
        let map = Map::new();
        let loc_names: Vec<_> = named_locations(&map).into_iter().map(|(n, _)| n).collect();
        let ter_names: Vec<_> = named_territories(&map).into_iter().map(|(n, _)| n).collect();
        let base = |reg: &mut Registry| -> Vec<(&'static str, Shape)> {
            vec![
                ("DiscoveryTokenType", Shape::kind("str")),
                ("Id", Shape::kind("num")),
                ("Neighbours", Shape::list(Shape::kind("ref:Location"))),
                ("Orientation", Shape::kind("str")),
                ("Sector", Shape::kind("num")),
                ("SpiceBlowAmount", Shape::kind("num")),
                ("Territory", Territory::canon_shape(reg)),
            ]
        };
        let attached = |reg: &mut Registry| -> Vec<(&'static str, Shape)> {
            let mut v = base(reg);
            v.push(("AttachedToLocation", Shape::kind("ref:Location").with("null")));
            v.push(("Game", Shape::kind("ref:Game").with("null")));
            v
        };
        let mut location_shape = reg.manual("Location", base);
        location_shape = location_shape.union(reg.manual("Homeworld", |reg| {
            let mut v = base(reg);
            for n in [
                "BattleBonusAndLasgunShieldLimitAtHighThreshold",
                "BattleBonusAndLasgunShieldLimitAtLowThreshold",
                "ResourceAmount",
                "Threshold",
            ] {
                v.push((n, Shape::kind("num")));
            }
            v.push(("Faction", Shape::kind("str")));
            v.push(("World", Shape::kind("str")));
            v.push(("IsHomeOfNormalForces", Shape::kind("bool")));
            v.push(("IsHomeOfSpecialForces", Shape::kind("bool")));
            v
        }));
        location_shape = location_shape.union(reg.manual("DiscoveredLocation", |reg| {
            let mut v = attached(reg);
            v.push(("Discovery", Shape::kind("str")));
            v
        }));
        location_shape = location_shape.union(reg.manual("HiddenMobileStronghold", attached));
        let territory_shape = reg.manual("Territory", |_| {
            let mut v: Vec<(&'static str, Shape)> = vec![
                ("Advantage", Shape::kind("str")),
                ("Id", Shape::kind("num")),
                ("Locations", Shape::list(Shape::kind("ref:Location"))),
            ];
            for n in ["HasReducedShippingCost", "IsDiscovery", "IsHomeworld", "IsProtectedFromStorm", "IsProtectedFromWorm", "IsStronghold"] {
                v.push((n, Shape::kind("bool")));
            }
            v
        });
        let lf = reg.manual("LocationFetcher", |_| vec![("map", Shape::kind("ref:Map"))]);
        let tf = reg.manual("TerritoryFetcher", |_| vec![("map", Shape::kind("ref:Map"))]);
        reg.manual("Map", move |_| {
            let mut v: Vec<(&'static str, Shape)> = Vec::new();
            for n in loc_names {
                v.push((n, Shape::kind("ref:Location")));
            }
            for n in ter_names {
                v.push((n, Shape::kind("ref:Territory")));
            }
            v.push(("LocationLookup", lf));
            v.push(("TerritoryLookup", tf));
            v.push(("_locations", Shape::list(location_shape)));
            v.push(("$territories", Shape::list(territory_shape)));
            v
        })
    }
}
