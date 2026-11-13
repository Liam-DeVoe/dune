//! The log: report headers, report lines, and the structured records attached to messages.
//!
//! A C# message can carry, besides its expression, the data the report's designs draw from:
//! `Message.Log` (a record of `Shared/Model/Log/`), `Route`, `Plans` and `Fact`. [`LogLine`]
//! carries each as JSON in the shape of the C# record: C# property names in camelCase, enum
//! values by member name, and game objects (locations, territories, heroes, cards, and the
//! `object` fields) as rich-text [`Part`]s, so the client can draw them as `ExpressionComponent`
//! would and knows their names. The C# reference serialises the same records by reflection
//! (`rust/view/csharp-ref/LogLines.cs`), and `tests/log_lines.rs` compares the two.

use serde::{Deserialize, Serialize};
use treachery_engine::messages::expression::ExpressionElement;
use treachery_engine::heroes::hero::Hero;
use treachery_engine::log as l;
use treachery_engine::messages::message::{self as m, Message};
use treachery_engine::prelude::*;
use treachery_engine::messages::report::Report;
use ts_rs::TS;

use crate::build::ResolvedViewer;
use crate::describe::Describer;
use crate::expr;
use crate::schema::{LogLine, LogView, Part, ReportHeader, ReportLines, RichText};

fn name<T: ToString>(v: T) -> String {
    v.to_string()
}

/// `ReportComponent.razor`: every report header, and the lines of the latest phase, filtered by
/// `m.Target == Faction.None || m.Target == Client.Faction`. `d` should hold the game
/// ([`crate::describe::InGame`]): old messages show the game's current contents of the
/// collections they hold, as in the C# client.
pub fn log_view(game: &Game, v: &ResolvedViewer, d: &dyn Describer) -> LogView {
    let reports = game.reports();
    let headers: Vec<ReportHeader> = reports
        .iter()
        .enumerate()
        .map(|(i, r)| ReportHeader {
            index: i as i32,
            turn: r.turn,
            about: name(r.about),
            is_battle: r.territory.is_some(),
            aggressor: name(r.aggressor),
            defender: name(r.defender),
            territory: r.territory.map(|t| t.id()),
            battle_decided: r.battle_decided,
            battle_winner: name(r.battle_winner),
        })
        .collect();
    let latest = match reports.last() {
        Some(last) => reports
            .iter()
            .enumerate()
            .filter(|(_, r)| r.turn == last.turn && r.about == last.about)
            .map(|(i, r)| report_lines(i, r, v, d))
            .collect(),
        None => vec![],
    };
    LogView { reports: headers, latest }
}

/// The lines of one report that the viewer may read (`RequestReport`). See [`log_view`] for `d`.
pub fn report_lines(index: usize, r: &Report, v: &ResolvedViewer, d: &dyn Describer) -> ReportLines {
    let lines = r.messages.iter().filter(|m| m.target == Faction::None || Some(m.target) == v.faction).map(|m| log_line(m, d)).collect();
    ReportLines { index: index as i32, lines }
}

/// One message as the log draws it.
pub fn log_line(m: &Message, d: &dyn Describer) -> LogLine {
    LogLine {
        text: expr::message(m, d),
        private: m.target != Faction::None,
        heading: m.heading.map(name),
        record: m.log.as_deref().map(|r| record(r, d).unwrap_or_else(|| panic!("log record not converted: {r:?}"))),
        route: m.route.as_ref().map(|r| route(r, d)),
        plans: m.plans.as_ref().map(|p| BattlePlans { aggressor: plan(&p.aggressor, d), defender: plan(&p.defender, d) }),
        fact: m.fact.as_ref().map(|f| fact(f, d)),
    }
}

// ---------------------------------------------------------------------------------------------
// Records
// ---------------------------------------------------------------------------------------------

/// `Message.Log`: one record of `Shared/Model/Log/`, tagged by its C# type name. The comments
/// are the C# ones, shortened; the C# files say more.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all_fields = "camelCase")]
pub enum LogRecord {
    // AsideLog.cs
    /// A message that can happen in any phase, drawn as an aside.
    AsideLog {
        #[ts(type = "AsideKind")]
        kind: String,
    },

    // BiddingLog.cs
    /// "N cards were drawn for bidding".
    BiddingDrawn { count: i32 },
    /// The treachery discard pile shuffled into a new deck.
    TreacheryReshuffled { count: i32 },
    /// No faction can buy cards, so there is no auction.
    BiddingSkipped {},
    /// A card sold. `paidTo` is `None` when the spice went to the bank; with Karama the card was
    /// won for free by playing `karamaCard`.
    CardSold {
        card_number: i32,
        #[ts(type = "Faction")]
        winner: String,
        price: i32,
        #[ts(type = "Faction")]
        paid_to: String,
        shares: Vec<BidShare>,
        karama_card: Option<Part>,
    },
    /// Private to the winner: the card they bought.
    CardWonShown { card: Part },
    /// Harkonnen's free extra card.
    ExtraCardReceived {
        #[ts(type = "Faction")]
        receiver: String,
    },
    /// Private: which card the extra card is.
    ExtraCardShown { card: Part },
    /// Everyone passed on `cardNumber`; bidding ends.
    BiddingPassedByAll { card_number: i32 },

    // BlowLog.cs
    /// The heading over one spice deck's lines ("A" or "B").
    BlowDeckLog { deck: String },
    /// A spice card came up: `amount` spice in `location`, or lost to the storm.
    BlowCardLog { card: Part, location: Part, amount: i32, lost: bool, doubled: bool },
    /// Shai Hulud appears in `territory`; `nth` counts its appearances in this deck.
    WormAppearedLog { territory: Part, nth: i32 },
    /// A faction sends a later worm to `territory`.
    WormSentLog {
        #[ts(type = "Faction")]
        by: String,
        territory: Part,
    },
    /// Shai Hulud devours a faction's forces in `location`.
    WormDevouredForcesLog {
        #[ts(type = "Faction")]
        faction: String,
        forces: i32,
        special_forces: i32,
        location: Part,
    },
    /// A faction's forces in `location` survive Shai Hulud.
    WormSurvivedLog {
        #[ts(type = "Faction")]
        faction: String,
        location: Part,
    },
    /// Shai Hulud devours the spice in `territory`.
    WormDevouredSpiceLog { amount: i32, territory: Part },
    /// Shai Hulud came up on turn 1 and was ignored.
    WormIgnoredLog {},
    /// The spice discard pile was shuffled into a new deck.
    BlowReshuffleLog { cards: i32 },
    /// A line of the Nexus that follows a worm.
    NexusLog { ride: Option<NexusRide>, alliance: Option<NexusAlliance> },

    // CharityLog.cs
    /// A faction claiming CHOAM Charity.
    CharityClaim {
        #[ts(type = "Faction")]
        faction: String,
        amount: i32,
    },

    // CollectionLog.cs
    /// A faction collecting spice. `field` is null for a divided collection.
    SpiceCollected {
        #[ts(type = "Faction")]
        faction: String,
        territory: Part,
        amount: i32,
        field: Option<Part>,
        income: bool,
    },

    // MentatLog.cs
    /// A faction moving the bribes it received into its reserves.
    BribesCollected {
        #[ts(type = "Faction")]
        faction: String,
        amount: i32,
    },
    /// A line saying why the game was won.
    WinReason {
        #[ts(type = "WinMethod")]
        method: String,
    },
    /// "The game has ended.", with the factions that won.
    GameEnded {
        #[ts(type = "Array<Faction>")]
        winners: Vec<String>,
    },
    /// One "[faction] win!" line.
    GameWinner {
        #[ts(type = "Faction")]
        faction: String,
    },

    // RevivalLog.cs
    /// One faction reviving: what came back and what it cost.
    RevivalRow {
        #[ts(type = "Faction")]
        faction: String,
        forces: i32,
        special_forces: i32,
        cost: i32,
        hero: Option<Part>,
        as_ghola: bool,
        paid_by_other: i32,
        #[ts(type = "Faction")]
        payer: String,
        card: Option<Part>,
        tleilaxu_income: i32,
    },

    // ShipMoveLog.cs
    /// A faction's shipment, or its pass.
    ShipLogged {
        #[ts(type = "Faction")]
        faction: String,
        #[ts(type = "ShipKind")]
        kind: String,
        forces: i32,
        special_forces: i32,
        from: Option<Part>,
        to: Option<Part>,
        cost: i32,
        ally_share: i32,
        #[ts(type = "Faction")]
        ally: String,
        guild_paid: bool,
        karama: Option<Part>,
        blocked: bool,
        blocked_by: Option<Part>,
        plain: bool,
    },
    /// Forces moving from one territory to `to`; one per source territory, sharing `move`.
    MoveLogged {
        #[ts(type = "Faction")]
        faction: String,
        #[serde(rename = "move")]
        move_: i32,
        from: Part,
        forces: i32,
        special_forces: i32,
        to: Part,
        hajr: bool,
        as_advisors: Option<bool>,
    },
    /// A faction passed their move.
    MovePassed {
        #[ts(type = "Faction")]
        faction: String,
    },
    /// The Bene Gesserit answering another faction's shipment or move. `place` is the C#
    /// `object` (a location or a territory).
    BeneGesseritReaction {
        #[ts(type = "BeneGesseritReactionKind")]
        kind: String,
        place: Option<RichText>,
        advisors: bool,
    },
    /// Forces of a faction lost during their turn.
    ShipMoveLoss {
        #[ts(type = "Faction")]
        faction: String,
        storm: bool,
        travelling: bool,
        forces: i32,
        territory: Option<Part>,
    },
    /// The Guild delayed their turn.
    GuildDelayed {},

    // StormLog.cs
    /// The two storm dials, in the order they were logged.
    StormDials {
        #[ts(type = "Faction")]
        first: String,
        first_dial: i32,
        #[ts(type = "Faction")]
        second: String,
        second_dial: i32,
    },
    /// The first storm landing in a sector (turn 1).
    StormPlaced { sector: i32 },
    /// The distance the storm will move.
    StormForecast { distance: i32 },
    /// The storm moving `distance` sectors; `by` and `card` for Weather Control.
    StormMoved {
        from: i32,
        distance: i32,
        #[ts(type = "Faction")]
        by: String,
        card: Option<Part>,
    },
    /// Forces the storm killed in a location.
    StormKilled {
        #[ts(type = "Faction")]
        faction: String,
        location: Part,
        forces: i32,
        special_forces: i32,
    },
    /// Spice the storm destroyed in a location.
    StormDestroyedSpice { location: Part, amount: i32 },
    /// A player using Family Atomics on the Shield Wall.
    FamilyAtomicsUsed {
        #[ts(type = "Faction")]
        faction: String,
        card: Option<Part>,
    },
    /// Forces Family Atomics killed on the Shield Wall.
    FamilyAtomicsKilled {
        #[ts(type = "Faction")]
        faction: String,
        forces: i32,
        special_forces: i32,
    },
    /// Bene Gesserit advisors flipping to fighters because they are alone.
    BeneGesseritFlipped { territory: Part },
}

/// The C# type names [`LogRecord`] covers. `tests/log_lines.rs` checks this against the
/// records declared in `Shared/Model/Log/`.
pub const RECORD_TYPES: &[&str] = &[
    "AsideLog",
    "BiddingDrawn",
    "TreacheryReshuffled",
    "BiddingSkipped",
    "CardSold",
    "CardWonShown",
    "ExtraCardReceived",
    "ExtraCardShown",
    "BiddingPassedByAll",
    "BlowDeckLog",
    "BlowCardLog",
    "WormAppearedLog",
    "WormSentLog",
    "WormDevouredForcesLog",
    "WormSurvivedLog",
    "WormDevouredSpiceLog",
    "WormIgnoredLog",
    "BlowReshuffleLog",
    "NexusLog",
    "CharityClaim",
    "SpiceCollected",
    "BribesCollected",
    "WinReason",
    "GameEnded",
    "GameWinner",
    "RevivalRow",
    "ShipLogged",
    "MoveLogged",
    "MovePassed",
    "BeneGesseritReaction",
    "ShipMoveLoss",
    "GuildDelayed",
    "StormDials",
    "StormPlaced",
    "StormForecast",
    "StormMoved",
    "StormKilled",
    "StormDestroyedSpice",
    "FamilyAtomicsUsed",
    "FamilyAtomicsKilled",
    "BeneGesseritFlipped",
];

/// The records that only occur inside other records.
pub const NESTED_RECORD_TYPES: &[&str] = &["BidShare", "NexusAlliance", "NexusRide"];

/// Part of a card's price paid by someone other than the winner.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BidShare {
    #[ts(type = "Faction")]
    pub by: String,
    pub amount: i32,
}

/// `Offered`: `from` offers an alliance to `to`. `Made`: they are now allies. `Ended`: their
/// alliance is over.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NexusAlliance {
    #[ts(type = "NexusAllianceChange")]
    pub change: String,
    #[ts(type = "Faction")]
    pub from: String,
    #[ts(type = "Faction")]
    pub to: String,
}

/// Forces ridden on the worm, from `from` (null: from reserves) to `to`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NexusRide {
    #[ts(type = "Faction")]
    pub faction: String,
    pub forces: i32,
    pub special_forces: i32,
    pub from: Option<Part>,
    pub to: Part,
}

/// `Message.Route`: forces travelling from `from` (empty: from reserves) to `to` (null: back
/// to reserves).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Route {
    #[ts(type = "Faction")]
    pub faction: String,
    pub from: Vec<Part>,
    pub to: Option<Part>,
}

/// `Message.Plans`: both revealed battle plans, aggressor first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BattlePlans {
    pub aggressor: PlanSummary,
    pub defender: PlanSummary,
}

/// One side's revealed plan. `spice` is null without advanced combat; `allyContribution` is
/// spice paid by `ally`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlanSummary {
    #[ts(type = "Faction")]
    pub faction: String,
    pub leader: Option<Part>,
    pub messiah: bool,
    pub dial: f32,
    pub spice: Option<i32>,
    pub ally_contribution: i32,
    #[ts(type = "Faction")]
    pub ally: String,
    pub weapon: Option<Part>,
    pub defense: Option<Part>,
}

/// `Message.Fact`: one fact of a battle's outcome. Only the fields its kind describes are set
/// (see `BattleFactKind` in `Shared/Model/Message.cs`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BattleFact {
    #[ts(type = "BattleFactKind")]
    pub kind: String,
    #[ts(type = "Faction")]
    pub faction: String,
    pub strength: f32,
    pub hero: Option<Part>,
    /// The C# `object` field: what killed the hero (a card, or the text "Treachery").
    pub cause: Option<RichText>,
    pub value: i32,
    pub forces: i32,
    pub special_forces: i32,
    pub all: bool,
    pub card: Option<Part>,
}

fn location(v: Location, d: &dyn Describer) -> Part {
    expr::part(&ExpressionElement::Location(v), d).expect("location")
}

fn territory(v: Territory, d: &dyn Describer) -> Part {
    expr::part(&ExpressionElement::Territory(v), d).expect("territory")
}

fn hero(v: Hero, d: &dyn Describer) -> Part {
    expr::hero_part(v, d)
}

fn card(v: TreacheryCard, d: &dyn Describer) -> Part {
    hero(Hero::TreacheryCard(v), d)
}

/// A C# `object` field: null, or the element as `ExpressionComponent` draws it.
fn object(v: &ExpressionElement, d: &dyn Describer) -> Option<RichText> {
    if matches!(v, ExpressionElement::Null) {
        return None;
    }
    let mut out = Vec::new();
    expr::element_into(v, d, &mut out);
    Some(out)
}

fn route(r: &m::Route, d: &dyn Describer) -> Route {
    Route { faction: name(r.faction), from: r.from.borrow().iter().map(|x| location(*x, d)).collect(), to: r.to.map(|x| location(x, d)) }
}

fn plan(p: &m::PlanSummary, d: &dyn Describer) -> PlanSummary {
    PlanSummary {
        faction: name(p.faction),
        leader: p.leader.map(|h| hero(h, d)),
        messiah: p.messiah,
        dial: p.dial,
        spice: p.spice,
        ally_contribution: p.ally_contribution,
        ally: name(p.ally),
        weapon: p.weapon.map(|c| card(c, d)),
        defense: p.defense.map(|c| card(c, d)),
    }
}

fn fact(f: &m::BattleFact, d: &dyn Describer) -> BattleFact {
    BattleFact {
        kind: name(f.kind),
        faction: name(f.faction),
        strength: f.strength,
        hero: f.hero.map(|h| hero(h, d)),
        cause: f.cause.as_ref().and_then(|c| object(c, d)),
        value: f.value,
        forces: f.forces,
        special_forces: f.special_forces,
        all: f.all,
        card: f.card.map(|c| card(c, d)),
    }
}

/// Converts `Message.Log`. `None` for a type this module doesn't know, which
/// [`log_line`] reports.
pub fn record(r: &dyn m::LogRecord, d: &dyn Describer) -> Option<LogRecord> {
    use LogRecord as R;
    let any = r.as_any();
    macro_rules! on {
        ($T:ident, |$r:ident| $body:expr) => {
            if let Some($r) = any.downcast_ref::<l::$T>() {
                return Some($body);
            }
        };
    }
    let f = |v: Faction| name(v);
    let loc = |v: Location| location(v, d);
    let ter = |v: Territory| territory(v, d);
    let crd = |v: TreacheryCard| card(v, d);

    on!(AsideLog, |r| R::AsideLog { kind: name(r.kind) });

    on!(BiddingDrawn, |r| R::BiddingDrawn { count: r.count });
    on!(TreacheryReshuffled, |r| R::TreacheryReshuffled { count: r.count });
    on!(BiddingSkipped, |_r| R::BiddingSkipped {});
    on!(CardSold, |r| R::CardSold {
        card_number: r.card_number,
        winner: f(r.winner),
        price: r.price,
        paid_to: f(r.paid_to),
        shares: r.shares.iter().map(|s| BidShare { by: f(s.by), amount: s.amount }).collect(),
        karama_card: r.karama_card.map(crd),
    });
    on!(CardWonShown, |r| R::CardWonShown { card: crd(r.card) });
    on!(ExtraCardReceived, |r| R::ExtraCardReceived { receiver: f(r.receiver) });
    on!(ExtraCardShown, |r| R::ExtraCardShown { card: crd(r.card) });
    on!(BiddingPassedByAll, |r| R::BiddingPassedByAll { card_number: r.card_number });

    on!(BlowDeckLog, |r| R::BlowDeckLog { deck: r.deck.clone() });
    on!(BlowCardLog, |r| R::BlowCardLog {
        card: expr::part(&ExpressionElement::ResourceCard(r.card), d).expect("resource card"),
        location: loc(r.location),
        amount: r.amount,
        lost: r.lost,
        doubled: r.doubled,
    });
    on!(WormAppearedLog, |r| R::WormAppearedLog { territory: ter(r.territory), nth: r.nth });
    on!(WormSentLog, |r| R::WormSentLog { by: f(r.by), territory: ter(r.territory) });
    on!(WormDevouredForcesLog, |r| R::WormDevouredForcesLog {
        faction: f(r.faction),
        forces: r.forces,
        special_forces: r.special_forces,
        location: loc(r.location),
    });
    on!(WormSurvivedLog, |r| R::WormSurvivedLog { faction: f(r.faction), location: loc(r.location) });
    on!(WormDevouredSpiceLog, |r| R::WormDevouredSpiceLog { amount: r.amount, territory: ter(r.territory) });
    on!(WormIgnoredLog, |_r| R::WormIgnoredLog {});
    on!(BlowReshuffleLog, |r| R::BlowReshuffleLog { cards: r.cards });
    on!(NexusLog, |r| R::NexusLog {
        ride: r.ride.as_ref().map(|x| NexusRide {
            faction: f(x.faction),
            forces: x.forces,
            special_forces: x.special_forces,
            from: x.from.map(loc),
            to: loc(x.to),
        }),
        alliance: r.alliance.as_ref().map(|x| NexusAlliance { change: name(x.change), from: f(x.from), to: f(x.to) }),
    });

    on!(CharityClaim, |r| R::CharityClaim { faction: f(r.faction), amount: r.amount });
    on!(SpiceCollected, |r| R::SpiceCollected {
        faction: f(r.faction),
        territory: ter(r.territory),
        amount: r.amount,
        field: r.field.map(loc),
        income: r.income,
    });

    on!(BribesCollected, |r| R::BribesCollected { faction: f(r.faction), amount: r.amount });
    on!(WinReason, |r| R::WinReason { method: name(r.method) });
    on!(GameEnded, |r| R::GameEnded { winners: r.winners.iter().map(|w| f(*w)).collect() });
    on!(GameWinner, |r| R::GameWinner { faction: f(r.faction) });

    on!(RevivalRow, |r| R::RevivalRow {
        faction: f(r.faction),
        forces: r.forces,
        special_forces: r.special_forces,
        cost: r.cost,
        hero: r.hero.map(|h| hero(h, d)),
        as_ghola: r.as_ghola,
        paid_by_other: r.paid_by_other,
        payer: f(r.payer),
        card: r.card.map(crd),
        tleilaxu_income: r.tleilaxu_income,
    });

    on!(ShipLogged, |r| R::ShipLogged {
        faction: f(r.faction),
        kind: name(r.kind),
        forces: r.forces,
        special_forces: r.special_forces,
        from: r.from.map(loc),
        to: r.to.map(loc),
        cost: r.cost,
        ally_share: r.ally_share,
        ally: f(r.ally),
        guild_paid: r.guild_paid,
        karama: r.karama.map(crd),
        blocked: r.blocked,
        blocked_by: r.blocked_by.map(crd),
        plain: r.plain,
    });
    on!(MoveLogged, |r| R::MoveLogged {
        faction: f(r.faction),
        move_: r.move_,
        from: ter(r.from),
        forces: r.forces,
        special_forces: r.special_forces,
        to: loc(r.to),
        hajr: r.hajr,
        as_advisors: r.as_advisors,
    });
    on!(MovePassed, |r| R::MovePassed { faction: f(r.faction) });
    on!(BeneGesseritReaction, |r| R::BeneGesseritReaction { kind: name(r.kind), place: object(&r.place, d), advisors: r.advisors });
    on!(ShipMoveLoss, |r| R::ShipMoveLoss {
        faction: f(r.faction),
        storm: r.storm,
        travelling: r.travelling,
        forces: r.forces,
        territory: r.territory.map(ter),
    });
    on!(GuildDelayed, |_r| R::GuildDelayed {});

    on!(StormDials, |r| R::StormDials { first: f(r.first), first_dial: r.first_dial, second: f(r.second), second_dial: r.second_dial });
    on!(StormPlaced, |r| R::StormPlaced { sector: r.sector });
    on!(StormForecast, |r| R::StormForecast { distance: r.distance });
    on!(StormMoved, |r| R::StormMoved { from: r.from, distance: r.distance, by: f(r.by), card: r.card.map(crd) });
    on!(StormKilled, |r| R::StormKilled {
        faction: f(r.faction),
        location: loc(r.location),
        forces: r.forces,
        special_forces: r.special_forces,
    });
    on!(StormDestroyedSpice, |r| R::StormDestroyedSpice { location: loc(r.location), amount: r.amount });
    on!(FamilyAtomicsUsed, |r| R::FamilyAtomicsUsed { faction: f(r.faction), card: r.card.map(crd) });
    on!(FamilyAtomicsKilled, |r| R::FamilyAtomicsKilled { faction: f(r.faction), forces: r.forces, special_forces: r.special_forces });
    on!(BeneGesseritFlipped, |r| R::BeneGesseritFlipped { territory: ter(r.territory) });

    None
}
