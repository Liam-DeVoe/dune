//! Bot_Knowledge against C#: results, exceptions and draw counts of every Knowledge member.
//!
//! `golden/knowledge-*.jsonl.gz` comes from `bots/csharp-ref` (see its Program.cs):
//!
//! ```sh
//! export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"
//! cd rust/bots/csharp-ref
//! dotnet run -c Release -- knowledge ../../traces/sample/bots-2026-00008.jsonl.gz ../tests/golden/knowledge-00008.jsonl.gz 20
//! ```
//!
//! The `77-*` files come from `Oracle bots --seed 77 --games 6` (games 1, 2 and 4, every 20, 35
//! and 35 steps), picked because they have alliances, Voice, Prescience and Clairvoyance. The
//! source traces aren't kept: the golden files hold the events themselves.
//!
//! The file holds the events of a bot game and, every few steps, one fresh bot per player with a
//! known seed and the members C# called on it. This test replays the events and makes the same
//! calls. Members that reach Bot_Battle (`WinnableNearbyStronghold`, `IsSafeAndNearby` with
//! `mayFight`) are left out until B2 lands.

use std::io::{BufRead, BufReader};

use flate2::read::GzDecoder;
use serde_json::Value;
use treachery_bots::prelude::*;
use treachery_engine::error::catch_panic;
use treachery_engine::game::Participation;

/// The C# `Fmt` of csharp-ref/Program.cs.
trait Fmt {
    fn fmt(&self) -> String;
}

impl Fmt for bool {
    fn fmt(&self) -> String {
        if *self { "true".into() } else { "false".into() }
    }
}
impl Fmt for i32 {
    fn fmt(&self) -> String {
        self.to_string()
    }
}
impl Fmt for f32 {
    fn fmt(&self) -> String {
        dotnet_compat::fmt::f32_to_string(*self)
    }
}
impl Fmt for Faction {
    fn fmt(&self) -> String {
        format!("F{}", self.0)
    }
}
impl Fmt for PlayerIndex {
    fn fmt(&self) -> String {
        format!("P{}", self.index)
    }
}
impl Fmt for Location {
    fn fmt(&self) -> String {
        format!("L{}", self.id())
    }
}
impl Fmt for Territory {
    fn fmt(&self) -> String {
        format!("T{}", self.id())
    }
}
impl Fmt for TreacheryCard {
    fn fmt(&self) -> String {
        format!("C{}", self.id())
    }
}
impl Fmt for Hero {
    fn fmt(&self) -> String {
        if let Some(c) = self.as_treachery_card() {
            c.fmt()
        } else {
            format!("H{}", self.id())
        }
    }
}
impl Fmt for Battalion {
    fn fmt(&self) -> String {
        format!("B{}:{}:{}", self.faction.0, self.amount_of_forces, self.amount_of_special_forces)
    }
}
impl Fmt for BattalionInLocation {
    fn fmt(&self) -> String {
        format!("{}={}", self.location.fmt(), self.battalion.fmt())
    }
}
impl Fmt for ClairVoyanceQandA {
    fn fmt(&self) -> String {
        "QandA".into()
    }
}
impl Fmt for Rc<Prescience> {
    fn fmt(&self) -> String {
        "Prescience".into()
    }
}
impl Fmt for Rc<Voice> {
    fn fmt(&self) -> String {
        "Voice".into()
    }
}
impl<T: Fmt> Fmt for Option<T> {
    fn fmt(&self) -> String {
        match self {
            None => "null".into(),
            Some(v) => v.fmt(),
        }
    }
}
impl<T: Fmt> Fmt for Vec<T> {
    fn fmt(&self) -> String {
        format!("[{}]", self.iter().map(Fmt::fmt).collect::<Vec<_>>().join(","))
    }
}
impl Fmt for NetDictionary<Location, Battalion> {
    fn fmt(&self) -> String {
        format!("[{}]", self.iter().map(|(l, b)| format!("{}={}", l.fmt(), b.fmt())).collect::<Vec<_>>().join(","))
    }
}
impl Fmt for IndexMap<Location, Battalion> {
    fn fmt(&self) -> String {
        format!("[{}]", self.iter().map(|(l, b)| format!("{}={}", l.fmt(), b.fmt())).collect::<Vec<_>>().join(","))
    }
}

struct Args<'a> {
    game: &'a Game,
    bot: &'a ClassicBot,
    a: Vec<String>,
}

impl Args<'_> {
    fn n(&self, i: usize) -> i32 {
        self.a[i][1..].parse().unwrap()
    }
    fn kind(&self, i: usize) -> char {
        self.a[i].chars().next().unwrap()
    }
    fn is_null(&self, i: usize) -> bool {
        self.a[i] == "N"
    }
    fn p(&self, i: usize) -> PlayerIndex {
        self.game.player_at(self.n(i) as usize)
    }
    fn opt_p(&self, i: usize) -> Option<PlayerIndex> {
        if self.is_null(i) { None } else { Some(self.p(i)) }
    }
    fn f(&self, i: usize) -> Faction {
        Faction(self.n(i))
    }
    fn l(&self, i: usize) -> Location {
        self.game.map.location_lookup().find(self.n(i)).unwrap()
    }
    fn t(&self, i: usize) -> Territory {
        self.game.map.territory_lookup().find(self.n(i)).unwrap()
    }
    fn c(&self, i: usize) -> TreacheryCard {
        TreacheryCardManager::lookup().find(self.n(i)).unwrap()
    }
    fn b(&self, i: usize) -> bool {
        self.a[i] == "true"
    }
    fn bat(&self, i: usize) -> Battalion {
        let l = self.game.map.location_lookup().find(self.a[i][4..].parse().unwrap()).unwrap();
        self.bot.forces_on_planet(self.game).get(&l).unwrap().clone()
    }
    fn hand(&self, i: usize) -> Vec<TreacheryCard> {
        let p = self.game.player_at(self.a[i][5..].parse::<usize>().unwrap());
        self.game.player(p).treachery_cards.clone()
    }
}

/// Calls the member C# called. Panics on a member or overload this table doesn't know.
fn call(m: &str, x: &Args) -> String {
    let (g, bot) = (x.game, x.bot);
    let len = x.a.len();
    match (m, len) {
        ("MaxExpectedStormMoves", 0) => bot.max_expected_storm_moves(g).fmt(),
        ("MayFlipToAdvisors", 0) => bot.may_flip_to_advisors(g).fmt(),
        ("ForcesOnPlanet", 0) => bot.forces_on_planet(g).fmt(),
        ("Others", 0) => bot.others(g).fmt(),
        ("Opponents", 0) => bot.opponents(g).fmt(),
        ("MeAndMyAlly", 0) => bot.me_and_my_ally(g).fmt(),
        ("LastTurn", 0) => bot.last_turn(g).fmt(),
        ("AlmostLastTurn", 0) => bot.almost_last_turn(g).fmt(),
        ("OpponentsToShipAndMove", 0) => bot.opponents_to_ship_and_move(g).fmt(),
        ("NrOfNonWinningPlayersToShipAndMoveIncludingMe", 0) => bot.nr_of_non_winning_players_to_ship_and_move_including_me(g).fmt(),
        ("IAmWinning", 0) => bot.i_am_winning(g).fmt(),
        ("OpponentsAreWinning", 0) => bot.opponents_are_winning(g).fmt(),
        ("MyPrescience", 0) => bot.my_prescience(g).fmt(),
        ("MyVoice", 0) => bot.my_voice(g).fmt(),
        ("MayUseUselessAsKarma", 0) => bot.may_use_useless_as_karma(g).fmt(),
        ("IAmDesperateForResources", 0) => bot.i_am_desperate_for_resources(g).fmt(),
        ("ResourcesIncludingAllyContribution", 0) => bot.resources_including_ally_contribution(g).fmt(),
        ("ResourcesIncludingAllyAndEmperorContribution", 0) => bot.resources_including_ally_and_emperor_contribution(g).fmt(),
        ("ResourcesFromAlly", 0) => bot.resources_from_ally(g).fmt(),
        ("ResourcesFromEmperor", 0) => bot.resources_from_emperor(g).fmt(),
        ("AllyResources", 0) => bot.ally_resources(g).fmt(),
        ("BattalionThatShouldBeMovedDueToAllyPresence", 0) => bot.battalion_that_should_be_moved_due_to_ally_presence(g).fmt(),
        ("BiggestBattalionThreatenedByStormWithoutSpice", 0) => bot.biggest_battalion_threatened_by_storm_without_spice(g).fmt(),
        ("BiggestBattalionInSpicelessNonStrongholdLocationOnRock", 0) => {
            bot.biggest_battalion_in_spiceless_non_stronghold_location_on_rock(g).fmt()
        }
        ("BiggestBattalionInSpicelessNonStrongholdLocationInSandOrNotNearStronghold", 0) => {
            bot.biggest_battalion_in_spiceless_non_stronghold_location_in_sand_or_not_near_stronghold(g).fmt()
        }
        ("BiggestBattalionInSpicelessNonStrongholdLocationNotNearStrongholdAndSpice", 0) => {
            bot.biggest_battalion_in_spiceless_non_stronghold_location_not_near_stronghold_and_spice(g).fmt()
        }
        ("BiggestLargeUnthreatenedMovableBattalionInStrongholdNearVacantStronghold", 0) => {
            bot.biggest_large_unthreatened_movable_battalion_in_stronghold_near_vacant_stronghold(g).fmt()
        }
        ("BiggestMovableStackOfAdvisorsInStrongholdNearVacantStronghold", 0) => {
            bot.biggest_movable_stack_of_advisors_in_stronghold_near_vacant_stronghold(g).fmt()
        }
        ("BiggestLargeUnthreatenedMovableBattalionInStrongholdNearSpice", 0) => {
            bot.biggest_large_unthreatened_movable_battalion_in_stronghold_near_spice(g).fmt()
        }
        ("IMustPayForForcesInBattle", 0) => bot.i_must_pay_for_forces_in_battle(g).fmt(),
        ("NrOfBattlesToFight", 0) => bot.nr_of_battles_to_fight(g).fmt(),
        ("SafeOrKnownTraitorLeaders", 0) => bot.safe_or_known_traitor_leaders(g).fmt(),

        ("MyClairvoyanceAboutEnemyDefenseInCurrentBattle", 2) => {
            ClassicBot::my_clairvoyance_about_enemy_defense_in_current_battle(g, x.p(1)).fmt()
        }
        ("PlayerClairvoyanceAboutEnemyWeaponInCurrentBattle", 2) => {
            ClassicBot::player_clairvoyance_about_enemy_weapon_in_current_battle(g, x.p(1)).fmt()
        }
        ("CardsUnknownToPlayer", 2) => ClassicBot::cards_unknown_to_player(g, x.p(1)).fmt(),
        ("CardsPlayerHasOrMightHave", 2) => ClassicBot::cards_player_has_or_might_have(g, x.p(1)).fmt(),
        ("CanShip", 2) => ClassicBot::can_ship(g, x.p(1)).fmt(),
        ("Weapons", 5) => ClassicBot::weapons(g, x.p(1), None, None, None).fmt(),
        ("Defenses", 4) => ClassicBot::defenses(g, x.p(1), None, None).fmt(),
        ("UselessAsWeapon", 3) => ClassicBot::useless_as_weapon(g, x.p(1), None).fmt(),
        ("UselessAsDefense", 3) => ClassicBot::useless_as_defense(g, x.p(1), None).fmt(),
        ("MayPlayNoWeapon", 3) => ClassicBot::may_play_no_weapon(g, x.p(1), None).fmt(),
        ("MayPlayNoDefense", 3) => ClassicBot::may_play_no_defense(g, x.p(1), None).fmt(),
        ("CountDifferentWeaponTypes", 1) => ClassicBot::count_different_weapon_types(&x.hand(0)).fmt(),
        ("CountDifferentDefenseTypes", 1) => ClassicBot::count_different_defense_types(&x.hand(0)).fmt(),

        ("WinWasPredictedByMeThisTurn", 1) => bot.win_was_predicted_by_me_this_turn(g, x.f(0)).fmt(),
        ("IsWinning", 1) if x.kind(0) == 'P' => bot.is_winning(g, x.p(0)).fmt(),
        ("IsWinning", 1) => bot.is_winning_faction(g, x.f(0)).fmt(),
        ("IsWinningOpponent", 1) if x.kind(0) == 'P' => bot.is_winning_opponent(g, x.p(0)).fmt(),
        ("IsWinningOpponent", 1) => bot.is_winning_opponent_faction(g, x.f(0)).fmt(),
        ("IsAlmostWinningOpponent", 1) => bot.is_almost_winning_opponent(g, x.p(0)).fmt(),
        ("IWillBeAggressorAgainst", 1) => bot.i_will_be_aggressor_against(g, Some(x.p(0))).fmt(),
        ("OpponentCardsUnknownToPlayer", 3) => ClassicBot::opponent_cards_unknown_to_player(g, x.p(1), x.p(2)).fmt(),
        ("KnownCardsHeldByOtherPlayer", 3) => ClassicBot::known_cards_held_by_other_player(g, x.p(1), x.p(2)).fmt(),
        ("KnownOpponentWeapons", 3) => ClassicBot::known_opponent_weapons(g, x.p(1), x.p(2)).fmt(),
        ("KnownOpponentCards", 3) => ClassicBot::known_opponent_cards(g, x.p(1), x.p(2)).fmt(),
        ("KnownOpponentDefenses", 3) => ClassicBot::known_opponent_defenses(g, x.p(1), x.p(2)).fmt(),
        ("MaxReinforcedDialTo", 4) => ClassicBot::max_reinforced_dial_to(g, x.p(1), x.p(2), x.t(3)).fmt(),
        ("CardQuality", 2) => bot.card_quality(g, x.c(0), x.opt_p(1)).fmt(),
        ("IsKnownTo", 3) => ClassicBot::is_known_to(g, x.p(1), x.c(2)).fmt(),

        ("ResourcesIn", 1) => bot.resources_in(g, x.l(0)).fmt(),
        ("HasResources", 1) => bot.has_resources(g, x.l(0)).fmt(),
        ("IsStronghold", 1) => bot.is_stronghold(g, x.l(0)).fmt(),
        ("NotOccupiedByOthers", 1) if x.kind(0) == 'L' => bot.not_occupied_by_others(g, x.l(0)).fmt(),
        ("NotOccupiedByOthers", 1) => bot.not_occupied_by_others(g, x.t(0)).fmt(),
        ("NotOccupied", 1) if x.kind(0) == 'L' => bot.not_occupied(g, x.l(0)).fmt(),
        ("NotOccupied", 1) => bot.not_occupied(g, x.t(0)).fmt(),
        ("Vacant", 1) if x.kind(0) == 'L' => bot.vacant(g, x.l(0)).fmt(),
        ("Vacant", 1) => bot.vacant(g, x.t(0)).fmt(),
        ("OccupiedByOpponent", 1) if x.kind(0) == 'L' => bot.occupied_by_opponent(g, x.l(0)).fmt(),
        ("OccupiedByOpponent", 1) => bot.occupied_by_opponent(g, x.t(0)).fmt(),
        ("GetOpponentThatOccupies", 1) if x.kind(0) == 'L' => bot.get_opponent_that_occupies(g, x.l(0)).fmt(),
        ("GetOpponentThatOccupies", 1) => bot.get_opponent_that_occupies(g, x.t(0)).fmt(),
        ("StormWillProbablyHit", 1) => bot.storm_will_probably_hit(g, x.l(0)).fmt(),
        ("VacantAndSafeFromStorm", 1) => bot.vacant_and_safe_from_storm(g, x.l(0)).fmt(),
        ("IDontHaveAdvisorsIn", 1) => bot.i_dont_have_advisors_in(g, x.l(0)).fmt(),
        ("AllyDoesntBlock", 1) if x.kind(0) == 'L' => bot.ally_doesnt_block(g, x.l(0)).fmt(),
        ("AllyDoesntBlock", 1) => bot.ally_doesnt_block(g, x.t(0)).fmt(),
        ("InStorm", 1) => bot.in_storm(g, x.l(0)).fmt(),
        ("MayFleeOutOf", 1) => bot.may_flee_out_of(g, x.l(0)).fmt(),
        ("WithinDistance", 3) => bot.within_distance(g, x.l(0), x.l(1), x.n(2)).fmt(),
        ("ProbablySafeFromMonster", 1) => bot.probably_safe_from_monster(g, x.t(0)).fmt(),
        ("OccupyingOpponentIn", 3) => bot.occupying_opponent_in(g, x.p(1), x.t(2)).fmt(),
        ("OccupyingOpponentsIn", 3) if x.kind(2) == 'L' => ClassicBot::occupying_opponents_in(g, x.p(1), x.l(2)).fmt(),
        ("OccupyingOpponentsIn", 3) => ClassicBot::occupying_opponents_in(g, x.p(1), x.t(2)).fmt(),
        ("MaxDial", 5) => ClassicBot::max_dial(g, x.p(1), x.t(2), x.opt_p(3), x.b(4)).fmt(),
        ("MaxDial", 4) => ClassicBot::max_dial_of_battalion(g, x.n(1), &x.bat(2), x.f(3)).fmt(),
        ("TotalMaxDialOfOpponents", 1) => bot.total_max_dial_of_opponents(g, x.t(0)).fmt(),

        ("ValidMovementLocations", 2) => bot.valid_movement_locations(g, x.l(0), &x.bat(1)).fmt(),
        ("VacantAndSafeNearbyStronghold", 2) => bot.vacant_and_safe_nearby_stronghold(g, x.l(0), &x.bat(1)).fmt(),
        ("UnthreatenedAndSafeNearbyStronghold", 2) => bot.unthreatened_and_safe_nearby_stronghold(g, x.l(0), &x.bat(1)).fmt(),
        ("WeakAndSafeNearbyStronghold", 2) => bot.weak_and_safe_nearby_stronghold(g, x.l(0), &x.bat(1)).fmt(),
        ("NearbyStrongholdOfWinningOpponent", 3) => bot.nearby_stronghold_of_winning_opponent(g, x.l(0), &x.bat(1), x.b(2)).fmt(),
        ("NearbyStrongholdOfAlmostWinningOpponent", 3) => {
            bot.nearby_stronghold_of_almost_winning_opponent(g, x.l(0), &x.bat(1), x.b(2)).fmt()
        }
        ("BestSafeAndNearbyResources", 3) => bot.best_safe_and_nearby_resources(g, x.l(0), &x.bat(1), x.b(2)).fmt(),
        ("BestSafeAndNearbyDiscovery", 3) => bot.best_safe_and_nearby_discovery(g, x.l(0), &x.bat(1), x.b(2)).fmt(),
        ("NearbyBattalionsOutsideStrongholds", 1) => bot.nearby_battalions_outside_strongholds(g, x.l(0)).fmt(),
        ("WithinRange", 3) => bot.within_range(g, x.l(0), x.l(1), &x.bat(2)).fmt(),
        ("IsSafeAndNearby", 4) => bot.is_safe_and_nearby(g, x.l(0), x.l(1), &x.bat(2), x.b(3)).fmt(),

        _ => panic!("no Rust call for {m} with {len} arguments"),
    }
}

/// As differ's `replay::participation` (and the oracle's Verifier).
fn participation(o: &Value) -> Participation {
    let int = |v: &Value| v.as_i64().unwrap() as i32;
    let mut p = Participation { bots_speed: int(&o["BotsSpeed"]), ..Default::default() };
    for kv in o["PlayerNames"].as_array().unwrap() {
        p.player_names.insert(int(&kv[0]), kv[1].as_str().unwrap().to_string());
    }
    for kv in o["SeatedPlayers"].as_array().unwrap() {
        p.seated_players.insert(int(&kv[0]), int(&kv[1]));
    }
    for (key, set) in [
        ("Observers", &mut p.observers),
        ("Hosts", &mut p.hosts),
        ("AvailableSeats", &mut p.available_seats),
        ("Kicked", &mut p.kicked),
    ] {
        for v in o[key].as_array().unwrap() {
            set.insert(int(v));
        }
    }
    p
}

fn run(golden: &str) {
    let path = format!("{}/tests/golden/{golden}", env!("CARGO_MANIFEST_DIR"));
    let file = std::fs::File::open(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let reader = BufReader::new(GzDecoder::new(file));

    let mut game: Option<Game> = None;
    let mut bot: Option<(i64, i64, ClassicBot)> = None;
    // After a difference, the later probes of that bot depend on its draws: skip them.
    let mut skip: Option<(i64, i64)> = None;
    let mut probes = 0;
    let mut failures: Vec<String> = Vec::new();

    for line in reader.lines() {
        let line = line.unwrap();
        let o: Value = serde_json::from_str(&line).unwrap();
        match o["k"].as_str().unwrap() {
            "header" => {
                let participation = participation(&o["participation"]);
                game = Some(Game::new_int32_participation(o["version"].as_i64().unwrap() as i32, &Shared::new(participation)));
            }
            "step" => {
                let game = game.as_mut().unwrap();
                let ev: GameEvent = treachery_engine::json::deserialize(o["event"].as_str().unwrap()).unwrap();
                ev.initialize_game(game);
                game.map.mark_neighbour_cache_stale();
                let (validate, is_host) = (o["validate"].as_bool().unwrap(), o["isHost"].as_bool().unwrap());
                let _ = catch_panic(|| ev.execute(game, validate, is_host));
            }
            "probe" => {
                let game = game.as_ref().unwrap();
                let (n, p) = (o["n"].as_i64().unwrap(), o["p"].as_i64().unwrap());
                if skip == Some((n, p)) {
                    continue;
                }
                if !matches!(&bot, Some((bn, bp, _)) if *bn == n && *bp == p) {
                    game.map.mark_neighbour_cache_stale();
                    let player = game.player_at(p as usize);
                    let faction = game.player(player).faction;
                    let seed = o["seed"].as_i64().unwrap() as i32;
                    let b = ClassicBot::new(player, BotParameters::for_faction(faction), SharedRandom::from_seed(seed));
                    bot = Some((n, p, b));
                }
                let b = &bot.as_ref().unwrap().2;
                let m = o["m"].as_str().unwrap();
                let a: Vec<String> = o["a"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
                let args = Args { game, bot: b, a: a.clone() };
                let actual = match catch_panic(|| call(m, &args)) {
                    Ok(r) => r,
                    Err(_) => "EX".to_string(),
                };
                let draws = b.random.get_count() as i64;
                let (expected, expected_draws) = (o["r"].as_str().unwrap(), o["d"].as_i64().unwrap());
                // A probe that throws in C# must panic in Rust; the texts aren't compared.
                let expected = if expected.starts_with("EX:") { "EX" } else { expected };
                probes += 1;
                if actual != expected || draws != expected_draws {
                    failures.push(format!(
                        "step {n} player {p} {m}({}): C# {expected} ({expected_draws} draws), Rust {actual} ({draws} draws)",
                        a.join(", ")
                    ));
                    skip = Some((n, p));
                }
            }
            k => panic!("unknown line kind {k}"),
        }
    }

    assert!(probes > 0, "{golden}: no probes");
    if !failures.is_empty() {
        let shown: Vec<&String> = failures.iter().take(40).collect();
        panic!("{golden}: {} of {probes} probes differ:\n{}", failures.len(), shown.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n"));
    }
}

#[test]
fn knowledge_00008() {
    run("knowledge-00008.jsonl.gz");
}

#[test]
fn knowledge_77_00001() {
    run("knowledge-77-00001.jsonl.gz");
}

#[test]
fn knowledge_77_00002() {
    run("knowledge-77-00002.jsonl.gz");
}

#[test]
fn knowledge_77_00004() {
    run("knowledge-77-00004.jsonl.gz");
}
