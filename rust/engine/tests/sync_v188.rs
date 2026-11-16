//! The rules added in game version 188: the
//! alliance phase that ends once every faction is done (AlliancePassed,
//! EnterAlliancePhase, EndNexusIfEveryoneIsDone, AllianceBroken and
//! AllianceOffered.ValidTargets), the v188 alliance status, and
//! PrescienceSeenPlans (BattleRevision records the withdrawn plan, Battle.Validate
//! holds the next plan to it, EndPhase.ResetBattle clears it).
//!
//! The test plays these steps and compares every "name = value" line it writes with the
//! snapshot `tests/golden/sync_v188.txt` (see common/mod.rs for how to update it).

mod common;

use std::rc::Rc;

use treachery_engine::error::catch_panic;
use treachery_engine::events::Event;
use treachery_engine::game::Game;
use treachery_engine::game::GameStatus;
use treachery_engine::json;
use treachery_engine::heroes::leader_manager::LeaderManager;
use treachery_engine::prelude::*;
use treachery_engine::cards::treachery_card_manager::TreacheryCardManager;

// Choam, Atreides and Richese (the same text as review_pkg_k.rs).
const ESTABLISH: &str = r#"{"$type":"EstablishPlayers","Settings":{"NumberOfPlayers":3,"MaximumTurns":8,"InitialRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"AllowedFactionsInPlay":[90,20,100],"AutoOpenEmptySeats":true,"AsyncPlay":false,"AsyncPlayMessageIntervalMinutes":0},"Seed":1073548723,"MaximumNumberOfPlayers":3,"MaximumTurns":8,"ApplicableRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"_players":"","_factionsInPlay":"Choam,Atreides,Richese","Initiator":0,"Time":"2024-01-01T12:02:17+00:00"}"#;

#[derive(Default)]
struct Lines(Vec<String>);

impl Lines {
    fn p(&mut self, name: &str, v: impl std::fmt::Display) {
        self.0.push(format!("{name} = {v}"));
    }

    /// The value, or "exception <panic message>".
    fn try_(&mut self, name: &str, f: impl FnOnce() -> String) {
        match catch_panic(f) {
            Ok(v) => self.p(name, v),
            Err(e) => self.p(name, format!("exception {}", e)),
        }
    }
}

fn new_game(version: i32) -> Game {
    // User 100 ("Human100") sits in seat 2.
    let mut g = Game::of_version(version);
    g.human_seats.insert(Seat::new(2));
    let e: HistoryEntry = json::deserialize(ESTABLISH).expect("event json");
    assert!(g.play_for_message(e, true, true).is_none(), "establish rejected");
    g
}

fn idx(g: &Game, f: Faction) -> PlayerId {
    g.get_player(Some(f)).unwrap()
}

fn factions(fs: impl IntoIterator<Item = Faction>) -> String {
    format!("[{}]", fs.into_iter().map(|f| f.to_string()).collect::<Vec<_>>().join(","))
}

fn message(m: Option<Message>, none: &str) -> String {
    m.map_or(none.to_string(), |m| m.plain_text())
}

fn applicable(g: &mut Game) -> String {
    let host: Vec<_> = g.get_applicable_events(None, true).into_iter().map(|t| t.name()).collect();
    let mut parts = vec![format!("host:{}", host.join("+"))];
    for i in 0..g.players.len() {
        let p = g.player_at(i);
        let names: Vec<_> = g.get_applicable_events(Some(p), false).into_iter().map(|t| t.name()).collect();
        parts.push(format!("{}:{}", treachery_engine::enum_type::name_or_absent(g.player(p).faction), names.join("+")));
    }
    parts.join(" ")
}

fn state(l: &mut Lines, g: &mut Game, at: &str) {
    l.p(&format!("{at}.phase"), g.turn.current_phase);
    l.p(&format!("{at}.acted"), factions(g.turn.has_acted_or_passed.clone()));
    let offers: Vec<_> = g.alliances.current_alliance_offers.iter().map(|o| format!("{}>{}", treachery_engine::enum_type::name_or_absent(o.initiator()), treachery_engine::enum_type::name_or_absent(o.target))).collect();
    l.p(&format!("{at}.offers"), offers.join(","));
    let allies: Vec<_> = g.players.iter().map(|p| format!("{}:{}", treachery_engine::enum_type::name_or_absent(p.faction), treachery_engine::enum_type::name_or_absent(p.ally))).collect();
    l.p(&format!("{at}.allies"), allies.join(","));
    l.p(&format!("{at}.nexus_occured"), if g.blow.nexus_has_occurred { "True" } else { "False" });
    let a = applicable(g);
    l.p(&format!("{at}.applicable"), a);
    for i in 0..g.players.len() {
        let p = g.player_at(i);
        let s = GameStatus::determine_status(g, Some(p), true);
        let waiting = factions(s.waiting_for_players.iter().filter_map(|w| g.player(w.unwrap()).faction));
        l.p(&format!("{at}.status.{}", treachery_engine::enum_type::name_or_absent(g.player(p).faction)), format!("{} | waiting {waiting}", s.get_message(p, false).plain_text()));
    }
}

fn exec(l: &mut Lines, g: &mut Game, at: &str, e: Event) {
    let by = e.legacy_initiator();
    let applicable = catch_panic(|| g.get_applicable_events(By::new(g, by).player, false).contains(&e.event_type()));
    match applicable {
        Ok(b) => l.p(&format!("{at}.applicable"), if b { "True" } else { "False" }),
        Err(x) => l.p(&format!("{at}.applicable"), format!("exception {}", x)),
    }
    l.try_(&format!("{at}.validate"), || message(e.validate_for(g, by).err().map(|r| r.0), "valid"));
    l.try_(&format!("{at}.message"), || e.describe(g, By::new(g, by)).plain_text());
    let r = catch_panic(|| message(g.play_for_message(HistoryEntry::new(by, e.clone()), false, true), "ok"));
    match r {
        Ok(v) => l.p(&format!("{at}.execute"), v),
        Err(x) => l.p(&format!("{at}.execute"), format!("exception {}", x)),
    }
    state(l, g, at);
}

fn offered(g: &Game, by: Faction, target: Faction) -> Event {
    let mut e = AllianceOffered::new(g, by);
    e.target = Some(target);
    Rc::new(e).into_event()
}

fn passed(g: &Game, by: Faction) -> Event {
    Rc::new(AlliancePassed::new(g, by)).into_event()
}

fn alliance_phase(l: &mut Lines, v: i32) {
    let at = format!("v{v}.phase");
    let mut g = new_game(v);
    g.turn.has_acted_or_passed.push(Faction::Atreides);
    g.enter_alliance_phase(true);
    state(l, &mut g, &format!("{at}.enter"));

    let e = offered(&g, Faction::Choam, Faction::Atreides);
    exec(l, &mut g, &format!("{at}.choam_offers"), e);
    let e = passed(&g, Faction::Richese);
    exec(l, &mut g, &format!("{at}.richese_passes"), e);
    let e = offered(&g, Faction::Atreides, Faction::Richese);
    exec(l, &mut g, &format!("{at}.atreides_offers_richese"), e);
    let e = offered(&g, Faction::Atreides, Faction::Choam);
    exec(l, &mut g, &format!("{at}.atreides_accepts"), e);

    let mut g2 = new_game(v);
    g2.turn.has_acted_or_passed.push(Faction::Richese);
    g2.enter_alliance_phase(false);
    state(l, &mut g2, &format!("{at}.enter_b"));
    for (name, f) in [("b_choam_passes", Faction::Choam), ("b_atreides_passes", Faction::Atreides), ("b_richese_passes", Faction::Richese)] {
        let e = passed(&g2, f);
        exec(l, &mut g2, &format!("{at}.{name}"), e);
    }
}

fn break_in_alliance_phase(l: &mut Lines, v: i32) {
    let at = format!("v{v}.break");
    let mut g = new_game(v);
    let choam = idx(&g, Faction::Choam);
    let atreides = idx(&g, Faction::Atreides);
    g.player_mut(choam).ally = Some(Faction::Atreides);
    g.player_mut(atreides).ally = Some(Faction::Choam);
    g.enter_alliance_phase(true);
    let e = offered(&g, Faction::Richese, Faction::Atreides);
    exec(l, &mut g, &format!("{at}.richese_offers_atreides"), e);
    let e = passed(&g, Faction::Atreides);
    exec(l, &mut g, &format!("{at}.atreides_passes"), e);
    let e = Rc::new(AllianceBroken::new(&g, Faction::Choam)).into_event();
    exec(l, &mut g, &format!("{at}.choam_breaks"), e);
    let e = offered(&g, Faction::Atreides, Faction::Richese);
    exec(l, &mut g, &format!("{at}.atreides_offers_richese"), e);
}

fn seen(g: &Game, plans: &[Option<BattlePlan>]) -> String {
    let entries: Vec<_> = g
        .battle.prescience_seen_plans
        .iter()
        .map(|(aspect, plan)| {
            let i = plans.iter().position(|p| p.as_ref() == Some(plan)).map_or(-1, |i| i as i32);
            format!("{}:{}:plan{i}", name_or_absent(*aspect), treachery_engine::enum_type::name_or_absent(plan.initiator()))
        })
        .collect();
    format!("[{}]", entries.join(","))
}

fn revise(g: &mut Game) -> String {
    let e = Event::from(BattleRevision::default());
    message(g.play_for_message(HistoryEntry::new(Faction::Choam, e), false, true), "ok")
}

fn prescience(l: &mut Lines, v: i32) {
    let at = format!("v{v}.prescience");
    let mut g = new_game(v);
    let choam = idx(&g, Faction::Choam);
    let cards = TreacheryCardManager::get_cards_in_and_outside_play();
    let weapons: Vec<_> = cards.iter().copied().filter(|c| c.is_projectile_weapon()).take(2).collect();
    let defense = cards.iter().copied().find(|c| c.is_projectile_defense()).unwrap();
    l.p(&format!("{at}.cards"), format!("{},{},{}", weapons[0].id(), weapons[1].id(), defense.id()));
    {
        let hand = &mut g.player_mut(choam).treachery_cards;
        hand.clear();
        hand.extend(weapons.iter().copied());
        hand.push(defense);
    }
    l.p(&format!("{at}.leaders_before"), g.player(choam).leaders.len());
    g.player_mut(choam).leaders.extend(LeaderManager::get_leaders(Faction::Choam));
    let leader0 = g.player(choam).leaders[0];
    let leader1 = g.player(choam).leaders[1];
    l.p(&format!("{at}.leaders"), format!("{},{}", leader0.id(), leader1.id()));
    let arrakeen = g.board.map.arrakeen();
    Player::add_forces(&mut g, choam, arrakeen.into(), 3, false).unwrap();
    let territory = g.board.map.arrakeen().territory();
    let battle = BattleInitiated { target: Some(Faction::Choam), territory: Some(territory) };
    g.battle.current_battle = Some(battle.battle(By::new(&g, Faction::Atreides), 0));

    let plan = |g: &Game, hero: Leader, weapon: Option<TreacheryCard>, def: Option<TreacheryCard>, half: u16| -> BattlePlan {
        let b = Battle { hero: Some(Hero::Leader(hero)), weapon, defense: def, forces_at_half_strength: half, ..Default::default() };
        b.plan(By::new(g, Faction::Choam))
    };
    let validate = |l: &mut Lines, g: &Game, name: &str, p: &BattlePlan| l.try_(name, || message(p.validate(g).err().map(|r| r.0), "valid"));

    let mut plans: Vec<Option<BattlePlan>> = vec![None; 6];
    plans[0] = Some(plan(&g, leader0, Some(weapons[0]), Some(defense), 0));
    validate(l, &g, &format!("{at}.plan0.validate"), plans[0].as_ref().unwrap());
    g.battle.defender_plan = plans[0].clone();

    g.battle.current_prescience = Some(PrescienceUsed { submitter: By::new(&g, Faction::Atreides), aspect: Some(PrescienceAspect::Weapon) });
    let aspects = |g: &Game, f: Faction| Battle::aspects_seen_by_opponent(g, idx(g, f)).iter().map(|a| name_or_absent(*a)).collect::<Vec<_>>().join(",");
    l.try_(&format!("{at}.aspects"), || aspects(&g, Faction::Choam));
    l.try_(&format!("{at}.aspects_of_atreides"), || aspects(&g, Faction::Atreides));
    l.try_(&format!("{at}.revise1"), || revise(&mut g));
    l.p(&format!("{at}.revise1.seen"), seen(&g, &plans));
    l.p(&format!("{at}.revise1.defender_plan_is_null"), if g.battle.defender_plan.is_none() { "True" } else { "False" });

    plans[1] = Some(plan(&g, leader0, Some(weapons[1]), Some(defense), 0));
    validate(l, &g, &format!("{at}.other_weapon.validate"), plans[1].as_ref().unwrap());
    plans[2] = Some(plan(&g, leader1, Some(weapons[0]), None, 0));
    validate(l, &g, &format!("{at}.same_weapon.validate"), plans[2].as_ref().unwrap());

    g.battle.defender_plan = plans[2].clone();
    g.battle.atreides_karma = true;
    g.battle.current_prescience = Some(PrescienceUsed { submitter: By::new(&g, Faction::Atreides), aspect: Some(PrescienceAspect::Dial) });
    l.try_(&format!("{at}.aspects_karma"), || aspects(&g, Faction::Choam));
    l.try_(&format!("{at}.revise2"), || revise(&mut g));
    l.p(&format!("{at}.revise2.seen"), seen(&g, &plans));

    plans[3] = Some(plan(&g, leader1, Some(weapons[0]), None, 0));
    validate(l, &g, &format!("{at}.identical.validate"), plans[3].as_ref().unwrap());
    plans[4] = Some(plan(&g, leader0, Some(weapons[0]), Some(defense), 0));
    validate(l, &g, &format!("{at}.leader_and_defense_changed.validate"), plans[4].as_ref().unwrap());
    plans[5] = Some(plan(&g, leader1, Some(weapons[0]), None, 1));
    validate(l, &g, &format!("{at}.dial_changed.validate"), plans[5].as_ref().unwrap());

    g.battle.defender_plan = None;
    l.try_(&format!("{at}.revise_without_plan"), || revise(&mut g));
    l.p(&format!("{at}.revise_without_plan.seen"), seen(&g, &plans));

    let richese = Battle { hero: None, weapon: None, defense: None, ..Default::default() };
    l.try_(&format!("{at}.outsider.validate"), || message(richese.validate(&g, By::new(&g, Faction::Richese)).err().map(|r| r.0), "valid"));

    l.try_(&format!("{at}.reset"), || {
        EndPhase::default().reset_battle(&mut g);
        "ok".to_string()
    });
    l.p(&format!("{at}.reset.seen"), seen(&g, &plans));
}

#[test]
fn v188_rules() {
    let mut l = Lines::default();
    alliance_phase(&mut l, Game::LATEST_VERSION);
    break_in_alliance_phase(&mut l, Game::LATEST_VERSION);
    prescience(&mut l, Game::LATEST_VERSION);
    common::snapshot_lines("sync_v188.txt", &l.0);
}
