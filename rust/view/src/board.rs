//! What the map shows: the planet, the ring of players, the card piles, the tanks, the bids and
//! the battle plans. A field that only some viewers may see is filled only for them (protocol.md,
//! Visibility).

use treachery_engine::game::{FlashInfo, GameStatus};
use treachery_engine::heroes::hero::Hero;
use treachery_engine::identity::IIdentifiable;
use treachery_engine::prelude::*;

use crate::build::ResolvedViewer;
use crate::describe::Describer;
use crate::expr;
use crate::schema::*;
use crate::shield::hero_ref;

fn name<T: ToString>(v: T) -> String {
    v.to_string()
}

fn seconds(t: treachery_engine::compat::date_time_offset::TimeSpan) -> f64 {
    t.ticks as f64 / treachery_engine::compat::date_time_offset::TICKS_PER_SECOND as f64
}

/// The viewer's faction, when they play (observers have none).
fn viewer_faction(v: &ResolvedViewer) -> Option<Faction> {
    v.player.and(v.faction)
}

/// The planet: everything on the map except the piles, tanks, bids and plans.
pub fn board(game: &Game, status: &GameStatus, v: &ResolvedViewer) -> BoardView {
    let me = viewer_faction(v);
    let mut forces = Vec::new();
    for (location, battalions) in game.forces_on_planet_excluding_empty_locations(true).iter() {
        for b in battalions {
            let threatened = game.get_player(Some(b.faction)).is_some_and(|p| game.threatened_by_ally_presence(p, location.territory()));
            forces.push(ForcesInLocation {
                location: location.id(),
                faction: name(b.faction),
                forces: b.amount_of_forces,
                special_forces: b.amount_of_special_forces,
                threatened,
            });
        }
    }

    let map = &game.map;
    let attached = map
        .locations(false)
        .filter_map(|l| l.as_attached_location())
        .filter_map(|a| a.attached_to_location(map).map(|to| AttachedLocationView { location: a.location().id(), attached_to: to.id(), sector: a.location().sector(map) }))
        .collect();

    // Every hero with a state that is alive and stands somewhere.
    let leaders = game
        .leader_state
        .iter()
        .filter(|(_, s)| s.alive())
        .filter_map(|(h, s)| s.current_territory.map(|t| LeaderOnPlanet { hero: hero_ref(*h), territory: t.id() }))
        .collect();

    // Only during ship and move, and not for moves to a homeworld.
    let mut recent_moves = Vec::new();
    if game.current_main_phase == MainPhase::ShipmentAndMove {
        for m in &game.recent_moves {
            let Some(to) = m.i_placement_to(game) else { continue };
            if to.is_homeworld() {
                continue;
            }
            let initiator = m.i_placement_initiator();
            for origin in m.i_placement_force_locations(game).keys() {
                let path = map.find_first_shortest_path(*origin, to, false, initiator, game);
                match path {
                    Some(path) if path.contains(&map.polar_sink()) => {
                        // One arrow per territory change along the path, and one into the
                        // destination.
                        let mut from: Option<Location> = None;
                        for step in path {
                            if let Some(f) = from {
                                if step == to || step.territory() != f.territory() {
                                    recent_moves.push(MoveArrow { faction: name(initiator), from: f.id(), to: step.id() });
                                }
                            }
                            if from.is_none_or(|f| step.territory() != f.territory()) {
                                from = Some(step);
                            }
                        }
                    }
                    _ => recent_moves.push(MoveArrow { faction: name(initiator), from: origin.id(), to: to.id() }),
                }
            }
        }
    }

    BoardView {
        sector_in_storm: game.sector_in_storm,
        shield_wall_destroyed: game.shield_wall_destroyed,
        forces,
        spice: game.resources_on_planet.iter().map(|(l, a)| SpiceInLocation { location: l.id(), amount: *a }).collect(),
        highlighted_territories: status.highlighted_territories.iter().flatten().map(|t| t.id()).collect(),
        strongholds: game.stronghold_ownership.iter().map(|(l, f)| StrongholdOwner { location: l.id(), owner: name(*f) }).collect(),
        terror: game
            .terror_on_planet
            .iter()
            .map(|(token, t)| TerrorOnPlanet { territory: t.id(), token: (me == Some(Faction::Moritani)).then(|| name(*token)) })
            .collect(),
        ambassadors: game
            .ambassadors_on_planet
            .iter()
            .filter(|(_, a)| **a != Ambassador::None)
            .map(|(t, a)| AmbassadorOnPlanet { territory: t.id(), ambassador: name(*a) })
            .collect(),
        discoveries: game
            .discoveries_on_planet
            .iter()
            .map(|(l, d)| {
                let known = (me == Some(Faction::Fremen) && d.token_type == DiscoveryTokenType::Fremen)
                    || (me == Some(Faction::Guild) && d.token_type == DiscoveryTokenType::Guild);
                DiscoveryOnPlanet { location: l.id(), token_type: name(d.token_type), token: known.then(|| name(d.token)) }
            })
            .collect(),
        attached,
        leaders,
        monsters: game.monsters.iter().map(|m| MonsterView { territory: m.territory.id(), is_great_monster: m.is_great_monster }).collect(),
        number_of_monsters: game.number_of_monsters,
        special_strongholds: map.territories(true).filter(|t| game.is_special_stronghold(*t)).map(|t| t.id()).collect(),
        atomics_aftermath: game.atomics_aftermath.map(|t| t.id()),
        protected_from_storm: map.locations(false).filter(|l| game.is_protected_from_storm(*l)).map(|l| l.id()).collect(),
        // The value for Richese, "?" for everyone else.
        no_field_value: (me == Some(Faction::Richese)).then_some(game.current_no_field_value),
        storm_prescience_moves: game.has_storm_prescience(v.player).then_some(game.next_storm_moves),
        recent_moves,
    }
}

/// What the ring shows of each player beyond `build::public_player`.
pub fn ring_extras(game: &Game, p: PlayerIndex, out: &mut PublicPlayer) {
    let pl = game.player(p);
    out.has_special_forces = pl.has_special_forces(game);
    out.has_room_for_cards = pl.has_room_for_cards(game);
    out.cards_owned = game.cards_owned_by(p).len() as i32;
    out.homeworlds = pl
        .home_worlds
        .iter()
        .map(|h| {
            let w = h.world();
            HomeworldView {
                world: name(w),
                location: h.location().id(),
                occupier: game.is_occupied_world(w).then(|| game.occupier_of(w).map(|o| name(game.player(o).faction))).flatten(),
                high_threshold: pl.has_high_threshold(game, w),
                low_threshold: pl.has_low_threshold(game, w),
            }
        })
        .collect();
    out.skilled_leaders = game
        .get_skilled_leaders(p)
        .filter(|l| game.is_in_front_of_shield(Some(Hero::Leader(*l))))
        .map(|l| SkilledLeader { id: l.id(), skill: name(game.skill(Some(Hero::Leader(l)))) })
        .collect();
    out.loyal_leader = (pl.faction == Faction::Ecaz).then_some(game.ecaz_loyal_leader).flatten().map(|l| l.id());
    out.latest_revealed_no_field = (pl.faction == Faction::Richese && game.latest_revealed_no_field_value >= 0).then_some(game.latest_revealed_no_field_value);
}

/// The card piles beside the map.
pub fn piles(game: &Game, v: &ResolvedViewer) -> PilesView {
    let me = v.player;
    let notekeeping = game.assisted_note_keeping_enabled(me);
    let treachery_deck = game.treachery_deck.as_ref().map(|d| d.items.len()).unwrap_or(0) as i32;
    let discard: &[TreacheryCard] = game.treachery_discard_pile.as_ref().map(|d| d.items.as_slice()).unwrap_or(&[]);
    let resource_deck = game.resource_card_deck.as_ref();
    let pile = |d: &Option<treachery_engine::cards::deck::Deck<ResourceCard>>| d.as_ref().map(|d| d.items.iter().map(|c| c.skin_id()).collect()).unwrap_or_default();
    let known = me.map(|p| game.known_cards(p)).unwrap_or_default();
    PilesView {
        treachery_deck_count: treachery_deck,
        treachery_discard_count: discard.len() as i32,
        treachery_discard_top: discard.first().map(|c| c.id()),
        treachery_discard: notekeeping.then(|| discard.iter().map(|c| known.contains(c).then(|| c.id())).collect()),
        resource_deck_count: resource_deck.map(|d| d.items.len()).unwrap_or(0) as i32,
        resource_deck_top: (game.current_main_phase == MainPhase::ShipmentAndMove && game.has_resource_deck_prescience(me))
            .then(|| resource_deck.and_then(|d| d.top()).map(|c| c.skin_id()))
            .flatten(),
        latest_spice_card_a: game.latest_spice_card_a.map(|c| c.skin_id()),
        latest_spice_card_b: game.latest_spice_card_b.map(|c| c.skin_id()),
        resource_discard_a: notekeeping.then(|| pile(&game.resource_card_discard_pile_a)),
        resource_discard_b: notekeeping.then(|| pile(&game.resource_card_discard_pile_b)),
        sand_trout: game.sand_trout_occured().then_some(game.sand_trout).flatten().map(|c| c.skin_id()),
        richese_cache_count: game.richese_cache.len() as i32,
        // Only Richese sees the cards of its cache.
        richese_cache: (viewer_faction(v) == Some(Faction::Richese)).then(|| game.richese_cache.iter().map(|c| c.id()).collect()),
    }
}

/// The tanks.
pub fn tanks(game: &Game, v: &ResolvedViewer) -> Vec<TanksEntry> {
    let me = viewer_faction(v);
    // Tanks.MayIdentify: a face-down leader is known to its owner and to Tleilaxu.
    let may_identify = |l: Leader| me.is_some_and(|f| f == l.faction() || f == Faction::Tleilaxu);
    game.players
        .iter()
        .map(|pl| {
            let mut corpses: Vec<(Leader, i32, bool)> = pl
                .leaders
                .iter()
                .filter(|l| !game.is_alive(Hero::Leader(**l)))
                .map(|l| {
                    let s = &game.leader_state[&Hero::Leader(*l)];
                    (*l, s.time_of_death, s.is_face_down_dead())
                })
                .collect();
            // OrderBy(TimeOfDeath) is a stable sort.
            corpses.sort_by_key(|(_, t, _)| *t);
            TanksEntry {
                faction: name(pl.faction),
                forces_killed: pl.forces_killed,
                special_forces_killed: pl.special_forces_killed,
                leaders: corpses
                    .into_iter()
                    .map(|(l, _, face_down)| DeadLeader { id: (!face_down || may_identify(l)).then(|| l.id()), face_down })
                    .collect(),
                messiah_dead: pl.faction == Faction::Atreides && !game.messiah_is_alive(),
            }
        })
        .collect()
}

/// The bids on the map during the bidding main phase, and the bidding panel's lot
/// (`actions/bidding.rs`).
pub fn bidding(game: &Game, v: &ResolvedViewer) -> Option<BiddingView> {
    if game.current_main_phase != MainPhase::Bidding {
        return None;
    }
    let silent = game.current_auction_type == AuctionType::BlackMarketSilent || game.current_auction_type == AuctionType::RicheseSilent;
    let bids = game
        .bids
        .iter()
        .map(|(f, bid)| {
            let shown = if silent {
                BidShown::Ready
            } else if bid.i_bid_passed() {
                BidShown::Pass
            } else {
                BidShown::Bid { amount: bid.i_bid_total_amount() }
            };
            MapBid { faction: name(*f), bid: shown }
        })
        .collect();
    Some(BiddingView { auction_type: name(game.current_auction_type), bids,
        panel: crate::actions::bidding::panel(game, v),
        auction_won: game.recent_milestones.contains(&Milestone::AuctionWon),
    })
}

/// The current battle and the plans this viewer may see.
pub fn battle(game: &Game, v: &ResolvedViewer) -> Option<BattleView> {
    let current = game.current_battle.clone()?;
    let aggressor = current.aggressor(game);
    let defender = current.defender(game);
    let territory = current.territory(game);
    let mut view = BattleView {
        territory: territory.map(|t| t.id()),
        aggressor: name(aggressor),
        defender: name(defender),
        aggressor_plan: None,
        defender_plan: None,
    };
    if game.current_main_phase != MainPhase::Battle || game.current_phase == Phase::BattleReport {
        return Some(view);
    }

    let me = viewer_faction(v);
    let my_ally = v.player.map(|p| game.player(p).ally);
    let sees_atreides_karma = game.atreides_karma && (me == Some(Faction::Atreides) || my_ally == Some(Faction::Atreides));
    let atreides_opponent = current.opponent_of_faction(game, Faction::Atreides);
    let both_in = game.aggressor_plan.is_some() && game.defender_plan.is_some();
    // What prescience revealed goes to the prescience user and their ally only (protocol.md,
    // Visibility).
    let on_side_of = |f: Faction| me == Some(f) || (my_ally == Some(f) && f != Faction::None);
    let affected_by_partial = |plan: &Battle| {
        let by = |initiator: Faction| current.opponent_of_faction(game, initiator) == Some(plan.player()) && on_side_of(initiator);
        game.current_prescience.as_ref().is_some_and(|pr| by(pr.initiator())) || game.current_nexus_prescience.as_ref().is_some_and(|np| by(np.initiator()))
    };
    let traitor = |a: &Option<std::rc::Rc<TreacheryCalled>>| a.as_ref().is_some_and(|t| t.succeeded(game));
    let facedanced = |f: Faction| game.current_phase == Phase::Facedancing && game.battle_winner == Some(f);

    let side = |plan: &Option<std::rc::Rc<Battle>>, opponent_plan: &Option<std::rc::Rc<Battle>>, is_aggressor: bool, traitor_called: bool| {
        let plan = plan.as_ref()?;
        let may_see = plan.initiator() == me.unwrap_or(Faction::None) && me.is_some() || (sees_atreides_karma && atreides_opponent == Some(plan.player()));
        let full = may_see || both_in;
        if !full && !affected_by_partial(plan) {
            return None;
        }
        let facedanced = facedanced(if is_aggressor { aggressor } else { defender });
        Some(plan_view(game, plan, opponent_plan.as_deref(), full, is_aggressor, if full { traitor_called } else { false }, facedanced, territory, if is_aggressor { defender } else { aggressor }))
    };
    view.aggressor_plan = side(&game.aggressor_plan, &game.defender_plan, true, traitor(&game.defender_traitor_action));
    view.defender_plan = side(&game.defender_plan, &game.aggressor_plan, false, traitor(&game.aggressor_traitor_action));
    Some(view)
}

/// One plan as this viewer may see it.
#[allow(clippy::too_many_arguments)]
fn plan_view(
    game: &Game,
    plan: &Battle,
    opponent_plan: Option<&Battle>,
    full: bool,
    is_aggressor: bool,
    traitor_called: bool,
    facedanced: bool,
    territory: Option<Territory>,
    opponent: Faction,
) -> PlanView {
    let aspect = |a: PrescienceAspect| {
        game.current_prescience.as_ref().is_some_and(|p| p.aspect == a) || game.current_nexus_prescience.as_ref().is_some_and(|n| n.atreides_prescience_aspect == a)
    };
    let pl = game.player(plan.player());
    let initiator = plan.initiator();

    let dial_shown = full || aspect(PrescienceAspect::Dial);
    let homeworld_bonus = territory.and_then(|t| {
        let amount = pl.get_homeworld_battle_contribution_and_lasgun_shield_limit(game, t);
        (amount != 0).then(|| HomeworldBonus { world: name(t.locations()[0].as_homeworld().map(|h| h.world()).unwrap_or_default()), amount })
    });

    let hero_shown = full || aspect(PrescienceAspect::Leader);
    let hero = if hero_shown {
        match plan.hero() {
            Some(Hero::Leader(l)) => {
                let h = Hero::Leader(l);
                let skill = game.skill(Some(h));
                let passive = game.get_skill(plan.player()).filter(|s| *s != skill && passive_skill_is_relevant_for_battle_plan(*s));
                let mut used = LeaderSkill::None;
                let bonus = Battle::determine_skill_bonus(game, plan, &mut used);
                let penalty = match opponent_plan {
                    Some(op) if game.current_phase > Phase::BattlePhase && game.current_phase < Phase::BattleConclusion => {
                        let amount = Battle::determine_skill_penalty(game, op, plan.player(), &mut used);
                        (amount != 0).then(|| SkillPenalty { amount, skill: name(used) })
                    }
                    _ => None,
                };
                Some(PlanHero::Leader {
                    id: l.id(),
                    skill: (skill != LeaderSkill::None).then(|| name(skill)),
                    passive_skill: passive.map(name),
                    bonus: (bonus != 0).then_some(bonus),
                    penalty,
                })
            }
            Some(Hero::TreacheryCard(c)) => Some(PlanHero::TreacheryCard { id: c.id() }),
            // Any other hero is drawn as neither a leader nor a card.
            _ => None,
        }
    } else {
        None
    };

    let defense_shown = full || aspect(PrescienceAspect::Defense);
    let antidote = Battle::is_using_portable_antidote(game, initiator).then(|| pl.card(TreacheryCardType::PortableAntidote)).flatten();
    let defense = if !defense_shown {
        None
    } else if let Some(a) = antidote {
        Some(a.id())
    } else {
        plan.defense().filter(|d| full || d.is_defense()).map(|d| d.id())
    };

    let weapon_shown = full || aspect(PrescienceAspect::Weapon);
    let weapon = if weapon_shown { plan.weapon().filter(|w| full || w.is_weapon()).map(|w| w.id()) } else { None };

    PlanView {
        faction: name(initiator),
        is_aggressor,
        full,
        dial: dial_shown.then(|| plan.dial(game, opponent) as f64),
        ecaz_contribution: (dial_shown && game.current_ecaz_battle_contribution > 0 && (plan.by(Faction::Ecaz) || plan.by_ally_of(game, Faction::Ecaz)))
            .then_some(game.current_ecaz_battle_contribution),
        ecaz_karma_bonus: (dial_shown && initiator == Faction::Ecaz && game.ecaz_karma_bonus != 0).then_some(game.ecaz_karma_bonus),
        homeworld_bonus: homeworld_bonus.filter(|_| dial_shown),
        reinforcements: dial_shown && full && plan.has_reinforcements(),
        cost: full.then(|| plan.cost(game)).filter(|c| *c > 0),
        free_resources_advantage: full && territory.is_some_and(|t| game.has_stronghold_advantage(initiator, StrongholdAdvantage::FreeResourcesForBattles, t)),
        hero_shown,
        hero,
        traitor_called: hero_shown && traitor_called,
        facedanced: hero_shown && facedanced,
        messiah: hero_shown && initiator == Faction::Atreides && plan.messiah,
        defense_shown,
        defense,
        weapon_shown,
        weapon,
    }
}

/// `BattlePlan.PassiveSkillIsRelevantForBattlePlan`.
fn passive_skill_is_relevant_for_battle_plan(skill: LeaderSkill) -> bool {
    matches!(
        skill,
        LeaderSkill::Diplomat
            | LeaderSkill::Graduate
            | LeaderSkill::Warmaster
            | LeaderSkill::Adept
            | LeaderSkill::Swordmaster
            | LeaderSkill::KillerMedic
            | LeaderSkill::MasterOfAssassins
    )
}

/// The bidders the prompt shows during the bidding main phase.
pub fn to_bid(game: &Game) -> Vec<SequenceEntry> {
    if game.current_main_phase != MainPhase::Bidding {
        return vec![];
    }
    let seq = if game.current_phase == Phase::BlackMarketAnnouncement || game.current_phase == Phase::BlackMarketBidding {
        BlackMarketBid::players_to_bid(game)
    } else {
        Bid::players_to_bid(game)
    };
    seq.iter().filter_map(|se| se.player.map(|p| SequenceEntry { faction: name(game.player(p).faction), has_turn: se.has_turn })).collect()
}

/// `GameStatus.FlashInfo`, rendered.
pub fn flash(flashes: &[FlashInfo], d: &dyn Describer) -> Vec<FlashView> {
    flashes
        .iter()
        .map(|f| {
            let mut parts = Vec::new();
            expr::element_into(&f.to_show, d, &mut parts);
            let (message, bordered) = expr::message_with_borders(&f.message, d);
            FlashView { to_show: parts.into_iter().next().unwrap_or(Part::None), message, bordered }
        })
        .collect()
}

/// The times the turn popover shows, and the prompt's bidding timer.
pub fn timing(game: &Game, timer: i32) -> TimingView {
    TimingView {
        started: game.started().to_json_string(),
        last_activity: game.history.last().map(|e| e.time().to_json_string()),
        duration: seconds(game.duration()),
        time_spent: game
            .players
            .iter()
            .map(|p| TimeSpent {
                faction: name(p.faction),
                bidding: seconds(game.time_spent(p.faction, MainPhase::Bidding)),
                shipment_and_move: seconds(game.time_spent(p.faction, MainPhase::ShipmentAndMove)),
                battle: seconds(game.time_spent(p.faction, MainPhase::Battle)),
            })
            .collect(),
        timer,
    }
}
