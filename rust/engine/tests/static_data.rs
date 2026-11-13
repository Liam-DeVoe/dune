//! Compares the Rust static data and pathfinding with the C# runtime.
//!
//! tests/golden/static_data.json is written by csharp-ref (a small C# program
//! referencing Treachery.Shared). Each test rebuilds one section of that JSON
//! from the Rust port and compares it value by value.
//!
//! Regenerate the golden file after a C# change:
//!   cd rust/engine/csharp-ref && dotnet run -c Release -- ../tests/golden/static_data.json

use std::cell::Cell;
use std::sync::OnceLock;

use serde_json::{json, Value};
use treachery_engine::error::catch_panic;
use treachery_engine::cards::deck::Deck;
use treachery_engine::enumerations::*;
use treachery_engine::compat::random_ext::RandomExt;
use treachery_engine::identity::IFetcher;
use treachery_engine::heroes::hero::Hero;
use treachery_engine::heroes::leader_manager::LeaderManager;
use treachery_engine::map::location::Location;
use treachery_engine::random::SharedRandom;
use treachery_engine::map::{Map, MapGame};
use treachery_engine::messages::message::BattleFactKind;
use treachery_engine::enum_type::{EnumTypeInfo, GameEnum};
use treachery_engine::cards::resource_card::ResourceCard;
use treachery_engine::map::territory::Territory;
use treachery_engine::cards::treachery_card::TreacheryCard;
use treachery_engine::cards::treachery_card_manager::TreacheryCardManager;

fn golden() -> &'static Value {
    static G: OnceLock<Value> = OnceLock::new();
    G.get_or_init(|| {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/static_data.json");
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    })
}

/// Panics with the path of the first difference. A C# exception (`"exception:<Type>:<Message>"`)
/// matches any panic (`"exception"`).
fn assert_same(path: &str, expected: &Value, actual: &Value) {
    match (expected, actual) {
        (Value::Object(e), Value::Object(a)) => {
            for (k, ev) in e {
                match a.get(k) {
                    Some(av) => assert_same(&format!("{path}.{k}"), ev, av),
                    None => panic!("{path}.{k}: missing in Rust (C#: {ev})"),
                }
            }
            for k in a.keys() {
                assert!(e.contains_key(k), "{path}.{k}: only in Rust");
            }
        }
        (Value::Array(e), Value::Array(a)) => {
            for (i, (ev, av)) in e.iter().zip(a).enumerate() {
                assert_same(&format!("{path}[{i}]"), ev, av);
            }
            assert_eq!(e.len(), a.len(), "{path}: length differs");
        }
        (Value::String(e), _) if e.starts_with("exception:") => assert_eq!(actual, "exception", "{path}: differs (C#: {e})"),
        _ => assert_eq!(expected, actual, "{path}: differs"),
    }
}

/// The C# dumper's `Try`: the value, `"null"` for null, or `"exception"` when the lookup
/// panics. The golden holds `"exception:<Type>:<Message>"` where C# threw; [`assert_same`]
/// compares that as `"exception"`, since the engine no longer matches .NET exception text.
fn try_value(f: impl FnOnce() -> Option<i32>) -> Value {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(Some(v)) => json!(v),
        Ok(None) => json!("null"),
        Err(_) => json!("exception"),
    }
}

fn ids<T: Into<Location> + Copy>(items: impl IntoIterator<Item = T>) -> Value {
    Value::Array(items.into_iter().map(|l| json!(l.into().id())).collect())
}

fn tids(items: impl IntoIterator<Item = Territory>) -> Value {
    Value::Array(items.into_iter().map(|t| json!(t.id())).collect())
}

fn opt_id(l: Option<Location>) -> Value {
    l.map_or(Value::Null, |l| json!(l.id()))
}

/// A `Game` with no players: what MapGame needs for the static tests.
struct StubGame {
    sector_in_storm: Cell<i32>,
    history_count: Cell<i32>,
}

impl StubGame {
    fn new() -> Self {
        StubGame { sector_in_storm: Cell::new(-1), history_count: Cell::new(0) }
    }
}

impl MapGame for StubGame {
    fn sector_in_storm(&self) -> i32 {
        self.sector_in_storm.get()
    }
    fn history_count(&self) -> i32 {
        self.history_count.get()
    }
    fn applicable(&self, _rule: Rule) -> bool {
        false
    }
    fn forces_battalion_factions(&self) -> Vec<(Location, Vec<Faction>)> {
        Vec::new()
    }
    fn player_ally(&self, _f: Faction) -> Option<Faction> {
        None
    }
    fn nr_of_occupants_excluding_faction(&self, _l: Location, _f: Faction) -> i32 {
        0
    }
    fn current_blocked_territories(&self) -> Vec<Territory> {
        Vec::new()
    }
}

// ---------------------------------------------------------------- enums

fn dump_enum<T: GameEnum>(info: &EnumTypeInfo) -> Value {
    let to_string: Vec<Value> =
        [-1i64, 1, 3, 5, 7, 11, 999999].iter().map(|v| json!(format!("{v}={}", info.format(*v as i32)))).collect();
    json!({
        "name": info.name,
        "flags": info.is_flags,
        "underlying": "Int32",
        "declared": info.declared.iter().map(|(n, v)| json!({"name": n, "value": v})).collect::<Vec<_>>(),
        "get_values": T::ALL.iter().map(|v| json!(v.value())).collect::<Vec<_>>(),
        "get_names": T::ALL.iter().map(|v| json!(info.name_of(v.value()).unwrap())).collect::<Vec<_>>(),
        "to_string": to_string,
    })
}

/// Every enum in the C# golden file must have a Rust counterpart: the loop
/// runs over the C# entries, so an enum Rust lacks fails instead of being
/// skipped. Add new enums to `rust_enums`.
#[test]
fn enums() {
    macro_rules! all {
        ($($t:ident),*) => { vec![$((stringify!($t), dump_enum::<$t>(<$t as GameEnum>::INFO))),*] };
    }
    let rust = all!(
        Faction, Nexus, Ambassador, TechToken, MainPhase, MainPhaseMoment, ChoamEconomicsStatus, Phase, Milestone,
        FactionAdvantage, FactionForce, FactionSpecialForce, Ruleset, DiscoveryToken, DiscoveryTokenType, Rule,
        RuleGroup, TreacheryCardType, Concept, WinMethod, HeroType, AuctionType, JuiceType, LeaderSkill,
        StrongholdAdvantage, World, CaptureDecision, DealType, TerrorType, IntrusionType, VidalMoment,
        ClairVoyanceAnswer, ClairvoyanceQuestion, PrescienceAspect, ShipmentPermission, UserStatus,
        SubscriptionType, GameListScope, ShipmentType, AutomationRuleType, ItemAction, BattleFactKind, AsideKind,
        NexusAllianceChange, ShipKind, BeneGesseritReactionKind, ErrorType
    );
    let mut missing = Vec::new();
    for expected in golden()["enums"].as_array().unwrap() {
        let name = expected["name"].as_str().unwrap();
        match rust.iter().find(|(n, _)| *n == name) {
            Some((_, value)) => assert_same(&format!("enums.{name}"), expected, value),
            None => missing.push(name),
        }
    }
    assert!(missing.is_empty(), "C# enums with no Rust port (add them to this test once ported): {missing:?}");
}

#[test]
fn enum_ordering_and_serde() {
    assert!(Faction::Fremen < Faction::Atreides);
    assert!(Rule::AdvancedCombat < Rule::BasicTreacheryCards);
    assert_eq!(serde_json::to_string(&Faction::Atreides).unwrap(), "20");
    assert_eq!(serde_json::from_str::<Faction>("13").unwrap(), Faction(13));
    assert_eq!(format!("{:?}", Faction::Atreides), "Faction::Atreides");
    assert_eq!(Faction::Moritani.to_nexus(), Nexus::Moritani);
    assert!(catch_panic(|| Faction(13).to_nexus()).is_err());
}

// ---------------------------------------------------------------- map

fn dump_location(m: &Map, l: Location) -> Value {
    let mut o = json!({
        "id": l.id(),
        "kind": l.type_name(),
        "sector": l.sector(m),
        "territory": l.territory().id(),
        "visible": l.visible(m),
        "neighbours": ids(l.neighbours(m).iter().copied()),
        "orientation": l.orientation(),
        "spice_blow_amount": l.spice_blow_amount(),
        "discovery_token_type": l.discovery_token_type().0,
        "is_stronghold": l.is_stronghold(),
        "is_homeworld": l.is_homeworld(),
        "is_protected_from_storm": l.is_protected_from_storm(),
        "advantage": l.advantage().0,
        "hash": l.id(),
    });
    if let Some(hw) = l.as_homeworld() {
        o["world"] = json!(hw.world().0);
        o["faction"] = json!(hw.faction().0);
        o["threshold"] = json!(hw.threshold());
        o["resource_amount"] = json!(hw.resource_amount());
        o["is_home_of_normal_forces"] = json!(hw.is_home_of_normal_forces());
        o["is_home_of_special_forces"] = json!(hw.is_home_of_special_forces());
        o["battle_bonus_high"] = json!(hw.battle_bonus_and_lasgun_shield_limit_at_high_threshold());
        o["battle_bonus_low"] = json!(hw.battle_bonus_and_lasgun_shield_limit_at_low_threshold());
    }
    if let Some(al) = l.as_attached_location() {
        o["attached_to"] = opt_id(al.attached_to_location(m));
    }
    if let Some(dl) = l.as_discovered_location() {
        o["discovery"] = json!(dl.discovery().0);
    }
    o
}

fn dump_territory(m: &Map, t: Territory) -> Value {
    json!({
        "id": t.id(),
        "skin_id": t.skin_id(),
        "is_homeworld": t.is_homeworld(),
        "has_reduced_shipping_cost": t.has_reduced_shipping_cost(),
        "is_discovery": t.is_discovery(),
        "is_discovered_stronghold": t.is_discovered_stronghold(),
        "is_visible": t.is_visible(m),
        "is_stronghold": t.is_stronghold(),
        "is_protected_from_storm": t.is_protected_from_storm(),
        "is_protected_from_worm": t.is_protected_from_worm(),
        "advantage": t.advantage().0,
        "locations": ids(t.locations().iter().copied()),
        "middle_location": t.middle_location().id(),
        "resource_blow_location": opt_id(t.resource_blow_location()),
        "discovery_token_location": opt_id(t.discovery_token_location()),
    })
}

fn dump_map(m: &Map) -> Value {
    let named = json!({
        "Arrakeen": format!("location:{}", m.arrakeen().id()),
        "BightOfTheCliff": format!("territory:{}", m.bight_of_the_cliff().id()),
        "BrokenLand": format!("territory:{}", m.broken_land().id()),
        "Carthag": format!("location:{}", m.carthag().id()),
        "CielagoEast": format!("territory:{}", m.cielago_east().id()),
        "CielagoWest": format!("territory:{}", m.cielago_west().id()),
        "Cistern": format!("location:{}", m.cistern().id()),
        "FalseWallEast": format!("territory:{}", m.false_wall_east().id()),
        "FalseWallSouth": format!("territory:{}", m.false_wall_south().id()),
        "FalseWallWest": format!("territory:{}", m.false_wall_west().id()),
        "FuneralPlain": format!("location:{}", m.funeral_plain().id()),
        "GaraKulon": format!("territory:{}", m.gara_kulon().id()),
        "HabbanyaErg": format!("territory:{}", m.habbanya_erg().id()),
        "HabbanyaSietch": format!("location:{}", m.habbanya_sietch().id()),
        "HaggaBasin": format!("territory:{}", m.hagga_basin().id()),
        "HiddenMobileStronghold": format!("location:{}", m.hidden_mobile_stronghold().id()),
        "HoleInTheRock": format!("territory:{}", m.hole_in_the_rock().id()),
        "ImperialBasin": format!("territory:{}", m.imperial_basin().id()),
        "Jacurutu": format!("location:{}", m.jacurutu().id()),
        "Meridan": format!("territory:{}", m.meridan().id()),
        "OldGap": format!("territory:{}", m.old_gap().id()),
        "PastyMesa": format!("territory:{}", m.pasty_mesa().id()),
        "PlasticBasin": format!("territory:{}", m.plastic_basin().id()),
        "PolarSink": format!("location:{}", m.polar_sink().id()),
        "ProcessingStation": format!("location:{}", m.processing_station().id()),
        "RockOutcroppings": format!("territory:{}", m.rock_outcroppings().id()),
        "ShieldWall": format!("territory:{}", m.shield_wall().id()),
        "Shrine": format!("location:{}", m.shrine().id()),
        "SietchTabr": format!("location:{}", m.sietch_tabr().id()),
        "SihayaRidge": format!("territory:{}", m.sihaya_ridge().id()),
        "TestingStation": format!("location:{}", m.testing_station().id()),
        "TheGreatFlat": format!("location:{}", m.the_great_flat().id()),
        "TheGreaterFlat": format!("location:{}", m.the_greater_flat().id()),
        "TheMinorErg": format!("territory:{}", m.the_minor_erg().id()),
        "Tsimpo": format!("territory:{}", m.tsimpo().id()),
        "TueksSietch": format!("location:{}", m.tueks_sietch().id()),
        "WindPass": format!("territory:{}", m.wind_pass().id()),
        "WindPassNorth": format!("territory:{}", m.wind_pass_north().id()),
    });

    let mut get_home_world = serde_json::Map::new();
    for w in World::ALL {
        get_home_world.insert(w.0.to_string(), opt_id(m.get_home_world(*w).map(|h| h.location())));
    }
    let mut get_discovery_stronghold = serde_json::Map::new();
    for d in DiscoveryToken::ALL {
        get_discovery_stronghold.insert(d.0.to_string(), try_value(|| Some(m.get_discovery_stronghold(*d).id())));
    }

    json!({
        "number_of_sectors": Map::NUMBER_OF_SECTORS,
        "locations": m.locations(true).map(|l| dump_location(m, l)).collect::<Vec<_>>(),
        "territories": m.territories(true).map(|t| dump_territory(m, t)).collect::<Vec<_>>(),
        "locations_true": ids(m.locations(true)),
        "locations_false": ids(m.locations(false)),
        "territories_true": tids(m.territories(true)),
        "territories_false": tids(m.territories(false)),
        "strongholds": ids(m.strongholds().collect::<Vec<_>>()),
        "homeworlds": ids(m.homeworlds()),
        "named": named,
        "get_home_world": get_home_world,
        "get_discovery_stronghold": get_discovery_stronghold,
        "territory_lookup": (-2..=62).map(|i| try_value(|| m.territory_lookup().find(i).map(|t| t.id()))).collect::<Vec<_>>(),
        "location_lookup": (-2..=106).map(|i| try_value(|| m.location_lookup().find(i).map(|l| l.id()))).collect::<Vec<_>>(),
    })
}

#[test]
fn map() {
    assert_same("map", &golden()["map"], &dump_map(&Map::new()));
}

#[test]
fn map_attached() {
    let mut result = Vec::new();
    {
        let version = treachery_engine::game::Game::LATEST_VERSION;
        let g = StubGame::new();
        let mut m = Map::new();
        let first_of = |t: Territory| t.locations()[0];
        let steps: Vec<(&str, Location, Location)> = vec![
            ("hms->arrakeen", m.hidden_mobile_stronghold().location(), m.arrakeen()),
            ("hms->polar sink", m.hidden_mobile_stronghold().location(), m.polar_sink()),
            ("hms->first of false wall east", m.hidden_mobile_stronghold().location(), first_of(m.false_wall_east())),
            ("jacurutu->great flat", m.jacurutu().location(), m.the_great_flat()),
            ("shrine->first of shield wall", m.shrine().location(), first_of(m.shield_wall())),
            ("hms->great flat", m.hidden_mobile_stronghold().location(), m.the_great_flat()),
            ("cistern->carthag", m.cistern().location(), m.carthag()),
            ("hms->carthag", m.hidden_mobile_stronghold().location(), m.carthag()),
            ("hms->carthag again", m.hidden_mobile_stronghold().location(), m.carthag()),
        ];
        for (name, which, to) in steps {
            which.as_attached_location().unwrap().point_at(&mut m, to);
            let snapshot = dump_map(&m);
            let mut neighbours = Vec::new();
            for l in m.locations(true) {
                for d in [1, 2] {
                    neighbours.push(ids(m.find_neighbours(l, d, false, Faction::None, &g, false)));
                }
            }
            let hms: Vec<Value> =
                m.locations(true).map(|l| ids(m.find_neighbours_for_hms_movement(l, 1, false, 9))).collect();
            result.push(json!({
                "version": version,
                "step": name,
                "locations": m.locations(true).map(|l| json!([
                    l.sector(&m), l.visible(&m), ids(l.neighbours(&m).iter().copied()),
                    l.as_attached_location().and_then(|a| a.attached_to_location(&m)).map(|a| a.id())
                ])).collect::<Vec<_>>(),
                "territories_visible": m.territories(true).map(|t| json!(t.is_visible(&m))).collect::<Vec<_>>(),
                "strongholds": snapshot["strongholds"].clone(),
                "find_neighbours": neighbours,
                "find_neighbours_for_hms_movement": hms,
            }));
        }
    }
    assert_same("map_attached", &golden()["map_attached"], &Value::Array(result));
}

#[test]
fn pathfinding() {
    let g = StubGame::new();
    let m = Map::new();
    let locations: Vec<Location> = m.locations(true).collect();
    let sectors = [-1, 4, 9, 13];

    let mut hms = Vec::new();
    let mut within = Vec::new();
    let mut neighbours = Vec::new();
    for sector in sectors {
        g.sector_in_storm.set(sector);
        for &l in &locations {
            for ignore_storm in [false, true] {
                within.push(ids(m.find_neighbours_within_territory(l, ignore_storm, sector)));
                for d in 0..=4 {
                    hms.push(ids(m.find_neighbours_for_hms_movement(l, d, ignore_storm, sector)));
                    neighbours.push(ids(m.find_neighbours(l, d, ignore_storm, Faction::None, &g, false)));
                }
            }
        }
    }

    let mut shortest = Vec::new();
    for sector in [-1, 9] {
        g.sector_in_storm.set(sector);
        for &from in &locations {
            for &to in &locations {
                shortest.push(match m.find_first_shortest_path(from, to, false, Faction::Atreides, &g) {
                    Some(p) => ids(p),
                    None => json!("null"),
                });
            }
        }
    }

    let starts = [0, 5, 9, 23, 42, 57, 68, 74, 86];
    let mut paths = Vec::new();
    for sector in [-1, 9] {
        g.sector_in_storm.set(sector);
        for s in starts {
            for &to in &locations {
                for d in [1, 2] {
                    for ignore_storm in [false, true] {
                        let found = m.find_paths(locations[s], to, d, ignore_storm, Faction::Atreides, &g);
                        paths.push(Value::Array(found.into_iter().map(ids).collect()));
                    }
                }
            }
        }
    }

    // Stale cache within one History.Count.
    let g2 = StubGame::new();
    let m2 = Map::new();
    let mut cache = Vec::new();
    g2.sector_in_storm.set(-1);
    cache.push(ids(m2.find_neighbours(m2.arrakeen(), 2, false, Faction::Atreides, &g2, true)));
    g2.sector_in_storm.set(9);
    cache.push(ids(m2.find_neighbours(m2.arrakeen(), 2, false, Faction::Atreides, &g2, true)));
    cache.push(ids(m2.find_neighbours(m2.arrakeen(), 2, false, Faction::Atreides, &g2, false)));
    cache.push(ids(m2.find_neighbours(m2.arrakeen(), 2, false, Faction::Harkonnen, &g2, true)));
    g2.history_count.set(1);
    cache.push(ids(m2.find_neighbours(m2.arrakeen(), 2, false, Faction::Atreides, &g2, true)));

    let actual = json!({
        "sectors": sectors,
        "within_territory": within,
        "hms_movement": hms,
        "find_neighbours": neighbours,
        "first_shortest_path": shortest,
        "path_starts": starts,
        "find_paths": paths,
        "cache": cache,
    });
    assert_same("pathfinding", &golden()["pathfinding"], &actual);
}

#[test]
fn resource_cards() {
    let actual: Vec<Value> = ResourceCard::get_resource_cards_in_and_outside_play()
        .iter()
        .map(|c| {
            json!({
                "skin_id": c.skin_id(),
                "is_sand_trout": c.is_sand_trout(),
                "is_great_maker": c.is_great_maker(),
                "location": opt_id(c.location()),
                "discovery_location": opt_id(c.discovery_location()),
                "is_shai_hulud": c.is_shai_hulud(),
                "is_spice_blow": c.is_spice_blow(),
                "is_discovery": c.is_discovery(),
                "territory": c.territory().map_or(Value::Null, |t| json!(t.id())),
            })
        })
        .collect();
    assert_same("resource_cards", &golden()["resource_cards"], &Value::Array(actual));
}

// ---------------------------------------------------------------- heroes and cards

fn dump_hero(h: Hero, opponents: &[Hero]) -> Value {
    let mut against = vec![json!(h.value_in_combat_against(None))];
    against.extend(opponents.iter().map(|o| json!(h.value_in_combat_against(Some(*o)))));
    json!({
        "id": h.id(),
        "skin_id": h.skin_id(),
        "value": h.value(),
        "faction": h.faction().0,
        "hero_type": h.hero_type().0,
        "cost_to_revive": h.cost_to_revive(),
        "value_in_combat_against": against,
        "is_traitor": opponents.iter().map(|o| json!(h.is_traitor(*o))).collect::<Vec<_>>(),
        "is_face_dancer": opponents.iter().map(|o| json!(h.is_face_dancer(*o))).collect::<Vec<_>>(),
        "is": Faction::ALL.iter().map(|f| json!(h.is(*f))).collect::<Vec<_>>(),
    })
}

#[test]
fn leaders() {
    let leaders = LeaderManager::leaders();
    let cards = TreacheryCardManager::items();
    let opponents = [
        Hero::Leader(leaders[0]),
        Hero::Leader(leaders[35]),
        Hero::Leader(leaders[56]),
        LeaderManager::messiah(),
        Hero::TreacheryCard(cards[17]),
        Hero::TreacheryCard(cards[0]),
    ];
    let mut by_faction = serde_json::Map::new();
    for f in Faction::ALL {
        by_faction.insert(f.0.to_string(), json!(LeaderManager::get_leaders(*f).map(|l| l.id()).collect::<Vec<_>>()));
    }
    let lookup_ids: Vec<i32> = (-3..107).chain(995..1070).collect();
    let actual = json!({
        "first_id": LeaderManager::FIRST_ID,
        "leaders": leaders.iter().map(|l| dump_hero(Hero::Leader(*l), &opponents)).collect::<Vec<_>>(),
        "messiah": dump_hero(LeaderManager::messiah(), &opponents),
        "get_leaders": by_faction,
        "lookup_ids": lookup_ids,
        "hero_lookup": lookup_ids.iter().map(|i| try_value(|| LeaderManager::hero_lookup().find(*i).map(|h| h.id()))).collect::<Vec<_>>(),
        "leader_lookup": lookup_ids.iter().map(|i| try_value(|| LeaderManager::leader_lookup().find(*i).map(|h| h.id()))).collect::<Vec<_>>(),
    });
    assert_same("leaders", &golden()["leaders"], &actual);
}

#[test]
fn cards() {
    let leaders = LeaderManager::leaders();
    let cards = TreacheryCardManager::items();
    let opponents = [
        Hero::Leader(leaders[0]),
        Hero::Leader(leaders[35]),
        LeaderManager::messiah(),
        Hero::TreacheryCard(cards[17]),
        Hero::TreacheryCard(cards[0]),
    ];
    let items: Vec<Value> = cards
        .iter()
        .map(|&c| {
            let mut o = dump_hero(Hero::TreacheryCard(c), &opponents);
            o["type"] = json!(c.type_().0);
            o["rules"] = json!(c.rules().iter().map(|r| r.0).collect::<Vec<_>>());
            o["is_poison_weapon"] = json!(c.is_poison_weapon());
            o["is_projectile_weapon"] = json!(c.is_projectile_weapon());
            o["is_poison_defense"] = json!(c.is_poison_defense());
            o["is_non_antidote_poison_defense"] = json!(c.is_non_antidote_poison_defense());
            o["is_shield"] = json!(c.is_shield());
            o["is_projectile_defense"] = json!(c.is_projectile_defense());
            o["is_laser"] = json!(c.is_laser());
            o["is_poison_tooth"] = json!(c.is_poison_tooth());
            o["is_artillery"] = json!(c.is_artillery());
            o["is_rock_melter"] = json!(c.is_rock_melter());
            o["is_mirror_weapon"] = json!(c.is_mirror_weapon());
            o["is_portable_antidote"] = json!(c.is_portable_antidote());
            o["is_weapon"] = json!(c.is_weapon());
            o["is_defense"] = json!(c.is_defense());
            o["is_useless"] = json!(c.is_useless());
            o["is_mercenary"] = json!(c.is_mercenary());
            o["is_atreides"] = json!(c.is_atreides());
            o
        })
        .collect();

    // GroupBy(Type).Select(First): first card of each type, in order of first appearance.
    let mut reps: Vec<TreacheryCard> = Vec::new();
    for &c in cards {
        if !reps.iter().any(|r| r.type_() == c.type_()) {
            reps.push(c);
        }
    }
    let mut countered = Vec::new();
    for &w in &reps {
        for &d in &reps {
            let mut s = String::new();
            for c in std::iter::once(None).chain(reps.iter().map(|r| Some(*r))) {
                s.push(if w.is_mirror_weapon() && c.is_some_and(|c| c.is_mirror_weapon()) {
                    'S'
                } else if w.countered_by(Some(d), c) {
                    '1'
                } else {
                    '0'
                });
            }
            countered.push(json!(s));
        }
    }

    let mut by_rule = serde_json::Map::new();
    for r in Rule::ALL {
        by_rule.insert(
            r.0.to_string(),
            json!(TreacheryCardManager::get_cards_in_play_by_rule(*r).map(|c| c.id()).collect::<Vec<_>>()),
        );
    }

    let actual = json!({
        "items": items,
        "countered_by_reps": reps.iter().map(|c| c.id()).collect::<Vec<_>>(),
        "countered_by": countered,
        "get_cards_in_play_by_rule": by_rule,
        "richese_cards": TreacheryCardManager::get_richese_cards().iter().map(|c| c.id()).collect::<Vec<_>>(),
        "lookup": (-3..=105).map(|i| try_value(|| TreacheryCardManager::lookup().find(i).map(|c| c.id()))).collect::<Vec<_>>(),
        "get": (-3..=105).map(|i| try_value(|| TreacheryCardManager::get(i).map(|c| c.id()))).collect::<Vec<_>>(),
        "constants": [TreacheryCard::NONE, TreacheryCard::UNKNOWN, TreacheryCardManager::CARD_BALISET,
            TreacheryCardManager::CARD_JUBBA_CLOAK, TreacheryCardManager::CARD_KULON, TreacheryCardManager::CARD_LALALA,
            TreacheryCardManager::CARD_TRIP_TO_GAMONT, TreacheryCardManager::CARD_KULL_WAHAD],
    });
    assert_same("cards", &golden()["cards"], &actual);
}

// ---------------------------------------------------------------- randomness

/// Needs `dotnet_compat::NetRandom`.
#[test]
fn deck_shuffle_and_random_or_default() {
    let mut shuffles = Vec::new();
    for seed in [0, 1, 42, 123456, i32::MAX] {
        for n in [0, 1, 2, 5, 33] {
            let random = SharedRandom::from_seed(seed);
            let mut deck = Deck::new(0..n, Some(random.clone()));
            deck.shuffle();
            deck.shuffle();
            let pick = (0..n).random_or_default(&random).unwrap_or_default();
            shuffles.push(json!({
                "seed": seed,
                "n": n,
                "items": deck.items,
                "random_or_default": pick,
                "count": random.get_count(),
            }));
        }
    }
    assert_same("random", &golden()["random"], &json!({ "shuffles": shuffles }));
}

#[test]
fn deck_basics() {
    let mut deck = Deck::new([1, 2, 3], None);
    deck.put_on_top_all([7, 8]);
    assert_eq!(deck.items, vec![8, 7, 1, 2, 3]);
    deck.put_on_bottom(9);
    assert_eq!(deck.draw(), 8);
    assert_eq!(deck.top(), Some(7));
    // Shuffling more than one item without a random generator panics.
    let mut copy = deck.clone();
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| copy.shuffle())).is_err());
    deck.clear();
    assert!(deck.is_empty());
    assert_eq!(Deck::<i32>::empty(None).try_draw().unwrap_err().message(), "Cannot draw from an empty deck");
}

// ---------------------------------------------------------------- messages

#[test]
fn messages() {
    use treachery_engine::{express, express_part, express_part_if, express_to};
    use treachery_engine::messages::expression::ExpressionElement as E;
    use treachery_engine::messages::payment::Payment;

    let m = Map::new();
    let start = treachery_engine::messages::message::Message::counter();
    let part = express_part_if!(false; "never");
    let msg = express!(
        Faction::Atreides,
        " transfer ",
        Payment::of(3),
        " to ",
        m.arrakeen(),
        Some(LeaderManager::leaders()[0]),
        None::<Location>,
        vec![Faction::Emperor, Faction::BeneGesserit],
        express_part!(" (", 2.5f32, ")"),
        part,
        express!("inner")
    );
    // The inner message is created first, as in C#.
    assert_eq!(msg.nr, start + 1);
    assert_eq!(msg.target, Faction::None);
    assert_eq!(msg.expression.elements[0], E::from(Faction::Atreides));
    assert_eq!(msg.expression.elements[6], E::Null);
    assert_eq!(
        msg.expression.elements[7],
        E::list(vec![Faction::Emperor, Faction::BeneGesserit])
    );
    assert_eq!(
        msg.to_cs_string(),
        "Atreides transfer Treachery.Shared.Payment to Treachery.Shared.LocationTreachery.Shared.Leader\
         System.Collections.Generic.List`1[Treachery.Shared.Faction] (2.5)inner"
    );
    let to = express_to!(Faction::Harkonnen; "secret");
    assert_eq!(to.target, Faction::Harkonnen);
    assert_eq!(to.nr, start + 2);
}
