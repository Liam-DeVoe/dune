//! Voice must be used before Prescience (rulebook FAQ, p. 22). Prescience waits while the Bene
//! Gesserit, or their ally when Voice is shared, may still use Voice; `VoiceDeclined` ends the
//! wait. A secret ally with the Bene Gesserit Nexus card isn't waited for, so nobody learns they
//! hold it, but may not Voice once Prescience is used. See behavior-changes.md, "Voice before
//! Prescience".

use treachery_engine::prelude::*;

fn game_with(factions: &[Faction]) -> Game {
    let mut g = Game::new();
    for f in factions {
        g.players.push(Player::new(Some(*f)));
    }
    g
}

fn idx(g: &Game, f: Faction) -> PlayerId {
    g.get_player(Some(f)).unwrap()
}

fn start_battle(g: &mut Game, initiator: Faction, target: Faction) {
    let mut b = BattleInitiated::new(g, initiator);
    b.target = Some(target);
    let t = g.board.map.territories(false).find(|t| !t.is_homeworld() && !t.is_stronghold()).unwrap();
    b.territory = Some(t);
    g.battle.current_battle = Some(Rc::new(b));
}

fn ally(g: &mut Game, a: Faction, b: Faction) {
    let (pa, pb) = (idx(g, a), idx(g, b));
    g.player_mut(pa).ally = Some(b);
    g.player_mut(pb).ally = Some(a);
}

fn decline(g: &mut Game, f: Faction) {
    let e = VoiceDeclined::new(g, f);
    assert_eq!(e.validate(g), None, "{f:?} may decline Voice");
    Rc::new(e).execute_concrete_event(g).unwrap();
}

fn voice(g: &mut Game, f: Faction) {
    let mut e = Voice::new(g, f);
    e.must = true;
    e.type_ = Some(TreacheryCardType::Laser);
    assert_eq!(e.validate(g), None, "{f:?} may use Voice");
    Rc::new(e).execute_concrete_event(g).unwrap();
}

fn prescience(g: &mut Game) {
    let mut e = Prescience::new(g, Faction::Atreides);
    e.aspect = Some(PrescienceAspect::Weapon);
    Rc::new(e).execute_concrete_event(g).unwrap();
}

#[test]
fn prescience_waits_until_voice_is_declined() {
    let mut g = game_with(&[Faction::BeneGesserit, Faction::Atreides]);
    let (bg, atreides) = (idx(&g, Faction::BeneGesserit), idx(&g, Faction::Atreides));
    start_battle(&mut g, Faction::BeneGesserit, Faction::Atreides);

    assert!(Prescience::prescience_right(&g, atreides));
    assert!(!Prescience::may_use_prescience(&g, atreides));
    assert!(Voice::may_use_voice(&g, bg));
    assert!(VoiceDeclined::may_decline(&g, bg));
    assert!(!VoiceDeclined::may_decline(&g, atreides));

    decline(&mut g, Faction::BeneGesserit);
    assert!(Prescience::may_use_prescience(&g, atreides));
    assert!(!Voice::may_use_voice(&g, bg), "a declined Voice is closed");
    assert!(!VoiceDeclined::may_decline(&g, bg));
}

#[test]
fn prescience_may_follow_voice() {
    let mut g = game_with(&[Faction::BeneGesserit, Faction::Atreides]);
    let atreides = idx(&g, Faction::Atreides);
    start_battle(&mut g, Faction::Atreides, Faction::BeneGesserit);

    voice(&mut g, Faction::BeneGesserit);
    assert!(Prescience::may_use_prescience(&g, atreides));
}

#[test]
fn prescience_does_not_wait_without_bene_gesserit_in_the_battle() {
    let mut g = game_with(&[Faction::BeneGesserit, Faction::Atreides, Faction::Emperor]);
    let (bg, atreides) = (idx(&g, Faction::BeneGesserit), idx(&g, Faction::Atreides));
    start_battle(&mut g, Faction::Atreides, Faction::Emperor);

    assert!(Prescience::may_use_prescience(&g, atreides));
    assert!(!VoiceDeclined::may_decline(&g, bg));
}

#[test]
fn nobody_is_asked_to_decline_when_no_one_could_use_prescience() {
    let mut g = game_with(&[Faction::BeneGesserit, Faction::Emperor]);
    let bg = idx(&g, Faction::BeneGesserit);
    start_battle(&mut g, Faction::BeneGesserit, Faction::Emperor);

    assert!(Voice::may_use_voice(&g, bg));
    assert!(!VoiceDeclined::may_decline(&g, bg));
}

#[test]
fn a_decline_by_one_bene_gesserit_side_closes_voice_for_both() {
    // The Bene Gesserit's ally fights the Atreides, and Voice is shared: either may Voice.
    let mut g = game_with(&[Faction::BeneGesserit, Faction::Emperor, Faction::Atreides]);
    ally(&mut g, Faction::BeneGesserit, Faction::Emperor);
    g.alliances.bene_gesserit_allows_use_of_voice = true;
    let (bg, emperor, atreides) = (idx(&g, Faction::BeneGesserit), idx(&g, Faction::Emperor), idx(&g, Faction::Atreides));
    start_battle(&mut g, Faction::Emperor, Faction::Atreides);

    assert!(VoiceDeclined::may_decline(&g, bg));
    assert!(VoiceDeclined::may_decline(&g, emperor));
    assert!(!Prescience::may_use_prescience(&g, atreides));

    decline(&mut g, Faction::Emperor);
    assert!(!Voice::may_use_voice(&g, bg));
    assert!(!Voice::may_use_voice(&g, emperor));
    assert!(Prescience::may_use_prescience(&g, atreides));
}

#[test]
fn prescience_does_not_wait_for_prevented_voice() {
    let mut g = game_with(&[Faction::BeneGesserit, Faction::Atreides, Faction::Emperor]);
    let atreides = idx(&g, Faction::Atreides);
    start_battle(&mut g, Faction::BeneGesserit, Faction::Atreides);

    g.prevent(Faction::Emperor, FactionAdvantage::BeneGesseritUsingVoice, false);
    assert!(Prescience::may_use_prescience(&g, atreides));
}

#[test]
fn a_secret_ally_is_not_waited_for_but_may_not_voice_after_prescience() {
    // No Bene Gesserit in the game; the Emperor holds their Nexus card.
    let mut g = game_with(&[Faction::Emperor, Faction::Atreides]);
    let (emperor, atreides) = (idx(&g, Faction::Emperor), idx(&g, Faction::Atreides));
    g.player_mut(emperor).nexus = Some(Faction::BeneGesserit);
    start_battle(&mut g, Faction::Emperor, Faction::Atreides);

    assert!(Voice::may_use_voice(&g, emperor));
    assert!(!VoiceDeclined::may_decline(&g, emperor), "declining would reveal the Nexus card");
    assert!(Prescience::may_use_prescience(&g, atreides));

    prescience(&mut g);
    assert!(!Voice::may_use_voice(&g, emperor));
}

#[test]
fn a_secret_ally_may_voice_before_prescience() {
    let mut g = game_with(&[Faction::Emperor, Faction::Atreides]);
    let (emperor, atreides) = (idx(&g, Faction::Emperor), idx(&g, Faction::Atreides));
    g.player_mut(emperor).nexus = Some(Faction::BeneGesserit);
    start_battle(&mut g, Faction::Emperor, Faction::Atreides);

    voice(&mut g, Faction::Emperor);
    assert!(Prescience::may_use_prescience(&g, atreides));
}
