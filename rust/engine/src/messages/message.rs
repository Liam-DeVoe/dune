//! `Message`, `MessagePart`, and the records the client uses to draw messages
//! (`Route`, `BattlePlans`, `PlanSummary`, `BattleFact`).
//!
//! Build messages with the macros, which mirror the C# static methods:
//!
//! | C#                                       | Rust                                    |
//! |------------------------------------------|-----------------------------------------|
//! | `Message.Express(a, b, c)`               | `express!(a, b, c)`                     |
//! | `Message.ExpressTo(f, a, b)`             | `express_to!(f; a, b)`                  |
//! | `MessagePart.Express(a, b)`              | `express_part!(a, b)`                   |
//! | `MessagePart.ExpressIf(cond, a, b)`      | `express_part_if!(cond; a, b)`          |
//!
//! Every argument goes through `ExpressionElement::from`, see expression.rs.

use std::any::Any;
use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

use dotnet_compat::math::equals_f32;

use crate::compat::cs::Shared;
use crate::enumerations::{Concept, Faction};
use crate::messages::expression::{Expression, ExpressionElement};
use crate::heroes::hero::Hero;
use crate::map::location::Location;
use crate::compat::net_enum::net_enum;
use crate::cards::treachery_card::TreacheryCard;

thread_local! {
    /// `Message.Counter` (a private static in C#). Thread-local so that tests
    /// running in parallel do not disturb each other; see [`Message::reset_counter`].
    static COUNTER: Cell<i32> = const { Cell::new(0) };
}

/// Structured log data attached with `WithLog` (the records in
/// `Shared/Model/Log/`). Any `Debug` type qualifies.
pub trait LogRecord: fmt::Debug + Any + crate::canon::Canon {
    fn as_any(&self) -> &dyn Any;
}

impl<T: fmt::Debug + Any + crate::canon::Canon> LogRecord for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Clone, canon_derive::Canon)]
pub struct Message {
    /// `Nr`: the value of the process-wide counter when the message was created.
    /// Left out of state dumps (trace-format.md section 8).
    #[canon(omit)]
    pub nr: i32,
    pub target: Faction,
    pub expression: Expression,
    pub route: Option<Route>,
    pub heading: Option<Concept>,
    pub plans: Option<BattlePlans>,
    /// `Log` (`object?` in C#). Shared, like the C# reference.
    pub log: Option<Rc<dyn LogRecord>>,
    pub fact: Option<BattleFact>,
}

impl Message {
    fn with_expression(target: Faction, e: Expression) -> Message {
        let nr = COUNTER.with(|c| {
            let v = c.get();
            c.set(v.wrapping_add(1));
            v
        });
        Message { nr, target, expression: e, route: None, heading: None, plans: None, log: None, fact: None }
    }

    /// `Message.Express(params object?[] list)`. Prefer [`express!`](crate::express).
    pub fn express(list: Vec<ExpressionElement>) -> Message {
        Self::with_expression(Faction::None, Expression::new(list))
    }

    /// `Message.ExpressTo(to, params object?[] list)`. Prefer [`express_to!`](crate::express_to).
    pub fn express_to(to: Faction, list: Vec<ExpressionElement>) -> Message {
        Self::with_expression(to, Expression::new(list))
    }

    /// Sets `Message.Counter`. The C# counter is never reset, so a replay
    /// must start from the value the C# process had.
    pub fn reset_counter(value: i32) {
        COUNTER.with(|c| c.set(value));
    }

    /// Current value of `Message.Counter`.
    pub fn counter() -> i32 {
        COUNTER.with(|c| c.get())
    }

    /// `WithRoute(faction, from, to)`
    pub fn with_route(mut self, faction: Faction, from: impl IntoIterator<Item = Location>, to: Option<Location>) -> Message {
        let to = to.and_then(crate::compat::cs::CsNull::opt);
        self.route = Some(Route { faction, from: Shared::new(from.into_iter().collect()), to });
        self
    }

    /// `AsHeading(icon)`
    pub fn as_heading(mut self, icon: Concept) -> Message {
        self.heading = Some(icon);
        self
    }

    /// `WithPlans(aggressor, defender)`
    pub fn with_plans(mut self, aggressor: PlanSummary, defender: PlanSummary) -> Message {
        self.plans = Some(BattlePlans { aggressor, defender });
        self
    }

    /// `WithLog(data)`
    pub fn with_log(mut self, data: impl LogRecord) -> Message {
        self.log = Some(Rc::new(data));
        self
    }

    /// `WithFact(fact)`
    pub fn with_fact(mut self, fact: BattleFact) -> Message {
        self.fact = Some(fact);
        self
    }

    /// `ToString()` without a describer: `string.Join("", Expression.Elements)`.
    pub fn to_cs_string(&self) -> String {
        self.expression.elements.iter().map(|e| e.to_cs_string()).collect()
    }
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Message")
            .field("nr", &self.nr)
            .field("target", &self.target)
            .field("expression", &self.expression)
            .field("route", &self.route)
            .field("heading", &self.heading)
            .field("plans", &self.plans)
            .field("log", &self.log)
            .field("fact", &self.fact)
            .finish()
    }
}

/// Field-wise equality; `log` compares by reference.
impl PartialEq for Message {
    fn eq(&self, other: &Self) -> bool {
        self.nr == other.nr
            && self.target == other.target
            && self.expression == other.expression
            && self.route == other.route
            && self.heading == other.heading
            && self.plans == other.plans
            && match (&self.log, &other.log) {
                (None, None) => true,
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                _ => false,
            }
            && self.fact == other.fact
    }
}

/// `MessagePart`
#[derive(Clone, Debug, PartialEq, canon_derive::Canon)]
pub struct MessagePart {
    pub expression: Expression,
}

impl MessagePart {
    /// `MessagePart.Express(params object[] list)`. Prefer [`express_part!`](crate::express_part).
    pub fn express(list: Vec<ExpressionElement>) -> MessagePart {
        MessagePart { expression: Expression::new(list) }
    }

    /// `MessagePart.ExpressIf(condition, params object?[] list)`. The
    /// arguments are evaluated either way, as in C#.
    pub fn express_if(condition: bool, list: Vec<ExpressionElement>) -> MessagePart {
        if condition {
            MessagePart { expression: Expression::new(list) }
        } else {
            MessagePart { expression: Expression::new(Vec::new()) }
        }
    }

    pub fn to_cs_string(&self) -> String {
        self.expression.elements.iter().map(|e| e.to_cs_string()).collect()
    }
}

// `params object?[]`: in C#, a single argument whose static type converts to
// `object?[]` IS the parameter array. That is `object[]` and, through array
// covariance, every reference-type array (`string[]`, `Location[]`). Its items
// become the elements. Anything else, including a value-type array
// (`Faction[]`, `int[]`), a `List<T>` or an array cast to `object`, is one
// element. Checked against .NET in tests/message_corpus.rs.
//
// The macros take each argument as one element. For the spread form write
// `@params`, followed by anything iterable whose items convert to elements:
//
//     Message.Express(objects)        express!(@params objects)
//     Message.Express(objects, "x")   express!(objects, "x")    (the array is one element)

/// `Message.Express(...)`: `express!(Faction::Atreides, " pays ", amount)`, or
/// `express!(@params array)` when C# passes one reference-type array.
#[macro_export]
macro_rules! express {
    (@params $arr:expr) => {
        $crate::messages::message::Message::express($crate::messages::message::params($arr))
    };
    ($($e:expr),* $(,)?) => {
        $crate::messages::message::Message::express(vec![$($crate::messages::expression::ExpressionElement::from($e)),*])
    };
}

/// `Message.ExpressTo(to, ...)`: `express_to!(Faction::Atreides; "You win")`.
#[macro_export]
macro_rules! express_to {
    ($to:expr; @params $arr:expr) => {
        $crate::messages::message::Message::express_to($to, $crate::messages::message::params($arr))
    };
    ($to:expr; $($e:expr),* $(,)?) => {
        $crate::messages::message::Message::express_to($to, vec![$($crate::messages::expression::ExpressionElement::from($e)),*])
    };
}

/// `MessagePart.Express(...)`
#[macro_export]
macro_rules! express_part {
    (@params $arr:expr) => {
        $crate::messages::message::MessagePart::express($crate::messages::message::params($arr))
    };
    ($($e:expr),* $(,)?) => {
        $crate::messages::message::MessagePart::express(vec![$($crate::messages::expression::ExpressionElement::from($e)),*])
    };
}

/// `MessagePart.ExpressIf(condition, ...)`: `express_part_if!(x > 0; " and ", x)`.
#[macro_export]
macro_rules! express_part_if {
    ($cond:expr; @params $arr:expr) => {
        $crate::messages::message::MessagePart::express_if($cond, $crate::messages::message::params($arr))
    };
    ($cond:expr; $($e:expr),* $(,)?) => {
        $crate::messages::message::MessagePart::express_if($cond, vec![$($crate::messages::expression::ExpressionElement::from($e)),*])
    };
}

/// The element list of a reference-type array passed as the whole
/// `params object?[]` argument (the `@params` form of the macros).
pub fn params<T: Into<ExpressionElement>>(array: impl IntoIterator<Item = T>) -> Vec<ExpressionElement> {
    array.into_iter().map(Into::into).collect()
}

/// `record Route(Faction Faction, List<Location> From, Location? To)`: where
/// forces went in a message.
///
/// Record equality compares `From` with `List<T>.Equals`, which is reference
/// equality, so `from` is a [`Shared`] list: two routes built from separate
/// lists are different even when the lists hold the same locations.
#[derive(Clone, Debug, PartialEq, canon_derive::Canon)]
pub struct Route {
    pub faction: Faction,
    pub from: Shared<Vec<Location>>,
    pub to: Option<Location>,
}

/// `record BattlePlans(PlanSummary Aggressor, PlanSummary Defender)`
#[derive(Clone, Debug, PartialEq, canon_derive::Canon)]
pub struct BattlePlans {
    pub aggressor: PlanSummary,
    pub defender: PlanSummary,
}

/// `record PlanSummary(...)`: one side's revealed plan. Equality is record
/// equality: `EqualityComparer<float>.Default` for `dial`, where NaN equals NaN.
#[derive(Clone, Debug, canon_derive::Canon)]
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
            && equals_f32(self.dial, other.dial)
            && self.spice == other.spice
            && self.ally_contribution == other.ally_contribution
            && self.ally == other.ally
            && self.weapon == other.weapon
            && self.defense == other.defense
    }
}

net_enum! {
    /// C# `BattleFactKind` (declared in Message.cs)
    BattleFactKind {
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
#[derive(Clone, Debug, canon_derive::Canon)]
pub struct BattleFact {
    pub kind: BattleFactKind,
    pub faction: Faction,
    pub strength: f32,
    pub hero: Option<Hero>,
    /// `object? Cause`
    pub cause: Option<ExpressionElement>,
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
            && equals_f32(self.strength, other.strength)
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
