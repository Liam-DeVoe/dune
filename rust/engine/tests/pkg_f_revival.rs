//! Focused tests for package F (revival and leaders). They build small game
//! states by hand and check rules whose C# behaviour is easy to get wrong.

use std::rc::Rc;

use treachery_engine::compat::cs;
use treachery_engine::prelude::*;

fn game_with(factions: &[Faction]) -> Game {
    let mut g = Game::new();
    for f in factions {
        let p = Player::new(*f);
        g.players.push(p);
        for l in LeaderManager::get_leaders(*f) {
            let last = g.player_at(g.players.len() - 1);
            g.player_mut(last).leaders.push(l);
            // Game::new already has a LeaderState for every leader (Game.cs:122).
        }
    }
    g
}

fn idx(g: &Game, f: Faction) -> PlayerIndex {
    g.get_player(Some(f)).unwrap()
}

fn as_player<E: GameEventMethods>(e: E, g: &Game, f: Faction) -> E {
    e.base().initiator.set(f);
    e.base().player.set(g.get_player(Some(f)));
    e
}

#[test]
fn raise_dead_max_amounts() {
    let mut g = game_with(&[Faction::Emperor, Faction::Atreides]);
    let emperor = idx(&g, Faction::Emperor);
    g.player_mut(emperor).special_forces_killed = 4;
    g.player_mut(emperor).forces_killed = 9;
    assert_eq!(RaiseDeadPlayed::valid_max_amount(&g, emperor, true), 1);
    assert_eq!(RaiseDeadPlayed::valid_max_amount(&g, emperor, false), 5);
    g.factions_that_revived_special_forces_this_turn.push(Faction::Emperor);
    assert_eq!(RaiseDeadPlayed::valid_max_amount(&g, emperor, true), 0);

    let atreides = idx(&g, Faction::Atreides);
    g.player_mut(atreides).special_forces_killed = 7;
    assert_eq!(RaiseDeadPlayed::valid_max_amount(&g, atreides, true), 5);
}

#[test]
fn karma_free_revival_max_amounts() {
    let mut g = game_with(&[Faction::Emperor]);
    let emperor = idx(&g, Faction::Emperor);
    g.player_mut(emperor).special_forces_killed = 4;
    g.player_mut(emperor).forces_killed = 2;
    assert_eq!(KarmaFreeRevival::valid_max_amount(&g, emperor, true), 1);
    assert_eq!(KarmaFreeRevival::valid_max_amount(&g, emperor, false), 2);
}

#[test]
fn raise_dead_negative_amount_checked_before_player_dereference() {
    // C# checks the sign before the first `p.` access, so no NullReferenceException.
    let g = game_with(&[Faction::Emperor]);
    let mut e = RaiseDeadPlayed::default();
    e.amount_of_forces = -1;
    assert!(e.validate(&g).is_some());

    e.amount_of_forces = 0;
    let r = cs::catch_panic(|| e.validate(&g));
    assert_eq!(r.unwrap_err().type_name, "NullReferenceException");
}

#[test]
fn karma_free_revival_accepts_negative_amounts() {
    // C# parity: no sign check.
    let mut g = game_with(&[Faction::Emperor]);
    let emperor = idx(&g, Faction::Emperor);
    g.player_mut(emperor).forces_killed = 0;
    let mut e = KarmaFreeRevival::default();
    e.amount_of_forces = -3;
    let e = as_player(e, &g, Faction::Emperor);
    assert!(e.validate(&g).is_none());
}

#[test]
fn tleilaxu_revival_targets_union_killed_and_gholas() {
    let mut g = game_with(&[Faction::Emperor, Faction::Tleilaxu]);
    let emperor = idx(&g, Faction::Emperor);
    let tleilaxu = idx(&g, Faction::Tleilaxu);
    let emperor_leaders: Vec<Leader> = LeaderManager::get_leaders(Faction::Emperor).collect();

    // A emperor leader that tleilaxu holds as a ghola.
    let ghola = emperor_leaders[0];
    g.player_mut(emperor).leaders.retain(|l| *l != ghola);
    g.player_mut(tleilaxu).leaders.push(ghola);

    // A dead emperor leader.
    g.leader_state.index_mut(&Hero::Leader(emperor_leaders[1])).kill(&mut None);

    let targets = RequestTleilaxuRevival::valid_targets(&g, emperor).collect::<Vec<_>>();
    assert_eq!(targets, vec![Hero::Leader(emperor_leaders[1]), Hero::Leader(ghola)]);
}

#[test]
fn deny_all_requests_adds_max_offers() {
    let mut g = game_with(&[Faction::Emperor, Faction::Tleilaxu]);
    let emperor_leaders: Vec<Leader> = LeaderManager::get_leaders(Faction::Emperor).collect();
    let mut req = RequestTleilaxuRevival::default();
    req.set_hero(Some(Hero::Leader(emperor_leaders[0])));
    g.current_revival_requests.push(Rc::new(req));

    // _heroId defaults to 0, and HeroLookup.Find(0) is treachery card 0, not null
    // (LeaderManager.cs:137). Deny-all is Hero == null, i.e. _heroId -1.
    let mut deny = AcceptOrCancelTleilaxuRevival::default();
    deny.set_hero(None);
    let deny = Rc::new(as_player(deny, &g, Faction::Tleilaxu));
    assert!(!deny.is_denial());
    deny.execute_concrete_event(&mut g).unwrap();
    assert_eq!(*g.early_revivals_offers.index(&Hero::Leader(emperor_leaders[0])), i32::MAX);
    assert!(g.current_revival_requests.is_empty());
}

#[test]
fn offer_for_hero_with_existing_denial_replaces_it() {
    let mut g = game_with(&[Faction::Emperor, Faction::Tleilaxu]);
    let emperor_leaders: Vec<Leader> = LeaderManager::get_leaders(Faction::Emperor).collect();
    let hero = Hero::Leader(emperor_leaders[0]);
    g.early_revivals_offers.add(hero, i32::MAX);

    let mut offer = AcceptOrCancelTleilaxuRevival::default();
    offer.set_hero(Some(hero));
    offer.price = 3;
    let offer = Rc::new(as_player(offer, &g, Faction::Tleilaxu));
    offer.execute_concrete_event(&mut g).unwrap();
    assert_eq!(*g.early_revivals_offers.index(&hero), 3);
}
