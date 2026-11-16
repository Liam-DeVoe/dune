//! The revival forms: `Revival`, `RaiseDeadPlayed`, `KarmaFreeRevival`,
//! `KarmaRevivalPrevention`, `ChoamFreeRevivalPrevention`, `RequestTleilaxuRevival`,
//! `AcceptOrCancelTleilaxuRevival` and `SetIncreasedRevivalLimits`.
//!
//! A form adjusts its inputs: a widget that is not drawn sets its field to 0 or false, a number
//! picker clamps its value into its range, a required image choice falls back to its first
//! value, and the map pick is dropped while it is disabled or no longer a target. [`normalize`]
//! does the same to a draft, so the validation next to the options is that of the inputs the
//! form shows, and the options carry the adjusted draft back to the client.

use treachery_engine::events::EventType;

use treachery_engine::prelude::*;

use crate::build::{event, validate, ResolvedViewer};
use crate::describe::Describer;
use crate::expr;
use crate::schema::*;
use crate::shield::hero_ref;
use super::widgets::{picker, required_opt, faction_options, hero_options, draft_json, describe, render};

/// The event types this module builds forms for.
pub const TYPES: [EventType; 8] = [
    EventType::Revival,
    EventType::RaiseDeadPlayed,
    EventType::KarmaFreeRevival,
    EventType::KarmaRevivalPrevention,
    EventType::ChoamFreeRevivalPrevention,
    EventType::RequestTleilaxuRevival,
    EventType::AcceptOrCancelTleilaxuRevival,
    EventType::SetIncreasedRevivalLimits,
];

/// A force picker drawn only while its maximum is positive, with the field set to 0 otherwise.
fn forces(value: i32, max: i32) -> i32 {
    if max > 0 { picker(value, 0, max) } else { 0 }
}

/// The first draft of the form, before the player touches it: its starting inputs, adjusted.
/// `None` for event types of other modules.
pub fn default_draft(game: &Game, faction: Option<Faction>, t: EventType) -> Option<Event> {
    let p = game.get_player(faction)?;
    let draft = match t {
        EventType::Revival => {
            let mut e = Revival::default();
            e.hero = None;
            event(e)
        }
        EventType::RaiseDeadPlayed => {
            let mut e = RaiseDeadPlayed::default();
            e.hero = None;
            event(e)
        }
        EventType::KarmaFreeRevival => {
            let mut e = KarmaFreeRevival::default();
            e.hero = None;
            event(e)
        }
        EventType::KarmaRevivalPrevention => event(KarmaRevivalPrevention::default()),
        EventType::ChoamFreeRevivalPrevention => event(ChoamFreeRevivalPrevention::default()),
        EventType::RequestTleilaxuRevival => {
            let mut e = RequestTleilaxuRevival::default();
            e.hero = None;
            event(e)
        }
        EventType::AcceptOrCancelTleilaxuRevival => {
            let mut e = AcceptOrCancelTleilaxuRevival::default();
            e.cancel = false;
            e.hero = None;
            event(e)
        }
        // OnInitialized: the factions that already have the increased limit.
        EventType::SetIncreasedRevivalLimits => {
            let mut e = SetIncreasedRevivalLimits::default();
            e.factions = Some(game.turn.revivals.factions_with_increased_revival_limits.clone().unwrap_or_default());
            event(e)
        }
        _ => return None,
    };
    Some(normalize(game, p, &draft))
}

/// The draft after one round of the form's adjustments, repeated until it is stable
/// (each step can only lower an amount or clear a choice, so this ends quickly).
pub fn normalize(game: &Game, p: PlayerId, draft: &Event) -> Event {
    let mut current = draft.clone();
    for _ in 0..8 {
        let next = normalize_once(game, p, &current);
        if next == current {
            return next;
        }
        current = next;
    }
    current
}

fn normalize_once(game: &Game, p: PlayerId, draft: &Event) -> Event {
    let pl = game.player(p);
    match draft {
        Event::Revival(e) => {
            let mut e = e.clone();
            if !(pl.nexus == Some(Faction::Emperor) && NexusPlayed::can_use_secret_ally(game, p)) {
                e.uses_emperor_secret_ally = false;
            }
            let max_forces = Revival::valid_max_revivals(game, p, false, e.uses_emperor_secret_ally);
            let max_special = Revival::valid_max_revivals(game, p, true, e.uses_emperor_secret_ally);
            e.amount_of_forces = treachery_engine::game::amounts::clamp_u16(forces(i32::from(e.amount_of_forces), max_forces));
            e.amount_of_special_forces = treachery_engine::game::amounts::clamp_u16(forces(i32::from(e.amount_of_special_forces), max_special));
            let by_emperor = Revival::valid_max_revivals_by_emperor(game, p);
            if by_emperor > 0 {
                e.extra_forces_paid_by_emperor = treachery_engine::game::amounts::clamp_u16(if max_forces > 0 { picker(i32::from(e.extra_forces_paid_by_emperor), 0, by_emperor) } else { 0 });
                e.extra_special_forces_paid_by_emperor = treachery_engine::game::amounts::clamp_u16(if max_special > 0 { picker(i32::from(e.extra_special_forces_paid_by_emperor), 0, by_emperor) } else { 0 });
            } else {
                e.extra_forces_paid_by_emperor = 0;
                e.extra_special_forces_paid_by_emperor = 0;
            }
            let (on_planet, special_on_planet) = revival_on_planet(game, p, &e);
            e.number_of_forces_in_location = treachery_engine::game::amounts::clamp_u16(forces(i32::from(e.number_of_forces_in_location), on_planet));
            e.number_of_special_forces_in_location = treachery_engine::game::amounts::clamp_u16(forces(i32::from(e.number_of_special_forces_in_location), special_on_planet));
            if Revival::valid_revival_heroes(game, p).next().is_none() {
                e.hero = None;
            }
            if !e.hero.is_some_and(|h| Revival::may_assign_skill(game, p, Some(h))) {
                e.assign_skill = false;
            }
            let pick_enabled = i32::from(e.number_of_forces_in_location) > 0 || i32::from(e.number_of_special_forces_in_location) > 0;
            let location = e.location.filter(|l| pick_enabled && Revival::valid_revived_force_locations(game, p).any(|v| v == *l));
            e.location = location;
            event(e)
        }
        Event::RaiseDeadPlayed(e) => {
            let mut e = e.clone();
            e.amount_of_forces = treachery_engine::game::amounts::clamp_u16(forces(i32::from(e.amount_of_forces), RaiseDeadPlayed::valid_max_amount(game, p, false)));
            e.amount_of_special_forces = treachery_engine::game::amounts::clamp_u16(forces(i32::from(e.amount_of_special_forces), RaiseDeadPlayed::valid_max_amount(game, p, true)));
            let special_on_planet = raise_dead_on_planet(game, p, &e);
            e.number_of_special_forces_in_location = treachery_engine::game::amounts::clamp_u16(forces(i32::from(e.number_of_special_forces_in_location), special_on_planet));
            if !e.hero.is_some_and(|h| RaiseDeadPlayed::valid_heroes(game, p).contains(&h)) {
                e.hero = None;
            }
            if !e.hero.is_some_and(|h| Revival::may_assign_skill(game, p, Some(h))) {
                e.assign_skill = false;
            }
            let pick_enabled = i32::from(e.number_of_special_forces_in_location) > 0;
            let location = e.location.filter(|l| pick_enabled && Revival::valid_revived_force_locations(game, p).any(|v| v == *l));
            e.location = location;
            event(e)
        }
        Event::KarmaFreeRevival(e) => {
            let mut e = e.clone();
            e.amount_of_forces = treachery_engine::game::amounts::clamp_u16(forces(i32::from(e.amount_of_forces), KarmaFreeRevival::valid_max_amount(game, p, false)));
            e.amount_of_special_forces = treachery_engine::game::amounts::clamp_u16(forces(i32::from(e.amount_of_special_forces), KarmaFreeRevival::valid_max_amount(game, p, true)));
            if !e.hero.is_some_and(|h| Revival::may_assign_skill(game, p, Some(h))) {
                e.assign_skill = false;
            }
            event(e)
        }
        Event::KarmaRevivalPrevention(e) => {
            let mut e = e.clone();
            let targets: Vec<Faction> = KarmaRevivalPrevention::get_valid_targets(game, p).collect();
            e.target = required_opt(e.target, &targets);
            event(e)
        }
        Event::ChoamFreeRevivalPrevention(e) => {
            let mut e = e.clone();
            let targets: Vec<Faction> = game.players_other_than(p).collect();
            e.target = required_opt(e.target, &targets);
            event(e)
        }
        Event::RequestTleilaxuRevival(e) => {
            let mut e = e.clone();
            let heroes: Vec<Hero> = RequestTleilaxuRevival::valid_targets(game, p).collect();
            let hero = e.hero;
            if !hero.is_some_and(|h| heroes.contains(&h)) {
                e.hero = heroes.first().copied();
            }
            event(e)
        }
        // The hero picker and the price are drawn only while there are requests.
        Event::AcceptOrCancelTleilaxuRevival(e) => {
            let mut e = e.clone();
            e.cancel = false;
            if !game.turn.revivals.current_revival_requests.is_empty() {
                let heroes = request_heroes(game);
                if !e.hero.is_some_and(|h| heroes.contains(&h)) {
                    e.hero = heroes.first().copied();
                }
                e.price = picker(e.price, 0, 50);
            }
            event(e)
        }
        _ => draft.clone(),
    }
}

/// For the `Revival` form: how many revived forces and special forces may go to the planet.
fn revival_on_planet(game: &Game, p: PlayerId, e: &Revival) -> (i32, i32) {
    if Revival::valid_revived_force_locations(game, p).next().is_none() {
        return (0, 0);
    }
    (
        Revival::number_of_forces_that_may_be_placed_on_planet(game, p, e.uses_emperor_secret_ally, i32::from(e.amount_of_forces) + i32::from(e.extra_forces_paid_by_emperor)),
        Revival::number_of_special_forces_that_may_be_placed_on_planet(game, p, i32::from(e.amount_of_special_forces) + i32::from(e.extra_special_forces_paid_by_emperor)),
    )
}

/// For the `RaiseDeadPlayed` form: how many revived special forces may go to the planet.
fn raise_dead_on_planet(game: &Game, p: PlayerId, e: &RaiseDeadPlayed) -> i32 {
    if Revival::valid_revived_force_locations(game, p).next().is_none() {
        return 0;
    }
    Revival::number_of_special_forces_that_may_be_placed_on_planet(game, p, i32::from(e.amount_of_special_forces))
}

fn request_heroes(game: &Game) -> Vec<Hero> {
    game.turn.revivals.current_revival_requests.iter().filter_map(|r| r.hero).collect()
}

/// The options and validation of a revival form for `draft`, or `None` for other event types.
/// The draft is first adjusted as the form would ([`normalize`]).
pub fn options_and_validation(game: &Game, v: &ResolvedViewer, draft: &Event, d: &dyn Describer) -> Option<(Option<ActionOptions>, Option<RichText>)> {
    if !TYPES.contains(&draft.event_type()) {
        return None;
    }
    let p = v.player?;
    let draft = normalize(game, p, draft);
    let options = options(game, p, &draft, d);
    Some((Some(options), validate(game, game.player(p).faction, draft, d)))
}

fn options(game: &Game, p: PlayerId, draft: &Event, d: &dyn Describer) -> ActionOptions {
    let pl = game.player(p);
    let json = draft_json(draft);
    match draft {
        Event::Revival(e) => {
            let (max_forces_on_planet, max_special_forces_on_planet) = revival_on_planet(game, p, e);
            let cost = Revival::determine_cost(
                game,
                p,
                e.hero,
                i32::from(e.amount_of_forces),
                i32::from(e.amount_of_special_forces),
                i32::from(e.extra_forces_paid_by_emperor),
                i32::from(e.extra_special_forces_paid_by_emperor),
                e.uses_emperor_secret_ally,
            );
            ActionOptions::Revival(RevivalOptions {
                may_use_secret_ally: pl.nexus == Some(Faction::Emperor) && NexusPlayed::can_use_secret_ally(game, p),
                max_forces: Revival::valid_max_revivals(game, p, false, e.uses_emperor_secret_ally),
                max_special_forces: Revival::valid_max_revivals(game, p, true, e.uses_emperor_secret_ally),
                max_by_emperor: Revival::valid_max_revivals_by_emperor(game, p),
                locations: Revival::valid_revived_force_locations(game, p).map(|l| l.id()).collect(),
                max_forces_on_planet,
                max_special_forces_on_planet,
                heroes: hero_options(Revival::valid_revival_heroes(game, p)),
                may_assign_skill: e.hero.is_some_and(|h| Revival::may_assign_skill(game, p, Some(h))),
                cost_for_player: cost.total_cost_for_player(),
                cost_for_emperor: (pl.ally == Some(Faction::Emperor)).then_some(cost.cost_for_emperor),
                draft: json,
            })
        }
        Event::RaiseDeadPlayed(e) => ActionOptions::RaiseDeadPlayed(RaiseDeadPlayedOptions {
            card_name: describe(d, TreacheryCardType::RaiseDead),
            max_forces: RaiseDeadPlayed::valid_max_amount(game, p, false),
            max_special_forces: RaiseDeadPlayed::valid_max_amount(game, p, true),
            locations: Revival::valid_revived_force_locations(game, p).map(|l| l.id()).collect(),
            max_special_forces_on_planet: raise_dead_on_planet(game, p, e),
            heroes: hero_options(RaiseDeadPlayed::valid_heroes(game, p)),
            may_assign_skill: e.hero.is_some_and(|h| Revival::may_assign_skill(game, p, Some(h))),
            draft: json,
        }),
        Event::KarmaFreeRevival(e) => ActionOptions::KarmaFreeRevival(KarmaFreeRevivalOptions {
            card_name: describe(d, TreacheryCardType::Karma),
            max_forces: KarmaFreeRevival::valid_max_amount(game, p, false),
            max_special_forces: KarmaFreeRevival::valid_max_amount(game, p, true),
            heroes: hero_options(game.killed_heroes(p)),
            may_assign_skill: e.hero.is_some_and(|h| Revival::may_assign_skill(game, p, Some(h))),
            draft: json,
        }),
        Event::KarmaRevivalPrevention(_) => ActionOptions::KarmaRevivalPrevention(KarmaRevivalPreventionOptions {
            card_name: describe(d, TreacheryCardType::Karma),
            targets: faction_options(KarmaRevivalPrevention::get_valid_targets(game, p)),
            draft: json,
        }),
        Event::ChoamFreeRevivalPrevention(_) => ActionOptions::ChoamFreeRevivalPrevention(ChoamFreeRevivalPreventionOptions {
            card: ChoamFreeRevivalPrevention::card_to_use(game, p).map(|c| expr::message(&treachery_engine::express!(c), d)),
            targets: faction_options(game.players_other_than(p)),
            draft: json,
        }),
        Event::RequestTleilaxuRevival(_) => ActionOptions::RequestTleilaxuRevival(RequestTleilaxuRevivalOptions {
            heroes: hero_options(RequestTleilaxuRevival::valid_targets(game, p)),
            draft: json,
        }),
        Event::AcceptOrCancelTleilaxuRevival(e) => ActionOptions::AcceptOrCancelTleilaxuRevival(AcceptOrCancelTleilaxuRevivalOptions {
            requests: hero_options(request_heroes(game)),
            bot_awaits: game.turn.revivals.current_revival_requests.iter().any(|r| game.player(r.player.expect("the request has a player")).is_bot(game)),
            offers: game
                .revival()
                .into_iter()
                .flat_map(|r| r.early_revivals_offers.iter())
                .map(|(h, price)| EarlyRevivalOffer {
                    hero: HeroOption { id: h.id(), hero: hero_ref(*h) },
                    price: (*price != i32::MAX).then_some(*price),
                })
                .collect(),
            offer_label: render(d, &parts!["Offer ", e.hero, " for ", e.price]),
            deny_label: render(d, &parts!["Deny revival of ", e.hero]),
            draft: json,
        }),
        Event::SetIncreasedRevivalLimits(_) => ActionOptions::SetIncreasedRevivalLimits(SetIncreasedRevivalLimitsOptions {
            targets: faction_options(SetIncreasedRevivalLimits::valid_targets(game, p)),
            draft: json,
        }),
        _ => unreachable!("options of a revival event"),
    }
}
