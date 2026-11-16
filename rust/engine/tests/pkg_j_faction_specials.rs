//! Focused tests for Karma, Nexus cards, Moritani, Ecaz, Choam and clairvoyance. They build small
//! game states by hand and check rules that are easy to get wrong.

use treachery_engine::error::catch_panic;
use treachery_engine::game::Intrusion;
use treachery_engine::random::SharedRandom;
use treachery_engine::prelude::*;

fn outcome<R>(f: impl FnOnce() -> R) -> Result<R, String> {
    catch_panic(f)
}

fn game_with(factions: &[Faction]) -> Game {
    let mut g = Game::new();
    g.decks.treachery_discard_pile = Some(Deck::new(Vec::new(), None));
    for f in factions {
        let p = Player::new(Some(*f));
        g.players.push(p);
    }
    g
}

fn idx(g: &Game, f: Faction) -> PlayerId {
    g.get_player(Some(f)).unwrap()
}

fn card_of(t: TreacheryCardType, nth: usize) -> TreacheryCard {
    TreacheryCardManager::items().iter().copied().filter(|c| c.type_() == t).nth(nth).unwrap()
}

fn a_stronghold(g: &Game) -> Territory {
    g.board.map.territories(false).find(|t| t.is_stronghold() && !t.is_homeworld()).unwrap()
}

/// Makes `faction` the triggering event of a terror intrusion in `t`.
fn trigger_terror(g: &mut Game, faction: Faction, t: Territory) {
    let mut e = NexusPlayed::new(g, faction);
    e.tleilaxu_location = Some(t.locations()[0]);
    let to = e.tleilaxu_location;
    g.ship_move.intrusions.push(Intrusion { initiator: faction, to, type_: IntrusionType::Terror });
}

// ---------------------------------------------------------------------------
// Karma
// ---------------------------------------------------------------------------

#[test]
fn karma_valid_advantages_depend_on_main_phase() {
    let mut g = game_with(&[Faction::Atreides, Faction::Fremen]);
    let atreides = idx(&g, Faction::Atreides);
    g.turn.current_main_phase = MainPhase::Bidding;
    let now = Karma::valid_faction_advantages(&g, atreides);
    assert!(!now.contains(&Some(FactionAdvantage::FremenExtraMove)));
    assert_eq!(now[0], None);
}

#[test]
fn karma_choam_discard_plays_first_karma_card_not_the_selected_one() {
    let mut g = game_with(&[Faction::Choam, Faction::Atreides]);
    let choam = idx(&g, Faction::Choam);
    let k1 = card_of(TreacheryCardType::Karma, 0);
    let k2 = card_of(TreacheryCardType::Karma, 1);
    let useless = card_of(TreacheryCardType::Useless, 0);
    g.player_mut(choam).treachery_cards.extend([k1, k2, useless]);

    let by = By::new(&g, Faction::Choam);
    let mut e = KarmaChoamDiscard::default();
    e.card_to_use = Some(k2);
    e.set_cards(&[useless]);
    assert!(e.validate(&g, by).is_ok());

    // The selected card is validated.
    let mut bad = KarmaChoamDiscard::default();
    bad.card_to_use = Some(k2);
    bad.set_cards(&[k2]);
    assert!(bad.validate(&g, by).is_err());

    e.apply(&mut g, by).unwrap();
    // C# parity: k1 (the first valid card) is played, k2 stays in hand.
    let hand = &g.player(choam).treachery_cards;
    assert!(!hand.contains(&k1));
    assert!(hand.contains(&k2));
    assert!(!hand.contains(&useless));
    assert_eq!(i32::from(g.player(choam).resources), 3);
    assert!(g.player(choam).special_karma_power_used);
}

// ---------------------------------------------------------------------------
// Nexus
// ---------------------------------------------------------------------------

#[test]
fn moritani_betrayal_without_a_token_returns_nothing() {
    let mut g = game_with(&[Faction::Atreides, Faction::Moritani]);
    g.random = Some(SharedRandom::from_seed(5));
    let atreides = idx(&g, Faction::Atreides);
    g.player_mut(atreides).nexus = Some(Faction::Moritani);
    let t = a_stronghold(&g);

    let mut e = NexusPlayed::new(&g, Faction::Atreides);
    e.faction = Some(Faction::Moritani);
    e.moritani_territory = Some(t);
    let e = Rc::new(e);
    assert!(e.is_betrayal(&g));
    // Validation rejects it, but the execution path is still defined.
    assert!(e.validate(&g).is_some());
    let unplaced = g.board.unplaced_terror_tokens.clone();
    e.execute_concrete_event(&mut g).unwrap();
    // C# added `TerrorType.None` to the unplaced tokens here. Since rewrite-plan.md 2d there is no
    // such token, so nothing is added.
    assert_eq!(g.board.unplaced_terror_tokens, unplaced);
}

#[test]
fn moritani_betrayal_picks_in_the_order_tokens_were_placed() {
    let mut g = game_with(&[Faction::Atreides, Faction::Moritani]);
    let atreides = idx(&g, Faction::Atreides);
    g.player_mut(atreides).nexus = Some(Faction::Moritani);
    let t = a_stronghold(&g);
    let other = g.board.map.territories(false).find(|x| x.is_stronghold() && *x != t && !x.is_homeworld()).unwrap();
    g.board.terror_on_planet.insert_new(TerrorType::Assassination, t);
    g.board.terror_on_planet.insert_new(TerrorType::Atomics, other);
    g.board.terror_on_planet.insert_new(TerrorType::Robbery, t);
    // A token placed after a removal goes last. (.NET's Dictionary put it in the freed slot, first.)
    g.board.terror_on_planet.shift_remove(&TerrorType::Assassination);
    g.board.terror_on_planet.insert_new(TerrorType::Sabotage, t);
    assert_eq!(g.terror_in(t).collect::<Vec<_>>(), vec![TerrorType::Robbery, TerrorType::Sabotage]);

    // Same seed, same draw: the pick is an index into the order above.
    let expected = {
        let r = SharedRandom::from_seed(11);
        [TerrorType::Robbery, TerrorType::Sabotage].into_iter().random_item(&r).unwrap()
    };
    g.random = Some(SharedRandom::from_seed(11));
    let mut e = NexusPlayed::new(&g, Faction::Atreides);
    e.faction = Some(Faction::Moritani);
    e.moritani_territory = Some(t);
    assert!(e.validate(&g).is_none());
    Rc::new(e).execute_concrete_event(&mut g).unwrap();
    assert!(!g.board.terror_on_planet.contains_key(&expected));
    assert_eq!(g.board.terror_on_planet.len(), 2);
    assert_eq!(g.board.unplaced_terror_tokens.last().copied(), Some(expected));
}

#[test]
fn nexus_tleilaxu_secret_ally_cost_is_never_paid() {
    let mut g = game_with(&[Faction::Atreides, Faction::Emperor]);
    let atreides = idx(&g, Faction::Atreides);
    g.player_mut(atreides).nexus = Some(Faction::Tleilaxu);
    g.player_mut(atreides).forces_killed = 3;
    g.player_mut(atreides).resources = 10;
    let mut e = NexusPlayed::new(&g, Faction::Atreides);
    e.faction = Some(Faction::Tleilaxu);
    e.tleilaxu_forces = 2;
    let e = Rc::new(e);
    assert!(e.is_secret_ally(&g));
    assert_eq!(e.determine_tleilaxu_cost(), 2);
    let _ = outcome(|| e.execute_concrete_event(&mut g));
    // C# parity: validated against resources, never deducted.
    assert_eq!(i32::from(g.player(atreides).resources), 10);
    assert_eq!(g.player(atreides).forces_killed, 1);
}

// ---------------------------------------------------------------------------
// Moritani terror
// ---------------------------------------------------------------------------

#[test]
fn terror_robbery_takes_half_rounded_up() {
    let mut g = game_with(&[Faction::Atreides, Faction::Moritani]);
    g.random = Some(SharedRandom::from_seed(1));
    let t = a_stronghold(&g);
    g.board.terror_on_planet.insert_new(TerrorType::Robbery, t);
    trigger_terror(&mut g, Faction::Atreides, t);
    let atreides = idx(&g, Faction::Atreides);
    let moritani = idx(&g, Faction::Moritani);
    g.player_mut(atreides).resources = 7;

    let by = By::new(&g, Faction::Moritani);
    let mut e = TerrorRevealed::default();
    e.type_ = Some(TerrorType::Robbery);
    assert!(e.validate(&g, by).is_ok());
    let _ = outcome(|| e.apply(&mut g, by));
    assert_eq!(g.player(moritani).resources, 4);
    assert_eq!(g.player(atreides).resources, 3);
    assert!(!g.board.terror_on_planet.contains_key(&TerrorType::Robbery));
}

#[test]
fn terror_types_follow_the_trigger_territory() {
    let mut g = game_with(&[Faction::Atreides, Faction::Moritani]);
    assert!(TerrorRevealed::get_types(&g).next().is_none());
    assert_eq!(TerrorRevealed::get_victim(&g), None);
    let t = a_stronghold(&g);
    g.board.terror_on_planet.insert_new(TerrorType::Extortion, t);
    trigger_terror(&mut g, Faction::Atreides, t);
    assert_eq!(TerrorRevealed::get_types(&g).collect::<Vec<_>>(), vec![TerrorType::Extortion]);
    assert_eq!(TerrorRevealed::get_victim(&g), Some(Faction::Atreides));

    // Only Moritani may reveal.
    let mut e = TerrorRevealed::default();
    e.type_ = Some(TerrorType::Extortion);
    assert!(e.validate(&g, By::new(&g, Faction::Atreides)).is_err());
}

#[test]
fn terror_planted_valid_types_is_an_ordered_union() {
    let mut g = game_with(&[Faction::Moritani]);
    let t = a_stronghold(&g);
    g.board.unplaced_terror_tokens = vec![TerrorType::Robbery, TerrorType::Atomics, TerrorType::Robbery];
    g.board.terror_on_planet.insert_new(TerrorType::Sabotage, t);
    g.board.terror_on_planet.insert_new(TerrorType::Atomics, t);
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

    let by = By::new(&g, Faction::Ecaz);
    let mut first = AmbassadorPlaced::default();
    first.ambassador = Some(Ambassador::Emperor);
    first.stronghold = Some(t);
    first.apply(&mut g, by).unwrap();

    let mut second = AmbassadorPlaced::default();
    second.ambassador = Some(Ambassador::BeneGesserit);
    second.stronghold = Some(t);
    assert!(second.validate(&g, by).is_err());
    let e = second.apply(&mut g, by).unwrap_err();
    assert_eq!(e.message(), "There already is an ambassador in Treachery.Shared.Territory");
    // Paid 1, then 2, before the failure; the BeneGesserit ambassador stays in supply.
    assert_eq!(i32::from(g.player(ecaz).resources), 7);
    assert_eq!(g.revival.ambassadors_placed_this_turn, 2);
    assert_eq!(g.player(ecaz).ambassadors, vec![Ambassador::BeneGesserit]);
}

#[test]
fn ambassador_placed_without_stronghold_fails() {
    let mut g = game_with(&[Faction::Ecaz]);
    let mut e = AmbassadorPlaced::default();
    e.stronghold = None;
    let by = By::new(&g, Faction::Ecaz);
    assert_eq!(e.apply(&mut g, by).unwrap_err().message(), "Stronghold is null");
    assert_eq!(g.revival.ambassadors_placed_this_turn, 0);
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
    e.question = Some(ClairvoyanceQuestion::DialOfMoreThanXInBattle);
    // "1,5" is 15 under the invariant culture (thousands separator).
    e.question_parameter1 = Some("1,5".to_string());
    assert_eq!(e.parameter1(&g), treachery_engine::messages::part::Part::Strength(treachery_engine::game::HalfPoints::whole(15)));
    e.question_parameter1 = Some("2.5".to_string());
    assert_eq!(e.parameter1(&g), treachery_engine::messages::part::Part::Strength(treachery_engine::game::HalfPoints::from_halves(5)));
    // A dial is whole or half (behavior-changes.md, "Battle strength in halves").
    e.question_parameter1 = Some("2.7".to_string());
    assert_eq!(outcome(|| e.parameter1(&g)).unwrap_err(), "FormatException: The input string '2.7' is not a whole or half number.");
    assert!(e.try_get_question(&g).is_err());
    e.question_parameter1 = None;
    assert_eq!(e.parameter1(&g), treachery_engine::messages::part::Part::Null);

    e.question = Some(ClairvoyanceQuestion::HasCardTypeInHand);
    e.question_parameter1 = Some("Laser".to_string());
    assert!(e.is_about(&g, TreacheryCardType::Laser));
    assert!(!e.is_about(&g, TreacheryCardType::Karma));
    e.question_parameter1 = Some("NoSuchCard".to_string());
    assert_eq!(
        outcome(|| e.parameter1(&g)).unwrap_err(),
        "ArgumentException: Requested value 'NoSuchCard' was not found."
    );

    e.question = Some(ClairvoyanceQuestion::Prediction);
    e.question_parameter1 = Some("Emperor".to_string());
    e.question_parameter2 = Some(String::new());
    assert_eq!(e.parameter2(), treachery_engine::messages::part::Part::Null);
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
    let qna = g.turn.latest_clairvoyance_qn_a.as_ref().unwrap();
    assert!(qna.question.is_none());
    assert_eq!(g.current_report().messages.len(), reports_before + 1);
}

// ---------------------------------------------------------------------------
// Others
// ---------------------------------------------------------------------------

#[test]
fn bene_gesserit_prediction_valid_turns() {
    let mut g = game_with(&[Faction::BeneGesserit, Faction::Emperor]);
    g.rules.settings.as_mut().unwrap().maximum_turns = Turn::new(3);
    assert_eq!(BeneGesseritPrediction::valid_turns(&g), vec![Turn::new(1), Turn::new(2), Turn::new(3)]);
}

#[test]
fn amal_takes_half_rounded_up() {
    let mut g = game_with(&[Faction::Atreides, Faction::Emperor]);
    let atreides = idx(&g, Faction::Atreides);
    let emperor = idx(&g, Faction::Emperor);
    g.player_mut(atreides).treachery_cards.push(card_of(TreacheryCardType::Amal, 0));
    g.player_mut(atreides).resources = u16::MAX;
    g.player_mut(emperor).resources = 3;
    let e = Rc::new(AmalPlayed::new(&g, Faction::Atreides));
    e.execute_concrete_event(&mut g).unwrap();
    assert_eq!(g.player(atreides).resources, u16::MAX - 32_768);
    assert_eq!(g.player(emperor).resources, 1);
}
