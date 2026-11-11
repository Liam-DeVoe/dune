//! `NexusPlayedComponent`: playing the Nexus card the player holds, as Betrayal, Cunning or
//! Secret Ally. The component's switch picks one case by the card's faction, the uses allowed
//! now and the phase; [`case`] is that switch, and each case draws one line and possibly inputs.

use treachery_engine::express;
use treachery_engine::net_enum::NetEnum;
use treachery_engine::prelude::*;

use crate::build::event;
use crate::describe::Describer;
use crate::expr::message;
use crate::schema::*;

use super::other::{card_ids, describe, enum_option, faction_options, hero_options, picker, required};

/// The branch of the component's `switch (Player.Nexus)` that applies, in the switch's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Case {
    AtreidesBetrayal,
    AtreidesPrescience,
    HarkonnenBetrayal,
    HarkonnenCunning,
    HarkonnenSecretAlly,
    FremenBetrayalBlow,
    FremenBetrayalMove,
    FremenCunning,
    FremenSecretAllyBlow,
    FremenSecretAllyRevival,
    EmperorBetrayalBidding,
    EmperorBetrayalBattle,
    EmperorCunning,
    GuildBetrayal,
    GuildCunning,
    GuildSecretAlly,
    BeneGesseritBetrayal,
    BeneGesseritCunning,
    IxianBetrayalSelecting,
    IxianBetrayalSwapping,
    IxianCunning,
    TleilaxuBetrayal,
    TleilaxuCunning,
    TleilaxuSecretAlly,
    ChoamBetrayal,
    ChoamSecretAllyCollection,
    ChoamSecretAllyBattle,
    RicheseBetrayal,
    EcazBetrayal,
    EcazCunning,
    EcazSecretAlly,
    MoritaniBetrayal,
}

fn case(game: &Game, p: PlayerIndex) -> Option<Case> {
    use Case::*;
    let cunning = NexusPlayed::can_use_cunning(game, p);
    let secret_ally = NexusPlayed::can_use_secret_ally(game, p);
    let betrayal = NexusPlayed::can_use_betrayal(game, p);
    let mp = game.current_main_phase;
    let phase = game.current_phase.value();
    let selecting = Phase::IxianSelectingCard.value();
    Some(match game.player(p).nexus {
        Faction::Atreides if betrayal => AtreidesBetrayal,
        Faction::Atreides if cunning || secret_ally => AtreidesPrescience,
        Faction::Harkonnen if betrayal => HarkonnenBetrayal,
        Faction::Harkonnen if cunning => HarkonnenCunning,
        Faction::Harkonnen if secret_ally => HarkonnenSecretAlly,
        Faction::Fremen if betrayal && mp == MainPhase::Blow => FremenBetrayalBlow,
        Faction::Fremen if betrayal && mp == MainPhase::ShipmentAndMove => FremenBetrayalMove,
        Faction::Fremen if cunning => FremenCunning,
        Faction::Fremen if secret_ally && mp == MainPhase::Blow => FremenSecretAllyBlow,
        Faction::Fremen if secret_ally && mp == MainPhase::Resurrection => FremenSecretAllyRevival,
        Faction::Emperor if betrayal && mp == MainPhase::Bidding => EmperorBetrayalBidding,
        Faction::Emperor if betrayal && mp == MainPhase::Battle => EmperorBetrayalBattle,
        Faction::Emperor if cunning => EmperorCunning,
        Faction::Guild if betrayal => GuildBetrayal,
        Faction::Guild if cunning => GuildCunning,
        Faction::Guild if secret_ally => GuildSecretAlly,
        Faction::BeneGesserit if betrayal => BeneGesseritBetrayal,
        Faction::BeneGesserit if cunning => BeneGesseritCunning,
        Faction::Ixian if betrayal && phase < selecting => IxianBetrayalSelecting,
        Faction::Ixian if betrayal && phase > selecting => IxianBetrayalSwapping,
        Faction::Ixian if cunning => IxianCunning,
        Faction::Tleilaxu if betrayal => TleilaxuBetrayal,
        Faction::Tleilaxu if cunning => TleilaxuCunning,
        Faction::Tleilaxu if secret_ally => TleilaxuSecretAlly,
        Faction::Choam if betrayal => ChoamBetrayal,
        Faction::Choam if secret_ally && mp == MainPhase::Collection => ChoamSecretAllyCollection,
        Faction::Choam if secret_ally && mp == MainPhase::Battle => ChoamSecretAllyBattle,
        Faction::Richese if betrayal => RicheseBetrayal,
        Faction::Ecaz if betrayal => EcazBetrayal,
        Faction::Ecaz if cunning => EcazCunning,
        Faction::Ecaz if secret_ally => EcazSecretAlly,
        Faction::Moritani if betrayal => MoritaniBetrayal,
        _ => return None,
    })
}

/// `CurrentPick`: which input the map sets.
#[derive(Clone, Copy, PartialEq, Eq)]
enum MapChoice {
    None,
    TleilaxuLocation,
    EcazTerritory,
    MoritaniTerritory,
}

fn map_choice(game: &Game, p: PlayerIndex, e: &NexusPlayed) -> MapChoice {
    match game.player(p).nexus {
        Faction::Tleilaxu if NexusPlayed::can_use_secret_ally(game, p) && e.tleilaxu_number_of_special_forces_in_location > 0 => MapChoice::TleilaxuLocation,
        Faction::Ecaz if NexusPlayed::can_use_betrayal(game, p) => MapChoice::EcazTerritory,
        Faction::Moritani if NexusPlayed::can_use_betrayal(game, p) => MapChoice::MoritaniTerritory,
        _ => MapChoice::None,
    }
}

/// `Revival.ValidRevivedForceLocations(...).Any() ? NumberOfSpecialForcesThatMayBePlacedOnPlanet : 0`.
fn max_special_on_planet(game: &Game, p: PlayerIndex, special: i32) -> i32 {
    if Revival::valid_revived_force_locations(game, p).next().is_some() {
        Revival::number_of_special_forces_that_may_be_placed_on_planet(game, p, special)
    } else {
        0
    }
}

pub(super) fn default_draft(game: &Game, faction: Faction) -> GameEvent {
    let mut e = NexusPlayed::new(game, faction);
    if let Some(p) = game.get_player(Some(faction)) {
        e.faction = game.player(p).nexus;
    }
    event(e)
}

/// The draft as the component holds it after rendering: the inputs of the drawn case adjusted,
/// the map pick dropped unless it is enabled and a target (`MapPicker.Sync`).
pub(super) fn normalize(game: &Game, p: PlayerIndex, draft: &GameEvent) -> GameEvent {
    let GameEvent::NexusPlayed(e) = draft else { return draft.clone() };
    let mut m = (**e).clone();
    m.faction = game.player(p).nexus;
    match case(game, p) {
        Some(Case::AtreidesPrescience) => {
            m.atreides_prescience_aspect = required(m.atreides_prescience_aspect, &Prescience::valid_aspects(game, p), PrescienceAspect::default());
        }
        Some(Case::TleilaxuSecretAlly) => {
            let max = NexusPlayed::valid_tleilaxu_max_amount(game, p, false);
            m.tleilaxu_forces = if max > 0 { picker(m.tleilaxu_forces, 0, max) } else { 0 };
            let max_special = NexusPlayed::valid_tleilaxu_max_amount(game, p, true);
            m.tleilaxu_special_forces = if max_special > 0 { picker(m.tleilaxu_special_forces, 0, max_special) } else { 0 };
            let on_planet = max_special_on_planet(game, p, m.tleilaxu_special_forces);
            m.tleilaxu_number_of_special_forces_in_location =
                if on_planet > 0 { picker(m.tleilaxu_number_of_special_forces_in_location, 0, on_planet) } else { 0 };
            let hero = m.tleilaxu_hero();
            if !(hero.is_some() && Revival::may_assign_skill(game, p, hero)) {
                m.tleilaxu_assign_skill = false;
            }
        }
        Some(Case::ChoamSecretAllyCollection) => {
            let cards = card_ids(NexusPlayed::valid_choam_cards(game, p));
            m.choam_card_id = required(m.choam_card_id, &cards, -1);
        }
        Some(Case::EcazSecretAlly) => {
            let fs: Vec<Faction> = NexusPlayed::valid_ecaz_factions(game).collect();
            m.ecaz_faction = required(m.ecaz_faction, &fs, Faction::None);
        }
        _ => {}
    }
    let pick = map_choice(game, p, &m);
    let territories: Vec<Territory> = match pick {
        MapChoice::EcazTerritory => NexusPlayed::valid_ecaz_territories(game).collect(),
        MapChoice::MoritaniTerritory => NexusPlayed::valid_moritani_territories(game).collect(),
        _ => vec![],
    };
    let ecaz = m.ecaz_territory(game).filter(|t| pick == MapChoice::EcazTerritory && territories.contains(t));
    m.set_ecaz_territory(game, ecaz);
    let moritani = m.moritani_territory(game).filter(|t| pick == MapChoice::MoritaniTerritory && territories.contains(t));
    m.set_moritani_territory(game, moritani);
    let location = m
        .tleilaxu_location(game)
        .filter(|l| pick == MapChoice::TleilaxuLocation && Revival::valid_revived_force_locations(game, p).any(|x| x == *l));
    m.set_tleilaxu_location(game, location);
    event(m)
}

pub(super) fn options(game: &Game, p: PlayerIndex, e: &NexusPlayed, d: &dyn Describer, draft: serde_json::Value) -> NexusPlayedOptions {
    use Case::*;
    let pl = game.player(p);
    let nexus = pl.nexus;
    let kind = if NexusPlayed::can_use_cunning(game, p) {
        "Cunning"
    } else if NexusPlayed::can_use_secret_ally(game, p) {
        "Secret Ally"
    } else {
        "Betrayal"
    };
    let m = |x: Message| Some(message(&x, d));
    let name = |x: treachery_engine::expression::ExpressionElement| describe(d, x);
    let c = case(game, p);
    let text = match c {
        Some(AtreidesBetrayal) => m(express!("Prevent ", name(FactionAdvantage::AtreidesBattlePlanPrescience.into()), "?")),
        Some(AtreidesPrescience) => None,
        Some(HarkonnenBetrayal) => m(express!("Cancel the revealed ", Faction::Harkonnen, " traitor?")),
        Some(HarkonnenCunning) => m(express!("Draw a new Traitor card, then shufle one of your traitors back into the traitor deck?")),
        Some(HarkonnenSecretAlly) => m(express!("Draw two Traitor cards, then shufle two of your traitors back into the traitor deck?")),
        Some(FremenBetrayalBlow) => m(express!("Prevent ", Faction::Fremen, " from riding ", Concept::Monster, "?")),
        Some(FremenBetrayalMove) => m(express!("Prevent ", Faction::Fremen, " moving 2 territories?")),
        Some(FremenCunning) => m(express!("Let forces in another territory where ", Concept::Monster, " appeared ride?")),
        Some(FremenSecretAllyBlow) => m(express!("Avoid ", Concept::Monster, " devouring your forces?")),
        Some(FremenSecretAllyRevival) => m(express!("Get 3 free revivals?")),
        Some(EmperorBetrayalBidding) => m(express!("Force ", Faction::Emperor, " to pay for the next card their ally buys?")),
        Some(EmperorBetrayalBattle) => m(express!("Let ", FactionSpecialForce::Emperor, " count as ", FactionForce::Emperor, " in this battle?")),
        Some(EmperorCunning) => m(express!("Let up to 5 ", FactionForce::Emperor, " count as ", FactionSpecialForce::Emperor, " in this battle?")),
        Some(GuildBetrayal) => m(express!("Gain the payment for this shipment in full?")),
        Some(GuildCunning) => m(express!("Make a second shipment after performing your move?")),
        Some(GuildSecretAlly) => m(express!("Ship at ", Faction::Guild, " rates or perform a shipment cross-planet or to reserves?")),
        Some(BeneGesseritBetrayal) => m(express!("Prevent ", name(FactionAdvantage::BeneGesseritUsingVoice.into()), "?")),
        Some(BeneGesseritCunning) => m(express!("Be able to flip advisor to fighters during ", name(MainPhase::ShipmentAndMove.into()), "?")),
        Some(IxianBetrayalSelecting) => m(express!("Prevent ", name(FactionAdvantage::IxianSelectingCardsOnAuction.into()), "?")),
        Some(IxianBetrayalSwapping) => m(express!("Prevent ", name(FactionAdvantage::IxianSwappingCard.into()), "?")),
        Some(IxianCunning) => m(express!("Let all ", FactionForce::Ixian, " in this battle count full strength?")),
        Some(TleilaxuBetrayal) => m(express!("Cancel the revealed ", Faction::Tleilaxu, " face dancer?")),
        Some(TleilaxuCunning) => m(express!("Replace your revealed face dancers?")),
        Some(TleilaxuSecretAlly) => m(express!("Revive up to 5 forces for ", Payment::of(1), " each and one leader for free?")),
        Some(ChoamBetrayal) => m(express!("Force ", Faction::Choam, " to discard a random card from their hand?")),
        Some(ChoamSecretAllyCollection) => {
            m(express!("Discard a ", name(TreacheryCardType::Useless.into()), " card for ", Payment::of(2), "?"))
        }
        Some(ChoamSecretAllyBattle) => m(express!("Look at a random card not used in battle owned by your opponent?")),
        Some(RicheseBetrayal) => {
            let richese_has_it = game
                .get_player(Some(Faction::Richese))
                .zip(game.card_just_won)
                .is_some_and(|(r, card)| game.player(r).treachery_cards.contains(&card));
            if richese_has_it {
                m(express!("Force ", Faction::Richese, " to discard the ", game.card_just_won, " they just bought?"))
            } else {
                m(express!("Prevent ", Faction::Richese, " from getting paid for a card they sold?"))
            }
        }
        Some(EcazBetrayal) => m(express!("Return all ", Faction::Ecaz, " forces ", FactionForce::Ecaz, " in a territory they occupy with their ally to reserves?")),
        Some(EcazCunning) => m(express!("Gain ", game.vidal(), " this turn?")),
        Some(EcazSecretAlly) => m(express!("Force a faction to reveal whether or not they have any of your leaders as traitor?")),
        Some(MoritaniBetrayal) => m(express!("Remove a Terror token from a stronghold?")),
        None => None,
    };

    let tleilaxu = (c == Some(TleilaxuSecretAlly)).then(|| TleilaxuNexusOptions {
        max_forces: NexusPlayed::valid_tleilaxu_max_amount(game, p, false),
        max_special_forces: NexusPlayed::valid_tleilaxu_max_amount(game, p, true),
        max_special_forces_on_planet: max_special_on_planet(game, p, e.tleilaxu_special_forces),
        locations: Revival::valid_revived_force_locations(game, p).map(|l| l.id()).collect(),
        heroes: hero_options(NexusPlayed::valid_tleilaxu_heroes(game, p)),
        may_assign_skill: e.tleilaxu_hero().is_some() && Revival::may_assign_skill(game, p, e.tleilaxu_hero()),
        cost: NexusPlayed::determine_tleilaxu_cost_int32_int32(e.tleilaxu_forces, e.tleilaxu_special_forces),
    });

    let pick = map_choice(game, p, e);
    let territories: Option<Vec<Territory>> = match pick {
        MapChoice::EcazTerritory => Some(NexusPlayed::valid_ecaz_territories(game).collect()),
        MapChoice::MoritaniTerritory => Some(NexusPlayed::valid_moritani_territories(game).collect()),
        _ => None,
    };
    let locations: Option<Vec<i32>> = match pick {
        MapChoice::TleilaxuLocation => Some(Revival::valid_revived_force_locations(game, p).map(|l| l.id()).collect()),
        MapChoice::None => None,
        _ => territories.as_ref().map(|ts| ts.iter().flat_map(|t| t.locations().iter().map(|l| l.id())).collect()),
    };

    NexusPlayedOptions {
        nexus: nexus.to_string(),
        kind: kind.to_string(),
        text,
        aspects: (c == Some(AtreidesPrescience)).then(|| Prescience::valid_aspects(game, p).into_iter().map(|a| enum_option(d, a)).collect()),
        tleilaxu,
        choam_cards: (c == Some(ChoamSecretAllyCollection)).then(|| card_ids(NexusPlayed::valid_choam_cards(game, p))),
        ecaz_factions: (c == Some(EcazSecretAlly)).then(|| faction_options(NexusPlayed::valid_ecaz_factions(game))),
        territories: territories.map(|ts| ts.iter().map(|t| t.id()).collect()),
        locations,
        map_member: match pick {
            MapChoice::None => None,
            MapChoice::TleilaxuLocation => Some("TleilaxuLocation".to_string()),
            MapChoice::EcazTerritory => Some("EcazTerritory".to_string()),
            MapChoice::MoritaniTerritory => Some("MoritaniTerritory".to_string()),
        },
        draft,
    }
}
