//! The end of the game: the winners and the story of the game, as the `ending` view field.
//!
//! The story replays the whole history in a fresh game and records after each event what the
//! story shows. The client draws the blocks in order (`web/src/components/StoryComponent.tsx`).

use serde::{Deserialize, Serialize};
use treachery_engine::events::Event;
use treachery_engine::messages::part::Part as P;
use treachery_engine::heroes::hero::Hero;
use treachery_engine::identity::Identifiable;
use treachery_engine::prelude::*;
use ts_rs::TS;

use crate::build::Seating;
use crate::describe::{Describer, InGame};
use crate::schema::{HeroRef, RichText};
use crate::shield::hero_ref;

fn name<T: ToString>(v: T) -> String {
    v.to_string()
}

/// In `Phase::GameEnded`: the winners, and the story unless the rules disable the report.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EndingView {
    /// `Game.Winners`, each with `Player.Name`.
    pub winners: Vec<StoryWinner>,
    /// The story, or null under `Rule.DisableEndOfGameReport`.
    pub story: Option<Vec<StoryBlock>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StoryWinner {
    #[ts(type = "Faction")]
    pub faction: String,
    pub name: String,
}

/// One block of the story, in the order the story shows them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StoryBlock {
    /// When faction selection ends: "Setup", the ruleset, custom rules, start time, players.
    #[serde(rename_all = "camelCase")]
    Setup {
        /// `Skin.Describe(Ruleset)`.
        ruleset: String,
        /// `Skin.Join(GetCustomRules())`, when there are any.
        custom_rules: Option<String>,
        /// The event's time as `yyyy-MM-dd HH:mm:ss "UTC"`.
        started: String,
        table: StoryTable,
    },
    /// Bene Gesserit's prediction.
    #[serde(rename_all = "camelCase")]
    Prediction {
        #[ts(type = "Faction")]
        to_win: String,
        #[ts(type = "number")]
        turn: treachery_engine::game::Turn,
    },
    /// A card bought: "<buyer> buy <card> for <amount> [using <karma>] [(<auction type>)]".
    #[serde(rename_all = "camelCase")]
    Purchase {
        #[ts(type = "Faction")]
        buyer: String,
        /// `CardJustWon`.
        card: Option<StoryCard>,
        amount: i32,
        karma_card: Option<StoryCard>,
        /// `Skin.DescribeDetailed(CurrentAuctionType)` when not a normal auction.
        auction: Option<String>,
    },
    /// "Turn N", when bidding starts.
    Turn {
        #[ts(type = "number")]
        turn: treachery_engine::game::Turn,
    },
    /// A "Situation at ..." heading with its table. `turn` is
    /// the current turn, which the end-of-turn heading names.
    Situation {
        when: SituationWhen,
        #[ts(type = "number")]
        turn: treachery_engine::game::Turn,
        table: StoryTable,
    },
    /// "Battle for <territory>".
    BattleFor { territory: i32 },
    /// A milestone event: its initiator's image and `GetMessage()`.
    #[serde(rename_all = "camelCase")]
    Milestone {
        #[ts(type = "Faction")]
        faction: String,
        text: RichText,
    },
    /// The battle plans of a battle report.
    #[serde(rename_all = "camelCase")]
    Battle { aggressor: StoryPlan, defender: StoryPlan, advanced_combat: bool },
    /// At the end of a turn, after its situation: the strongholds table.
    #[serde(rename_all = "camelCase")]
    Strongholds { owner_column: bool, terror_column: bool, rows: Vec<StoryStronghold> },
    /// "<faction> <name> wins <method>!"
    #[serde(rename_all = "camelCase")]
    Winner {
        #[ts(type = "Faction")]
        faction: String,
        name: String,
        method: String,
    },
    /// "Game ended: <time>".
    Ended { time: String },
}

/// Which "Situation at ..." heading a story table has.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum SituationWhen {
    StartOfBidding,
    EndOfBidding,
    EndOfShipmentAndMove,
    EndOfTurn,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StoryCard {
    pub id: i32,
    /// `Skin.Describe(card)`.
    pub label: String,
}

/// A table of players: which columns are shown, and one row per player.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StoryTable {
    pub show_name: bool,
    pub show_ally: bool,
    /// `Rule.NexusCards`: the ally column reads "Ally or Nexus*".
    pub nexus_column: bool,
    pub show_cards: bool,
    pub show_traitors: bool,
    pub show_tech_tokens: bool,
    pub show_resources: bool,
    pub show_forces: bool,
    pub show_leaders: bool,
    pub show_tanks: bool,
    pub show_points: bool,
    pub rows: Vec<StoryRow>,
}

/// One player in a story table. Fields of columns that are not shown are left empty.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StoryRow {
    #[ts(type = "Faction")]
    pub faction: String,
    pub name: String,
    #[ts(type = "Faction | null")]
    pub ally: Option<String>,
    /// The Nexus card held, when there is no ally.
    #[ts(type = "Faction | null")]
    pub nexus: Option<String>,
    pub cards: Vec<i32>,
    /// Traitors, then discarded traitors (greyed), then face dancers.
    pub traitors: Vec<StoryTraitor>,
    #[ts(type = "Array<TechToken>")]
    pub tech_tokens: Vec<String>,
    /// `Player.Resources`, when the table shows spice.
    pub resources: Option<i32>,
    pub forces: Vec<StoryForces>,
    /// Living leaders.
    pub leaders: Vec<HeroRef>,
    /// Atreides with 7 forces killed in battle and the Messiah alive.
    pub messiah_available: bool,
    pub forces_killed: i32,
    pub special_forces_killed: i32,
    pub dead_leaders: Vec<HeroRef>,
    /// Atreides, under `AtreidesMessiah`, with the Messiah dead.
    pub messiah_dead: bool,
    pub points: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StoryTraitor {
    pub hero: HeroRef,
    pub discarded: bool,
}

/// Forces in one location, drawn with the territory's place chip.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StoryForces {
    pub territory: i32,
    /// Occupied by the player and by another faction (not the Polar Sink): drawn red with "! ".
    pub contested: bool,
    pub forces: i32,
    pub special_forces: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StoryPlan {
    #[ts(type = "Faction")]
    pub faction: String,
    pub winner: bool,
    pub hero: Option<HeroRef>,
    pub messiah: bool,
    pub weapon: Option<i32>,
    pub defense: Option<i32>,
    /// The plan's dial against the opponent, printed as a decimal number.
    pub dial: String,
    /// `Battle.Cost(Game)` under advanced combat.
    pub cost: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StoryStronghold {
    pub location: i32,
    pub battalions: Vec<StoryBattalion>,
    #[ts(type = "Faction | null")]
    pub owner: Option<String>,
    #[ts(type = "Ambassador | null")]
    pub ambassador: Option<String>,
    #[ts(type = "Array<TerrorType>")]
    pub terror: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StoryBattalion {
    #[ts(type = "Faction")]
    pub faction: String,
    pub forces: i32,
    pub special_forces: i32,
}

/// The `ending` field: after `GameEnded` only.
pub fn ending(game: &Game, s: &Seating, d: &dyn Describer) -> Option<EndingView> {
    if game.phase() != Phase::GameEnded {
        return None;
    }
    let winners = game.winners().iter().map(|&p| StoryWinner { faction: name_or_absent(game.player(p).faction), name: s.player_name(game, p) }).collect();
    let story = (!game.applicable(Rule::DisableEndOfGameReport)).then(|| story(game, s, d));
    Some(EndingView { winners, story })
}

/// `yyyy-MM-dd HH:mm:ss "UTC"` of `ToUniversalTime()`.
fn utc(t: DateTimeOffset) -> String {
    let s = DateTimeOffset::new(t.utc_ticks(), 0).to_json_string();
    format!("{} {} UTC", &s[0..10], &s[11..19])
}

fn card(c: TreacheryCard, d: &dyn Describer) -> StoryCard {
    StoryCard { id: c.id(), label: d.describe(&P::from(c)) }
}

/// `Skin.DescribeDetailed(AuctionType)`.
fn describe_detailed(t: Option<AuctionType>, d: &dyn Describer) -> String {
    let richese = || d.describe(&P::from(Faction::Richese));
    match t {
        Some(AuctionType::Normal) => "Normal".to_string(),
        Some(AuctionType::BlackMarketNormal) => "Black Market / Normal".to_string(),
        Some(AuctionType::BlackMarketSilent) => "Black Market / Silent".to_string(),
        Some(AuctionType::BlackMarketOnceAround) => "Black Market / Once Around".to_string(),
        Some(AuctionType::RicheseSilent) => format!("{} / Silent", richese()),
        Some(AuctionType::RicheseOnceAround) => format!("{} / Once Around", richese()),
        None => "None".to_string(),
    }
}

/// The events the story shows as milestones.
fn is_milestone(e: &Event) -> bool {
    use Event as E;
    match e {
        E::FaceDanced(e) => e.face_dancer_called,
        E::FaceDancerRevealed(e) => !e.passed,
        E::TreacheryCalled(e) => e.traitor_called,
        E::IxianSwappedCardOnBid(e) => !e.passed,
        E::TerrorRevealed(e) => e.type_ == Some(TerrorType::Atomics),
        E::PoisonToothCancelled(_)
        | E::ReplacedCardWon(_)
        | E::Voice(_)
        | E::Prescience(_)
        | E::Move(_)
        | E::Caravan(_)
        | E::Karma(_)
        | E::Shipment(_)
        | E::KarmaHandSwap(_)
        | E::KarmaFreeRevival(_)
        | E::KarmaMonster(_)
        | E::KarmaHmsMovement(_)
        | E::KarmaPrescience(_)
        | E::MetheorPlayed(_)
        | E::StormSpellPlayed(_)
        | E::AmalPlayed(_)
        | E::ClairVoyancePlayed(_)
        | E::ChoamEconomics(_)
        | E::RaiseDeadPlayed(_)
        | E::PortableAntidoteUsed(_)
        | E::ClairVoyanceAnswered(_) => true,
        _ => false,
    }
}

/// The story: the history, replayed in a fresh game.
fn story(game: &Game, s: &Seating, d: &dyn Describer) -> Vec<StoryBlock> {
    let mut out = Vec::new();
    let mut previous_phase = Phase::AwaitingPlayers;
    let mut previous_main_phase = MainPhase::Started;
    let mut previous_winning_bid: Option<PlacedBid> = None;
    let mut replay = game.new_like();

    for original in &game.history.entries {
        let entry = original.detached();
        let evt = entry.event.clone();
        let _ = replay.play(entry, false, true);
        let r = &replay;
        let rd = InGame::new(d, r);
        let rd: &dyn Describer = &rd;

        if matches!(previous_phase, Phase::TradingFactions | Phase::SelectingFactions)
            && !matches!(r.phase(), Phase::TradingFactions | Phase::SelectingFactions)
        {
            let custom = r.get_custom_rules();
            out.push(StoryBlock::Setup {
                ruleset: rd.describe(&P::from(r.rules.ruleset)),
                custom_rules: (!custom.is_empty())
                    .then(|| custom.iter().map(|rule| rd.describe(&P::from(*rule))).collect::<Vec<_>>().join(", ")),
                started: utc(original.at),
                table: table(r, game, s, StoryTable { show_name: true, ..Default::default() }),
            });
        }

        if let Event::BeneGesseritPrediction(bp) = &evt {
            out.push(StoryBlock::Prediction { to_win: name_or_absent(bp.to_win), turn: bp.turn });
        }

        if matches!(evt, Event::Bid(_) | Event::BlackMarketBid(_)) {
            if let Some(wb) = &r.bidding.winning_bid {
                if previous_winning_bid.as_ref() != Some(wb) {
                    let buyer = wb.initiator;
                    out.push(StoryBlock::Purchase {
                        buyer: name_or_absent(buyer),
                        card: r.bidding.card_just_won.map(|c| card(c, rd)),
                        amount: wb.bid_total_amount(),
                        karma_card: wb.karma_card.map(|c| card(c, rd)),
                        auction: (r.bidding.current_auction_type != Some(AuctionType::Normal)).then(|| describe_detailed(r.bidding.current_auction_type, rd)),
                    });
                }
                previous_winning_bid = Some(wb.clone());
            }
        }

        let main_phase_started = |m: MainPhase| r.turn.current_main_phase != previous_main_phase && r.turn.current_main_phase == m;
        let show_ally = r.players.iter().any(|p| p.ally != None);

        if main_phase_started(MainPhase::Bidding) {
            out.push(StoryBlock::Turn { turn: r.turn.current_turn });
            let columns = StoryTable {
                show_ally,
                show_cards: true,
                show_traitors: true,
                show_resources: true,
                show_tech_tokens: r.applicable(Rule::TechTokens),
                ..Default::default()
            };
            out.push(StoryBlock::Situation { when: SituationWhen::StartOfBidding, turn: r.turn.current_turn, table: table(r, game, s, columns) });
        }

        if main_phase_started(MainPhase::Resurrection) {
            let columns = StoryTable { show_cards: true, show_resources: true, ..Default::default() };
            out.push(StoryBlock::Situation { when: SituationWhen::EndOfBidding, turn: r.turn.current_turn, table: table(r, game, s, columns) });
        }

        if main_phase_started(MainPhase::Battle) {
            let columns = StoryTable { show_ally, show_resources: true, show_forces: true, show_leaders: true, show_points: true, ..Default::default() };
            out.push(StoryBlock::Situation { when: SituationWhen::EndOfShipmentAndMove, turn: r.turn.current_turn, table: table(r, game, s, columns) });
        }

        if matches!(evt, Event::BattleInitiated(_)) {
            if let Some(t) = r.battle.battle_about_to_start.as_ref().and_then(|b| b.territory) {
                out.push(StoryBlock::BattleFor { territory: t.id() });
            }
        }

        if is_milestone(&evt) {
            // Rendered with the finished game, so the live collections in the message show their
            // contents at the end of the game.
            out.push(StoryBlock::Milestone { faction: name_or_absent(original.by), text: crate::expr::message(&evt.describe(r, By::new(r, original.by)), d) });
        }

        if r.phase() != previous_phase && r.phase() == Phase::BattleReport {
            if let (Some(a), Some(dp)) = (&r.battle.aggressor_plan, &r.battle.defender_plan) {
                let advanced = r.applicable(Rule::AdvancedCombat);
                out.push(StoryBlock::Battle {
                    aggressor: plan(r, a, dp.initiator().expect("faction"), advanced),
                    defender: plan(r, dp, a.initiator().expect("faction"), advanced),
                    advanced_combat: advanced,
                });
            }
        }

        if r.phase() != previous_phase && r.phase() == Phase::TurnConcluded {
            let columns = StoryTable {
                show_ally,
                show_forces: true,
                show_tanks: true,
                show_resources: true,
                show_tech_tokens: true,
                show_points: true,
                ..Default::default()
            };
            out.push(StoryBlock::Situation { when: SituationWhen::EndOfTurn, turn: r.turn.current_turn, table: table(r, game, s, columns) });
            out.push(strongholds(r));
        }

        if main_phase_started(MainPhase::Ended) {
            let win_method = r.ending().and_then(|e| e.win_method);
            for &w in r.winners() {
                out.push(StoryBlock::Winner {
                    faction: name_or_absent(r.player(w).faction),
                    name: s.player_name(r, w),
                    method: rd.describe(&P::from(win_method)),
                });
            }
            out.push(StoryBlock::Ended { time: utc(original.at) });
        }

        previous_phase = r.phase();
        previous_main_phase = r.turn.current_main_phase;
    }
    out
}

/// A table of players with the columns set in `columns`. `finished` is the game at its end:
/// every table shows the discarded traitors of the end of the game, not of the moment the table
/// stands for.
fn table(g: &Game, finished: &Game, s: &Seating, mut columns: StoryTable) -> StoryTable {
    columns.nexus_column = columns.show_ally && g.applicable(Rule::NexusCards);
    let c = &columns;
    let rows = g
        .player_indices()
        .map(|pi| {
            let p = g.player(pi);
            let mut row = StoryRow { faction: name_or_absent(p.faction), ..Default::default() };
            if c.show_name {
                row.name = s.player_name(g, pi);
            }
            if c.show_ally {
                if p.ally != None {
                    row.ally = Some(name_or_absent(p.ally));
                } else if p.nexus != None {
                    row.nexus = Some(name_or_absent(p.nexus));
                }
            }
            if c.show_cards {
                row.cards = p.treachery_cards.iter().map(|c| c.id()).collect();
            }
            if c.show_traitors {
                let t = p.traitors.iter().map(|h| StoryTraitor { hero: hero_ref(*h), discarded: false });
                let discarded = finished.get_player(p.faction).map(|f| &finished.player(f).discarded_traitors);
                let dt = discarded.into_iter().flatten().map(|h| StoryTraitor { hero: hero_ref(*h), discarded: true });
                let fd = p.face_dancers.iter().map(|h| StoryTraitor { hero: hero_ref(*h), discarded: false });
                row.traitors = t.chain(dt).chain(fd).collect();
            }
            if c.show_tech_tokens {
                row.tech_tokens = p.tech_tokens.iter().map(|t| name(*t)).collect();
            }
            if c.show_resources {
                row.resources = Some(i32::from(p.resources));
            }
            if c.show_forces {
                let mut locations: Vec<Location> =
                    p.locations_with_any_forces().into_iter().filter(|l| g.applicable(Rule::Homeworlds) || !l.territory().is_homeworld()).collect();
                // OrderByDescending(IsStronghold).ThenBy(Orientation): stable, as LINQ.
                locations.sort_by(|a, b| b.is_stronghold().cmp(&a.is_stronghold()).then_with(|| a.orientation().cmp(b.orientation())));
                row.forces = locations
                    .into_iter()
                    .map(|l| StoryForces {
                        territory: l.territory().id(),
                        contested: l != g.board.map.polar_sink()
                            && p.occupies_territory(g, Some(l.territory()))
                            && g.nr_of_occupants_of_territory_excluding(l.territory(), p.faction) > 0,
                        forces: p.forces_in(Some(l)),
                        special_forces: p.special_forces_in(Some(l)),
                    })
                    .collect();
            }
            if c.show_leaders {
                row.leaders = p.leaders.iter().filter(|l| g.is_alive(Hero::Leader(**l))).map(|l| hero_ref(Hero::Leader(*l))).collect();
                row.messiah_available = g.applicable(Rule::AtreidesMessiah)
                    && p.faction == Some(Faction::Atreides)
                    && p.total_forces_killed_in_battle >= 7
                    && g.is_alive(Hero::Messiah);
            }
            if c.show_tanks {
                row.forces_killed = i32::from(p.forces_killed);
                row.special_forces_killed = i32::from(p.special_forces_killed);
                row.dead_leaders = p.leaders.iter().filter(|l| !g.is_alive(Hero::Leader(**l))).map(|l| hero_ref(Hero::Leader(*l))).collect();
                row.messiah_dead = p.faction == Some(Faction::Atreides) && g.applicable(Rule::AtreidesMessiah) && !g.messiah_is_alive();
            }
            if c.show_points {
                row.points = g.number_of_victory_points(pi, g.applicable(Rule::ContestedStongholdsCountAsOccupied));
            }
            row
        })
        .collect();
    StoryTable { rows, ..columns }
}

/// One side of a battle in the story.
fn plan(g: &Game, b: &BattlePlan, opponent: Faction, advanced: bool) -> StoryPlan {
    StoryPlan {
        faction: name_or_absent(b.initiator()),
        winner: g.battle.battle_winner == Some(b.initiator()),
        hero: b.hero.map(hero_ref),
        messiah: b.messiah,
        weapon: b.weapon().map(|c| c.id()),
        defense: b.defense().map(|c| c.id()),
        dial: b.dial(g, opponent).to_string(),
        cost: advanced.then(|| b.cost(g)),
    }
}

/// The strongholds table at the end of a turn.
fn strongholds(g: &Game) -> StoryBlock {
    let owner_column = g.applicable(Rule::StrongholdBonus);
    let terror_column = g.is_playing(Faction::Ecaz) || g.is_playing(Faction::Moritani);
    let rows = g
        .board.map
        .strongholds()
        .map(|sh| StoryStronghold {
            location: sh.id(),
            battalions: g
                .battalions_in(sh)
                .iter()
                .map(|b| StoryBattalion { faction: name(b.faction), forces: b.forces(), special_forces: b.special_forces() })
                .collect(),
            owner: if owner_column { g.owner_of_location(sh).map(|p| name_or_absent(g.player(p).faction)) } else { None },
            ambassador: if terror_column {
                g.ambassador_in(sh.territory()).map(name)
            } else {
                None
            },
            terror: if terror_column { g.terror_in(sh.territory()).map(name).collect() } else { vec![] },
        })
        .collect();
    StoryBlock::Strongholds { owner_column, terror_column, rows }
}
