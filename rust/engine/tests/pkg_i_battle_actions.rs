//! Focused tests for the actions around a fight: Voice, Prescience, Thought, Diplomacy, audits,
//! face dancers, Juice, Residual, ... They build small game states by hand and check rules that
//! are easy to get wrong.

use treachery_engine::error::catch_panic;
use treachery_engine::prelude::*;

fn outcome<R>(f: impl FnOnce() -> R) -> Result<R, String> {
    catch_panic(f)
}

fn game_with(factions: &[Faction]) -> Game {
    let mut g = Game::new();
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

fn battle(g: &Game, initiator: Faction, target: Faction) -> InitiatedBattle {
    // A plain sand territory (not a homeworld).
    let t = g.board.map.territories(false).find(|t| !t.is_homeworld() && !t.is_stronghold()).unwrap();
    BattleInitiated { target: Some(target), territory: Some(t) }.battle(By::new(g, initiator), EntryId::default())
}

fn plan(g: &Game, f: Faction, weapon: Option<TreacheryCard>, defense: Option<TreacheryCard>) -> BattlePlan {
    Battle { weapon, defense, ..Default::default() }.plan(By::new(g, f))
}

#[test]
fn voice_is_voiced_by_without_special_card_rule() {
    let g = game_with(&[Faction::BeneGesserit, Faction::Emperor]);
    use TreacheryCardType as T;
    // Identity always matches.
    assert!(Voice::is_voiced_by(&g, true, true, T::Laser, Some(T::Laser)));
    // Chemistry is a poison only as a weapon and only when voice is "may not".
    assert!(Voice::is_voiced_by(&g, true, false, T::Chemistry, Some(T::Poison)));
    assert!(!Voice::is_voiced_by(&g, true, true, T::Chemistry, Some(T::Poison)));
    assert!(!Voice::is_voiced_by(&g, false, false, T::Chemistry, Some(T::Poison)));
    // Chemistry as defense is a poison defense, whatever "must" is.
    assert!(Voice::is_voiced_by(&g, false, true, T::Chemistry, Some(T::PoisonDefense)));
    // Portable antidote counts as poison defense.
    assert!(Voice::is_voiced_by(&g, false, true, T::PortableAntidote, Some(T::PoisonDefense)));
    // Weirding way: projectile as weapon, projectile defense only for "may not" and as defense.
    assert!(Voice::is_voiced_by(&g, true, true, T::WeirdingWay, Some(T::Projectile)));
    assert!(!Voice::is_voiced_by(&g, false, true, T::WeirdingWay, Some(T::Projectile)));
    assert!(Voice::is_voiced_by(&g, false, false, T::WeirdingWay, Some(T::ProjectileDefense)));
    assert!(!Voice::is_voiced_by(&g, false, true, T::WeirdingWay, Some(T::ProjectileDefense)));
    // Unlisted voiced types only match themselves.
    assert!(!Voice::is_voiced_by(&g, true, true, T::Projectile, Some(T::Laser)));
}

#[test]
fn voice_is_voiced_by_with_special_card_rule() {
    let mut g = game_with(&[Faction::BeneGesserit, Faction::Emperor]);
    g.rules.active.push(Rule::BeneGesseritVoiceMustNameSpecialCards);
    use TreacheryCardType as T;
    assert!(Voice::is_voiced_by(&g, false, true, T::Antidote, Some(T::PoisonDefense)));
    assert!(!Voice::is_voiced_by(&g, false, true, T::ShieldAndAntidote, Some(T::PoisonDefense)));
    assert!(Voice::is_voiced_by(&g, false, true, T::Shield, Some(T::ProjectileDefense)));
    assert!(!Voice::is_voiced_by(&g, true, true, T::ProjectileAndPoison, Some(T::Poison)));
}

#[test]
fn voice_valid_types_by_rules() {
    let mut g = game_with(&[Faction::BeneGesserit, Faction::Emperor]);
    use TreacheryCardType as T;
    assert_eq!(
        Voice::valid_types(&g),
        vec![T::Mercenary, T::Laser, T::Poison, T::Projectile, T::Shield, T::Antidote, T::Useless, T::PoisonDefense, T::ProjectileDefense]
    );
    g.rules.active.push(Rule::BeneGesseritVoiceMustNameSpecialCards);
    assert_eq!(Voice::valid_types(&g).len(), 7);
}

#[test]
fn voice_validate_with_null_player() {
    let mut g = game_with(&[Faction::Emperor, Faction::Atreides]);
    // BeneGesserit is not playing, so Player stays null.
    let v = Voice::default();
    let by = By::new(&g, Faction::BeneGesserit);
    assert!(by.player.is_none());
    // No battle: MayUseVoice returns false before touching the player.
    let r = outcome(|| v.validate(&g, by).err().map(|m| m.0.plain_text()));
    assert_eq!(r, Ok(Some("You cannot use Voice".to_string())));
    // With a battle, IsInvolved(p) dereferences the null player.
    g.battle.current_battle = Some(battle(&g, Faction::Emperor, Faction::Atreides));
    let r = outcome(|| v.validate(&g, by).err().map(|m| m.0.plain_text()));
    assert!(r.is_err());
}

#[test]
fn may_call_face_dancer_distinguishes_null_and_none_winner() {
    let mut g = game_with(&[Faction::Tleilaxu, Faction::Emperor, Faction::Atreides]);
    let tleilaxu = idx(&g, Faction::Tleilaxu);
    // No battle.
    assert!(!FaceDanced::may_call_face_dancer(&g, tleilaxu));
    g.battle.current_battle = Some(battle(&g, Faction::Emperor, Faction::Atreides));
    // BattleWinner == Faction.None
    g.battle.battle_winner = Some(None);
    assert!(!FaceDanced::may_call_face_dancer(&g, tleilaxu));

    // Emperor wins with a leader that is one of Tleilaxu's face dancers.
    let emperor_leader = LeaderManager::get_leaders(Faction::Emperor).next().unwrap();
    let mut p = plan(&g, Faction::Emperor, None, None);
    p.hero = Some(Hero::Leader(emperor_leader));
    g.battle.aggressor_plan = Some(p);
    g.battle.battle_winner = Some(Some(Faction::Emperor));
    g.player_mut(tleilaxu).face_dancers.push(Hero::Leader(emperor_leader));
    assert!(FaceDanced::may_call_face_dancer(&g, tleilaxu));
    // A revealed face dancer can't be called again.
    g.player_mut(tleilaxu).revealed_face_dancers.insert(Hero::Leader(emperor_leader));
    assert!(!FaceDanced::may_call_face_dancer(&g, tleilaxu));
}

#[test]
fn face_danced_source_locations_message() {
    let g = game_with(&[Faction::Tleilaxu]);
    let locations: Vec<Location> = g.board.map.locations(false).take(2).collect();
    let by = By::new(&g, Faction::Tleilaxu);
    let mut fd = FaceDanced::default();
    fd.force_locations = Some(format!("{}:2|0,{}:0|0", locations[0].id(), locations[1].id()));
    let part = fd.determine_source_locations(&g, by);
    let rendered = part.plain_text();
    assert_eq!(
        rendered,
        "[, 2 from Treachery.Shared.Location]"
    );

    // An empty string gives a condition of false, so no elements.
    fd.force_locations = Some(String::new());
    assert_eq!(fd.determine_source_locations(&g, by).plain_text(), "");
}

#[test]
fn diplomacy_valid_cards() {
    let mut g = game_with(&[Faction::Emperor, Faction::Atreides]);
    g.battle.current_battle = Some(battle(&g, Faction::Emperor, Faction::Atreides));
    let emperor = idx(&g, Faction::Emperor);
    let useless1 = card_of(TreacheryCardType::Useless, 0);
    let useless2 = card_of(TreacheryCardType::Useless, 1);
    g.battle.aggressor_plan = Some(plan(&g, Faction::Emperor, Some(useless1), Some(useless2)));
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
    g.turn.current_main_phase = MainPhase::Battle;
    g.battle.current_battle = Some(battle(&g, Faction::Emperor, Faction::Atreides));
    g.battle.battle_winner = None;
    assert_eq!(JuicePlayed::valid_types(&g, emperor), vec![JuiceType::GoLast]);
    g.battle.battle_winner = Some(None);
    assert_eq!(JuicePlayed::valid_types(&g, emperor), vec![JuiceType::Aggressor, JuiceType::GoLast]);
    g.battle.current_battle = None;
    assert_eq!(JuicePlayed::valid_types(&g, emperor), vec![JuiceType::GoFirst, JuiceType::GoLast]);
}

#[test]
fn traitor_discarded_appends_without_shuffling() {
    let mut g = game_with(&[Faction::Harkonnen, Faction::Emperor]);
    let harkonnen = idx(&g, Faction::Harkonnen);
    let leaders: Vec<Hero> = LeaderManager::get_leaders(Faction::Emperor).take(3).map(Hero::Leader).collect();
    g.decks.traitor_deck = Some(Deck::new(vec![leaders[0]]));
    g.player_mut(harkonnen).traitors = vec![leaders[1], leaders[2]];
    g.turn.number_of_traitors_to_discard = 1;
    g.turn.phase_before_discarding_traitor = Some(Phase::BattlePhase);

    let by = By::new(&g, Faction::Harkonnen);
    let mut e = TraitorDiscarded::default();
    e.traitor = Some(leaders[2]);
    assert!(e.validate(&g, by).is_ok());
    e.apply(&mut g, by).unwrap();
    assert_eq!(g.decks.traitor_deck.as_ref().unwrap().items, vec![leaders[0], leaders[2]]);
    assert_eq!(g.player(harkonnen).traitors, vec![leaders[1]]);
    assert_eq!(g.turn.number_of_traitors_to_discard, 0);
    assert_eq!(g.turn.current_phase, Phase::BattlePhase);

    let mut bad = TraitorDiscarded::default();
    bad.traitor = Some(leaders[0]);
    assert_eq!(bad.validate(&g, by).err().map(|m| m.0.plain_text()), Some("Invalid traitor".to_string()));
}

#[test]
fn thought_answered_shows_only_the_asked_card_when_held() {
    let mut g = game_with(&[Faction::Emperor, Faction::Atreides]);
    let atreides = idx(&g, Faction::Atreides);
    let laser = card_of(TreacheryCardType::Laser, 0);
    let shield = card_of(TreacheryCardType::Shield, 0);
    g.player_mut(atreides).treachery_cards = vec![shield, laser];
    g.battle.current_thought = Some(ThoughtAsked { submitter: By::new(&g, Faction::Emperor), card: Some(laser) });
    assert_eq!(ThoughtAnswered::valid_cards(&g, atreides), vec![laser]);

    g.player_mut(atreides).treachery_cards = vec![shield];
    g.battle.current_thought = Some(ThoughtAsked { submitter: By::new(&g, Faction::Emperor), card: Some(card_of(TreacheryCardType::Laser, 0)) });
    assert_eq!(ThoughtAnswered::valid_cards(&g, atreides), vec![shield]);

    // No thought: CurrentThought.Card throws.
    g.battle.current_thought = None;
    let r = outcome(|| ThoughtAnswered::valid_cards(&g, atreides));
    assert!(r.is_err());
}
