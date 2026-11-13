//! Focused tests for package J (Karma, Nexus, Moritani, Ecaz, Choam, clairvoyance).
//! They build small game states by hand and check rules whose C# behaviour is
//! easy to get wrong in a port.

use treachery_engine::canon;
use treachery_engine::error::catch_panic;
use treachery_engine::events::GameEventMethods;
use treachery_engine::game::Intrusion;
use treachery_engine::random::SharedRandom;
use treachery_engine::prelude::*;

fn outcome<R>(f: impl FnOnce() -> R) -> Result<R, String> {
    catch_panic(f)
}

fn game_with(factions: &[Faction]) -> Game {
    let mut g = Game::new();
    g.treachery_discard_pile = Some(Deck::new(Vec::new(), None));
    for f in factions {
        let p = Player::new(*f);
        g.players.push(p);
    }
    g
}

fn idx(g: &Game, f: Faction) -> PlayerIndex {
    g.get_player(Some(f)).unwrap()
}

fn card_of(t: TreacheryCardType, nth: usize) -> TreacheryCard {
    TreacheryCardManager::items().iter().copied().filter(|c| c.type_() == t).nth(nth).unwrap()
}

fn a_stronghold(g: &Game) -> Territory {
    g.map.territories(false).find(|t| t.is_stronghold() && !t.is_homeworld()).unwrap()
}

/// Makes `faction` the triggering event of a terror intrusion in `t`.
fn trigger_terror(g: &mut Game, faction: Faction, t: Territory) {
    let mut e = NexusPlayed::new(g, faction);
    e.tleilaxu_location_id = t.locations()[0].id();
    let e = Rc::new(e).into_event();
    g.intrusions.push(Intrusion::new(&e, IntrusionType::Terror));
}

// ---------------------------------------------------------------------------
// Messages that keep a live hand (AmbassadorActivated.cs:333, hazards 4.5)
// ---------------------------------------------------------------------------

#[test]
fn live_hand_in_a_message_dumps_the_current_hand() {
    let mut g = game_with(&[Faction::Ecaz, Faction::Atreides]);
    let atreides = idx(&g, Faction::Atreides);
    let c1 = card_of(TreacheryCardType::Karma, 0);
    let c2 = card_of(TreacheryCardType::Useless, 0);
    g.player_mut(atreides).treachery_cards.push(c1);

    let live = ExpressionElement::live_player_treachery_cards(&g, atreides);
    let snapshot = ExpressionElement::list(g.player(atreides).treachery_cards.iter().copied());
    let before = canon::dump_detached(&live, Some(&g));
    assert_eq!(before, canon::dump_detached(&snapshot, Some(&g)));

    // The hand changes after the message was logged: C# shows the new contents.
    g.player_mut(atreides).treachery_cards.push(c2);
    let after = String::from_utf8(canon::dump_detached(&live, Some(&g))).unwrap();
    assert!(after.contains(&format!("\"id\":{}", c2.id())), "{after}");
    assert_eq!(canon::dump_detached(&snapshot, Some(&g)), before);
    // Without a game there is nothing to resolve against: the snapshot is written.
    assert_eq!(canon::dump_detached(&live, None), before);
}

// ---------------------------------------------------------------------------
// Karma
// ---------------------------------------------------------------------------

#[test]
fn karma_valid_advantages_depend_on_main_phase() {
    let mut g = game_with(&[Faction::Atreides, Faction::Fremen]);
    let atreides = idx(&g, Faction::Atreides);
    g.current_main_phase = MainPhase::Bidding;
    let now = Karma::valid_faction_advantages(&g, atreides);
    assert!(!now.contains(&FactionAdvantage::FremenExtraMove));
    assert_eq!(now[0], FactionAdvantage::None);
}

#[test]
fn karma_choam_discard_plays_first_karma_card_not_the_selected_one() {
    let mut g = game_with(&[Faction::Choam, Faction::Atreides]);
    let choam = idx(&g, Faction::Choam);
    let k1 = card_of(TreacheryCardType::Karma, 0);
    let k2 = card_of(TreacheryCardType::Karma, 1);
    let useless = card_of(TreacheryCardType::Useless, 0);
    g.player_mut(choam).treachery_cards.extend([k1, k2, useless]);

    let mut e = KarmaChoamDiscard::new(&g, Faction::Choam);
    e.set_card_to_use(Some(k2));
    e.set_cards(&[useless]);
    assert!(e.validate(&g).is_none());

    // The selected card is validated.
    let mut bad = KarmaChoamDiscard::new(&g, Faction::Choam);
    bad.set_card_to_use(Some(k2));
    bad.set_cards(&[k2]);
    assert!(bad.validate(&g).is_some());

    let e = Rc::new(e);
    e.execute_concrete_event(&mut g).unwrap();
    // C# parity: k1 (the first valid card) is played, k2 stays in hand.
    let hand = &g.player(choam).treachery_cards;
    assert!(!hand.contains(&k1));
    assert!(hand.contains(&k2));
    assert!(!hand.contains(&useless));
    assert_eq!(g.player(choam).resources, 3);
    assert!(g.player(choam).special_karma_power_used);
}

// ---------------------------------------------------------------------------
// Nexus
// ---------------------------------------------------------------------------

#[test]
fn moritani_betrayal_without_a_token_removes_and_returns_default_terror() {
    let mut g = game_with(&[Faction::Atreides, Faction::Moritani]);
    g.random = Some(SharedRandom::from_seed(5));
    let atreides = idx(&g, Faction::Atreides);
    g.player_mut(atreides).nexus = Faction::Moritani;
    let t = a_stronghold(&g);

    let mut e = NexusPlayed::new(&g, Faction::Atreides);
    e.faction = Faction::Moritani;
    e.moritani_territory_id = t.id();
    let e = Rc::new(e);
    assert!(e.is_betrayal(&g));
    // Validation rejects it, but the execution path is still defined.
    assert!(e.validate(&g).is_some());
    e.execute_concrete_event(&mut g).unwrap();
    // C# parity: RandomOrDefault of an empty sequence is TerrorType.None, which is added (v169+).
    assert_eq!(g.unplaced_terror_tokens.last().copied(), Some(TerrorType::None));
}

#[test]
fn moritani_betrayal_picks_from_dictionary_slot_order() {
    let mut g = game_with(&[Faction::Atreides, Faction::Moritani]);
    let atreides = idx(&g, Faction::Atreides);
    g.player_mut(atreides).nexus = Faction::Moritani;
    let t = a_stronghold(&g);
    let other = g.map.territories(false).find(|x| x.is_stronghold() && *x != t && !x.is_homeworld()).unwrap();
    let k = |k: &TerrorType| ExpressionElement::from(*k).to_cs_string();
    g.terror_on_planet.add_with(TerrorType::Assassination, t, k);
    g.terror_on_planet.add_with(TerrorType::Atomics, other, k);
    g.terror_on_planet.add_with(TerrorType::Robbery, t, k);
    // Free slot 0; the next Add reuses it, so Sabotage iterates first.
    g.terror_on_planet.remove(&TerrorType::Assassination);
    g.terror_on_planet.add_with(TerrorType::Sabotage, t, k);
    assert_eq!(g.terror_in(t).collect::<Vec<_>>(), vec![TerrorType::Sabotage, TerrorType::Robbery]);

    // Same seed, same draw: the pick is an index into the slot order above.
    let expected = {
        let r = SharedRandom::from_seed(11);
        let i = r.next(2);
        [TerrorType::Sabotage, TerrorType::Robbery][i as usize]
    };
    g.random = Some(SharedRandom::from_seed(11));
    let mut e = NexusPlayed::new(&g, Faction::Atreides);
    e.faction = Faction::Moritani;
    e.moritani_territory_id = t.id();
    assert!(e.validate(&g).is_none());
    Rc::new(e).execute_concrete_event(&mut g).unwrap();
    assert!(!g.terror_on_planet.contains_key(&expected));
    assert_eq!(g.terror_on_planet.len(), 2);
    assert_eq!(g.unplaced_terror_tokens.last().copied(), Some(expected));
}

#[test]
fn nexus_tleilaxu_secret_ally_cost_is_never_paid() {
    let mut g = game_with(&[Faction::Atreides, Faction::Emperor]);
    let atreides = idx(&g, Faction::Atreides);
    g.player_mut(atreides).nexus = Faction::Tleilaxu;
    g.player_mut(atreides).forces_killed = 3;
    g.player_mut(atreides).resources = 10;
    let mut e = NexusPlayed::new(&g, Faction::Atreides);
    e.faction = Faction::Tleilaxu;
    e.tleilaxu_forces = 2;
    let e = Rc::new(e);
    assert!(e.is_secret_ally(&g));
    assert_eq!(e.determine_tleilaxu_cost(), 2);
    let _ = outcome(|| e.execute_concrete_event(&mut g));
    // C# parity: validated against resources, never deducted.
    assert_eq!(g.player(atreides).resources, 10);
    assert_eq!(g.player(atreides).forces_killed, 1);
}

// ---------------------------------------------------------------------------
// Moritani terror
// ---------------------------------------------------------------------------

#[test]
fn terror_robbery_rounds_in_float() {
    let mut g = game_with(&[Faction::Atreides, Faction::Moritani]);
    g.random = Some(SharedRandom::from_seed(1));
    let t = a_stronghold(&g);
    g.terror_on_planet.add_with(TerrorType::Robbery, t, |k| ExpressionElement::from(*k).to_cs_string());
    trigger_terror(&mut g, Faction::Atreides, t);
    let atreides = idx(&g, Faction::Atreides);
    let moritani = idx(&g, Faction::Moritani);
    // 16777217 is not a float: (int)Math.Ceiling(0.5f * 16777217) is 8388608, not 8388609.
    g.player_mut(atreides).resources = 16_777_217;

    let mut e = TerrorRevealed::new(&g, Faction::Moritani);
    e.type_ = TerrorType::Robbery;
    let e = Rc::new(e);
    assert!(e.validate(&g).is_none());
    let _ = outcome(|| e.execute_concrete_event(&mut g));
    assert_eq!(g.player(moritani).resources, 8_388_608);
    assert_eq!(g.player(atreides).resources, 16_777_217 - 8_388_608);
    assert!(!g.terror_on_planet.contains_key(&TerrorType::Robbery));
}

#[test]
fn terror_types_follow_the_trigger_territory() {
    let mut g = game_with(&[Faction::Atreides, Faction::Moritani]);
    assert!(TerrorRevealed::get_types(&g).next().is_none());
    assert_eq!(TerrorRevealed::get_victim(&g), Faction::None);
    let t = a_stronghold(&g);
    g.terror_on_planet.add_with(TerrorType::Extortion, t, |k| ExpressionElement::from(*k).to_cs_string());
    trigger_terror(&mut g, Faction::Atreides, t);
    assert_eq!(TerrorRevealed::get_types(&g).collect::<Vec<_>>(), vec![TerrorType::Extortion]);
    assert_eq!(TerrorRevealed::get_victim(&g), Faction::Atreides);

    // Only Moritani may reveal.
    let mut e = TerrorRevealed::new(&g, Faction::Atreides);
    e.type_ = TerrorType::Extortion;
    assert!(e.validate(&g).is_some());
}

#[test]
fn terror_planted_valid_types_is_an_ordered_union() {
    let mut g = game_with(&[Faction::Moritani]);
    let t = a_stronghold(&g);
    g.unplaced_terror_tokens = vec![TerrorType::Robbery, TerrorType::Atomics, TerrorType::Robbery];
    g.terror_on_planet.add_with(TerrorType::Sabotage, t, |k| ExpressionElement::from(*k).to_cs_string());
    g.terror_on_planet.add_with(TerrorType::Atomics, t, |k| ExpressionElement::from(*k).to_cs_string());
    assert_eq!(
        TerrorPlanted::valid_terror_types(&g, false).collect::<Vec<_>>(),
        vec![TerrorType::Robbery, TerrorType::Atomics, TerrorType::Sabotage]
    );
    assert_eq!(TerrorPlanted::valid_terror_types(&g, true).collect::<Vec<_>>(), vec![TerrorType::Sabotage, TerrorType::Atomics]);
}

// ---------------------------------------------------------------------------
// Ecaz ambassadors
// ---------------------------------------------------------------------------

#[test]
fn ambassador_placed_twice_in_a_stronghold_fails_after_paying() {
    let mut g = game_with(&[Faction::Ecaz]);
    let ecaz = idx(&g, Faction::Ecaz);
    g.player_mut(ecaz).resources = 10;
    g.player_mut(ecaz).ambassadors = vec![Ambassador::Emperor, Ambassador::BeneGesserit];
    let t = a_stronghold(&g);

    let mut first = AmbassadorPlaced::new(&g, Faction::Ecaz);
    first.ambassador = Ambassador::Emperor;
    first.set_stronghold(&g, Some(t));
    Rc::new(first).execute_concrete_event(&mut g).unwrap();

    let mut second = AmbassadorPlaced::new(&g, Faction::Ecaz);
    second.ambassador = Ambassador::BeneGesserit;
    second.set_stronghold(&g, Some(t));
    let second = Rc::new(second);
    assert!(second.validate(&g).is_some());
    let e = second.execute_concrete_event(&mut g).unwrap_err();
    assert_eq!(e.message(), "There already is an ambassador in Treachery.Shared.Territory");
    // Paid 1, then 2, before the failure; the BeneGesserit ambassador stays in supply.
    assert_eq!(g.player(ecaz).resources, 7);
    assert_eq!(g.ambassadors_placed_this_turn, 2);
    assert_eq!(g.player(ecaz).ambassadors, vec![Ambassador::BeneGesserit]);
}

#[test]
fn ambassador_placed_without_stronghold_fails() {
    let mut g = game_with(&[Faction::Ecaz]);
    let mut e = AmbassadorPlaced::new(&g, Faction::Ecaz);
    e.set_stronghold(&g, None);
    let e = Rc::new(e);
    assert_eq!(e.execute_concrete_event(&mut g).unwrap_err().message(), "Stronghold is null");
    assert_eq!(g.ambassadors_placed_this_turn, 0);
}

// ---------------------------------------------------------------------------
// Choam
// ---------------------------------------------------------------------------

#[test]
fn choam_discarded_valid_cards() {
    let mut g = game_with(&[Faction::Choam]);
    let choam = idx(&g, Faction::Choam);
    let useless = card_of(TreacheryCardType::Useless, 0);
    let karma1 = card_of(TreacheryCardType::Karma, 0);
    let karma2 = card_of(TreacheryCardType::Karma, 1);
    let proj1 = card_of(TreacheryCardType::Projectile, 0);
    let proj2 = card_of(TreacheryCardType::Projectile, 1);
    let laser = card_of(TreacheryCardType::Laser, 0);
    g.player_mut(choam).treachery_cards = vec![useless, karma1, proj1, karma2, proj2, laser];
    // Useless (no high threshold), duplicates except projectiles and poisons.
    assert_eq!(ChoamDiscarded::valid_cards(&g, choam).collect::<Vec<_>>(), vec![useless, karma1, karma2]);
}

#[test]
fn choam_karma_prevention_uses_kull_wahad() {
    let mut g = game_with(&[Faction::Choam, Faction::Atreides]);
    let choam = idx(&g, Faction::Choam);
    assert!(!ChoamKarmaPrevention::can_be_played_by(&g, choam));
    let kull = TreacheryCardManager::get(TreacheryCardManager::CARD_KULL_WAHAD).unwrap();
    g.player_mut(choam).treachery_cards.push(kull);
    assert!(ChoamKarmaPrevention::can_be_played_by(&g, choam));
    assert_eq!(ChoamKarmaPrevention::card_to_use(&g, choam), Some(kull));
}

// ---------------------------------------------------------------------------
// Clairvoyance
// ---------------------------------------------------------------------------

#[test]
fn clairvoyance_parameters_parse_like_dotnet() {
    let g = game_with(&[Faction::Atreides, Faction::Emperor]);
    let mut e = ClairVoyancePlayed::new(&g, Faction::Atreides);
    e.question = ClairvoyanceQuestion::DialOfMoreThanXInBattle;
    // "1,5" is 15 under the invariant culture (thousands separator).
    e.question_parameter1 = Some("1,5".to_string());
    assert_eq!(e.parameter1(&g), ExpressionElement::Float(15.0));
    e.question_parameter1 = None;
    assert_eq!(e.parameter1(&g), ExpressionElement::Null);

    e.question = ClairvoyanceQuestion::HasCardTypeInHand;
    e.question_parameter1 = Some("Laser".to_string());
    assert!(e.is_about(&g, TreacheryCardType::Laser));
    assert!(!e.is_about(&g, TreacheryCardType::Karma));
    e.question_parameter1 = Some("NoSuchCard".to_string());
    assert_eq!(
        outcome(|| e.parameter1(&g)).unwrap_err(),
        "ArgumentException: Requested value 'NoSuchCard' was not found."
    );

    e.question = ClairvoyanceQuestion::Prediction;
    e.question_parameter1 = Some("Emperor".to_string());
    e.question_parameter2 = Some(String::new());
    assert_eq!(e.parameter2(), ExpressionElement::Null);
    // A faction is not a card type.
    assert!(!e.is_about(&g, TreacheryCardType::Laser));
    e.question_parameter2 = Some("x".to_string());
    assert!(outcome(|| e.parameter2()).unwrap_err().starts_with("FormatException"));
}

#[test]
fn clairvoyance_answer_without_question_stores_qna_then_fails() {
    let mut g = game_with(&[Faction::Atreides, Faction::Emperor]);
    let e = Rc::new(ClairVoyanceAnswered::new(&g, Faction::Emperor));
    let reports_before = g.current_report().messages.len();
    // No clairvoyance was played, so there is no question to answer.
    assert!(e.execute_concrete_event(&mut g).is_err());
    // C#: `new ClairVoyanceQandA(Game.LatestClairvoyance!, this)` and the log run first.
    let qna = g.latest_clairvoyance_qn_a.as_ref().unwrap();
    assert!(qna.question.is_none());
    assert_eq!(g.current_report().messages.len(), reports_before + 1);
}

// ---------------------------------------------------------------------------
// Others
// ---------------------------------------------------------------------------

#[test]
fn bene_gesserit_prediction_valid_turns() {
    let g = game_with(&[Faction::BeneGesserit, Faction::Emperor]);
    g.settings.as_ref().unwrap().borrow_mut().maximum_turns = 3;
    assert_eq!(BeneGesseritPrediction::valid_turns(&g), vec![1, 2, 3]);
    // A negative turn limit panics (C# threw ArgumentOutOfRangeException from Enumerable.Range).
    g.settings.as_ref().unwrap().borrow_mut().maximum_turns = -1;
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| BeneGesseritPrediction::valid_turns(&g))).is_err());
}

#[test]
fn amal_rounds_up_in_double() {
    let mut g = game_with(&[Faction::Atreides, Faction::Emperor]);
    let atreides = idx(&g, Faction::Atreides);
    let emperor = idx(&g, Faction::Emperor);
    g.player_mut(atreides).treachery_cards.push(card_of(TreacheryCardType::Amal, 0));
    g.player_mut(atreides).resources = 16_777_217;
    g.player_mut(emperor).resources = -3;
    let e = Rc::new(AmalPlayed::new(&g, Faction::Atreides));
    e.execute_concrete_event(&mut g).unwrap();
    // ceil(8388608.5) = 8388609 in double; ceil(-1.5) = -1.
    assert_eq!(g.player(atreides).resources, 16_777_217 - 8_388_609);
    assert_eq!(g.player(emperor).resources, -2);
}
