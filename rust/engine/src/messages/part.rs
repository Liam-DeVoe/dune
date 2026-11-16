//! A [`Part`] is one piece of a message: a text, a number, a game object, an enum value, a list
//! or a nested phrase. Turning parts into words is the skin's job (`treachery_skin`), so the
//! engine stores the values themselves.
//!
//! Parts are built with `From`, so the [`express!`](crate::express) family of macros accepts
//! anything that converts:
//!
//! ```
//! use treachery_engine::{express, enumerations::{Concept, Faction}};
//! let m = express!(Faction::Atreides, " pays ", 3, " ", Concept::Resource);
//! assert_eq!(m.parts.len(), 5);
//! ```

use std::borrow::Cow;

use crate::cards::resource_card::ResourceCard;
use crate::cards::treachery_card::TreacheryCard;
use crate::enum_type::{value_or_absent, EnumTypeInfo, GameEnum};
use crate::enumerations::*;
use crate::heroes::hero::Hero;
use crate::heroes::leader::Leader;
use crate::map::location::Location;
use crate::map::territory::Territory;
use crate::messages::message::{BattleFactKind, Message, MessagePart};
use crate::messages::payment::Payment;

/// An enum that has its own [`Part`] variant.
pub trait PartEnum: GameEnum {
    /// The value if `part` is of this enum: `Some(None)` for the absent value of an enum whose
    /// C# `None` member is absence in Rust.
    fn from_part(part: &Part) -> Option<Option<Self>>;
    /// The part of a value, or of the absent value. For an enum without one, absence is
    /// [`Part::Null`].
    fn part(v: Option<Self>) -> Part;
}

/// Declares [`Part`]. `plain` enums are logged as themselves. `optional` enums are those whose
/// C# `None` member is `Option::None` in Rust (`absent None = 0;` in `game_enum!`): their part
/// holds an `Option`, because the skin and the views name the absent value "None".
macro_rules! part {
    (plain [$($p:ident),* $(,)?] optional [$($o:ident),* $(,)?]) => {
        /// One piece of a message. Enum variants are named after their enum.
        #[derive(Clone, Debug, PartialEq)]
        pub enum Part {
            /// No value (a C# `null`): no text, and a bold "none" in the client.
            Null,
            Text(Cow<'static, str>),
            /// "Yes" or "No".
            Bool(bool),
            Number(i32),
            /// A `float`, printed the way .NET prints one (a battle strength, a dial).
            Float(f32),
            /// A `double`, printed the way .NET prints one.
            Double(f64),
            /// An amount of spice, paid by `by` if there is one.
            Payment { amount: i32, by: Option<Faction> },
            Location(Location),
            Territory(Territory),
            Leader(Leader),
            Messiah,
            Card(TreacheryCard),
            SpiceCard(ResourceCard),
            /// A list of values. The skin separates them with ", ".
            List(Vec<Part>),
            /// The parts of an `express_part!`, shown in line.
            Phrase(Vec<Part>),
            /// Why an accepted event failed (the "Game Error" message of
            /// `GameEventMethods::execute`), shown as text.
            Error(crate::error::GameError),
            /// A message nested in another, shown in line. [`Game::log`](crate::game::Game::log)
            /// keeps the detail of a message logged on its own.
            Message(Box<Message>),
            $($p($p),)*
            $($o(Option<$o>),)*
        }

        $(
            impl From<$p> for Part {
                fn from(v: $p) -> Self {
                    Part::$p(v)
                }
            }
            impl NullPart for $p {}
            impl PartEnum for $p {
                fn from_part(part: &Part) -> Option<Option<Self>> {
                    if let Part::$p(v) = part { Some(Some(*v)) } else { None }
                }
                fn part(v: Option<Self>) -> Part {
                    v.map_or(Part::Null, Part::$p)
                }
            }
        )*

        $(
            impl From<$o> for Part {
                fn from(v: $o) -> Self {
                    Part::$o(Some(v))
                }
            }
            /// Absent is the C# `None` member, which the skin names.
            impl From<Option<$o>> for Part {
                fn from(v: Option<$o>) -> Self {
                    Part::$o(v)
                }
            }
            impl PartEnum for $o {
                fn from_part(part: &Part) -> Option<Option<Self>> {
                    if let Part::$o(v) = part { Some(*v) } else { None }
                }
                fn part(v: Option<Self>) -> Part {
                    Part::$o(v)
                }
            }
        )*

        impl Part {
            /// The enum type and number of an enum part.
            pub fn enum_value(&self) -> Option<(&'static EnumTypeInfo, i32)> {
                match self {
                    $(Part::$p(v) => Some((<$p as GameEnum>::INFO, v.value())),)*
                    $(Part::$o(v) => Some((<$o as GameEnum>::INFO, value_or_absent(*v))),)*
                    _ => None,
                }
            }
        }
    };
}

part! {
    plain [
        ShipmentPermission, UserStatus, SubscriptionType, GameListScope, ErrorType, DealType, ShipmentType, AutomationRuleType, ItemAction, AsideKind, NexusAllianceChange, ShipKind,
        BeneGesseritReactionKind, BattleFactKind,
    ]
    optional [
        Faction, MainPhase, Phase, Nexus, Ambassador, TechToken, MainPhaseMoment, ChoamEconomicsStatus, Milestone, FactionAdvantage, FactionForce,
        FactionSpecialForce, Ruleset, DiscoveryToken, DiscoveryTokenType, Rule, RuleGroup, TreacheryCardType, Concept,
        WinMethod, HeroType, AuctionType, JuiceType, LeaderSkill, StrongholdAdvantage, World, CaptureDecision, TerrorType,
        IntrusionType, VidalMoment, ClairVoyanceAnswer, ClairvoyanceQuestion, PrescienceAspect,
    ]
}

impl Part {
    /// A list of values.
    pub fn list<T: Into<Part>>(items: impl IntoIterator<Item = T>) -> Self {
        Part::List(items.into_iter().map(Into::into).collect())
    }

    /// The value if this is a part of enum `T` (not its absent value).
    pub fn get<T: PartEnum>(&self) -> Option<T> {
        T::from_part(self).flatten()
    }

    /// The value if this is a part of enum `T`: `Some(None)` for its absent value.
    pub fn get_opt<T: PartEnum>(&self) -> Option<Option<T>> {
        T::from_part(self)
    }

    /// The text without a skin, for tests and debugging: enum values by member name, game
    /// objects by the type name the C# engine printed for them, list items in brackets.
    pub fn plain_text(&self) -> String {
        use dotnet_compat::exception::CsToString;
        match self {
            Part::Null => String::new(),
            Part::Text(s) => s.to_string(),
            Part::Bool(b) => if *b { "True" } else { "False" }.to_string(),
            Part::Number(i) => i.to_string(),
            Part::Float(f) => dotnet_compat::fmt::f32_to_string(*f),
            Part::Double(f) => dotnet_compat::fmt::f64_to_string(*f),
            Part::Payment { .. } => "Treachery.Shared.Payment".to_string(),
            Part::Location(l) => l.cs_to_string(),
            Part::Territory(_) => "Treachery.Shared.Territory".to_string(),
            Part::Leader(l) => Hero::Leader(*l).cs_to_string(),
            Part::Messiah => Hero::Messiah.cs_to_string(),
            Part::Card(c) => Hero::TreacheryCard(*c).cs_to_string(),
            Part::SpiceCard(_) => "Treachery.Shared.ResourceCard".to_string(),
            Part::List(items) => format!("[{}]", items.iter().map(Part::plain_text).collect::<Vec<_>>().join(", ")),
            Part::Phrase(parts) => parts.iter().map(Part::plain_text).collect(),
            Part::Error(e) => e.to_string(),
            Part::Message(m) => m.plain_text(),
            other => {
                let (info, value) = other.enum_value().expect("every part that isn't an enum is named above");
                info.format(value)
            }
        }
    }

    /// The hero this part names, if any.
    pub fn hero(&self) -> Option<Hero> {
        match self {
            Part::Leader(l) => Some(Hero::Leader(*l)),
            Part::Messiah => Some(Hero::Messiah),
            Part::Card(c) => Some(Hero::TreacheryCard(*c)),
            _ => None,
        }
    }
}

impl From<&'static str> for Part {
    fn from(v: &'static str) -> Self {
        Part::Text(Cow::Borrowed(v))
    }
}
impl From<String> for Part {
    fn from(v: String) -> Self {
        Part::Text(Cow::Owned(v))
    }
}
impl From<&String> for Part {
    fn from(v: &String) -> Self {
        Part::Text(Cow::Owned(v.clone()))
    }
}
impl From<bool> for Part {
    fn from(v: bool) -> Self {
        Part::Bool(v)
    }
}
impl From<i32> for Part {
    fn from(v: i32) -> Self {
        Part::Number(v)
    }
}
/// An amount of forces or spice.
impl From<u16> for Part {
    fn from(v: u16) -> Self {
        Part::Number(v.into())
    }
}
impl From<crate::map::sector::Sector> for Part {
    fn from(v: crate::map::sector::Sector) -> Self {
        Part::Number(v.into())
    }
}
impl From<crate::game::Turn> for Part {
    fn from(v: crate::game::Turn) -> Self {
        Part::Number(v.into())
    }
}
impl From<f32> for Part {
    fn from(v: f32) -> Self {
        Part::Float(v)
    }
}
impl From<f64> for Part {
    fn from(v: f64) -> Self {
        Part::Double(v)
    }
}
impl From<Location> for Part {
    fn from(v: Location) -> Self {
        Part::Location(v)
    }
}
impl From<crate::map::homeworld::Homeworld> for Part {
    fn from(v: crate::map::homeworld::Homeworld) -> Self {
        Part::Location(v.location())
    }
}
impl From<crate::map::discovery::DiscoveredLocation> for Part {
    fn from(v: crate::map::discovery::DiscoveredLocation) -> Self {
        Part::Location(v.location())
    }
}
impl From<crate::map::hidden_mobile_stronghold::HiddenMobileStronghold> for Part {
    fn from(v: crate::map::hidden_mobile_stronghold::HiddenMobileStronghold) -> Self {
        Part::Location(v.location())
    }
}
impl From<Territory> for Part {
    fn from(v: Territory) -> Self {
        Part::Territory(v)
    }
}
impl From<Hero> for Part {
    fn from(v: Hero) -> Self {
        match v {
            Hero::Leader(l) => Part::Leader(l),
            Hero::Messiah => Part::Messiah,
            Hero::TreacheryCard(c) => Part::Card(c),
        }
    }
}
impl From<Leader> for Part {
    fn from(v: Leader) -> Self {
        Part::Leader(v)
    }
}
impl From<TreacheryCard> for Part {
    fn from(v: TreacheryCard) -> Self {
        Part::Card(v)
    }
}
impl From<ResourceCard> for Part {
    fn from(v: ResourceCard) -> Self {
        Part::SpiceCard(v)
    }
}
/// A `Player` object (only the Nexus card play logs one) prints as its C# type name.
impl From<crate::game::PlayerId> for Part {
    fn from(_: crate::game::PlayerId) -> Self {
        Part::Text(Cow::Borrowed("Treachery.Shared.Player"))
    }
}
impl From<Payment> for Part {
    fn from(v: Payment) -> Self {
        Part::Payment { amount: v.amount, by: v.by }
    }
}
impl From<crate::error::GameError> for Part {
    fn from(v: crate::error::GameError) -> Self {
        Part::Error(v)
    }
}
impl From<Message> for Part {
    fn from(v: Message) -> Self {
        Part::Message(Box::new(v))
    }
}
impl From<MessagePart> for Part {
    fn from(v: MessagePart) -> Self {
        Part::Phrase(v.parts)
    }
}

/// A type whose absence is [`Part::Null`] in a message. An enum whose absent value is its C#
/// `None` member doesn't have it: its `Option` logs that member.
pub trait NullPart {}
impl NullPart for &str {}
impl NullPart for String {}
impl NullPart for bool {}
impl NullPart for i32 {}
impl NullPart for u16 {}
impl NullPart for Location {}
impl NullPart for Territory {}
impl NullPart for Hero {}
impl NullPart for Leader {}
impl NullPart for TreacheryCard {}
impl NullPart for ResourceCard {}
impl NullPart for Payment {}
impl NullPart for Message {}
impl NullPart for MessagePart {}

/// `None` becomes [`Part::Null`].
impl<T: Into<Part> + NullPart> From<Option<T>> for Part {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(v) => v.into(),
            None => Part::Null,
        }
    }
}
/// A C# nullable of an enum whose `None` member is absence in Rust (`Faction?`): `None` is the
/// C# `null` and prints nothing, `Some(None)` is the `None` member, which the skin names.
impl<T: PartEnum> From<Option<Option<T>>> for Part {
    fn from(v: Option<Option<T>>) -> Self {
        match v {
            Some(v) => T::part(v),
            None => Part::Null,
        }
    }
}
impl<T: Into<Part>> From<Vec<T>> for Part {
    fn from(v: Vec<T>) -> Self {
        Part::list(v)
    }
}
impl<T: Into<Part> + Clone> From<&[T]> for Part {
    fn from(v: &[T]) -> Self {
        Part::list(v.iter().cloned())
    }
}
impl<T: Into<Part> + Clone> From<&Vec<T>> for Part {
    fn from(v: &Vec<T>) -> Self {
        Part::list(v.iter().cloned())
    }
}
impl<T: Into<Part>, const N: usize> From<[T; N]> for Part {
    fn from(v: [T; N]) -> Self {
        Part::list(v)
    }
}

/// `vec![Part::from(a), ...]`: the parts of a text, in the syntax of [`express!`](crate::express).
#[macro_export]
macro_rules! parts {
    ($($e:expr),* $(,)?) => {
        vec![$($crate::messages::part::Part::from($e)),*]
    };
}
