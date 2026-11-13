//! Focused tests for package H (battle plans and resolution). They build small
//! game states by hand and check rules whose C# behaviour is easy to get wrong.

use treachery_engine::cs;
use treachery_engine::prelude::*;

fn game_with(factions: &[Faction]) -> Game {
    let mut g = Game::new();
    for f in factions {
        let p = Player::new(&g, *f);
        g.players.push(p);
    }
    g
}

fn idx(g: &Game, f: Faction) -> PlayerIndex {
    g.get_player(Some(f)).unwrap()
}

fn plan(g: &Game, f: Faction) -> Rc<Battle> {
    Rc::new(Battle::new(g, f))
}

#[test]
fn take_losses_half_of_rounds_up_in_double_after_int_wrap() {
    assert_eq!(TakeLosses::half_of(3, 0), 2);
    assert_eq!(TakeLosses::half_of(2, 2), 2);
    assert_eq!(TakeLosses::half_of(-3, 0), -1);
    // The int addition wraps before the conversion to double.
    assert_eq!(TakeLosses::half_of(i32::MAX, 1), i32::MIN / 2);
}

#[test]
fn take_losses_without_pending_losses_uses_an_empty_loss() {
    let g = game_with(&[Faction::Emperor]);
    let l = TakeLosses::losses_to_take(&g);
    assert_eq!(l.amount, 0);
    assert!(l.location.is_none());
    assert_eq!(TakeLosses::valid_max_force_amount(&g, idx(&g, Faction::Emperor)), 0);
}

#[test]
fn battle_concluded_capture_decision_round_trip() {
    let mut e = BattleConcluded::default();
    // Field initialiser: Capture = true
    assert_eq!(e.decision_to_capture(), CaptureDecision::Capture);
    e.set_decision_to_capture(CaptureDecision::Kill);
    assert_eq!(e.decision_to_capture(), CaptureDecision::Kill);
    // None and DontCapture leave Kill as it was.
    e.set_decision_to_capture(CaptureDecision::None);
    assert!(!e.capture);
    assert!(e.kill);
    assert_eq!(e.decision_to_capture(), CaptureDecision::DontCapture);
}

#[test]
fn battle_concluded_capture_decisions() {
    let g = game_with(&[Faction::Harkonnen]);
    assert_eq!(BattleConcluded::valid_capture_decisions(&g), vec![CaptureDecision::Capture, CaptureDecision::Kill]);
}

#[test]
fn battle_concluded_discarded_cards_null_string() {
    let e = BattleConcluded::default();
    assert!(e.card_ids.is_none());
    assert!(e.discarded_cards().is_empty());
}

#[test]
fn treachery_succeeded_respects_harkonnen_cancellation() {
    let mut g = game_with(&[Faction::Harkonnen, Faction::Atreides]);
    let mut t = TreacheryCalled::new(&g, Faction::Harkonnen);
    t.traitor_called = true;
    assert!(t.succeeded(&g));
    g.harkonnen_traitor_was_cancelled = true;
    assert!(!t.succeeded(&g));
    let mut atreides = TreacheryCalled::new(&g, Faction::Atreides);
    atreides.traitor_called = true;
    assert!(atreides.succeeded(&g));
}

#[test]
fn has_traitor_is_false_for_a_missing_hero() {
    let g = game_with(&[Faction::Harkonnen]);
    let harkonnen = idx(&g, Faction::Harkonnen);
    assert!(!TreacheryCalled::has_traitor(&g, harkonnen, None));
}

#[test]
fn winner_hero_with_null_battle_winner() {
    // BattleWinner starts null; `null != Faction.None` is true, and `null == AggressorPlan?.Initiator`
    // is false when there is a plan, so the defender's plan is used.
    let mut g = game_with(&[Faction::Emperor, Faction::Atreides]);
    assert_eq!(g.battle_winner, None);
    assert_eq!(g.winner_hero(), None);
    g.aggressor_plan = Some(plan(&g, Faction::Emperor));
    let mut d = Battle::new(&g, Faction::Atreides);
    d.set_hero(Some(Hero::Leader(LeaderManager::get_leaders(Faction::Atreides).next().unwrap())));
    g.defender_plan = Some(Rc::new(d));
    assert_eq!(g.winner_hero(), Some(Hero::Leader(LeaderManager::get_leaders(Faction::Atreides).next().unwrap())));
    assert!(g.winner_battle_action().is_none());
    g.battle_winner = Some(Faction::None);
    assert_eq!(g.winner_hero(), None);
}

#[test]
fn battle_revision_moves_the_plan_aside() {
    let mut g = game_with(&[Faction::Emperor, Faction::Atreides]);
    let mut b = BattleInitiated::new(&g, Faction::Emperor);
    b.target = Faction::Atreides;
    g.current_battle = Some(Rc::new(b));
    let p = plan(&g, Faction::Atreides);
    g.defender_plan = Some(p.clone());
    let rev = Rc::new(BattleRevision::new(&g, Faction::Atreides));
    rev.execute_concrete_event(&mut g);
    assert!(g.defender_plan.is_none());
    assert!(Rc::ptr_eq(g.previous_defender_plan.as_ref().unwrap(), &p));
}

#[test]
fn battle_revision_without_battle_throws() {
    let mut g = game_with(&[Faction::Emperor]);
    let rev = Rc::new(BattleRevision::new(&g, Faction::Emperor));
    let e = cs::catch(std::panic::AssertUnwindSafe(|| rev.execute_concrete_event(&mut g))).unwrap_err();
    assert_eq!(e.type_name, "Exception");
    assert_eq!(e.message, "BattleRevision event executed when there is no current battle");
}

#[test]
fn aggressor_by_juice_swaps_roles() {
    let mut g = game_with(&[Faction::Emperor, Faction::Atreides]);
    let mut b = BattleInitiated::new(&g, Faction::Emperor);
    b.target = Faction::Atreides;
    assert_eq!(b.aggressor(&g), Faction::Emperor);
    assert_eq!(b.defender(&g), Faction::Atreides);

    let mut j = JuicePlayed::new(&g, Faction::Atreides);
    j.type_ = JuiceType::Aggressor;
    g.current_juice = Some(Rc::new(j));
    assert_eq!(b.aggressor(&g), Faction::Atreides);
    assert_eq!(b.defender(&g), Faction::Emperor);
    assert!(BattleInitiated::is_aggressor_by_juice(&g, Faction::Atreides));
}

#[test]
fn opponent_and_plan_lookups() {
    let mut g = game_with(&[Faction::Emperor, Faction::Atreides, Faction::Fremen]);
    // OpponentOf reads p.Ally (BattleInitiated.cs:124), so the alliance must be set on both sides.
    g.player_mut(idx(&g, Faction::Atreides)).ally = Faction::Fremen;
    g.player_mut(idx(&g, Faction::Fremen)).ally = Faction::Atreides;
    let mut b = BattleInitiated::new(&g, Faction::Emperor);
    b.target = Faction::Atreides;
    let emperor = idx(&g, Faction::Emperor);
    let fremen = idx(&g, Faction::Fremen);
    // The ally of the target sees the initiator as its opponent.
    assert_eq!(b.opponent_of(&g, fremen), Some(emperor));
    assert!(b.is_involved(&g, fremen));
    assert!(!b.is_aggressor_or_defender(&g, fremen));

    let a = plan(&g, Faction::Emperor);
    g.aggressor_plan = Some(a.clone());
    assert!(Rc::ptr_eq(&b.plan_of_faction(&g, Faction::Emperor).unwrap(), &a));
    assert!(b.plan_of_faction(&g, Faction::Fremen).is_none());
}

#[test]
fn treachery_of_matches_ally_initiator() {
    let mut g = game_with(&[Faction::Harkonnen, Faction::Emperor, Faction::Atreides]);
    g.player_mut(idx(&g, Faction::Emperor)).ally = Faction::Harkonnen;
    let mut b = BattleInitiated::new(&g, Faction::Emperor);
    b.target = Faction::Atreides;
    let t = Rc::new(TreacheryCalled::new(&g, Faction::Harkonnen));
    g.aggressor_traitor_action = Some(t.clone());
    let emperor = idx(&g, Faction::Emperor);
    assert!(Rc::ptr_eq(&b.treachery_of(&g, Some(emperor)).unwrap(), &t));
    let atreides = idx(&g, Faction::Atreides);
    assert!(Rc::ptr_eq(&b.treachery_of_opponent(&g, atreides).unwrap(), &t));
}

#[test]
fn lose_cards_skips_cards_the_player_no_longer_has() {
    let mut g = game_with(&[Faction::Emperor]);
    let card = TreacheryCardManager::items()[0];
    let p = Battle::new(&g, Faction::Emperor);
    p.set_weapon(Some(card));
    // _defenseCardId defaults to 0, which Lookup.Find resolves to card 0 (Battle.cs:63).
    p.set_defense(None);
    g.lose_cards(&p, true);
    assert!(g.cards_to_be_discarded_by_loser_after_battle.is_empty());
}

#[test]
fn loser_concluded_applicability() {
    let mut g = game_with(&[Faction::Emperor, Faction::Atreides]);
    let emperor = idx(&g, Faction::Emperor);
    assert!(!LoserConcluded::is_applicable(&g, None));
    g.battle_loser = Some(Faction::Emperor);
    assert!(!LoserConcluded::is_applicable(&g, Some(emperor)));
    g.loser_may_try_to_assassinate = true;
    assert!(LoserConcluded::is_applicable(&g, Some(emperor)));
}

#[test]
fn may_substitute_force_losses_only_for_ixian() {
    let g = game_with(&[Faction::Ixian, Faction::Emperor]);
    assert!(g.may_substitute_force_losses(idx(&g, Faction::Ixian)));
    assert!(!g.may_substitute_force_losses(idx(&g, Faction::Emperor)));
}
