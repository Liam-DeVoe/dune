//! An `Expression` is a list of values: strings, numbers, enum values, game
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

use crate::heroes::hero::Hero;
use crate::heroes::leader::Leader;
use crate::map::location::Location;
use crate::messages::message::{Message, MessagePart};
use crate::compat::net_enum::{EnumTypeInfo, NetEnum};
use crate::messages::payment::Payment;
use crate::cards::resource_card::ResourceCard;
use crate::map::territory::Territory;
use crate::cards::treachery_card::TreacheryCard;

/// A value of some game enum type, tagged with its type.
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

/// One element of an expression, by runtime type.
///
/// Equality is by value for numbers, strings and enums (a NaN float equals
/// itself), each type's own `==` for game objects, and structural for
/// messages and lists.
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
    /// Any enum value (Faction, Concept, TreacheryCardType, ...).
    Enum(EnumValue),
    Location(Location),
    Territory(Territory),
    /// A `Leader`, the `Messiah`, or a `TreacheryCard` (cards are `IHero`).
    Hero(Hero),
    ResourceCard(ResourceCard),
    Payment(Payment),
    Message(Box<Message>),
    MessagePart(Box<MessagePart>),
    /// Why an accepted event failed (the "Game Error" message of `GameEventMethods::execute`).
    Error(crate::error::GameError),
    /// A `Player` object (only the Nexus card play logs one). It prints as its
    /// type name.
    Player(crate::game::PlayerIndex),
    /// Any other collection (lists, arrays, lazy queries). `type_name` is
    /// the .NET runtime type name it prints as, such as
    /// ``System.Collections.Generic.List`1[Treachery.Shared.Faction]``.
    ///
    /// `items` is a snapshot taken when the message is built. For the few
    /// live game collections that are logged (hazards 4.5), `live` names the
    /// source to read again when the state is dumped.
    List { type_name: String, items: Vec<ExpressionElement>, live: Option<LiveList> },
}

/// A game collection that a message refers to rather than copies, so the state
/// dump shows its contents at dump time rather than at log time (hazards 4.5).
///
/// Every site that puts a collection into a message is listed, with its
/// class, in `rust/docs/live-lists.md`; the sites of class (b) use one of these.
/// A variant names the collection by where it lives in the game. That only
/// identifies the logged collection because none of these fields is ever
/// reassigned (only cloning a player copies them), so a new variant needs the
/// same check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiveList {
    /// `Player.TreacheryCards`, which is never reassigned. Logged when an
    /// ambassador is activated.
    PlayerTreacheryCards(crate::game::PlayerIndex),
    /// `Game.AuditedCards`, which is cleared and refilled on the next audit.
    /// Logged when an audit is cancelled.
    GameAuditedCards,
    /// `Player.DiscardedTraitors`, a set that `TraitorsSelected` adds to.
    /// Logged when a deal is accepted.
    PlayerDiscardedTraitors(crate::game::PlayerIndex),
}

impl LiveList {
    /// The .NET runtime type name the collection prints as.
    pub fn type_name(self) -> String {
        match self {
            LiveList::PlayerTreacheryCards(_) | LiveList::GameAuditedCards => Vec::<TreacheryCard>::cs_type_name(),
            LiveList::PlayerDiscardedTraitors(_) => format!("System.Collections.Generic.HashSet`1[{}]", Hero::cs_type_name()),
        }
    }

    /// The collection's contents now, in enumeration order, or `None` if
    /// `game` can't reach that player. A player of an earlier game (after
    /// `Undo`) is read from `game.other_games`.
    pub fn read(self, game: &crate::game::Game) -> Option<Vec<ExpressionElement>> {
        let cards = |c: &[TreacheryCard]| c.iter().map(|c| ExpressionElement::Hero(Hero::TreacheryCard(*c))).collect();
        let player = |p: crate::game::PlayerIndex| {
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

/// The .NET runtime type of a value that goes into a collection element, for
/// the type name a collection prints (`List<T>.ToString()`).
pub trait CsType {
    /// A .NET value type (enums, numbers, bool): `Option<T>` is then `Nullable<T>`.
    const IS_VALUE_TYPE: bool = false;
    /// The namespace-qualified .NET type name, as `Type.ToString()` prints it.
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
    crate::map::homeworld::Homeworld => "Treachery.Shared.Homeworld", false;
    crate::map::discovery::DiscoveredLocation => "Treachery.Shared.DiscoveredLocation", false;
    crate::map::hidden_mobile_stronghold::HiddenMobileStronghold => "Treachery.Shared.HiddenMobileStronghold", false;
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
    /// A `List<T>` element.
    pub fn list<T: Into<ExpressionElement> + CsType>(items: impl IntoIterator<Item = T>) -> Self {
        Self::collection(Vec::<T>::cs_type_name(), items)
    }

    /// A `T[]` element. Only for value-type arrays (`Faction[]`, `int[]`):
    /// a reference-type array passed as the only argument of `Express` is
    /// the element list itself (see [`express!`](crate::express)).
    pub fn array<T: Into<ExpressionElement> + CsType>(items: impl IntoIterator<Item = T>) -> Self {
        Self::collection(format!("{}[]", T::cs_type_name()), items)
    }

    /// Any other collection, with its .NET runtime type name (a lazy query
    /// prints e.g. ``System.Linq.Enumerable+WhereSelectListIterator`2[A,B]``).
    pub fn collection<T: Into<ExpressionElement>>(type_name: impl Into<String>, items: impl IntoIterator<Item = T>) -> Self {
        ExpressionElement::List { type_name: type_name.into(), items: items.into_iter().map(Into::into).collect(), live: None }
    }

    /// A game collection that a message refers to rather than copies: the
    /// contents now, plus the source the state dump reads again. See [`LiveList`].
    pub fn live(game: &crate::game::Game, source: LiveList) -> Self {
        ExpressionElement::List { type_name: source.type_name(), items: source.read(game).unwrap_or_default(), live: Some(source) }
    }

    /// The live `List<TreacheryCard>` of a player's hand (`p.TreacheryCards`
    /// passed to a message).
    pub fn live_player_treachery_cards(game: &crate::game::Game, p: crate::game::PlayerIndex) -> Self {
        Self::live(game, LiveList::PlayerTreacheryCards(p))
    }

    /// The live `Game.AuditedCards` list passed to a message.
    pub fn live_game_audited_cards(game: &crate::game::Game) -> Self {
        Self::live(game, LiveList::GameAuditedCards)
    }

    /// The element as plain text, as used when no describer is set. Game
    /// objects print their type name.
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
            ExpressionElement::Error(e) => e.to_string(),
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
            (E::Error(a), E::Error(b)) => a == b,
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
        if crate::compat::cs::CsNull::is_null(v) {
            return ExpressionElement::Null;
        }
        ExpressionElement::Location(v)
    }
}
impl From<crate::map::homeworld::Homeworld> for ExpressionElement {
    fn from(v: crate::map::homeworld::Homeworld) -> Self {
        ExpressionElement::Location(v.location())
    }
}
impl From<crate::map::discovery::DiscoveredLocation> for ExpressionElement {
    fn from(v: crate::map::discovery::DiscoveredLocation) -> Self {
        ExpressionElement::Location(v.location())
    }
}
impl From<crate::map::hidden_mobile_stronghold::HiddenMobileStronghold> for ExpressionElement {
    fn from(v: crate::map::hidden_mobile_stronghold::HiddenMobileStronghold) -> Self {
        ExpressionElement::Location(v.location())
    }
}
impl From<Territory> for ExpressionElement {
    fn from(v: Territory) -> Self {
        if crate::compat::cs::CsNull::is_null(v) {
            return ExpressionElement::Null;
        }
        ExpressionElement::Territory(v)
    }
}
impl From<Hero> for ExpressionElement {
    fn from(v: Hero) -> Self {
        if crate::compat::cs::CsNull::is_null(v) {
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
        if crate::compat::cs::CsNull::is_null(v) {
            return ExpressionElement::Null;
        }
        ExpressionElement::ResourceCard(v)
    }
}
impl From<crate::game::PlayerIndex> for ExpressionElement {
    fn from(v: crate::game::PlayerIndex) -> Self {
        if crate::compat::cs::CsNull::is_null(v) {
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
impl From<crate::error::GameError> for ExpressionElement {
    fn from(v: crate::error::GameError) -> Self {
        ExpressionElement::Error(v)
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
/// `None` becomes [`ExpressionElement::Null`].
impl<T: Into<ExpressionElement>> From<Option<T>> for ExpressionElement {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(v) => v.into(),
            None => ExpressionElement::Null,
        }
    }
}
/// A `List<T>`. For an array, use [`ExpressionElement::array`] instead.
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
