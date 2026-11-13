//! The engine's static data that the web client needs beside the skin: what never changes during
//! a game (locations, territories, leaders, cards, homeworlds, rule groups). The view sends only
//! ids; the client looks the rest up here. Map geometry, names and images come from the skin.
//!
//! `examples/static_data.rs` writes it to `web/public/static/data.json`, and
//! `tests/static_data.rs` fails when that file is stale. The TypeScript type is generated with
//! the view's (`tests/ts_bindings.rs`).

use serde::{Deserialize, Serialize};
use treachery_engine::shared::Shared;
use treachery_engine::identity::IIdentifiable;
use treachery_engine::heroes::leader_manager::LeaderManager;
use treachery_engine::map::LocationKind;
use treachery_engine::game::Participation;
use treachery_engine::game::Player;
use treachery_engine::prelude::*;
use treachery_engine::cards::resource_card::ResourceCard;
use treachery_engine::cards::treachery_card_manager::TreacheryCardManager;
use ts_rs::TS;

fn name<T: ToString>(v: T) -> String {
    v.to_string()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StaticData {
    pub locations: Vec<LocationInfo>,
    pub territories: Vec<TerritoryInfo>,
    pub leaders: Vec<LeaderInfo>,
    pub treachery_cards: Vec<TreacheryCardInfo>,
    pub resource_cards: Vec<ResourceCardInfo>,
    pub homeworlds: Vec<HomeworldInfo>,
    pub factions: Vec<FactionInfo>,
    /// `Game.GetRuleGroup(rule)` for every rule (the turn popover groups the rules by it).
    pub rules: Vec<RuleInfo>,
    /// The locations and territories `Map` names, which the client draws specially.
    pub map: NamedPlaces,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LocationInfo {
    pub id: i32,
    pub territory: i32,
    pub orientation: String,
    /// `Sector` of an ordinary location. Attached locations (the Hidden Mobile Stronghold and the
    /// discovered strongholds) take their host's sector, which is in the view
    /// (`board.attached`); here they have -1.
    pub sector: i32,
    pub spice_blow_amount: i32,
    #[ts(type = "DiscoveryTokenType")]
    pub discovery_token_type: String,
    /// `normal`, `homeworld`, `hiddenMobileStronghold` or `discovered`.
    pub kind: String,
    /// For a discovered stronghold, the discovery token that reveals it.
    #[ts(type = "DiscoveryToken | null")]
    pub discovery: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerritoryInfo {
    pub id: i32,
    pub is_stronghold: bool,
    pub is_homeworld: bool,
    pub is_discovery: bool,
    pub is_protected_from_storm: bool,
    pub is_protected_from_worm: bool,
    pub has_reduced_shipping_cost: bool,
    #[ts(type = "StrongholdAdvantage")]
    pub advantage: String,
    pub middle_location: i32,
    pub locations: Vec<i32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LeaderInfo {
    pub id: i32,
    #[ts(type = "Faction")]
    pub faction: String,
    pub value: i32,
    #[ts(type = "HeroType")]
    pub hero_type: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TreacheryCardInfo {
    pub id: i32,
    pub skin_id: i32,
    #[ts(type = "TreacheryCardType")]
    pub card_type: String,
    pub is_weapon: bool,
    pub is_defense: bool,
    pub is_useless: bool,
    /// The rules that put the card in play (`TreacheryCard.Rules`).
    #[ts(type = "Array<Rule>")]
    pub rules: Vec<String>,
}

/// A resource card, by `SkinId` (the id the view uses).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCardInfo {
    pub skin_id: i32,
    /// The location where its spice blows.
    pub location: Option<i32>,
    pub discovery_location: Option<i32>,
    pub is_shai_hulud: bool,
    pub is_sand_trout: bool,
    pub is_great_maker: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HomeworldInfo {
    #[ts(type = "World")]
    pub world: String,
    pub location: i32,
    #[ts(type = "Faction")]
    pub faction: String,
    pub is_home_of_normal_forces: bool,
    pub is_home_of_special_forces: bool,
    pub threshold: i32,
}

/// `Player.Force` and `Player.SpecialForce` of a faction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FactionInfo {
    #[ts(type = "Faction")]
    pub faction: String,
    #[ts(type = "FactionForce")]
    pub force: String,
    #[ts(type = "FactionSpecialForce")]
    pub special_force: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RuleInfo {
    #[ts(type = "Rule")]
    pub rule: String,
    #[ts(type = "RuleGroup")]
    pub group: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NamedPlaces {
    pub polar_sink: i32,
    pub arrakeen: i32,
    pub carthag: i32,
    pub tueks_sietch: i32,
    pub hidden_mobile_stronghold: i32,
    /// Territories.
    pub shield_wall: i32,
    pub imperial_basin: i32,
    pub rock_outcroppings: i32,
}

pub fn static_data() -> StaticData {
    let game = Game::new_int32_participation(Game::LATEST_VERSION, &Shared::new(Participation::default()));
    let map = &game.map;

    let locations = map
        .locations(true)
        .map(|l| {
            let (kind, discovery) = match &l.data().kind {
                LocationKind::Normal => ("normal", None),
                LocationKind::Homeworld(_) => ("homeworld", None),
                LocationKind::HiddenMobileStronghold => ("hiddenMobileStronghold", None),
                LocationKind::Discovered { discovery } => ("discovered", Some(name(*discovery))),
            };
            LocationInfo {
                id: l.id(),
                territory: l.territory().id(),
                orientation: l.orientation().to_string(),
                sector: l.sector(map),
                spice_blow_amount: l.spice_blow_amount(),
                discovery_token_type: name(l.discovery_token_type()),
                kind: kind.to_string(),
                discovery,
            }
        })
        .collect();

    let territories = map
        .territories(true)
        .map(|t| TerritoryInfo {
            id: t.id(),
            is_stronghold: t.is_stronghold(),
            is_homeworld: t.is_homeworld(),
            is_discovery: t.is_discovery(),
            is_protected_from_storm: t.is_protected_from_storm(),
            is_protected_from_worm: t.is_protected_from_worm(),
            has_reduced_shipping_cost: t.has_reduced_shipping_cost(),
            advantage: name(t.advantage()),
            middle_location: t.middle_location().id(),
            locations: t.locations().iter().map(|l| l.id()).collect(),
        })
        .collect();

    let leaders = LeaderManager::leaders()
        .iter()
        .map(|l| LeaderInfo { id: l.id(), faction: name(l.faction()), value: l.value(), hero_type: name(l.hero_type()) })
        .collect();

    let treachery_cards = TreacheryCardManager::get_cards_in_and_outside_play()
        .iter()
        .map(|c| TreacheryCardInfo {
            id: c.id(),
            skin_id: c.skin_id(),
            card_type: name(c.type_()),
            is_weapon: c.is_weapon(),
            is_defense: c.is_defense(),
            is_useless: c.is_useless(),
            rules: c.rules().iter().map(|r| name(*r)).collect(),
        })
        .collect();

    let resource_cards = ResourceCard::get_resource_cards_in_and_outside_play()
        .iter()
        .map(|c| ResourceCardInfo {
            skin_id: c.skin_id(),
            location: c.location().map(|l| l.id()),
            discovery_location: c.discovery_location().map(|l| l.id()),
            is_shai_hulud: c.is_shai_hulud(),
            is_sand_trout: c.is_sand_trout(),
            is_great_maker: c.is_great_maker(),
        })
        .collect();

    let homeworlds = map
        .homeworlds()
        .map(|h| HomeworldInfo {
            world: name(h.world()),
            location: h.location().id(),
            faction: name(h.faction()),
            is_home_of_normal_forces: h.is_home_of_normal_forces(),
            is_home_of_special_forces: h.is_home_of_special_forces(),
            threshold: h.threshold(),
        })
        .collect();

    let factions = Faction::INFO
        .declared
        .iter()
        .map(|(_, v)| Faction(*v))
        .filter(|f| *f != Faction::None)
        .map(|f| {
            let p = Player::new(f);
            FactionInfo { faction: name(f), force: name(p.force()), special_force: name(p.special_force()) }
        })
        .collect();

    let rules = Rule::INFO.declared.iter().map(|(_, v)| Rule(*v)).map(|r| RuleInfo { rule: name(r), group: name(Game::get_rule_group(r)) }).collect();

    StaticData {
        locations,
        territories,
        leaders,
        treachery_cards,
        resource_cards,
        homeworlds,
        factions,
        rules,
        map: NamedPlaces {
            polar_sink: map.polar_sink().id(),
            arrakeen: map.arrakeen().id(),
            carthag: map.carthag().id(),
            tueks_sietch: map.tueks_sietch().id(),
            hidden_mobile_stronghold: map.hidden_mobile_stronghold().location().id(),
            shield_wall: map.shield_wall().id(),
            imperial_basin: map.imperial_basin().id(),
            rock_outcroppings: map.rock_outcroppings().id(),
        },
    }
}
