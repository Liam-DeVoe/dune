//! The side column's own data (client-plan.md package 5): the deals the menu bar counts and the
//! trade popover lists (`MenuComponent.razor`, `DealComponent.razor`, `DealToasts.razor`), and the
//! host's bot speed (`MenuComponent.razor`).

use std::rc::Rc;

use treachery_engine::events::deal_accepted::DealAccepted;
use treachery_engine::events::deal_offered::DealOffered;
use treachery_engine::prelude::*;

use crate::build::{validate, ResolvedViewer};
use crate::describe::Describer;
use crate::expr::message;
use crate::schema::*;

fn name<T: ToString>(v: T) -> String {
    v.to_string()
}

/// An event in savegame JSON, for the client to submit as it is.
fn event_json(e: GameEvent) -> serde_json::Value {
    serde_json::from_str(&treachery_engine::json::serialize(&e)).expect("events serialize to JSON")
}

/// `DealComponent.razor` for a seated player (the trade icons are not shown to observers):
/// offers to the viewer with their acceptance, the viewer's own offers with each recipient's
/// answer, and every deal in force. `DealAccepted.AcceptableDeals`, `CancellableDeals` and
/// `CurrentDeals`.
pub fn deals(game: &Game, v: &ResolvedViewer, d: &dyn Describer) -> Option<DealsView> {
    let p = v.player.filter(|_| !v.is_observer)?;
    let faction = game.player(p).faction;
    // An offer is the event that made it, so its place in the history names it across views.
    let id_of = |o: &Rc<DealOffered>| {
        game.history
            .iter()
            .position(|e| matches!(e, GameEvent::DealOffered(x) if Rc::ptr_eq(x, o)))
            .map_or(-1, |i| i as i32)
    };
    let acceptable = DealAccepted::acceptable_deals(game, Some(p))
        .map(|o| {
            let acceptance = o.acceptance(game, faction).expect("Acceptance builds an event");
            DealOfferView {
                id: id_of(&o),
                initiator: name(o.initiator()),
                price: o.price,
                description: message(&o.get_deal_description(game), d),
                message: message(&o.get_message(game), d),
                accept_error: validate(game, GameEvent::from(acceptance.clone()), d),
                accept: event_json(GameEvent::from(acceptance)),
                decline: event_json(GameEvent::from(o.rejection(game, faction).expect("Rejection builds an event"))),
                withdraw: None,
                answers: vec![],
            }
        })
        .collect();
    let cancellable = DealAccepted::cancellable_deals(game, Some(p))
        .map(|o| DealOfferView {
            id: id_of(&o),
            initiator: name(o.initiator()),
            price: o.price,
            description: message(&o.get_deal_description(game), d),
            message: message(&o.get_message(game), d),
            accept_error: None,
            accept: serde_json::Value::Null,
            decline: serde_json::Value::Null,
            withdraw: Some(event_json(GameEvent::from(o.cancellation().expect("Cancellation builds an event")))),
            answers: game
                .deal_recipients(&o)
                .map(|r| DealAnswer { faction: name(r), accepted: game.deal_response(&o, r) })
                .collect(),
        })
        .collect();
    let current = DealAccepted::current_deals(game)
        .iter()
        .map(|deal| CurrentDeal {
            bound: name(deal.bound_faction),
            consuming: name(deal.consuming_faction),
            description: message(&deal.deal_contents_description(game), d),
        })
        .collect();
    Some(DealsView { acceptable, cancellable, current })
}

/// `Game.Participation.BotsSpeed`, for the host's bot controls.
pub fn bots_speed(game: &Game) -> i32 {
    game.participation.borrow().bots_speed
}

/// `TrackerComponent.razor`, shown to Atreides from `Phase.SelectingTraitors` on
/// (`TradePopover.razor`).
pub fn tracker(game: &Game, v: &ResolvedViewer) -> Option<TrackerView> {
    use treachery_engine::treachery_card_manager::TreacheryCardManager;
    let p = v.player.filter(|_| !v.is_observer)?;
    if game.player(p).faction != Faction::Atreides || game.current_phase < Phase::SelectingTraitors {
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
    let mut resource_cards = skin_ids(&game.resource_card_deck);
    resource_cards.sort();
    Some(TrackerView {
        cards_in_play: cards,
        traitors_in_play: game.traitors_in_play().into_iter().map(crate::shield::hero_ref).collect(),
        resource_cards,
        resource_discard_a: skin_ids(&game.resource_card_discard_pile_a),
        resource_discard_b: skin_ids(&game.resource_card_discard_pile_b),
        players: game
            .player_indices()
            .map(|q| {
                let pl = game.player(q);
                TrackerPlayer {
                    faction: name(pl.faction),
                    maximum_number_of_cards: pl.maximum_number_of_cards(game),
                    number_of_traitors: pl.number_of_traitors(),
                    number_of_face_dancers: pl.number_of_face_dancers(),
                }
            })
            .collect(),
    })
}

/// `MentatComponent.razor`: the cards in play, the same for every viewer.
pub fn mentat(game: &Game) -> MentatView {
    use treachery_engine::treachery_card_manager::TreacheryCardManager;
    MentatView {
        treachery_cards: TreacheryCardManager::get_cards_in_play(game).map(|c| c.id()).collect(),
        richese_cards: if game.is_playing(Faction::Richese) { TreacheryCardManager::get_richese_cards().iter().map(|c| c.id()).collect() } else { vec![] },
        resource_cards: Map::get_resource_cards_in_play(game).map(|c| c.skin_id()).collect(),
    }
}
