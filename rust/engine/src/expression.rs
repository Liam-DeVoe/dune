//! Port of Model/Expression.cs.
//!
//! A C# `Expression` is an `object?[]`: strings, numbers, enum values, game
//! objects, nested messages and lists. Turning it into text is the client's
//! job (`Skin.Describe`), so the engine stores the objects themselves.
//! [`ExpressionElement`] is the closed set of runtime types that occur.
//!
//! Elements are built with `From`, so the [`express!`](crate::express) family
//! of macros accepts anything that converts:
//!
//! ```
//! use treachery_engine::{express, enumerations::{Concept, Faction}};
//! let m = express!(Faction::Atreides, " pays ", 3, " ", Concept::Resource);
//! assert_eq!(m.expression.elements.len(), 5);
//! ```

use std::fmt;

use crate::i_hero::Hero;
use crate::leader::Leader;
use crate::location::Location;
use crate::message::{Message, MessagePart};
use crate::net_enum::{EnumTypeInfo, NetEnum};
use crate::payment::Payment;
use crate::resource_card::ResourceCard;
use crate::territory::Territory;
use crate::treachery_card::TreacheryCard;

/// A value of some C# enum type, boxed as `object`.
#[derive(Clone, Copy)]
pub struct EnumValue {
    pub ty: &'static EnumTypeInfo,
    pub value: i32,
}

impl EnumValue {
    pub fn of<T: NetEnum>(v: T) -> Self {
        EnumValue { ty: T::INFO, value: v.value() }
    }

    /// Returns the value if it is of enum type `T`.
    pub fn get<T: NetEnum>(&self) -> Option<T> {
        if std::ptr::eq(self.ty, T::INFO) || self.ty.name == T::INFO.name { Some(T::from_value(self.value)) } else { None }
    }
}

impl PartialEq for EnumValue {
    fn eq(&self, other: &Self) -> bool {
        self.ty.name == other.ty.name && self.value == other.value
    }
}

impl fmt::Debug for EnumValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", self.ty.name, self.ty.format(self.value))
    }
}

/// One element of an expression: the runtime type of the C# `object`.
///
/// Equality is C# `object.Equals` for the value types (a NaN float equals
/// itself) and structural for the rest; C# compares messages and lists by
/// reference, which Rust cannot observe.
#[derive(Clone, Debug)]
pub enum ExpressionElement {
    /// `null`
    Null,
    String(String),
    Bool(bool),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    /// Any C# enum value (Faction, Concept, TreacheryCardType, ...).
    Enum(EnumValue),
    Location(Location),
    Territory(Territory),
    /// A `Leader`, the `Messiah`, or a `TreacheryCard` (cards are `IHero`).
    Hero(Hero),
    ResourceCard(ResourceCard),
    Payment(Payment),
    Message(Box<Message>),
    MessagePart(Box<MessagePart>),
    /// An `Exception` object (the "Game Error" message of `GameEvent.Execute`).
    /// C# renders it with `e.ToString()`, including a stack trace, which
    /// cannot be reproduced; compare type and message only.
    Exception(crate::cs::CsException),
    /// A `Player` object (only `NexusPlayed.cs:385` logs one). `Player` doesn't
    /// override `ToString()`, so it prints as its type name.
    Player(crate::player::PlayerIndex),
    /// Any other `IEnumerable` (lists, arrays, LINQ queries). `type_name` is
    /// the C# runtime type as `ToString()` prints it, such as
    /// ``System.Collections.Generic.List`1[Treachery.Shared.Faction]``.
    ///
    /// C# keeps the enumerable object itself and enumerates it when the
    /// message is rendered, so a live `List` shows its contents at render
    /// time. This stores a snapshot taken when the message is built, plus,
    /// for the few live game collections that C# logs (hazards 4.5), the
    /// source to read again when the state is dumped.
    List { type_name: String, items: Vec<ExpressionElement>, live: Option<LiveList> },
}

/// A game collection that a C# message keeps by reference, so the state dump
/// shows its contents at dump time rather than at log time (hazards 4.5).
///
/// Every C# site that puts a collection into a message is listed, with its
/// class, in `rust/docs/live-lists.md`; the sites of class (b) use one of these.
/// A variant names the collection by where it lives in the game. That is the
/// same object C# keeps only because none of these fields is ever reassigned
/// (each is a get-only property, or a `set` that only `Player.Clone` uses), so
/// a new variant needs the same check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiveList {
    /// `Player.TreacheryCards`, which C# never reassigns (only `Player.Clone`
    /// copies it). Logged at `AmbassadorActivated.cs:333`.
    PlayerTreacheryCards(crate::player::PlayerIndex),
    /// `Game.AuditedCards`, which C# clears and refills on the next audit
    /// (`Game_Battle.cs:427-429`). Logged at `AuditCancelled.cs:89`.
    GameAuditedCards,
    /// `Player.DiscardedTraitors`, a get-only `HashSet<IHero>` that
    /// `TraitorsSelected` adds to. Logged at `DealAccepted.cs:152`.
    PlayerDiscardedTraitors(crate::player::PlayerIndex),
}

impl LiveList {
    /// The runtime type the C# collection prints for `ToString()`.
    pub fn type_name(self) -> String {
        match self {
            LiveList::PlayerTreacheryCards(_) | LiveList::GameAuditedCards => Vec::<TreacheryCard>::cs_type_name(),
            LiveList::PlayerDiscardedTraitors(_) => format!("System.Collections.Generic.HashSet`1[{}]", Hero::cs_type_name()),
        }
    }

    /// The collection's contents now, in C# enumeration order, or `None` if
    /// `game` can't reach that player. A player of an earlier game (after
    /// `Undo`) is read from `game.other_games`, as C# reads that object.
    pub fn read(self, game: &crate::game::Game) -> Option<Vec<ExpressionElement>> {
        let cards = |c: &[TreacheryCard]| c.iter().map(|c| ExpressionElement::Hero(Hero::TreacheryCard(*c))).collect();
        let player = |p: crate::player::PlayerIndex| {
            let owner = if p.game == game.id { Some(game) } else { game.other_games.iter().find(|g| g.id == p.game) };
            owner.and_then(|g| g.players.get(p.index))
        };
        match self {
            LiveList::PlayerTreacheryCards(p) => player(p).map(|p| cards(&p.treachery_cards)),
            LiveList::GameAuditedCards => Some(cards(&game.audited_cards)),
            LiveList::PlayerDiscardedTraitors(p) => {
                player(p).map(|p| p.discarded_traitors.iter().map(|h| ExpressionElement::Hero(*h)).collect())
            }
        }
    }

    /// Every live list `game` can reach, so the state dump can read them all up front.
    pub fn all(game: &crate::game::Game) -> Vec<LiveList> {
        let mut all = vec![LiveList::GameAuditedCards];
        for g in std::iter::once(game).chain(game.other_games.iter()) {
            for i in 0..g.players.len() {
                let p = g.player_at(i);
                all.push(LiveList::PlayerTreacheryCards(p));
                all.push(LiveList::PlayerDiscardedTraitors(p));
            }
        }
        all
    }
}

/// The C# runtime type of a value that goes into a collection element, for
/// the type name a collection prints (`List<T>.ToString()`).
pub trait CsType {
    /// C# value type (enums, numbers, bool): `Option<T>` is then `Nullable<T>`.
    const IS_VALUE_TYPE: bool = false;
    /// The namespace-qualified C# type name, as `Type.ToString()` prints it.
    fn cs_type_name() -> String;
}

macro_rules! cs_type {
    ($($t:ty => $name:expr, $value:expr;)*) => {$(
        impl CsType for $t {
            const IS_VALUE_TYPE: bool = $value;
            fn cs_type_name() -> String { $name.to_string() }
        }
    )*};
}
cs_type! {
    &str => "System.String", false;
    String => "System.String", false;
    bool => "System.Boolean", true;
    i32 => "System.Int32", true;
    i64 => "System.Int64", true;
    f32 => "System.Single", true;
    f64 => "System.Double", true;
    Location => "Treachery.Shared.Location", false;
    crate::homeworld::Homeworld => "Treachery.Shared.Homeworld", false;
    crate::discovery_stronghold::DiscoveredLocation => "Treachery.Shared.DiscoveredLocation", false;
    crate::hidden_mobile_stronghold::HiddenMobileStronghold => "Treachery.Shared.HiddenMobileStronghold", false;
    Territory => "Treachery.Shared.Territory", false;
    Leader => "Treachery.Shared.Leader", false;
    Hero => "Treachery.Shared.IHero", false;
    TreacheryCard => "Treachery.Shared.TreacheryCard", false;
    ResourceCard => "Treachery.Shared.ResourceCard", false;
    Payment => "Treachery.Shared.Payment", false;
    Message => "Treachery.Shared.Message", false;
    MessagePart => "Treachery.Shared.MessagePart", false;
    ExpressionElement => "System.Object", false;
}

/// `T?`: `Nullable<T>` for value types, `T` itself for reference types.
impl<T: CsType> CsType for Option<T> {
    const IS_VALUE_TYPE: bool = T::IS_VALUE_TYPE;
    fn cs_type_name() -> String {
        if T::IS_VALUE_TYPE { format!("System.Nullable`1[{}]", T::cs_type_name()) } else { T::cs_type_name() }
    }
}

/// `List<T>`
impl<T: CsType> CsType for Vec<T> {
    fn cs_type_name() -> String {
        format!("System.Collections.Generic.List`1[{}]", T::cs_type_name())
    }
}

impl ExpressionElement {
    /// A `List<T>` element (C# passing a `List<T>`, or `ToList()`).
    pub fn list<T: Into<ExpressionElement> + CsType>(items: impl IntoIterator<Item = T>) -> Self {
        Self::collection(Vec::<T>::cs_type_name(), items)
    }

    /// A `T[]` element. Only for value-type arrays (`Faction[]`, `int[]`):
    /// a reference-type array passed as the only argument of `Express` is
    /// the element list itself (see [`express!`](crate::express)).
    pub fn array<T: Into<ExpressionElement> + CsType>(items: impl IntoIterator<Item = T>) -> Self {
        Self::collection(format!("{}[]", T::cs_type_name()), items)
    }

    /// Any other collection, with its C# runtime type name (a LINQ query
    /// prints e.g. ``System.Linq.Enumerable+WhereSelectListIterator`2[A,B]``).
    pub fn collection<T: Into<ExpressionElement>>(type_name: impl Into<String>, items: impl IntoIterator<Item = T>) -> Self {
        ExpressionElement::List { type_name: type_name.into(), items: items.into_iter().map(Into::into).collect(), live: None }
    }

    /// A game collection that C# passes to a message by reference: the
    /// contents now, plus the source the state dump reads again. See [`LiveList`].
    pub fn live(game: &crate::game::Game, source: LiveList) -> Self {
        ExpressionElement::List { type_name: source.type_name(), items: source.read(game).unwrap_or_default(), live: Some(source) }
    }

    /// The live `List<TreacheryCard>` of a player's hand (`p.TreacheryCards`
    /// passed to a message).
    pub fn live_player_treachery_cards(game: &crate::game::Game, p: crate::player::PlayerIndex) -> Self {
        Self::live(game, LiveList::PlayerTreacheryCards(p))
    }

    /// The live `Game.AuditedCards` list passed to a message.
    pub fn live_game_audited_cards(game: &crate::game::Game) -> Self {
        Self::live(game, LiveList::GameAuditedCards)
    }

    /// C# `object.ToString()` of the element (what `string.Join("", Elements)`
    /// produces when no describer is set). Game objects print their C# type
    /// name, as `base.ToString()` does.
    pub fn to_cs_string(&self) -> String {
        match self {
            ExpressionElement::Null => String::new(),
            ExpressionElement::String(s) => s.clone(),
            ExpressionElement::Bool(b) => if *b { "True" } else { "False" }.to_string(),
            ExpressionElement::Int(i) => i.to_string(),
            ExpressionElement::Long(i) => i.to_string(),
            // Invariant culture, which the oracle renders under (hazards 11.2).
            ExpressionElement::Float(f) => dotnet_compat::fmt::f32_to_string(*f),
            ExpressionElement::Double(f) => dotnet_compat::fmt::f64_to_string(*f),
            ExpressionElement::Enum(e) => e.ty.format(e.value),
            ExpressionElement::Location(l) => match l.type_name() {
                "Homeworld" => "Homeworld".to_string(),
                n => format!("Treachery.Shared.{n}"),
            },
            ExpressionElement::Territory(_) => "Treachery.Shared.Territory".to_string(),
            ExpressionElement::Hero(Hero::Leader(_)) => "Treachery.Shared.Leader".to_string(),
            ExpressionElement::Hero(Hero::Messiah) => "Treachery.Shared.Messiah".to_string(),
            ExpressionElement::Hero(Hero::TreacheryCard(_)) => "Treachery.Shared.TreacheryCard".to_string(),
            ExpressionElement::ResourceCard(_) => "Treachery.Shared.ResourceCard".to_string(),
            ExpressionElement::Payment(_) => "Treachery.Shared.Payment".to_string(),
            ExpressionElement::Message(m) => m.to_cs_string(),
            ExpressionElement::MessagePart(m) => m.to_cs_string(),
            ExpressionElement::Exception(e) => crate::cs::exception_to_string(e),
            ExpressionElement::Player(_) => "Treachery.Shared.Player".to_string(),
            ExpressionElement::List { type_name, .. } => type_name.clone(),
        }
    }
}

impl PartialEq for ExpressionElement {
    fn eq(&self, other: &Self) -> bool {
        use ExpressionElement as E;
        match (self, other) {
            (E::Null, E::Null) => true,
            (E::String(a), E::String(b)) => a == b,
            (E::Bool(a), E::Bool(b)) => a == b,
            (E::Int(a), E::Int(b)) => a == b,
            (E::Long(a), E::Long(b)) => a == b,
            (E::Float(a), E::Float(b)) => dotnet_compat::math::equals_f32(*a, *b),
            (E::Double(a), E::Double(b)) => dotnet_compat::math::equals_f64(*a, *b),
            (E::Enum(a), E::Enum(b)) => a == b,
            (E::Location(a), E::Location(b)) => a == b,
            (E::Territory(a), E::Territory(b)) => a == b,
            (E::Hero(a), E::Hero(b)) => a == b,
            (E::ResourceCard(a), E::ResourceCard(b)) => a == b,
            (E::Payment(a), E::Payment(b)) => a == b,
            (E::Message(a), E::Message(b)) => a == b,
            (E::MessagePart(a), E::MessagePart(b)) => a == b,
            (E::Exception(a), E::Exception(b)) => a == b,
            (E::Player(a), E::Player(b)) => a == b,
            (E::List { type_name: ta, items: a, live: la }, E::List { type_name: tb, items: b, live: lb }) => ta == tb && a == b && la == lb,
            _ => false,
        }
    }
}

impl From<&str> for ExpressionElement {
    fn from(v: &str) -> Self {
        ExpressionElement::String(v.to_string())
    }
}
impl From<String> for ExpressionElement {
    fn from(v: String) -> Self {
        ExpressionElement::String(v)
    }
}
impl From<&String> for ExpressionElement {
    fn from(v: &String) -> Self {
        ExpressionElement::String(v.clone())
    }
}
impl From<bool> for ExpressionElement {
    fn from(v: bool) -> Self {
        ExpressionElement::Bool(v)
    }
}
impl From<i32> for ExpressionElement {
    fn from(v: i32) -> Self {
        ExpressionElement::Int(v)
    }
}
impl From<i64> for ExpressionElement {
    fn from(v: i64) -> Self {
        ExpressionElement::Long(v)
    }
}
impl From<f32> for ExpressionElement {
    fn from(v: f32) -> Self {
        ExpressionElement::Float(v)
    }
}
impl From<f64> for ExpressionElement {
    fn from(v: f64) -> Self {
        ExpressionElement::Double(v)
    }
}
impl From<Location> for ExpressionElement {
    fn from(v: Location) -> Self {
        if crate::cs::CsNull::is_null(v) {
            return ExpressionElement::Null;
        }
        ExpressionElement::Location(v)
    }
}
impl From<crate::homeworld::Homeworld> for ExpressionElement {
    fn from(v: crate::homeworld::Homeworld) -> Self {
        ExpressionElement::Location(v.location())
    }
}
impl From<crate::discovery_stronghold::DiscoveredLocation> for ExpressionElement {
    fn from(v: crate::discovery_stronghold::DiscoveredLocation) -> Self {
        ExpressionElement::Location(v.location())
    }
}
impl From<crate::hidden_mobile_stronghold::HiddenMobileStronghold> for ExpressionElement {
    fn from(v: crate::hidden_mobile_stronghold::HiddenMobileStronghold) -> Self {
        ExpressionElement::Location(v.location())
    }
}
impl From<Territory> for ExpressionElement {
    fn from(v: Territory) -> Self {
        if crate::cs::CsNull::is_null(v) {
            return ExpressionElement::Null;
        }
        ExpressionElement::Territory(v)
    }
}
impl From<Hero> for ExpressionElement {
    fn from(v: Hero) -> Self {
        if crate::cs::CsNull::is_null(v) {
            return ExpressionElement::Null;
        }
        ExpressionElement::Hero(v)
    }
}
impl From<Leader> for ExpressionElement {
    fn from(v: Leader) -> Self {
        ExpressionElement::from(Hero::from(v))
    }
}
impl From<TreacheryCard> for ExpressionElement {
    fn from(v: TreacheryCard) -> Self {
        ExpressionElement::from(Hero::from(v))
    }
}
impl From<ResourceCard> for ExpressionElement {
    fn from(v: ResourceCard) -> Self {
        if crate::cs::CsNull::is_null(v) {
            return ExpressionElement::Null;
        }
        ExpressionElement::ResourceCard(v)
    }
}
impl From<crate::player::PlayerIndex> for ExpressionElement {
    fn from(v: crate::player::PlayerIndex) -> Self {
        if crate::cs::CsNull::is_null(v) {
            return ExpressionElement::Null;
        }
        ExpressionElement::Player(v)
    }
}
impl From<Payment> for ExpressionElement {
    fn from(v: Payment) -> Self {
        ExpressionElement::Payment(v)
    }
}
impl From<crate::cs::CsException> for ExpressionElement {
    fn from(v: crate::cs::CsException) -> Self {
        ExpressionElement::Exception(v)
    }
}
impl From<Message> for ExpressionElement {
    fn from(v: Message) -> Self {
        ExpressionElement::Message(Box::new(v))
    }
}
impl From<MessagePart> for ExpressionElement {
    fn from(v: MessagePart) -> Self {
        ExpressionElement::MessagePart(Box::new(v))
    }
}
/// A nullable reference: `None` is C# `null`.
impl<T: Into<ExpressionElement>> From<Option<T>> for ExpressionElement {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(v) => v.into(),
            None => ExpressionElement::Null,
        }
    }
}
/// A `List<T>`. A C# array becomes [`ExpressionElement::array`] instead.
impl<T: Into<ExpressionElement> + CsType> From<Vec<T>> for ExpressionElement {
    fn from(v: Vec<T>) -> Self {
        ExpressionElement::list(v)
    }
}
impl<T: Into<ExpressionElement> + CsType + Clone> From<&[T]> for ExpressionElement {
    fn from(v: &[T]) -> Self {
        ExpressionElement::list(v.iter().cloned())
    }
}
impl<T: Into<ExpressionElement> + CsType + Clone> From<&Vec<T>> for ExpressionElement {
    fn from(v: &Vec<T>) -> Self {
        ExpressionElement::list(v.iter().cloned())
    }
}
/// A value-type array such as `new[] { Faction.Atreides }`.
impl<T: Into<ExpressionElement> + CsType, const N: usize> From<[T; N]> for ExpressionElement {
    fn from(v: [T; N]) -> Self {
        ExpressionElement::array(v)
    }
}

/// `Expression`
#[derive(Clone, Debug, PartialEq, Default, canon_derive::Canon)]
pub struct Expression {
    pub elements: Vec<ExpressionElement>,
}

impl Expression {
    /// `new Expression(params object?[] elements)`
    pub fn new(elements: Vec<ExpressionElement>) -> Self {
        Expression { elements }
    }

    /// `Expression.Express(params object[] list)`
    pub fn express(elements: Vec<ExpressionElement>) -> Self {
        Expression { elements }
    }
}
