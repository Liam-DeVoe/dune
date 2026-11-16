//! The side column's own data: the deals the menu bar counts and the trade popover lists, the
//! host's bot speed, Atreides's card tracker and the list of cards in play.

use treachery_engine::events::DealAccepted;
use treachery_engine::prelude::*;

use crate::build::{validate, ResolvedViewer};
use crate::describe::Describer;
use crate::expr::message;
use crate::schema::*;

fn name<T: ToString>(v: T) -> String {
    v.to_string()
}

/// An event for the client to submit as it is: in the engine's format (`{"type":..}`), without
/// the submitter or a time, which the server fills in.
fn event_json(e: Event) -> serde_json::Value {
    serde_json::to_value(e).expect("events serialize")
}

/// The deals panel of a seated player (observers have no trade icons):
/// offers to the viewer with their acceptance, the viewer's own offers with each recipient's
/// answer, and every deal in force. `DealAccepted.AcceptableDeals`, `CancellableDeals` and
pub fn deals(game: &Game, v: &ResolvedViewer, d: &dyn Describer) -> Option<DealsView> {
    let p = v.player.filter(|_| !v.is_observer)?;
    let faction = game.player(p).faction;
    // An offer's id is its place in the history, which names it across views.
    let id_of = |o: &DealOffer| o.id.0 as i32;
    let acceptable = DealAccepted::acceptable_deals(game, Some(p))
        .map(|o| {
            let acceptance = o.acceptance();
            DealOfferView {
                id: id_of(o),
                initiator: name_or_absent(o.initiator),
                price: i32::from(o.price),
                description: message(&o.get_deal_description(game), d),
                message: message(&o.describe(game), d),
                accept_error: validate(game, faction, Event::from(acceptance.clone()), d),
                accept: event_json(Event::from(acceptance)),
                decline: event_json(Event::from(o.rejection())),
                withdraw: None,
                answers: vec![],
            }
        })
        .collect();
    let cancellable = DealAccepted::cancellable_deals(game, Some(p))
        .map(|o| DealOfferView {
            id: id_of(o),
            initiator: name_or_absent(o.initiator),
            price: i32::from(o.price),
            description: message(&o.get_deal_description(game), d),
            message: message(&o.describe(game), d),
            accept_error: None,
            accept: serde_json::Value::Null,
            decline: serde_json::Value::Null,
            withdraw: Some(event_json(o.cancellation(game).event)),
            answers: game
                .deal_recipients(o)
                .map(|r| DealAnswer { faction: name(r), accepted: game.deal_response(o.id, r) })
                .collect(),
        })
        .collect();
    let current = DealAccepted::current_deals(game)
        .iter()
        .map(|deal| CurrentDeal {
            bound: name_or_absent(deal.bound_faction),
            consuming: name_or_absent(deal.consuming_faction),
            description: message(&deal.deal_contents_description(game), d),
        })
        .collect();
    Some(DealsView { acceptable, cancellable, current })
}

/// Atreides's card tracker, shown from `Phase::SelectingTraitors` on.
pub fn tracker(game: &Game, v: &ResolvedViewer) -> Option<TrackerView> {
    use treachery_engine::cards::treachery_card_manager::TreacheryCardManager;
    let p = v.player.filter(|_| !v.is_observer)?;
    if game.player(p).faction != Some(Faction::Atreides) || game.phase() < Phase::SelectingTraitors {
        return None;
    }
    let mut cards: Vec<i32> = TreacheryCardManager::get_cards_in_play(game).map(|c| c.id()).collect();
    if game.is_playing(Faction::Richese) {
        for c in TreacheryCardManager::get_richese_cards() {
            if !cards.contains(&c.id()) {
                cards.push(c.id());
            }
        }
    }
    let skin_ids = |d: &Option<Deck<ResourceCard>>| d.as_ref().map(|d| d.items.iter().map(|c| c.skin_id()).collect::<Vec<_>>()).unwrap_or_default();
    let mut resource_cards = skin_ids(&game.decks.resource_card_deck);
    resource_cards.sort();
    Some(TrackerView {
        cards_in_play: cards,
        traitors_in_play: game.traitors_in_play().into_iter().map(crate::shield::hero_ref).collect(),
        resource_cards,
        resource_discard_a: skin_ids(&game.decks.resource_card_discard_pile_a),
        resource_discard_b: skin_ids(&game.decks.resource_card_discard_pile_b),
        players: game
            .player_indices()
            .map(|q| {
                let pl = game.player(q);
                TrackerPlayer {
                    faction: name_or_absent(pl.faction),
                    maximum_number_of_cards: pl.maximum_number_of_cards(game),
                    number_of_traitors: pl.number_of_traitors(),
                    number_of_face_dancers: pl.number_of_face_dancers(),
                }
            })
            .collect(),
    })
}

/// The menu's list of cards in play, the same for every viewer.
pub fn mentat(game: &Game) -> MentatView {
    use treachery_engine::cards::treachery_card_manager::TreacheryCardManager;
    MentatView {
        treachery_cards: TreacheryCardManager::get_cards_in_play(game).map(|c| c.id()).collect(),
        richese_cards: if game.is_playing(Faction::Richese) { TreacheryCardManager::get_richese_cards().iter().map(|c| c.id()).collect() } else { vec![] },
        resource_cards: Map::get_resource_cards_in_play(game).map(|c| c.skin_id()).collect(),
    }
}
