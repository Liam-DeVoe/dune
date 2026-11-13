//! Boundary inputs for every public member of every ported C# type.
//!
//! csharp-ref reflects over the types listed in tests/coverage/ported_types.txt
//! and calls each public member with boundary inputs (every enum value plus
//! -1, 0, 13, int.MinValue and int.MaxValue; int extremes; null where the
//! parameter is nullable; sample game objects), recording the result or the
//! exception text in tests/golden/boundary.json. See csharp-ref/Boundary.cs.
//!
//! This test fails when:
//! - a C# member has neither an entry in [`table`] nor a line in
//!   tests/coverage/not_ported.txt;
//! - an entry gives a different result or exception for any case;
//! - an entry or a not-ported line names a member C# does not have;
//! - an entry is `Covered` although some C# case throws (throwing members
//!   must be run here).
//!
//! When you port a new type: add it to ported_types.txt, regenerate the golden
//! files, run this test, and add an entry for each member it reports.

mod common;

use std::collections::HashMap;

use common::*;
use serde_json::{json, Value};
use treachery_engine::compat::cs;
use treachery_engine::enumerations::*;
use treachery_engine::messages::expression::ExpressionElement;
use treachery_engine::identity::IFetcher;
use treachery_engine::heroes::hero::Hero;
use treachery_engine::heroes::leader_manager::LeaderManager;
use treachery_engine::map::location::Location;
use treachery_engine::random::LoggedRandom;
use treachery_engine::map::Map;
use treachery_engine::messages::message::{Message, MessagePart};
use treachery_engine::map::point::PointD;
use treachery_engine::messages::report::Report;
use treachery_engine::cards::treachery_card::TreacheryCard;
use treachery_engine::cards::treachery_card_manager::TreacheryCardManager;
use treachery_engine::{express, express_part};

type Run = fn(&Value, &[Value]) -> Value;

enum Entry {
    /// Rebuild the receiver and arguments, call the Rust port, encode the result.
    Run(Run),
    /// The member is ported and checked elsewhere (named here). Only allowed
    /// when no C# case throws, or C# lists no cases (unsupported parameters).
    Covered(&'static str),
}

use Entry::{Covered, Run as R};

fn map() -> Map {
    Map::new()
}

/// `C# object.ToString()` of an element, encoded as a string result.
fn to_string(e: impl Into<ExpressionElement>) -> Value {
    let e = e.into();
    // `x.ToString()` on a null reference.
    if matches!(e, ExpressionElement::Null) {
        cs::null_reference();
    }
    e.to_cs_string().enc()
}

fn logged_random(label: &Value) -> LoggedRandom {
    let s = label.as_str().unwrap();
    let seed = s.strip_prefix("new:LoggedRandom(").and_then(|s| s.strip_suffix(')')).unwrap();
    LoggedRandom::new(seed.parse().unwrap())
}

fn report(label: &Value) -> Report {
    assert_eq!(label, "new:Report(Bidding,3)");
    Report::new(MainPhase::Bidding, 3)
}

fn opt_card(v: &Value) -> Option<TreacheryCard> {
    opt(v, card)
}

fn opt_hero(v: &Value) -> Option<Hero> {
    opt(v, hero)
}

#[rustfmt::skip]
fn table() -> Vec<(&'static str, Entry)> {
    vec![
        // ------------------------------------------------ Enumerations.cs
        ("Enumerations.GetValues<T>()", Covered("static_data.rs enums (get_values)")),
        ("Enumerations.GetValuesExceptDefault<T>(T)", Covered("enumerations.rs: GetValues minus the default; static_data.rs enums")),
        ("EnumerationExtensions.ToFaction(Nexus)", R(|_, a| en::<Nexus>(&a[0]).to_faction().enc())),
        ("EnumerationExtensions.ToNexus(Faction)", R(|_, a| en::<Faction>(&a[0]).to_nexus().enc())),

        // ------------------------------------------------ ExtensionMethods.cs, LoggedRandom.cs, Deck.cs
        ("ExtensionMethods.HighestOrDefault<T>(IEnumerable<T>,Func<T,IComparable>)", Covered("random_helpers.rs")),
        ("ExtensionMethods.HighestOrDefault<T>(IEnumerable<T>,LoggedRandom,Func<T,IComparable>)", Covered("random_helpers.rs")),
        ("ExtensionMethods.LowestOrDefault<T>(IEnumerable<T>,Func<T,IComparable>)", Covered("random_helpers.rs")),
        ("ExtensionMethods.LowestOrDefault<T>(IEnumerable<T>,LoggedRandom,Func<T,IComparable>)", Covered("random_helpers.rs")),
        ("ExtensionMethods.OneOfHighestNOrDefault<T>(IEnumerable<T>,Func<T,IComparable>,Int32)", Covered("random_helpers.rs")),
        ("ExtensionMethods.OneOfHighestNOrDefault<T>(IEnumerable<T>,LoggedRandom,Func<T,IComparable>,Int32)", Covered("random_helpers.rs")),
        ("ExtensionMethods.OneOfLowestNOrDefault<T>(IEnumerable<T>,Func<T,IComparable>,Int32)", Covered("random_helpers.rs")),
        ("ExtensionMethods.OneOfLowestNOrDefault<T>(IEnumerable<T>,LoggedRandom,Func<T,IComparable>,Int32)", Covered("random_helpers.rs")),
        ("ExtensionMethods.RandomOrDefault<T>(IEnumerable<T>)", Covered("random_helpers.rs")),
        ("ExtensionMethods.RandomOrDefault<T>(IEnumerable<T>,LoggedRandom)", Covered("random_helpers.rs")),
        ("ExtensionMethods.TakeRandomN<T>(IEnumerable<T>,Int32)", Covered("random_helpers.rs")),
        ("LoggedRandom..ctor(Int32)", R(|_, a| { LoggedRandom::new(int(&a[0])); json!("?:LoggedRandom") })),
        ("LoggedRandom.GetCount()", R(|r, _| logged_random(r).get_count().enc())),
        ("LoggedRandom.Next(Int32)", R(|r, a| logged_random(r).next(int(&a[0])).enc())),
        ("Deck<T>..ctor(IEnumerable<T>,LoggedRandom)", Covered("random_helpers.rs deck, static_data.rs random")),
        ("Deck<T>..ctor(LoggedRandom)", Covered("static_data.rs deck_basics")),
        ("Deck<T>.Clear()", Covered("static_data.rs deck_basics")),
        ("Deck<T>.Draw()", Covered("random_helpers.rs deck")),
        ("Deck<T>.PutOnBottom(T)", Covered("static_data.rs deck_basics")),
        ("Deck<T>.PutOnTop(IEnumerable<T>)", Covered("static_data.rs deck_basics")),
        ("Deck<T>.PutOnTop(T)", Covered("static_data.rs deck_basics")),
        ("Deck<T>.Shuffle()", Covered("random_helpers.rs deck, static_data.rs random")),
        ("Deck<T>.Shuffle(List<T>,LoggedRandom)", Covered("deck.rs shuffle_list; static_data.rs random")),
        ("Deck<T>.IsEmpty", Covered("static_data.rs deck_basics")),
        ("Deck<T>.Items", Covered("public field; static_data.rs random")),
        ("Deck<T>.Random", Covered("public field")),
        ("Deck<T>.Top", Covered("random_helpers.rs deck")),

        // ------------------------------------------------ Location and subclasses
        ("Location.Equals(Object)", Covered("location.rs: PartialEq compares ids, like the C# override")),
        ("Location.GetHashCode()", R(|r, _| location(r).id().enc())),
        ("Location.ToString()", R(|r, _| to_string(location(r)))),
        ("Location.Advantage", R(|r, _| location(r).advantage().enc())),
        ("Location.DiscoveryTokenType", R(|r, _| location(r).discovery_token_type().enc())),
        ("Location.Id", R(|r, _| location(r).id().enc())),
        ("Location.IsHomeworld", R(|r, _| location(r).is_homeworld().enc())),
        ("Location.IsProtectedFromStorm", R(|r, _| location(r).is_protected_from_storm().enc())),
        ("Location.IsStronghold", R(|r, _| location(r).is_stronghold().enc())),
        ("Location.Neighbours", R(|r, _| location(r).neighbours(&map()).enc())),
        ("Location.Orientation", R(|r, _| location(r).orientation().enc())),
        ("Location.Sector", R(|r, _| location(r).sector(&map()).enc())),
        ("Location.SpiceBlowAmount", R(|r, _| location(r).spice_blow_amount().enc())),
        ("Location.Territory", R(|r, _| location(r).territory().enc())),
        ("Location.Visible", R(|r, _| location(r).visible(&map()).enc())),
        ("Homeworld.ToString()", R(|r, _| to_string(location(r)))),
        ("Homeworld.BattleBonusAndLasgunShieldLimitAtHighThreshold", R(|r, _| location(r).as_homeworld().unwrap().battle_bonus_and_lasgun_shield_limit_at_high_threshold().enc())),
        ("Homeworld.BattleBonusAndLasgunShieldLimitAtLowThreshold", R(|r, _| location(r).as_homeworld().unwrap().battle_bonus_and_lasgun_shield_limit_at_low_threshold().enc())),
        ("Homeworld.Faction", R(|r, _| location(r).as_homeworld().unwrap().faction().enc())),
        ("Homeworld.IsHomeOfNormalForces", R(|r, _| location(r).as_homeworld().unwrap().is_home_of_normal_forces().enc())),
        ("Homeworld.IsHomeOfSpecialForces", R(|r, _| location(r).as_homeworld().unwrap().is_home_of_special_forces().enc())),
        ("Homeworld.ResourceAmount", R(|r, _| location(r).as_homeworld().unwrap().resource_amount().enc())),
        ("Homeworld.Sector", R(|r, _| location(r).sector(&map()).enc())),
        ("Homeworld.Threshold", R(|r, _| location(r).as_homeworld().unwrap().threshold().enc())),
        ("Homeworld.World", R(|r, _| location(r).as_homeworld().unwrap().world().enc())),
        ("AttachedLocation.PointAt(Game,Location)", Covered("static_data.rs map_attached")),
        ("AttachedLocation.AttachedToLocation", R(|r, _| location(r).as_attached_location().unwrap().attached_to_location(&map()).enc())),
        ("AttachedLocation.Game", R(|r, _| map().attached_to_game(location(r)).then(|| json!("?:Game")).enc())),
        ("AttachedLocation.Neighbours", R(|r, _| location(r).neighbours(&map()).enc())),
        ("AttachedLocation.Sector", R(|r, _| location(r).sector(&map()).enc())),
        ("AttachedLocation.Visible", R(|r, _| location(r).visible(&map()).enc())),
        ("DiscoveredLocation.Discovery", R(|r, _| location(r).as_discovered_location().unwrap().discovery().enc())),

        // ------------------------------------------------ Territory.cs
        ("Territory.ToString()", R(|r, _| to_string(territory(r)))),
        ("Territory.Advantage", R(|r, _| territory(r).advantage().enc())),
        ("Territory.DiscoveryTokenLocation", R(|r, _| territory(r).discovery_token_location().enc())),
        ("Territory.HasReducedShippingCost", R(|r, _| territory(r).has_reduced_shipping_cost().enc())),
        ("Territory.Id", R(|r, _| territory(r).id().enc())),
        ("Territory.IsDiscoveredStronghold", R(|r, _| territory(r).is_discovered_stronghold().enc())),
        ("Territory.IsDiscovery", R(|r, _| territory(r).is_discovery().enc())),
        ("Territory.IsHomeworld", R(|r, _| territory(r).is_homeworld().enc())),
        ("Territory.IsProtectedFromStorm", R(|r, _| territory(r).is_protected_from_storm().enc())),
        ("Territory.IsProtectedFromWorm", R(|r, _| territory(r).is_protected_from_worm().enc())),
        ("Territory.IsStronghold", R(|r, _| territory(r).is_stronghold().enc())),
        ("Territory.IsVisible", R(|r, _| territory(r).is_visible(&map()).enc())),
        ("Territory.Locations", R(|r, _| territory(r).locations().enc())),
        ("Territory.MiddleLocation", R(|r, _| territory(r).middle_location().enc())),
        ("Territory.ResourceBlowLocation", R(|r, _| territory(r).resource_blow_location().enc())),
        ("Territory.SkinId", R(|r, _| territory(r).skin_id().enc())),

        // ------------------------------------------------ Map.cs
        ("Map..ctor()", R(|_, _| { map(); json!("?:Map") })),
        ("Map.LocationLookup", R(|_, _| { map().location_lookup(); json!("?:LocationFetcher") })),
        ("Map.TerritoryLookup", R(|_, _| { map().territory_lookup(); json!("?:TerritoryFetcher") })),
        ("Map.NumberOfSectors", R(|_, _| Map::NUMBER_OF_SECTORS.enc())),
        ("Map.FindFirstShortestPath(Location,Location,Boolean,Faction,Game)", Covered("static_data.rs pathfinding (no forces yet)")),
        ("Map.FindNeighbours(Location,Int32,Boolean,Faction,Game,Boolean)", Covered("static_data.rs pathfinding, map_attached (no forces yet)")),
        ("Map.FindPaths(Location,Location,Int32,Boolean,Faction,Game)", Covered("static_data.rs pathfinding (no forces yet)")),
        ("Map.GetResourceCardsInPlay(Game)", Covered("needs a Game: covered by oracle traces once Game is ported")),
        ("Map.FindNeighboursForHmsMovement(Location,Int32,Boolean,Int32)", R(|_, a| map().find_neighbours_for_hms_movement(location(&a[0]), int(&a[1]), boolean(&a[2]), int(&a[3])).enc())),
        ("Map.FindNeighboursWithinTerritory(Location,Boolean,Int32)", R(|_, a| map().find_neighbours_within_territory(location(&a[0]), boolean(&a[1]), int(&a[2])).enc())),
        ("Map.GetDiscoveryStronghold(DiscoveryToken)", R(|_, a| map().get_discovery_stronghold(en(&a[0])).enc())),
        ("Map.GetHomeWorld(World)", R(|_, a| map().get_home_world(en(&a[0])).enc())),
        ("Map.Locations(Boolean)", R(|_, a| map().locations(boolean(&a[0])).collect::<Vec<_>>().enc())),
        ("Map.Territories(Boolean)", R(|_, a| map().territories(boolean(&a[0])).collect::<Vec<_>>().enc())),
        ("Map.Homeworlds", R(|_, _| map().homeworlds().collect::<Vec<_>>().enc())),
        ("Map.Strongholds", R(|_, _| map().strongholds().collect::<Vec<_>>().enc())),
        ("Map.Arrakeen", R(|_, _| map().arrakeen().enc())),
        ("Map.BightOfTheCliff", R(|_, _| map().bight_of_the_cliff().enc())),
        ("Map.BrokenLand", R(|_, _| map().broken_land().enc())),
        ("Map.Carthag", R(|_, _| map().carthag().enc())),
        ("Map.CielagoWest", R(|_, _| map().cielago_west().enc())),
        ("Map.Cistern", R(|_, _| map().cistern().enc())),
        ("Map.FalseWallEast", R(|_, _| map().false_wall_east().enc())),
        ("Map.FalseWallSouth", R(|_, _| map().false_wall_south().enc())),
        ("Map.FalseWallWest", R(|_, _| map().false_wall_west().enc())),
        ("Map.FuneralPlain", R(|_, _| map().funeral_plain().enc())),
        ("Map.HabbanyaErg", R(|_, _| map().habbanya_erg().enc())),
        ("Map.HabbanyaSietch", R(|_, _| map().habbanya_sietch().enc())),
        ("Map.HaggaBasin", R(|_, _| map().hagga_basin().enc())),
        ("Map.HiddenMobileStronghold", R(|_, _| map().hidden_mobile_stronghold().enc())),
        ("Map.ImperialBasin", R(|_, _| map().imperial_basin().enc())),
        ("Map.Jacurutu", R(|_, _| map().jacurutu().enc())),
        ("Map.PastyMesa", R(|_, _| map().pasty_mesa().enc())),
        ("Map.PlasticBasin", R(|_, _| map().plastic_basin().enc())),
        ("Map.PolarSink", R(|_, _| map().polar_sink().enc())),
        ("Map.ProcessingStation", R(|_, _| map().processing_station().enc())),
        ("Map.RockOutcroppings", R(|_, _| map().rock_outcroppings().enc())),
        ("Map.ShieldWall", R(|_, _| map().shield_wall().enc())),
        ("Map.Shrine", R(|_, _| map().shrine().enc())),
        ("Map.SietchTabr", R(|_, _| map().sietch_tabr().enc())),
        ("Map.TestingStation", R(|_, _| map().testing_station().enc())),
        ("Map.TheGreatFlat", R(|_, _| map().the_great_flat().enc())),
        ("Map.TheGreaterFlat", R(|_, _| map().the_greater_flat().enc())),
        ("Map.Tsimpo", R(|_, _| map().tsimpo().enc())),
        ("Map.TueksSietch", R(|_, _| map().tueks_sietch().enc())),
        ("Map.WindPass", R(|_, _| map().wind_pass().enc())),
        ("Map.WindPassNorth", R(|_, _| map().wind_pass_north().enc())),
        ("Map.LocationFetcher..ctor(Map)", Covered("map.rs location_lookup")),
        ("Map.LocationFetcher.Find(Int32)", R(|_, a| map().location_lookup().find(int(&a[0])).enc())),
        ("Map.LocationFetcher.GetId(Location)", R(|_, a| map().location_lookup().get_id(opt(&a[0], location)).enc())),
        ("Map.TerritoryFetcher..ctor(Map)", Covered("map.rs territory_lookup")),
        ("Map.TerritoryFetcher.Find(Int32)", R(|_, a| map().territory_lookup().find(int(&a[0])).enc())),
        ("Map.TerritoryFetcher.GetId(Territory)", R(|_, a| map().territory_lookup().get_id(opt(&a[0], territory)).enc())),

        // ------------------------------------------------ ResourceCard.cs, Point.cs
        ("ResourceCard.ToString()", R(|r, _| to_string(resource_card(r)))),
        ("ResourceCard.DiscoveryLocation", R(|r, _| resource_card(r).discovery_location().enc())),
        ("ResourceCard.IsDiscovery", R(|r, _| resource_card(r).is_discovery().enc())),
        ("ResourceCard.IsGreatMaker", R(|r, _| resource_card(r).is_great_maker().enc())),
        ("ResourceCard.IsSandTrout", R(|r, _| resource_card(r).is_sand_trout().enc())),
        ("ResourceCard.IsShaiHulud", R(|r, _| resource_card(r).is_shai_hulud().enc())),
        ("ResourceCard.IsSpiceBlow", R(|r, _| resource_card(r).is_spice_blow().enc())),
        ("ResourceCard.Location", R(|r, _| resource_card(r).location().enc())),
        ("ResourceCard.SkinId", R(|r, _| resource_card(r).skin_id().enc())),
        ("ResourceCard.Territory", R(|r, _| resource_card(r).territory().enc())),
        ("PointD..ctor(Double,Double)", R(|_, a| PointD::new(double(&a[0]), double(&a[1])).enc())),
        ("PointD.X", R(|r, _| point(r).x.enc())),
        ("PointD.Y", R(|r, _| point(r).y.enc())),
        ("PointD.Equals(Object)", Covered("point.rs: PartialEq, as Equals(PointD)")),
        ("PointD.Equals(PointD)", R(|r, a| (point(r) == point(&a[0])).enc())),
        ("PointD.op_Equality(PointD,PointD)", R(|_, a| (point(&a[0]) == point(&a[1])).enc())),
        ("PointD.op_Inequality(PointD,PointD)", R(|_, a| (point(&a[0]) != point(&a[1])).enc())),
        ("PointD.Translate(Double,Double)", R(|r, a| point(r).translate(double(&a[0]), double(&a[1])).enc())),

        // ------------------------------------------------ heroes
        ("IIdentifiable.Id", R(|r, _| hero(r).id().enc())),
        ("IHero.Is(Faction)", R(|r, a| hero(r).is(en(&a[0])).enc())),
        ("IHero.IsFaceDancer(IHero)", R(|r, a| hero(r).is_face_dancer(hero(&a[0])).enc())),
        ("IHero.IsTraitor(IHero)", R(|r, a| hero(r).is_traitor(hero(&a[0])).enc())),
        ("IHero.ValueInCombatAgainst(IHero)", R(|r, a| hero(r).value_in_combat_against(opt_hero(&a[0])).enc())),
        ("IHero.CostToRevive", R(|r, _| hero(r).cost_to_revive().enc())),
        ("IHero.Faction", R(|r, _| hero(r).faction().enc())),
        ("IHero.HeroType", R(|r, _| hero(r).hero_type().enc())),
        ("IHero.SkinId", R(|r, _| hero(r).skin_id().enc())),
        ("IHero.Value", R(|r, _| hero(r).value().enc())),
        ("IFetcher<T>.Find(Int32)", Covered("the Find entries of each fetcher")),
        ("IFetcher<T>.GetId(T)", Covered("the GetId entries of each fetcher")),
        ("Leader.GetHashCode()", R(|r, _| leader(r).id().enc())),
        ("Leader.Is(Faction)", R(|r, a| leader(r).is(en(&a[0])).enc())),
        ("Leader.IsFaceDancer(IHero)", R(|r, a| leader(r).is_face_dancer(hero(&a[0])).enc())),
        ("Leader.IsTraitor(IHero)", R(|r, a| leader(r).is_traitor(hero(&a[0])).enc())),
        ("Leader.ToString()", R(|r, _| to_string(leader(r)))),
        ("Leader.ValueInCombatAgainst(IHero)", R(|r, a| leader(r).value_in_combat_against(opt_hero(&a[0])).enc())),
        ("Leader.CostToRevive", R(|r, _| leader(r).cost_to_revive().enc())),
        ("Leader.Faction", R(|r, _| leader(r).faction().enc())),
        ("Leader.HeroType", R(|r, _| leader(r).hero_type().enc())),
        ("Leader.Id", R(|r, _| leader(r).id().enc())),
        ("Leader.SkinId", R(|r, _| leader(r).skin_id().enc())),
        ("Leader.Value", R(|r, _| leader(r).value().enc())),
        ("Messiah.Is(Faction)", R(|r, a| hero(r).is(en(&a[0])).enc())),
        ("Messiah.IsFaceDancer(IHero)", R(|r, a| hero(r).is_face_dancer(hero(&a[0])).enc())),
        ("Messiah.IsTraitor(IHero)", R(|r, a| hero(r).is_traitor(hero(&a[0])).enc())),
        ("Messiah.ToString()", R(|r, _| to_string(hero(r)))),
        ("Messiah.ValueInCombatAgainst(IHero)", R(|r, a| hero(r).value_in_combat_against(opt_hero(&a[0])).enc())),
        ("Messiah.CostToRevive", R(|r, _| hero(r).cost_to_revive().enc())),
        ("Messiah.Faction", R(|r, _| hero(r).faction().enc())),
        ("Messiah.HeroType", R(|r, _| hero(r).hero_type().enc())),
        ("Messiah.Id", R(|r, _| hero(r).id().enc())),
        ("Messiah.SkinId", R(|r, _| hero(r).skin_id().enc())),
        ("Messiah.Value", R(|r, _| hero(r).value().enc())),
        ("LeaderManager.FirstId", R(|_, _| LeaderManager::FIRST_ID.enc())),
        ("LeaderManager.GetLeaders(Faction)", R(|_, a| LeaderManager::get_leaders(en(&a[0])).collect::<Vec<_>>().enc())),
        ("LeaderManager.HeroLookup", R(|_, _| { LeaderManager::hero_lookup(); json!("?:HeroFetcher") })),
        ("LeaderManager.LeaderLookup", R(|_, _| { LeaderManager::leader_lookup(); json!("?:LeaderFetcher") })),
        ("LeaderManager.Leaders", R(|_, _| LeaderManager::leaders().enc())),
        ("LeaderManager.Messiah", R(|_, _| LeaderManager::messiah().enc())),
        ("LeaderManager.LeaderFetcher..ctor()", R(|_, _| json!("?:LeaderFetcher"))),
        ("LeaderManager.LeaderFetcher.Find(Int32)", R(|_, a| LeaderManager::leader_lookup().find(int(&a[0])).enc())),
        ("LeaderManager.LeaderFetcher.GetId(Leader)", R(|_, a| LeaderManager::leader_lookup().get_id(opt(&a[0], leader)).enc())),
        ("LeaderManager.HeroFetcher..ctor()", R(|_, _| json!("?:HeroFetcher"))),
        ("LeaderManager.HeroFetcher.Find(Int32)", R(|_, a| LeaderManager::hero_lookup().find(int(&a[0])).enc())),
        ("LeaderManager.HeroFetcher.GetId(IHero)", R(|_, a| LeaderManager::hero_lookup().get_id(opt_hero(&a[0])).enc())),

        // ------------------------------------------------ treachery cards
        ("TreacheryCard.None", R(|_, _| TreacheryCard::NONE.enc())),
        ("TreacheryCard.Unknown", R(|_, _| TreacheryCard::UNKNOWN.enc())),
        ("TreacheryCard.CounteredBy(TreacheryCard,TreacheryCard)", R(|r, a| card(r).countered_by(opt_card(&a[0]), opt_card(&a[1])).enc())),
        ("TreacheryCard.GetHashCode()", R(|r, _| card(r).id().enc())),
        ("TreacheryCard.Is(Faction)", R(|r, a| card(r).is(en(&a[0])).enc())),
        ("TreacheryCard.IsFaceDancer(IHero)", R(|r, a| card(r).is_face_dancer(hero(&a[0])).enc())),
        ("TreacheryCard.IsTraitor(IHero)", R(|r, a| card(r).is_traitor(hero(&a[0])).enc())),
        ("TreacheryCard.ToString()", R(|r, _| to_string(card(r)))),
        ("TreacheryCard.ValueInCombatAgainst(IHero)", R(|r, a| card(r).value_in_combat_against(opt_hero(&a[0])).enc())),
        ("TreacheryCard.CostToRevive", R(|r, _| card(r).cost_to_revive().enc())),
        ("TreacheryCard.Faction", R(|r, _| card(r).faction().enc())),
        ("TreacheryCard.HeroType", R(|r, _| card(r).hero_type().enc())),
        ("TreacheryCard.Id", R(|r, _| card(r).id().enc())),
        ("TreacheryCard.IsArtillery", R(|r, _| card(r).is_artillery().enc())),
        ("TreacheryCard.IsDefense", R(|r, _| card(r).is_defense().enc())),
        ("TreacheryCard.IsAtreides", R(|r, _| card(r).is_atreides().enc())),
        ("TreacheryCard.IsLaser", R(|r, _| card(r).is_laser().enc())),
        ("TreacheryCard.IsMercenary", R(|r, _| card(r).is_mercenary().enc())),
        ("TreacheryCard.IsMirrorWeapon", R(|r, _| card(r).is_mirror_weapon().enc())),
        ("TreacheryCard.IsNonAntidotePoisonDefense", R(|r, _| card(r).is_non_antidote_poison_defense().enc())),
        ("TreacheryCard.IsPoisonDefense", R(|r, _| card(r).is_poison_defense().enc())),
        ("TreacheryCard.IsPoisonTooth", R(|r, _| card(r).is_poison_tooth().enc())),
        ("TreacheryCard.IsPoisonWeapon", R(|r, _| card(r).is_poison_weapon().enc())),
        ("TreacheryCard.IsPortableAntidote", R(|r, _| card(r).is_portable_antidote().enc())),
        ("TreacheryCard.IsProjectileDefense", R(|r, _| card(r).is_projectile_defense().enc())),
        ("TreacheryCard.IsProjectileWeapon", R(|r, _| card(r).is_projectile_weapon().enc())),
        ("TreacheryCard.IsRockMelter", R(|r, _| card(r).is_rock_melter().enc())),
        ("TreacheryCard.IsShield", R(|r, _| card(r).is_shield().enc())),
        ("TreacheryCard.IsUseless", R(|r, _| card(r).is_useless().enc())),
        ("TreacheryCard.IsWeapon", R(|r, _| card(r).is_weapon().enc())),
        ("TreacheryCard.Rules", R(|r, _| card(r).rules().enc())),
        ("TreacheryCard.SkinId", R(|r, _| card(r).skin_id().enc())),
        ("TreacheryCard.Type", R(|r, _| card(r).type_().enc())),
        ("TreacheryCard.Value", R(|r, _| card(r).value().enc())),
        ("TreacheryCardManager.CardBaliset", R(|_, _| TreacheryCardManager::CARD_BALISET.enc())),
        ("TreacheryCardManager.CardJubbaCloak", R(|_, _| TreacheryCardManager::CARD_JUBBA_CLOAK.enc())),
        ("TreacheryCardManager.CardKullWahad", R(|_, _| TreacheryCardManager::CARD_KULL_WAHAD.enc())),
        ("TreacheryCardManager.CardKulon", R(|_, _| TreacheryCardManager::CARD_KULON.enc())),
        ("TreacheryCardManager.CardLalala", R(|_, _| TreacheryCardManager::CARD_LALALA.enc())),
        ("TreacheryCardManager.CardTripToGamont", R(|_, _| TreacheryCardManager::CARD_TRIP_TO_GAMONT.enc())),
        ("TreacheryCardManager.Items", R(|_, _| TreacheryCardManager::items().enc())),
        ("TreacheryCardManager.Lookup", R(|_, _| { TreacheryCardManager::lookup(); json!("?:TreacheryCardFetcher") })),
        ("TreacheryCardManager.CreateTreacheryDeck(Game,LoggedRandom)", Covered("needs a Game: covered by oracle traces once Game is ported")),
        ("TreacheryCardManager.GetCardsInPlay(Game)", Covered("needs a Game: covered by oracle traces once Game is ported")),
        ("TreacheryCardManager.Get(Int32)", R(|_, a| TreacheryCardManager::get(int(&a[0])).enc())),
        ("TreacheryCardManager.GetCardsInAndOutsidePlay()", R(|_, _| TreacheryCardManager::get_cards_in_and_outside_play().enc())),
        ("TreacheryCardManager.GetId(TreacheryCard)", R(|_, a| TreacheryCardManager::get_id(opt_card(&a[0])).enc())),
        ("TreacheryCardManager.GetRicheseCards()", R(|_, _| TreacheryCardManager::get_richese_cards().enc())),

        // ------------------------------------------------ messages
        ("Expression..ctor(Object[])", Covered("message_corpus.rs params")),
        ("Expression.Express(Object[])", Covered("message_corpus.rs params")),
        ("Expression.Elements", R(|_, _| express!("x").expression.elements.iter().map(|e| e.to_cs_string().enc()).collect::<Vec<_>>().enc())),
        ("Message.AsHeading(Concept)", R(|_, a| express!("x").as_heading(en(&a[0])).enc())),
        ("Message.Express(Object[])", Covered("message_corpus.rs params")),
        ("Message.ExpressTo(Faction,Object[])", Covered("message_corpus.rs params")),
        ("Message.ToString()", R(|_, _| express!("x").to_cs_string().enc())),
        ("Message.WithFact(BattleFact)", Covered("message.rs builder; review_foundation.rs records")),
        ("Message.WithLog(Object)", Covered("message.rs builder")),
        ("Message.WithPlans(PlanSummary,PlanSummary)", Covered("message.rs builder; review_foundation.rs records")),
        ("Message.WithRoute(Faction,IEnumerable<Location>,Location)", Covered("message.rs builder; review_foundation.rs records")),
        ("Message.Expression", R(|_, _| { let _ = express!("x").expression; json!("?:Expression") })),
        ("Message.Fact", R(|_, _| express!("x").fact.map(|_| json!("?:BattleFact")).enc())),
        ("Message.Heading", R(|_, _| express!("x").heading.enc())),
        ("Message.Log", R(|_, _| express!("x").log.map(|_| json!("?:Log")).enc())),
        ("Message.Plans", R(|_, _| express!("x").plans.map(|_| json!("?:BattlePlans")).enc())),
        ("Message.Route", R(|_, _| express!("x").route.map(|_| json!("?:Route")).enc())),
        ("Message.Target", R(|_, _| express!("x").target.enc())),
        ("MessagePart.Express(Object[])", Covered("message_corpus.rs")),
        ("MessagePart.ExpressIf(Boolean,Object[])", Covered("message_corpus.rs; static_data.rs messages")),
        ("MessagePart.ToString()", R(|_, _| express_part!("x").to_cs_string().enc())),
        ("MessagePart.Expression", R(|_, _| { let _ = express_part!("x").expression; json!("?:Expression") })),

        // Records: the fields are public Rust fields; equality is checked in
        // review_foundation.rs (NaN dials, Route compares its list by reference).
        ("Route..ctor(Faction,List<Location>,Location)", Covered("message.rs Route struct")),
        ("Route.Equals(Object)", Covered("review_foundation.rs route_equality_compares_list_by_reference")),
        ("Route.Equals(Route)", Covered("review_foundation.rs route_equality_compares_list_by_reference")),
        ("Route.op_Equality(Route,Route)", Covered("review_foundation.rs route_equality_compares_list_by_reference")),
        ("Route.op_Inequality(Route,Route)", Covered("review_foundation.rs route_equality_compares_list_by_reference")),
        ("Route.Faction", Covered("public field")),
        ("Route.From", Covered("public field")),
        ("Route.To", Covered("public field")),
        ("BattlePlans..ctor(PlanSummary,PlanSummary)", Covered("message.rs BattlePlans struct")),
        ("BattlePlans.Equals(Object)", Covered("derived PartialEq over PlanSummary's record equality")),
        ("BattlePlans.Equals(BattlePlans)", Covered("derived PartialEq over PlanSummary's record equality")),
        ("BattlePlans.op_Equality(BattlePlans,BattlePlans)", Covered("derived PartialEq over PlanSummary's record equality")),
        ("BattlePlans.op_Inequality(BattlePlans,BattlePlans)", Covered("derived PartialEq over PlanSummary's record equality")),
        ("BattlePlans.Aggressor", Covered("public field")),
        ("BattlePlans.Defender", Covered("public field")),
        ("PlanSummary..ctor(Faction,IHero,Boolean,Single,Int32?,Int32,Faction,TreacheryCard,TreacheryCard)", Covered("message.rs PlanSummary struct (cannot throw)")),
        ("PlanSummary.Equals(Object)", Covered("review_foundation.rs plan_summary_equality_with_nan_dial")),
        ("PlanSummary.Equals(PlanSummary)", Covered("review_foundation.rs plan_summary_equality_with_nan_dial")),
        ("PlanSummary.op_Equality(PlanSummary,PlanSummary)", Covered("review_foundation.rs plan_summary_equality_with_nan_dial")),
        ("PlanSummary.op_Inequality(PlanSummary,PlanSummary)", Covered("review_foundation.rs plan_summary_equality_with_nan_dial")),
        ("PlanSummary.Ally", Covered("public field")),
        ("PlanSummary.AllyContribution", Covered("public field")),
        ("PlanSummary.Defense", Covered("public field")),
        ("PlanSummary.Dial", Covered("public field")),
        ("PlanSummary.Faction", Covered("public field")),
        ("PlanSummary.Leader", Covered("public field")),
        ("PlanSummary.Messiah", Covered("public field")),
        ("PlanSummary.Spice", Covered("public field")),
        ("PlanSummary.Weapon", Covered("public field")),
        ("BattleFact..ctor(BattleFactKind,Faction)", Covered("message.rs BattleFact::new (cannot throw)")),
        ("BattleFact.Equals(Object)", Covered("message.rs: PartialEq with record semantics (NaN strength)")),
        ("BattleFact.Equals(BattleFact)", Covered("message.rs: PartialEq with record semantics (NaN strength)")),
        ("BattleFact.op_Equality(BattleFact,BattleFact)", Covered("message.rs: PartialEq with record semantics (NaN strength)")),
        ("BattleFact.op_Inequality(BattleFact,BattleFact)", Covered("message.rs: PartialEq with record semantics (NaN strength)")),
        ("BattleFact.All", Covered("public field")),
        ("BattleFact.Card", Covered("public field")),
        ("BattleFact.Cause", Covered("public field")),
        ("BattleFact.Faction", Covered("public field")),
        ("BattleFact.Forces", Covered("public field")),
        ("BattleFact.Hero", Covered("public field")),
        ("BattleFact.Kind", Covered("public field")),
        ("BattleFact.SpecialForces", Covered("public field")),
        ("BattleFact.Strength", Covered("public field")),
        ("BattleFact.Value", Covered("public field")),

        // ------------------------------------------------ Payment.cs, Report.cs
        ("Payment..ctor()", Covered("payment.rs Payment::default (cannot throw)")),
        ("Payment..ctor(Int32)", Covered("payment.rs Payment::new (cannot throw)")),
        ("Payment..ctor(Int32,Faction)", Covered("payment.rs Payment::new_by (cannot throw)")),
        ("Payment..ctor(Int32,Faction,Faction,GameEvent)", Covered("payment.rs (with GameEvent)")),
        ("Payment.Of(Int32)", Covered("payment.rs Payment::of (cannot throw)")),
        ("Payment.Of(Int32,Faction)", Covered("payment.rs Payment::of_by (cannot throw)")),
        ("Payment.Amount", Covered("public field")),
        ("Payment.By", Covered("public field")),
        ("Payment.Reason", Covered("public field")),
        ("Payment.To", Covered("public field")),
        ("Report..ctor(MainPhase,Int32)", Covered("report.rs Report::new (cannot throw)")),
        ("Report.Messages", R(|r, _| report(r).messages.enc())),
        ("Report.Express(Object[])", Covered("report.rs: Message::express")),
        ("Report.Express(GameEvent)", Covered("report.rs (with GameEvent)")),
        ("Report.Express(Message)", Covered("report.rs express_message")),
        ("Report.ExpressTo(Faction,Object[])", Covered("report.rs: Message::express_to")),
        ("Report.About", R(|r, _| report(r).about.enc())),
        ("Report.Aggressor", R(|r, _| report(r).aggressor.enc())),
        ("Report.BattleDecided", R(|r, _| report(r).battle_decided.enc())),
        ("Report.BattleWinner", R(|r, _| report(r).battle_winner.enc())),
        ("Report.Defender", R(|r, _| report(r).defender.enc())),
        ("Report.IsBattle", R(|r, _| report(r).is_battle().enc())),
        ("Report.Territory", R(|r, _| report(r).territory.enc())),
        ("Report.Turn", R(|r, _| report(r).turn.enc())),
    ]
}

/// tests/coverage/not_ported.txt: `signature | reason`.
fn not_ported() -> Vec<(String, String)> {
    coverage_list("not_ported.txt")
        .into_iter()
        .map(|l| {
            let (sig, reason) = l.split_once('|').unwrap_or_else(|| panic!("not_ported.txt: missing '| reason' in {l}"));
            (sig.trim().to_string(), reason.trim().to_string())
        })
        .collect()
}

#[test]
fn ported_types_match_golden() {
    let golden_types: Vec<String> =
        golden("boundary.json")["ported_types"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
    assert_eq!(
        golden_types,
        coverage_list("ported_types.txt"),
        "tests/coverage/ported_types.txt changed: regenerate the golden files with csharp-ref"
    );
}

#[test]
fn every_member_matches_csharp() {
    let members = golden("boundary.json")["members"].as_array().unwrap();
    let table: HashMap<&str, Entry> = table().into_iter().collect();
    let not_ported: HashMap<String, String> = not_ported().into_iter().collect();
    let mut failures = Failures::default();
    let mut seen = std::collections::HashSet::new();
    let mut compared = 0;

    for m in members {
        let sig = m["sig"].as_str().unwrap();
        seen.insert(sig.to_string());
        let cases = m["cases"].as_array();
        match (table.get(sig), not_ported.contains_key(sig)) {
            (Some(_), true) => failures.push(format!("{sig}: both in the table and in not_ported.txt")),
            (None, true) => {}
            (None, false) => failures.push(format!(
                "{sig}: no Rust entry and no not_ported.txt line ({})",
                m.get("unsupported").and_then(|u| u.as_str()).map_or("C# has cases".to_string(), |u| format!("unsupported: {u}"))
            )),
            (Some(Covered(by)), false) => {
                let throws = cases.into_iter().flatten().any(|c| c[2].as_str().is_some_and(|s| s.starts_with("x:")));
                if throws {
                    failures.push(format!("{sig}: some C# cases throw, so it needs a Run entry, not Covered ({by})"));
                }
            }
            (Some(R(run)), false) => {
                let Some(cases) = cases else {
                    failures.push(format!("{sig}: Run entry but C# lists no cases"));
                    continue;
                };
                for c in cases {
                    let (recv, args, expected) = (&c[0], c[1].as_array().unwrap(), &c[2]);
                    let actual = catch_any(|| run(recv, args)).unwrap_or_else(|e| e);
                    failures.check(|| format!("{sig} recv={recv} args={}", Value::Array(args.clone())), expected, &actual);
                    compared += 1;
                }
            }
        }
    }
    for sig in table.keys() {
        if !seen.contains(*sig) {
            failures.push(format!("{sig}: table entry for a member C# does not have"));
        }
    }
    for sig in not_ported.keys() {
        if !seen.contains(sig) {
            failures.push(format!("{sig}: not_ported.txt names a member C# does not have"));
        }
    }
    eprintln!("boundary: {} members, {compared} cases compared", members.len());
    failures.finish("boundary");
}

/// A C# member access on a null reference of a game-object type throws
/// NullReferenceException before the member runs (`callvirt` checks the
/// receiver), whatever the member and its arguments. Rust handles have a null
/// value (engine-conventions.md, "Null handles") that must do the same, so
/// every member that `every_member_matches_csharp` runs on a game object is run
/// here on the null handle, with the arguments of each C# case.
#[test]
fn null_receivers_throw_null_reference() {
    let members = golden("boundary.json")["members"].as_array().unwrap();
    let table: HashMap<&str, Entry> = table().into_iter().collect();
    let expected = json!(format!("x:NullReferenceException:{}", cs::NULL_REFERENCE_MESSAGE));
    let mut failures = Failures::default();
    let mut compared = 0;
    for m in members {
        let sig = m["sig"].as_str().unwrap();
        let Some(R(run)) = table.get(sig) else { continue };
        let Some(cases) = m["cases"].as_array() else { continue };
        // Instance members of a game object: the receiver is an `o:` or `rc:` label.
        let on_game_object =
            cases.iter().any(|c| c[0].as_str().is_some_and(|r| r.starts_with("o:") || r.starts_with("rc:")));
        if !on_game_object {
            continue;
        }
        for c in cases {
            let args = c[1].as_array().unwrap();
            let actual = catch_any(|| run(&Value::Null, args)).unwrap_or_else(|e| e);
            failures.check(|| format!("{sig} on null, args={}", Value::Array(args.clone())), &expected, &actual);
            compared += 1;
        }
    }
    assert!(compared > 100, "only {compared} null-receiver cases");
    eprintln!("boundary: {compared} null-receiver cases compared");
    failures.finish("null receivers");
}

/// Keeps `Location` and `Message` imports used when entries change.
#[allow(dead_code)]
fn _unused(_: Location, _: Message, _: MessagePart) {}

// ---------------------------------------------------------------- lazy producers
//
// The "lazy" section of boundary.json (csharp-ref/LazyBoundary.cs): every lazy
// IEnumerable producer that takes a Game or Player, called on game states whose
// hands, traitor lists and discovery lists hold a null or throwing element
// after a matching one. Each case is the items yielded, in order, up to the
// first exception. A port that builds the whole list before yielding throws
// before the first item, so the prefix differs from C#.

mod lazy {
    use super::*;
    use treachery_engine::events::*;
    use treachery_engine::game::Game;
    use treachery_engine::json;
    use treachery_engine::game::Participation;
    use treachery_engine::game::{Player, PlayerIndex};
    use treachery_engine::cards::resource_card::ResourceCard;

    type Sink<'a> = &'a mut dyn FnMut(Value);
    type Producer = fn(&Game, PlayerIndex, Sink);

    fn each<T>(items: impl Iterator<Item = T>, sink: Sink, enc: impl Fn(T) -> Value) {
        for x in items {
            sink(enc(x));
        }
    }

    fn e<T: Enc>(x: T) -> Value {
        x.enc()
    }

    /// `Player?` parameters: C# null is `None`.
    fn opt(p: PlayerIndex) -> Option<PlayerIndex> {
        (p != PlayerIndex::NULL).then_some(p)
    }

    #[rustfmt::skip]
    pub fn producers() -> Vec<(&'static str, Producer)> {
        vec![
            ("AllianceOffered.ValidTargets", |g, p, s| each(AllianceOffered::valid_targets(g, p), s, e)),
            ("AmbassadorActivated.ValidFremenTargets", |g, p, s| each(AmbassadorActivated::valid_fremen_targets(g, p), s, e)),
            ("AmbassadorActivated.ValidGuildTargets", |g, p, s| each(AmbassadorActivated::valid_guild_targets(g, p), s, e)),
            ("AmbassadorPlaced.ValidStrongholds", |g, p, s| each(AmbassadorPlaced::valid_strongholds(g, p), s, e)),
            ("BattleConcluded.ValidTraitorsToReplace", |g, p, s| each(BattleConcluded::valid_traitors_to_replace(g, p), s, e)),
            ("Bid.ValidKarmaCards", |g, p, s| each(Bid::valid_karma_cards(g, opt(p)), s, e)),
            ("BeneGesseritBattleAnnouncement.ValidTerritories", |g, p, s| each(BeneGesseritBattleAnnouncement::valid_territories(g, p), s, e)),
            ("BeneGesseritPrediction.ValidTargets", |g, p, s| each(BeneGesseritPrediction::valid_targets(g, p), s, e)),
            ("ChoamDiscarded.ValidCards", |g, p, s| each(ChoamDiscarded::valid_cards(g, p), s, e)),
            ("ChoamMovePrevention.ValidTerritories", |g, p, s| each(ChoamMovePrevention::valid_territories(g, p), s, e)),
            ("ChoamRemoveForce.ValidLocations", |g, _, s| each(ChoamRemoveForce::valid_locations(g), s, e)),
            ("ChoamRemoveForce.ValidFactions", |g, p, s| each(ChoamRemoveForce::valid_factions(g, opt(p), None), s, e)),
            ("ClairVoyancePlayed.ValidTargets", |g, p, s| each(ClairVoyancePlayed::valid_targets(g, p), s, e)),
            ("DealAccepted.AcceptableDeals", |g, p, s| each(DealAccepted::acceptable_deals(g, opt(p)), s, |_| json!("?:DealOffered"))),
            ("DealAccepted.CancellableDeals", |g, p, s| each(DealAccepted::cancellable_deals(g, opt(p)), s, |_| json!("?:DealOffered"))),
            ("DiscardedTaken.ValidCards", |g, p, s| each(DiscardedTaken::valid_cards(g, p), s, e)),
            ("DiscoveryEntered.ValidTargets", |g, p, s| each(DiscoveryEntered::valid_targets(g, p), s, e)),
            ("DiscoveryRevealed.GetLocations", |g, p, s| each(DiscoveryRevealed::get_locations(g, p), s, e)),
            ("DistransUsed.ValidCards", |g, p, s| each(DistransUsed::valid_cards(g, p), s, e)),
            ("DistransUsed.ValidTargets", |g, p, s| each(DistransUsed::valid_targets(g, p), s, e)),
            ("Donated.ValidTargets", |g, p, s| each(Donated::valid_targets(g, p), s, e)),
            ("FaceDanced.ValidSourceLocations", |g, p, s| each(FaceDanced::valid_source_locations(g, p), s, e)),
            ("FaceDanced.ValidTargetLocations", |g, _, s| each(FaceDanced::valid_target_locations(g), s, e)),
            ("FaceDancerReplaced.ValidFaceDancers", |g, p, s| each(FaceDancerReplaced::valid_face_dancers(g, p), s, e)),
            ("FactionSelected.ValidFactions", |g, _, s| each(FactionSelected::valid_factions(g), s, e)),
            ("FactionTradeOffered.ValidTargets", |g, p, s| each(FactionTradeOffered::valid_targets(g, p), s, e)),
            ("Karma.ValidKarmaCards", |g, p, s| each(Karma::valid_karma_cards(g, p), s, e)),
            ("KarmaHmsMovement.ValidLocations", |g, _, s| each(KarmaHmsMovement::valid_locations(g), s, e)),
            ("KarmaMonster.ValidTargets", |g, _, s| each(KarmaMonster::valid_targets(g), s, e)),
            ("KarmaRevivalPrevention.GetValidTargets", |g, p, s| each(KarmaRevivalPrevention::get_valid_targets(g, p), s, e)),
            ("KarmaShipmentPrevention.GetValidTargets", |g, p, s| each(KarmaShipmentPrevention::get_valid_targets(g, p), s, e)),
            ("NexusPlayed.ValidChoamCards", |g, p, s| each(NexusPlayed::valid_choam_cards(g, p), s, e)),
            ("NexusPlayed.ValidEcazTerritories", |g, _, s| each(NexusPlayed::valid_ecaz_territories(g), s, e)),
            ("NexusPlayed.ValidEcazFactions", |g, _, s| each(NexusPlayed::valid_ecaz_factions(g), s, e)),
            ("NexusPlayed.ValidMoritaniTerritories", |g, _, s| each(NexusPlayed::valid_moritani_territories(g), s, e)),
            ("PerformBeneGesseritPlacement.ValidLocations", |g, _, s| each(PerformBeneGesseritPlacement::valid_locations(g), s, e)),
            ("PerformMoritaniSetup.ValidLocations", |g, _, s| each(PerformMoritaniSetup::valid_locations(g), s, e)),
            ("PerformHmsMovement.ValidLocations", |g, _, s| each(PerformHmsMovement::valid_locations(g), s, e)),
            ("PerformHmsPlacement.ValidLocations", |g, p, s| each(PerformHmsPlacement::valid_locations(g, p), s, e)),
            ("PlayerReplaced.ValidTargets", |g, _, s| each(PlayerReplaced::valid_targets(g), s, e)),
            ("RequestTleilaxuRevival.ValidTargets", |g, p, s| each(RequestTleilaxuRevival::valid_targets(g, p), s, e)),
            ("ResourcesAudited.ValidFactions", |g, p, s| each(ResourcesAudited::valid_factions(g, p), s, e)),
            ("Retreat.ValidTargets", |g, p, s| each(Retreat::valid_targets(g, p), s, e)),
            ("Revival.ValidRevivalHeroes", |g, p, s| each(Revival::valid_revival_heroes(g, p), s, e)),
            ("Revival.ValidRevivedForceLocations", |g, p, s| each(Revival::valid_revived_force_locations(g, p), s, e)),
            ("SetIncreasedRevivalLimits.ValidTargets", |g, p, s| each(SetIncreasedRevivalLimits::valid_targets(g, p), s, e)),
            ("SetShipmentPermission.ValidTargets", |g, p, s| each(SetShipmentPermission::valid_targets(g, p), s, e)),
            ("Shipment.ValidShipmentLocations", |g, p, s| each(Shipment::valid_shipment_locations(g, p, false, false), s, e)),
            ("Shipment.ValidSourceLocations", |g, p, s| each(Shipment::valid_source_locations(g, p), s, e)),
            ("TerrorPlanted.ValidStrongholds", |g, p, s| each(TerrorPlanted::valid_strongholds(g, p), s, e)),
            ("TerrorPlanted.ValidTerrorTypes", |g, _, s| each(TerrorPlanted::valid_terror_types(g, false), s, enc_enum)),
            ("TerrorRevealed.GetTypes", |g, _, s| each(TerrorRevealed::get_types(g), s, enc_enum)),
            ("TerrorRevealed.ValidSneakAttackTargets", |g, p, s| each(TerrorRevealed::valid_sneak_attack_targets(g, p), s, e)),
            ("Thought.ValidCards", |g, _, s| each(Thought::valid_cards(g), s, e)),
            ("RicheseGaveCard.ValidCards", |g, p, s| each(RicheseGaveCard::valid_cards(g, p), s, e)),
            ("FremenRidesMonster.ValidSources", |g, _, s| each(FremenRidesMonster::valid_sources(g), s, e)),
            ("FremenRidesMonster.ValidTargets", |g, p, s| each(FremenRidesMonster::valid_targets(g, p), s, e)),
            ("FremenSentMonster.ValidTargets", |g, _, s| each(FremenSentMonster::valid_targets(g), s, e)),
            ("Game.LocationsWithAnyForcesNotInStorm", |g, p, s| each(g.locations_with_any_forces_not_in_storm(p), s, e)),
            ("Game.GetSkilledLeaders", |g, p, s| each(g.get_skilled_leaders(p), s, e)),
            ("Game.PlayersOtherThan", |g, p, s| each(g.players_other_than(p), s, e)),
            ("Map.GetResourceCardsInPlay", |g, _, s| each(Map::get_resource_cards_in_play(g), s, resource_card_data)),
            ("Player.TerritoriesWithForces", |g, p, s| each(g.player(p).territories_with_forces(g), s, e)),
            ("Player.UnrevealedTraitors", |g, p, s| each(g.player(p).unrevealed_traitors(), s, e)),
            ("Player.UnrevealedFaceDancers", |g, p, s| each(g.player(p).unrevealed_face_dancers(), s, e)),
            ("TreacheryCardManager.GetCardsInPlay", |g, _, s| each(TreacheryCardManager::get_cards_in_play(g), s, e)),
        ]
    }

    /// LazyBoundary.Item: C# makes new resource card objects on each call, so they are compared by data.
    fn resource_card_data(rc: ResourceCard) -> Value {
        let id = |l: Option<Location>| l.map(|l| l.id().to_string()).unwrap_or_default();
        let b = |x: bool| if x { "True" } else { "False" };
        json!(format!(
            "rcd:{},{},{},{},{}",
            rc.skin_id(),
            id(rc.location()),
            id(rc.discovery_location()),
            b(rc.is_sand_trout()),
            b(rc.is_great_maker())
        ))
    }

    fn new_game(establish: &str) -> Game {
        let mut p = Participation::new();
        p.player_names.insert(100, "Human100".to_string());
        p.seated_players.insert(100, 2);
        let mut g = Game::new_int32_participation(Game::LATEST_VERSION, &cs::Shared::new(p));
        let ev: GameEvent = json::deserialize(establish).expect("event json");
        ev.initialize_game(&g);
        assert!(ev.execute(&mut g, true, true).is_none(), "establish rejected");
        g
    }

    /// The scenario's game state, built as LazyBoundary.Build does.
    fn build(establish: &str, spec: &Value) -> Game {
        let mut g = new_game(establish);
        let hand: Vec<TreacheryCard> = spec["hand"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| if v.is_null() { TreacheryCard::NULL } else { TreacheryCardManager::lookup().find(int(v)).unwrap() })
            .collect();
        let heroes = spec["heroes"].as_array().unwrap();
        for i in 0..g.players.len() {
            let h: Vec<Hero> = heroes[i]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| if v.is_null() { Hero::NULL } else { LeaderManager::hero_lookup().find(int(v)).unwrap() })
                .collect();
            let p = &mut g.players[i];
            p.treachery_cards = hand.clone();
            p.traitors = h.clone();
            p.face_dancers = h;
            if !spec["ally"].is_null() {
                p.ally = Faction(int(&spec["ally"]));
            }
        }
        if let Some(forces) = spec["forces"].as_array() {
            for f in forces {
                let me = g.player_at(int(&f[0]) as usize);
                let l = g.map.location_lookup().find(int(&f[1])).unwrap();
                Player::add_forces(&mut g, me, l, int(&f[2]), false);
            }
        }
        if let Some(discovery) = spec["discovery"].as_array() {
            for d in discovery {
                let ds = g.map.location_lookup().find(int(&d[0])).unwrap().as_discovered_location().unwrap();
                if !d[1].is_null() {
                    let target = g.map.location_lookup().find(int(&d[1])).unwrap();
                    ds.point_at(&mut g.map, target);
                }
                g.just_revealed_discovery_strongholds.push(ds);
            }
        }
        g
    }

    #[test]
    fn lazy_queries_yield_like_csharp() {
        let lazy = &golden("boundary.json")["lazy"];
        let establish = lazy["establish"].as_str().unwrap();
        let table: HashMap<&str, Producer> = producers().into_iter().collect();

        let golden_names: Vec<&str> = lazy["producers"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
        let missing: Vec<_> = golden_names.iter().filter(|n| !table.contains_key(*n)).collect();
        assert!(missing.is_empty(), "producers in boundary.json without a Rust entry: {missing:?}");
        let extra: Vec<_> = table.keys().filter(|n| !golden_names.contains(n)).collect();
        assert!(extra.is_empty(), "Rust entries that LazyBoundary.cs doesn't call: {extra:?}");

        let mut games: HashMap<String, Game> = HashMap::new();
        for spec in lazy["scenarios"].as_array().unwrap() {
            games.insert(spec["name"].as_str().unwrap().to_string(), build(establish, spec));
        }

        let mut failures = Vec::new();
        for case in lazy["cases"].as_array().unwrap() {
            let (scenario, name, who) = (case[0].as_str().unwrap(), case[1].as_str().unwrap(), &case[2]);
            let g = &games[scenario];
            let p = if who.is_null() {
                PlayerIndex::NULL
            } else {
                g.get_player(Some(Faction(int(who)))).expect("player of the case")
            };
            let mut items = Vec::new();
            let r = catch_any(|| table[name](g, p, &mut |v| items.push(v)));
            let got = json!([items, r.err()]);
            let want = json!([case[3], case[4]]);
            if got[0] != want[0] || !same(&want[1], &got[1]) {
                failures.push(format!("{scenario} {name} player {who}:\n  C#:   {want}\n  Rust: {got}"));
            }
        }
        assert!(failures.is_empty(), "{} of {} lazy cases differ:\n{}", failures.len(), lazy["cases"].as_array().unwrap().len(), failures.join("\n"));
    }
}
