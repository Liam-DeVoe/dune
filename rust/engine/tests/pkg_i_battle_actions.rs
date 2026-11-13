//! Focused tests for package I (actions around the fight: Voice, Prescience,
//! Thought, Diplomacy, audits, face dancers, Juice, Residual, ...). They build
//! small game states by hand and check rules whose C# behaviour is easy to get wrong.

use treachery_engine::error::catch_panic;
use treachery_engine::events::GameEventMethods;
use treachery_engine::prelude::*;

fn outcome<R>(f: impl FnOnce() -> R) -> Result<R, String> {
    catch_panic(f)
}

fn game_with(factions: &[Faction]) -> Game {
    let mut g = Game::new();
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

fn battle(g: &Game, initiator: Faction, target: Faction) -> Rc<BattleInitiated> {
    let mut b = BattleInitiated::new(g, initiator);
    b.target = target;
    // A plain sand territory (not a homeworld).
    let t = g.map.territories(false).find(|t| !t.is_homeworld() && !t.is_stronghold()).unwrap();
    b.territory_id = t.id();
    Rc::new(b)
}

fn plan(g: &Game, f: Faction, weapon: Option<TreacheryCard>, defense: Option<TreacheryCard>) -> Rc<Battle> {
    let b = Battle::new(g, f);
    b.weapon_card_id.set(TreacheryCardManager::get_id(weapon));
    b.defense_card_id.set(TreacheryCardManager::get_id(defense));
    Rc::new(b)
}

#[test]
fn voice_is_voiced_by_without_special_card_rule() {
    let g = game_with(&[Faction::BeneGesserit, Faction::Emperor]);
    use TreacheryCardType as T;
    // Identity always matches.
    assert!(Voice::is_voiced_by(&g, true, true, T::Laser, T::Laser));
    // Chemistry is a poison only as a weapon and only when voice is "may not".
    assert!(Voice::is_voiced_by(&g, true, false, T::Chemistry, T::Poison));
    assert!(!Voice::is_voiced_by(&g, true, true, T::Chemistry, T::Poison));
    assert!(!Voice::is_voiced_by(&g, false, false, T::Chemistry, T::Poison));
    // Chemistry as defense is a poison defense, whatever "must" is.
    assert!(Voice::is_voiced_by(&g, false, true, T::Chemistry, T::PoisonDefense));
    // Portable antidote counts as poison defense.
    assert!(Voice::is_voiced_by(&g, false, true, T::PortableAntidote, T::PoisonDefense));
    // Weirding way: projectile as weapon, projectile defense only for "may not" and as defense.
    assert!(Voice::is_voiced_by(&g, true, true, T::WeirdingWay, T::Projectile));
    assert!(!Voice::is_voiced_by(&g, false, true, T::WeirdingWay, T::Projectile));
    assert!(Voice::is_voiced_by(&g, false, false, T::WeirdingWay, T::ProjectileDefense));
    assert!(!Voice::is_voiced_by(&g, false, true, T::WeirdingWay, T::ProjectileDefense));
    // Unlisted voiced types only match themselves.
    assert!(!Voice::is_voiced_by(&g, true, true, T::Projectile, T::Laser));
}

#[test]
fn voice_is_voiced_by_with_special_card_rule() {
    let mut g = game_with(&[Faction::BeneGesserit, Faction::Emperor]);
    g.rules.push(Rule::BeneGesseritVoiceMustNameSpecialCards);
    use TreacheryCardType as T;
    assert!(Voice::is_voiced_by(&g, false, true, T::Antidote, T::PoisonDefense));
    assert!(!Voice::is_voiced_by(&g, false, true, T::ShieldAndAntidote, T::PoisonDefense));
    assert!(Voice::is_voiced_by(&g, false, true, T::Shield, T::ProjectileDefense));
    assert!(!Voice::is_voiced_by(&g, true, true, T::ProjectileAndPoison, T::Poison));
}

#[test]
fn voice_valid_types_by_rules() {
    let mut g = game_with(&[Faction::BeneGesserit, Faction::Emperor]);
    use TreacheryCardType as T;
    assert_eq!(
        Voice::valid_types(&g),
        vec![T::Mercenary, T::Laser, T::Poison, T::Projectile, T::Shield, T::Antidote, T::Useless, T::PoisonDefense, T::ProjectileDefense]
    );
    g.rules.push(Rule::BeneGesseritVoiceMustNameSpecialCards);
    assert_eq!(Voice::valid_types(&g).len(), 7);
}

#[test]
fn voice_validate_with_null_player() {
    let mut g = game_with(&[Faction::Emperor, Faction::Atreides]);
    // BeneGesserit is not playing, so Player stays null.
    let v = Voice::new(&g, Faction::BeneGesserit);
    assert!(v.player_opt().is_none());
    // No battle: MayUseVoice returns false before touching the player.
    let r = outcome(|| v.validate(&g).map(|m| m.to_cs_string()));
    assert_eq!(r, Ok(Some("You cannot use Voice".to_string())));
    // With a battle, IsInvolved(p) dereferences the null player.
    g.current_battle = Some(battle(&g, Faction::Emperor, Faction::Atreides));
    let r = outcome(|| v.validate(&g).map(|m| m.to_cs_string()));
    assert!(r.is_err());
}

#[test]
fn may_call_face_dancer_distinguishes_null_and_none_winner() {
    let mut g = game_with(&[Faction::Tleilaxu, Faction::Emperor, Faction::Atreides]);
    let tleilaxu = idx(&g, Faction::Tleilaxu);
    // No battle.
    assert!(!FaceDanced::may_call_face_dancer(&g, tleilaxu));
    g.current_battle = Some(battle(&g, Faction::Emperor, Faction::Atreides));
    // BattleWinner == Faction.None
    g.battle_winner = Some(Faction::None);
    assert!(!FaceDanced::may_call_face_dancer(&g, tleilaxu));

    // Emperor wins with a leader that is one of Tleilaxu's face dancers.
    let emperor_leader = LeaderManager::get_leaders(Faction::Emperor).next().unwrap();
    let mut p = Battle::new(&g, Faction::Emperor);
    p.hero_id = emperor_leader.id();
    g.aggressor_plan = Some(Rc::new(p));
    g.battle_winner = Some(Faction::Emperor);
    g.player_mut(tleilaxu).face_dancers.push(Hero::Leader(emperor_leader));
    assert!(FaceDanced::may_call_face_dancer(&g, tleilaxu));
    // A revealed face dancer can't be called again.
    g.player_mut(tleilaxu).revealed_face_dancers.insert(Hero::Leader(emperor_leader));
    assert!(!FaceDanced::may_call_face_dancer(&g, tleilaxu));
}

#[test]
fn face_danced_source_locations_message_is_a_lazy_query() {
    let g = game_with(&[Faction::Tleilaxu]);
    let locations: Vec<Location> = g.map.locations(false).take(2).collect();
    let mut fd = FaceDanced::new(&g, Faction::Tleilaxu);
    fd.force_locations = Some(format!("{}:2|0,{}:0|0", locations[0].id(), locations[1].id()));
    let part = fd.determine_source_locations(&g);
    let rendered = part.to_cs_string();
    assert_eq!(
        rendered,
        "System.Linq.Enumerable+IEnumerableWhereSelectIterator`2[System.Collections.Generic.KeyValuePair`2[Treachery.Shared.Location,Treachery.Shared.Battalion],Treachery.Shared.MessagePart]"
    );

    // An empty string gives a condition of false, so no elements.
    fd.force_locations = Some(String::new());
    assert_eq!(fd.determine_source_locations(&g).to_cs_string(), "");
}

#[test]
fn diplomacy_valid_cards() {
    let mut g = game_with(&[Faction::Emperor, Faction::Atreides]);
    g.current_battle = Some(battle(&g, Faction::Emperor, Faction::Atreides));
    let emperor = idx(&g, Faction::Emperor);
    let useless1 = card_of(TreacheryCardType::Useless, 0);
    let useless2 = card_of(TreacheryCardType::Useless, 1);
    g.aggressor_plan = Some(plan(&g, Faction::Emperor, Some(useless1), Some(useless2)));
    assert_eq!(Diplomacy::valid_cards(&g, emperor), vec![useless2]);
    // No plan: plan! throws.
    let atreides = idx(&g, Faction::Atreides);
    let r = outcome(|| Diplomacy::valid_cards(&g, atreides));
    assert!(r.is_err());
}

#[test]
fn juice_valid_types_with_null_battle_winner() {
    let mut g = game_with(&[Faction::Emperor, Faction::Atreides]);
    let emperor = idx(&g, Faction::Emperor);
    g.current_main_phase = MainPhase::Battle;
    g.current_battle = Some(battle(&g, Faction::Emperor, Faction::Atreides));
    g.battle_winner = None;
    assert_eq!(JuicePlayed::valid_types(&g, emperor), vec![JuiceType::GoLast]);
    g.battle_winner = Some(Faction::None);
    assert_eq!(JuicePlayed::valid_types(&g, emperor), vec![JuiceType::Aggressor, JuiceType::GoLast]);
    g.current_battle = None;
    assert_eq!(JuicePlayed::valid_types(&g, emperor), vec![JuiceType::GoFirst, JuiceType::GoLast]);
}

#[test]
fn traitor_discarded_appends_without_shuffling() {
    let mut g = game_with(&[Faction::Harkonnen, Faction::Emperor]);
    let harkonnen = idx(&g, Faction::Harkonnen);
    let leaders: Vec<Hero> = LeaderManager::get_leaders(Faction::Emperor).take(3).map(Hero::Leader).collect();
    g.traitor_deck = Some(Deck::new(vec![leaders[0]], None));
    g.player_mut(harkonnen).traitors = vec![leaders[1], leaders[2]];
    g.number_of_traitors_to_discard = 1;
    g.phase_before_discarding_traitor = Phase::BattlePhase;

    let mut e = TraitorDiscarded::new(&g, Faction::Harkonnen);
    e.set_traitor(Some(leaders[2]));
    assert!(e.validate(&g).is_none());
    let e = Rc::new(e);
    e.execute_concrete_event(&mut g).unwrap();
    assert_eq!(g.traitor_deck.as_ref().unwrap().items, vec![leaders[0], leaders[2]]);
    assert_eq!(g.player(harkonnen).traitors, vec![leaders[1]]);
    assert_eq!(g.number_of_traitors_to_discard, 0);
    assert_eq!(g.current_phase, Phase::BattlePhase);

    let mut bad = TraitorDiscarded::new(&g, Faction::Harkonnen);
    bad.set_traitor(Some(leaders[0]));
    assert_eq!(bad.validate(&g).map(|m| m.to_cs_string()), Some("Invalid traitor".to_string()));
}

#[test]
fn thought_answered_shows_only_the_asked_card_when_held() {
    let mut g = game_with(&[Faction::Emperor, Faction::Atreides]);
    let atreides = idx(&g, Faction::Atreides);
    let laser = card_of(TreacheryCardType::Laser, 0);
    let shield = card_of(TreacheryCardType::Shield, 0);
    g.player_mut(atreides).treachery_cards = vec![shield, laser];
    let mut t = Thought::new(&g, Faction::Emperor);
    t.set_card(Some(laser));
    g.current_thought = Some(Rc::new(t));
    assert_eq!(ThoughtAnswered::valid_cards(&g, atreides), vec![laser]);

    let mut t = Thought::new(&g, Faction::Emperor);
    t.set_card(Some(card_of(TreacheryCardType::Laser, 0)));
    g.player_mut(atreides).treachery_cards = vec![shield];
    g.current_thought = Some(Rc::new(t));
    assert_eq!(ThoughtAnswered::valid_cards(&g, atreides), vec![shield]);

    // No thought: CurrentThought.Card throws.
    g.current_thought = None;
    let r = outcome(|| ThoughtAnswered::valid_cards(&g, atreides));
    assert!(r.is_err());
}

/// `AuditCancelled.cs:89` logs the live `Game.AuditedCards` list. The next audit clears and
/// refills it (`Game_Battle.cs:427-429`), and the C# state dump then shows the new contents
/// in the old message (hazards 4.5).
#[test]
fn audit_cancelled_logs_audited_cards_as_a_live_list() {
    let mut g = game_with(&[Faction::Choam, Faction::Emperor]);
    let first = card_of(TreacheryCardType::Laser, 0);
    let (second, third) = (card_of(TreacheryCardType::Shield, 0), card_of(TreacheryCardType::Shield, 1));
    g.audited_cards = vec![first];

    let mut ev = AuditCancelled::new(&g, Faction::Emperor);
    ev.cancelled = false;
    let ev = Rc::new(ev);
    outcome(|| ev.execute_concrete_event(&mut g)).unwrap().unwrap();

    let msg = g.current_report().messages.last().cloned().unwrap();
    assert_eq!(msg.target, Faction::Emperor);
    let element = msg.expression.elements[2].clone();
    let dump = |g: &Game, e: &ExpressionElement| String::from_utf8(treachery_engine::canon::dump_detached(e, Some(g))).unwrap();
    assert_eq!(dump(&g, &element), dump(&g, &ExpressionElement::list(vec![first])));

    g.audited_cards.clear();
    g.audited_cards.push(second);
    g.audited_cards.push(third);
    assert_eq!(dump(&g, &element), dump(&g, &ExpressionElement::list(vec![second, third])));
    assert_eq!(element.to_cs_string(), "System.Collections.Generic.List`1[Treachery.Shared.TreacheryCard]");
}
