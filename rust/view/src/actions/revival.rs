//! The revival forms: `Revival`, `RaiseDeadPlayed`, `KarmaFreeRevival`,
//! `KarmaRevivalPrevention`, `ChoamFreeRevivalPrevention`, `RequestTleilaxuRevival`,
//! `AcceptOrCancelTleilaxuRevival` and `SetIncreasedRevivalLimits`.
//!
//! A form adjusts its inputs: a widget that is not drawn sets its field to 0 or false, a number
//! picker clamps its value into its range, a required image choice falls back to its first
//! value, and the map pick is dropped while it is disabled or no longer a target. [`normalize`]
//! does the same to a draft, so the validation next to the options is that of the inputs the
//! form shows, and the options carry the adjusted draft back to the client.

use treachery_engine::events::GameEventType;
use treachery_engine::messages::expression::ExpressionElement;
use treachery_engine::json;
use treachery_engine::prelude::*;

use crate::build::{event, validate, ResolvedViewer};
use crate::describe::Describer;
use crate::expr;
use crate::schema::*;
use crate::shield::hero_ref;
use super::widgets::{picker, required, faction_options, hero_options, draft_json, describe};

/// The event types this module builds forms for.
pub const TYPES: [GameEventType; 8] = [
    GameEventType::Revival,
    GameEventType::RaiseDeadPlayed,
    GameEventType::KarmaFreeRevival,
    GameEventType::KarmaRevivalPrevention,
    GameEventType::ChoamFreeRevivalPrevention,
    GameEventType::RequestTleilaxuRevival,
    GameEventType::AcceptOrCancelTleilaxuRevival,
    GameEventType::SetIncreasedRevivalLimits,
];

/// A force picker drawn only while its maximum is positive, with the field set to 0 otherwise.
fn forces(value: i32, max: i32) -> i32 {
    if max > 0 { picker(value, 0, max) } else { 0 }
}

/// `Skin.Format(m, args)`: `string.Format` over the described arguments; `m` itself when that
/// fails.
fn format(d: &dyn Describer, m: &str, args: &[ExpressionElement]) -> String {
    let described: Vec<String> = args.iter().map(|a| d.describe_in(a, None)).collect();
    let refs: Vec<Option<&str>> = described.iter().map(|s| Some(s.as_str())).collect();
    dotnet_compat::fmt::string_format(m, &refs).unwrap_or_else(|_| m.to_string())
}

/// The first draft of the form, before the player touches it: its starting inputs, adjusted.
/// `None` for event types of other modules.
pub fn default_draft(game: &Game, faction: Faction, t: GameEventType) -> Option<GameEvent> {
    let p = game.get_player(Some(faction))?;
    let draft = match t {
        GameEventType::Revival => {
            let mut e = Revival::new(game, faction);
            e.set_hero(None);
            event(e)
        }
        GameEventType::RaiseDeadPlayed => {
            let mut e = RaiseDeadPlayed::new(game, faction);
            e.set_hero(None);
            event(e)
        }
        GameEventType::KarmaFreeRevival => {
            let mut e = KarmaFreeRevival::new(game, faction);
            e.set_hero(None);
            event(e)
        }
        GameEventType::KarmaRevivalPrevention => event(KarmaRevivalPrevention::new(game, faction)),
        GameEventType::ChoamFreeRevivalPrevention => event(ChoamFreeRevivalPrevention::new(game, faction)),
        GameEventType::RequestTleilaxuRevival => {
            let mut e = RequestTleilaxuRevival::new(game, faction);
            e.set_hero(None);
            event(e)
        }
        GameEventType::AcceptOrCancelTleilaxuRevival => {
            let mut e = AcceptOrCancelTleilaxuRevival::new(game, faction);
            e.cancel = false;
            e.set_hero(None);
            event(e)
        }
        // OnInitialized: the factions that already have the increased limit.
        GameEventType::SetIncreasedRevivalLimits => {
            let mut e = SetIncreasedRevivalLimits::new(game, faction);
            e.factions = Some(game.factions_with_increased_revival_limits.clone().unwrap_or_default());
            event(e)
        }
        _ => return None,
    };
    Some(normalize(game, p, &draft))
}

/// The draft after one round of the form's adjustments, repeated until it is stable
/// (each step can only lower an amount or clear a choice, so this ends quickly).
pub fn normalize(game: &Game, p: PlayerIndex, draft: &GameEvent) -> GameEvent {
    let mut current = draft.clone();
    for _ in 0..8 {
        let next = normalize_once(game, p, &current);
        if json::serialize(&next) == json::serialize(&current) {
            return next;
        }
        current = next;
    }
    current
}

fn normalize_once(game: &Game, p: PlayerIndex, draft: &GameEvent) -> GameEvent {
    let pl = game.player(p);
    match draft {
        GameEvent::Revival(e) => {
            let mut e = (**e).clone();
            if !(pl.nexus == Faction::Emperor && NexusPlayed::can_use_secret_ally(game, p)) {
                e.uses_emperor_secret_ally = false;
            }
            let max_forces = Revival::valid_max_revivals(game, p, false, e.uses_emperor_secret_ally);
            let max_special = Revival::valid_max_revivals(game, p, true, e.uses_emperor_secret_ally);
            e.amount_of_forces = forces(e.amount_of_forces, max_forces);
            e.amount_of_special_forces = forces(e.amount_of_special_forces, max_special);
            let by_emperor = Revival::valid_max_revivals_by_emperor(game, p);
            if by_emperor > 0 {
                e.extra_forces_paid_by_emperor = if max_forces > 0 { picker(e.extra_forces_paid_by_emperor, 0, by_emperor) } else { 0 };
                e.extra_special_forces_paid_by_emperor = if max_special > 0 { picker(e.extra_special_forces_paid_by_emperor, 0, by_emperor) } else { 0 };
            } else {
                e.extra_forces_paid_by_emperor = 0;
                e.extra_special_forces_paid_by_emperor = 0;
            }
            let (on_planet, special_on_planet) = revival_on_planet(game, p, &e);
            e.number_of_forces_in_location = forces(e.number_of_forces_in_location, on_planet);
            e.number_of_special_forces_in_location = forces(e.number_of_special_forces_in_location, special_on_planet);
            if Revival::valid_revival_heroes(game, p).next().is_none() {
                e.set_hero(None);
            }
            if !e.hero().is_some_and(|h| Revival::may_assign_skill(game, p, Some(h))) {
                e.assign_skill = false;
            }
            let pick_enabled = e.number_of_forces_in_location > 0 || e.number_of_special_forces_in_location > 0;
            let location = e.location(game).filter(|l| pick_enabled && Revival::valid_revived_force_locations(game, p).any(|v| v == *l));
            e.set_location(game, location);
            event(e)
        }
        GameEvent::RaiseDeadPlayed(e) => {
            let mut e = (**e).clone();
            e.amount_of_forces = forces(e.amount_of_forces, RaiseDeadPlayed::valid_max_amount(game, p, false));
            e.amount_of_special_forces = forces(e.amount_of_special_forces, RaiseDeadPlayed::valid_max_amount(game, p, true));
            let special_on_planet = raise_dead_on_planet(game, p, &e);
            e.number_of_special_forces_in_location = forces(e.number_of_special_forces_in_location, special_on_planet);
            if !e.hero().is_some_and(|h| RaiseDeadPlayed::valid_heroes(game, p).contains(&h)) {
                e.set_hero(None);
            }
            if !e.hero().is_some_and(|h| Revival::may_assign_skill(game, p, Some(h))) {
                e.assign_skill = false;
            }
            let pick_enabled = e.number_of_special_forces_in_location > 0;
            let location = e.location(game).filter(|l| pick_enabled && Revival::valid_revived_force_locations(game, p).any(|v| v == *l));
            e.set_location(game, location);
            event(e)
        }
        GameEvent::KarmaFreeRevival(e) => {
            let mut e = (**e).clone();
            e.amount_of_forces = forces(e.amount_of_forces, KarmaFreeRevival::valid_max_amount(game, p, false));
            e.amount_of_special_forces = forces(e.amount_of_special_forces, KarmaFreeRevival::valid_max_amount(game, p, true));
            if !e.hero().is_some_and(|h| Revival::may_assign_skill(game, p, Some(h))) {
                e.assign_skill = false;
            }
            event(e)
        }
        GameEvent::KarmaRevivalPrevention(e) => {
            let mut e = (**e).clone();
            let targets: Vec<Faction> = KarmaRevivalPrevention::get_valid_targets(game, p).collect();
            e.target = required(e.target, &targets, Faction::None);
            event(e)
        }
        GameEvent::ChoamFreeRevivalPrevention(e) => {
            let mut e = (**e).clone();
            let targets: Vec<Faction> = game.players_other_than(p).collect();
            e.target = required(e.target, &targets, Faction::None);
            event(e)
        }
        GameEvent::RequestTleilaxuRevival(e) => {
            let mut e = (**e).clone();
            let heroes: Vec<Hero> = RequestTleilaxuRevival::valid_targets(game, p).collect();
            let hero = e.hero();
            if !hero.is_some_and(|h| heroes.contains(&h)) {
                e.set_hero(heroes.first().copied());
            }
            event(e)
        }
        // The hero picker and the price are drawn only while there are requests.
        GameEvent::AcceptOrCancelTleilaxuRevival(e) => {
            let mut e = (**e).clone();
            e.cancel = false;
            if !game.current_revival_requests.is_empty() {
                let heroes = request_heroes(game);
                if !e.hero().is_some_and(|h| heroes.contains(&h)) {
                    e.set_hero(heroes.first().copied());
                }
                e.price = picker(e.price, 0, 50);
            }
            event(e)
        }
        _ => draft.clone(),
    }
}

/// For the `Revival` form: how many revived forces and special forces may go to the planet.
fn revival_on_planet(game: &Game, p: PlayerIndex, e: &Revival) -> (i32, i32) {
    if Revival::valid_revived_force_locations(game, p).next().is_none() {
        return (0, 0);
    }
    (
        Revival::number_of_forces_that_may_be_placed_on_planet(game, p, e.uses_emperor_secret_ally, e.amount_of_forces + e.extra_forces_paid_by_emperor),
        Revival::number_of_special_forces_that_may_be_placed_on_planet(game, p, e.amount_of_special_forces + e.extra_special_forces_paid_by_emperor),
    )
}

/// For the `RaiseDeadPlayed` form: how many revived special forces may go to the planet.
fn raise_dead_on_planet(game: &Game, p: PlayerIndex, e: &RaiseDeadPlayed) -> i32 {
    if Revival::valid_revived_force_locations(game, p).next().is_none() {
        return 0;
    }
    Revival::number_of_special_forces_that_may_be_placed_on_planet(game, p, e.amount_of_special_forces)
}

fn request_heroes(game: &Game) -> Vec<Hero> {
    game.current_revival_requests.iter().filter_map(|r| r.hero()).collect()
}

/// The options and validation of a revival form for `draft`, or `None` for other event types.
/// The draft is first adjusted as the form would ([`normalize`]).
pub fn options_and_validation(game: &Game, v: &ResolvedViewer, draft: &GameEvent, d: &dyn Describer) -> Option<(Option<ActionOptions>, Option<RichText>)> {
    if !TYPES.contains(&draft.event_type()) {
        return None;
    }
    let p = v.player?;
    let draft = normalize(game, p, draft);
    let options = options(game, p, &draft, d);
    Some((Some(options), validate(game, draft, d)))
}

fn options(game: &Game, p: PlayerIndex, draft: &GameEvent, d: &dyn Describer) -> ActionOptions {
    let pl = game.player(p);
    let json = draft_json(draft);
    match draft {
        GameEvent::Revival(e) => {
            let (max_forces_on_planet, max_special_forces_on_planet) = revival_on_planet(game, p, e);
            let cost = Revival::determine_cost(
                game,
                p,
                e.hero(),
                e.amount_of_forces,
                e.amount_of_special_forces,
                e.extra_forces_paid_by_emperor,
                e.extra_special_forces_paid_by_emperor,
                e.uses_emperor_secret_ally,
            );
            ActionOptions::Revival(RevivalOptions {
                may_use_secret_ally: pl.nexus == Faction::Emperor && NexusPlayed::can_use_secret_ally(game, p),
                max_forces: Revival::valid_max_revivals(game, p, false, e.uses_emperor_secret_ally),
                max_special_forces: Revival::valid_max_revivals(game, p, true, e.uses_emperor_secret_ally),
                max_by_emperor: Revival::valid_max_revivals_by_emperor(game, p),
                locations: Revival::valid_revived_force_locations(game, p).map(|l| l.id()).collect(),
                max_forces_on_planet,
                max_special_forces_on_planet,
                heroes: hero_options(Revival::valid_revival_heroes(game, p)),
                may_assign_skill: e.hero().is_some_and(|h| Revival::may_assign_skill(game, p, Some(h))),
                cost_for_player: cost.total_cost_for_player(),
                cost_for_emperor: (pl.ally == Faction::Emperor).then_some(cost.cost_for_emperor),
                draft: json,
            })
        }
        GameEvent::RaiseDeadPlayed(e) => ActionOptions::RaiseDeadPlayed(RaiseDeadPlayedOptions {
            card_name: describe(d, TreacheryCardType::RaiseDead),
            max_forces: RaiseDeadPlayed::valid_max_amount(game, p, false),
            max_special_forces: RaiseDeadPlayed::valid_max_amount(game, p, true),
            locations: Revival::valid_revived_force_locations(game, p).map(|l| l.id()).collect(),
            max_special_forces_on_planet: raise_dead_on_planet(game, p, e),
            heroes: hero_options(RaiseDeadPlayed::valid_heroes(game, p)),
            may_assign_skill: e.hero().is_some_and(|h| Revival::may_assign_skill(game, p, Some(h))),
            draft: json,
        }),
        GameEvent::KarmaFreeRevival(e) => ActionOptions::KarmaFreeRevival(KarmaFreeRevivalOptions {
            card_name: describe(d, TreacheryCardType::Karma),
            max_forces: KarmaFreeRevival::valid_max_amount(game, p, false),
            max_special_forces: KarmaFreeRevival::valid_max_amount(game, p, true),
            heroes: hero_options(game.killed_heroes(p)),
            may_assign_skill: e.hero().is_some_and(|h| Revival::may_assign_skill(game, p, Some(h))),
            draft: json,
        }),
        GameEvent::KarmaRevivalPrevention(_) => ActionOptions::KarmaRevivalPrevention(KarmaRevivalPreventionOptions {
            card_name: describe(d, TreacheryCardType::Karma),
            targets: faction_options(KarmaRevivalPrevention::get_valid_targets(game, p)),
            draft: json,
        }),
        GameEvent::ChoamFreeRevivalPrevention(_) => ActionOptions::ChoamFreeRevivalPrevention(ChoamFreeRevivalPreventionOptions {
            card: ChoamFreeRevivalPrevention::card_to_use(game, p).map(|c| expr::message(&treachery_engine::express!(c), d)),
            targets: faction_options(game.players_other_than(p)),
            draft: json,
        }),
        GameEvent::RequestTleilaxuRevival(_) => ActionOptions::RequestTleilaxuRevival(RequestTleilaxuRevivalOptions {
            heroes: hero_options(RequestTleilaxuRevival::valid_targets(game, p)),
            draft: json,
        }),
        GameEvent::AcceptOrCancelTleilaxuRevival(e) => ActionOptions::AcceptOrCancelTleilaxuRevival(AcceptOrCancelTleilaxuRevivalOptions {
            requests: hero_options(request_heroes(game)),
            bot_awaits: game.current_revival_requests.iter().any(|r| game.player(r.player()).is_bot(game)),
            offers: game
                .early_revivals_offers
                .iter()
                .map(|(h, price)| EarlyRevivalOffer {
                    hero: HeroOption { id: h.id(), hero: hero_ref(*h) },
                    price: (*price != i32::MAX).then_some(*price),
                })
                .collect(),
            offer_label: format(d, "Offer {0} for {1}", &[e.hero().into(), e.price.into()]),
            deny_label: format(d, "Deny revival of {0}", &[e.hero().into()]),
            draft: json,
        }),
        GameEvent::SetIncreasedRevivalLimits(_) => ActionOptions::SetIncreasedRevivalLimits(SetIncreasedRevivalLimitsOptions {
            targets: faction_options(SetIncreasedRevivalLimits::valid_targets(game, p)),
            draft: json,
        }),
        _ => unreachable!("options of a revival event"),
    }
}
