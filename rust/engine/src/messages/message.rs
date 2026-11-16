//! `Message`, `MessagePart`, and the records the client uses to draw messages
//! (`Route`, `BattlePlans`, `PlanSummary`, `BattleFact`).
//!
//! Build messages with the macros:
//!
//! - `express!(a, b, c)` builds a `Message` for everyone.
//! - `express_to!(f; a, b)` builds a `Message` addressed to faction `f`.
//! - `express_part!(a, b)` builds a `MessagePart`.
//! - `express_part_if!(cond; a, b)` builds a `MessagePart` that is empty
//!   unless `cond` holds.
//!
//! Every argument goes through `Part::from`, see part.rs.

use crate::enumerations::{Concept, Faction};
use crate::messages::part::Part;
use crate::heroes::hero::Hero;
use crate::map::location::Location;
use crate::enum_type::game_enum;
use crate::cards::treachery_card::TreacheryCard;
use crate::log::LogRecord;

/// Who may read a message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Audience {
    Everyone,
    Faction(Faction),
}

impl Audience {
    /// `Faction::None` means everyone.
    pub fn of(faction: Faction) -> Audience {
        if faction == Faction::None { Audience::Everyone } else { Audience::Faction(faction) }
    }

    /// The faction a private message is for, or `Faction::None` for everyone.
    pub fn faction(self) -> Faction {
        match self {
            Audience::Everyone => Faction::None,
            Audience::Faction(f) => f,
        }
    }

    pub fn is_everyone(self) -> bool {
        self == Audience::Everyone
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub to: Audience,
    pub parts: Vec<Part>,
    /// What the log draws besides the text. Most messages have none.
    pub detail: Option<Box<LogDetail>>,
}

/// The data a log line is drawn from besides its text.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct LogDetail {
    /// The concept whose icon heads a section of the log.
    pub heading: Option<Concept>,
    pub route: Option<Route>,
    pub plans: Option<BattlePlans>,
    pub fact: Option<BattleFact>,
    /// The record a phase's log design draws from (the types in `log/`).
    pub record: Option<LogRecord>,
}

impl Message {
    /// A message for everyone. Prefer [`express!`](crate::express).
    pub fn express(parts: Vec<Part>) -> Message {
        Message { to: Audience::Everyone, parts, detail: None }
    }

    /// A message for one faction. Prefer [`express_to!`](crate::express_to).
    pub fn express_to(to: Faction, parts: Vec<Part>) -> Message {
        Message { to: Audience::of(to), parts, detail: None }
    }

    /// The text without a skin, for tests and debugging ([`Part::plain_text`]).
    pub fn plain_text(&self) -> String {
        self.parts.iter().map(Part::plain_text).collect()
    }

    fn detail_mut(&mut self) -> &mut LogDetail {
        self.detail.get_or_insert_with(Default::default)
    }

    pub fn detail(&self) -> Option<&LogDetail> {
        self.detail.as_deref()
    }

    pub fn with_route(mut self, faction: Faction, from: impl IntoIterator<Item = Location>, to: Option<Location>) -> Message {
        self.detail_mut().route = Some(Route { faction, from: from.into_iter().collect(), to });
        self
    }

    pub fn as_heading(mut self, icon: Concept) -> Message {
        self.detail_mut().heading = Some(icon);
        self
    }

    pub fn with_plans(mut self, aggressor: PlanSummary, defender: PlanSummary) -> Message {
        self.detail_mut().plans = Some(BattlePlans { aggressor, defender });
        self
    }

    pub fn with_log(mut self, record: impl Into<LogRecord>) -> Message {
        self.detail_mut().record = Some(record.into());
        self
    }

    pub fn with_fact(mut self, fact: BattleFact) -> Message {
        self.detail_mut().fact = Some(fact);
        self
    }
}

/// A message holding just this text, as `express!(text)` makes it.
impl From<&'static str> for Message {
    fn from(text: &'static str) -> Message {
        Message::express(vec![Part::from(text)])
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MessagePart {
    pub parts: Vec<Part>,
}

impl MessagePart {
    /// Prefer [`express_part!`](crate::express_part).
    pub fn express(parts: Vec<Part>) -> MessagePart {
        MessagePart { parts }
    }

    /// The text without a skin, for tests and debugging ([`Part::plain_text`]).
    pub fn plain_text(&self) -> String {
        self.parts.iter().map(Part::plain_text).collect()
    }

    /// Prefer [`express_part_if!`](crate::express_part_if). The arguments are evaluated either
    /// way.
    pub fn express_if(condition: bool, parts: Vec<Part>) -> MessagePart {
        MessagePart { parts: if condition { parts } else { Vec::new() } }
    }
}

// Each argument of the macros is one part. `@params` spreads an iterable into one part per
// item instead: `express!(@params items)`.

/// A message for everyone: `express!(Faction::Atreides, " pays ", amount)`, or
/// `express!(@params items)` to spread one iterable into separate elements.
#[macro_export]
macro_rules! express {
    (@params $arr:expr) => {
        $crate::messages::message::Message::express($crate::messages::message::params($arr))
    };
    ($($e:expr),* $(,)?) => {
        $crate::messages::message::Message::express(vec![$($crate::messages::part::Part::from($e)),*])
    };
}

/// `Message.ExpressTo(to, ...)`: `express_to!(Faction::Atreides; "You win")`.
#[macro_export]
macro_rules! express_to {
    ($to:expr; @params $arr:expr) => {
        $crate::messages::message::Message::express_to($to, $crate::messages::message::params($arr))
    };
    ($to:expr; $($e:expr),* $(,)?) => {
        $crate::messages::message::Message::express_to($to, vec![$($crate::messages::part::Part::from($e)),*])
    };
}

#[macro_export]
macro_rules! express_part {
    (@params $arr:expr) => {
        $crate::messages::message::MessagePart::express($crate::messages::message::params($arr))
    };
    ($($e:expr),* $(,)?) => {
        $crate::messages::message::MessagePart::express(vec![$($crate::messages::part::Part::from($e)),*])
    };
}

/// `MessagePart.ExpressIf(condition, ...)`: `express_part_if!(x > 0; " and ", x)`.
#[macro_export]
macro_rules! express_part_if {
    ($cond:expr; @params $arr:expr) => {
        $crate::messages::message::MessagePart::express_if($cond, $crate::messages::message::params($arr))
    };
    ($cond:expr; $($e:expr),* $(,)?) => {
        $crate::messages::message::MessagePart::express_if($cond, vec![$($crate::messages::part::Part::from($e)),*])
    };
}

/// One part per item (the `@params` form of the macros).
pub fn params<T: Into<Part>>(array: impl IntoIterator<Item = T>) -> Vec<Part> {
    array.into_iter().map(Into::into).collect()
}

/// Where forces went in a message.
#[derive(Clone, Debug, PartialEq)]
pub struct Route {
    pub faction: Faction,
    pub from: Vec<Location>,
    pub to: Option<Location>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BattlePlans {
    pub aggressor: PlanSummary,
    pub defender: PlanSummary,
}

/// `record PlanSummary(...)`: one side's revealed plan. Equality is record
/// equality: `EqualityComparer<float>.Default` for `dial`, where NaN equals NaN.
#[derive(Clone, Debug)]
pub struct PlanSummary {
    pub faction: Faction,
    pub leader: Option<Hero>,
    pub messiah: bool,
    pub dial: f32,
    pub spice: Option<i32>,
    pub ally_contribution: i32,
    pub ally: Faction,
    pub weapon: Option<TreacheryCard>,
    pub defense: Option<TreacheryCard>,
}

impl PartialEq for PlanSummary {
    fn eq(&self, other: &Self) -> bool {
        self.faction == other.faction
            && self.leader == other.leader
            && self.messiah == other.messiah
            && self.dial == other.dial
            && self.spice == other.spice
            && self.ally_contribution == other.ally_contribution
            && self.ally == other.ally
            && self.weapon == other.weapon
            && self.defense == other.defense
    }
}

game_enum! {
    /// What a [`BattleFact`] records.
    closed BattleFactKind {
        Strength = 0,
        Winner = 1,
        Killed = 2,
        Traitor = 3,
        ForcesLost = 4,
        Discard = 5,
    }
    get_values [Strength, Winner, Killed, Traitor, ForcesLost, Discard]
}

/// `record BattleFact(BattleFactKind Kind, Faction Faction) { ... }`. Only the
/// fields its kind describes are set; the rest keep their defaults. Equality
/// is record equality: NaN `strength` equals NaN, `cause` uses `object.Equals`.
#[derive(Clone, Debug)]
pub struct BattleFact {
    pub kind: BattleFactKind,
    pub faction: Faction,
    pub strength: f32,
    pub hero: Option<Hero>,
    pub cause: Option<Part>,
    pub value: i32,
    pub forces: i32,
    pub special_forces: i32,
    pub all: bool,
    pub card: Option<TreacheryCard>,
}

impl PartialEq for BattleFact {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.faction == other.faction
            && self.strength == other.strength
            && self.hero == other.hero
            && self.cause == other.cause
            && self.value == other.value
            && self.forces == other.forces
            && self.special_forces == other.special_forces
            && self.all == other.all
            && self.card == other.card
    }
}

impl BattleFact {
    /// `new BattleFact(kind, faction)` with every init-only property at its default.
    pub fn new(kind: BattleFactKind, faction: Faction) -> Self {
        BattleFact {
            kind,
            faction,
            strength: 0.0,
            hero: None,
            cause: None,
            value: 0,
            forces: 0,
            special_forces: 0,
            all: false,
            card: None,
        }
    }
}

/// Messages are plain data that can move between threads.
const _: fn() = || {
    fn plain_data<T: Clone + PartialEq + Send + Sync + 'static>() {}
    plain_data::<Message>();
};
