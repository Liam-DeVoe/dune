//! The bot fixes of behavior-changes.md (step 4 of native-plan.md): each test shows the
//! behavior a fix brought, on a state built from a fixture in `scripts/fixtures`. The fall-through
//! after an invalid decision and the Mirror Weapon pick have unit tests next to their code
//! (`bot/mod.rs`, `bot/battle.rs`), since the methods involved are private.

use treachery_bots::prelude::*;
use treachery_engine::canon::dump_game;
use treachery_engine::game::{GameState, Participation};

use TreacheryCardType as T;

fn fixtures_dir() -> String {
    format!("{}/../../scripts/fixtures", env!("CARGO_MANIFEST_DIR"))
}

fn load(name: &str) -> Game {
    let path = format!("{}/{name}.json", fixtures_dir());
    let state = GameState::load(&std::fs::read_to_string(&path).unwrap()).unwrap();
    Game::load(&state, &Shared::new(Participation::default()), false, false).unwrap_or_else(|_| panic!("{name} loads"))
}

fn bot(game: &Game, player: PlayerIndex) -> ClassicBot {
    ClassicBot::new(player, BotParameters::for_faction(game.player(player).faction), SharedRandom::from_seed(1))
}

fn player_of(game: &Game, faction: Faction) -> PlayerIndex {
    game.get_player(Some(faction)).unwrap()
}

/// A card of `type_` nobody holds yet. Cards are taken out of the decks only as far as hands go,
/// which is all these tests look at.
fn card(game: &Game, type_: TreacheryCardType) -> TreacheryCard {
    let held: Vec<TreacheryCard> = game.player_indices().flat_map(|p| game.player(p).treachery_cards.clone()).collect();
    TreacheryCardManager::items().iter().copied().find(|c| c.type_() == type_ && !held.contains(c)).expect("a card of that type")
}

/// No decision of any bot, at any priority, changes the game, in any fixture state.
#[test]
fn deciding_never_changes_the_game() {
    let mut names: Vec<String> = std::fs::read_dir(fixtures_dir())
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok()?.strip_suffix(".json").map(str::to_string))
        .collect();
    names.sort();
    assert!(names.len() > 40);
    let mut decisions = 0;
    for name in names {
        let mut game = load(&name);
        for player in game.player_indices().collect::<Vec<_>>() {
            let allowed = game.get_applicable_events(Some(player), true);
            let mut b = bot(&game, player);
            for priority in Priority::ALL {
                let before = dump_game(&game);
                if b.decide(&game, priority, &allowed).is_some() {
                    decisions += 1;
                }
                assert!(dump_game(&game) == before, "{name}: deciding at {priority:?} for player {player:?} changed the game");
            }
        }
    }
    assert!(decisions > 50, "only {decisions} decisions");
}

/// The Rock Melter decision works out the battle with the opponent's Mirror Weapon in play. The
/// battle itself logs "<Mirror Weapon> becomes a <card>"; deciding no longer does.
#[test]
fn rock_melter_decision_with_a_mirror_weapon_leaves_the_game_alone() {
    let mut game = load("mc-treachery");
    let (agg, def) = (game.aggressor_plan.clone().unwrap(), game.defender_plan.clone().unwrap());
    assert!(agg.hero().is_some() && def.hero().is_some());
    agg.set_weapon(Some(card(&game, T::MirrorWeapon)));
    def.set_weapon(Some(card(&game, T::Laser)));

    let messages = game.current_report().messages.len();
    let before = dump_game(&game);
    let decision = bot(&game, def.player()).determine_rock_was_melted(&game);
    assert!(dump_game(&game) == before, "deciding changed the game");
    assert_eq!(agg.weapon().map(|c| c.type_()), Some(T::MirrorWeapon), "the live plan keeps its Mirror Weapon");

    // The same outcome, worked out for real, logs the Mirror Weapon and picks the same winner.
    let territory = game.current_battle.as_ref().unwrap().territory(&game).unwrap();
    let outcome = Battle::determine_battle_outcome(&agg, &def, territory, &mut game);
    assert_eq!(game.current_report().messages.len(), messages + 1);
    assert_eq!(decision.kill, outcome.winner == Some(def.player()));
}

/// Shipping into a sand territory costs 2 per force, half that with Karama. A Bene Gesserit
/// known to hold a Worthless card can use it as Karama under BeneGesseritWorthlessAsKarma, so the
/// bot now expects them to ship as many forces as with a real Karama card.
#[test]
fn worthless_cards_count_as_karama_for_bene_gesserit() {
    let mut game = load("bid");
    let me = player_of(&game, Faction::Atreides);
    let bg = player_of(&game, Faction::BeneGesserit);
    let sand = game.map.the_great_flat().territory();
    game.player_mut(bg).resources = 6;

    let reinforcements_holding = |game: &mut Game, type_: TreacheryCardType, rule: bool| {
        let c = card(game, type_);
        game.player_mut(bg).treachery_cards = vec![c];
        game.player_mut(me).known_cards.add(c);
        game.rules.retain(|r| *r != Rule::BeneGesseritWorthlessAsKarma);
        if rule {
            game.rules.push(Rule::BeneGesseritWorthlessAsKarma);
        }
        ClassicBot::max_reinforced_dial_to(game, me, bg, sand)
    };

    let with_karama = reinforcements_holding(&mut game, T::Karma, true);
    let with_worthless = reinforcements_holding(&mut game, T::Useless, true);
    let with_worthless_without_rule = reinforcements_holding(&mut game, T::Useless, false);
    assert!(with_karama > with_worthless_without_rule, "{with_karama} vs {with_worthless_without_rule}");
    assert_eq!(with_worthless, with_karama);
}

/// The opponent about to ship has no ally and 5 spice, holds three strongholds and has forces in
/// reserve. Before the fix only an opponent with an ally could be blocked.
#[test]
fn karama_shipment_prevention_blocks_a_winning_opponent_without_ally() {
    let mut game = load("ship");
    let shipper = game.shipment_and_move_sequence.as_ref().unwrap().current_player(&game).unwrap();
    assert_eq!(game.player(shipper).ally, Faction::None);
    assert_ne!(game.player(shipper).faction, Faction::Fremen);
    let strongholds = [game.map.arrakeen(), game.map.carthag(), game.map.sietch_tabr(), game.map.habbanya_sietch()];
    for s in strongholds {
        Player::add_forces(&mut game, shipper, s, 1, true);
    }
    game.player_mut(shipper).resources = 5;
    assert!(game.player(shipper).forces_in_reserve() + game.player(shipper).special_forces_in_reserve() > 2);

    let blocker = game.player_indices().find(|p| *p != shipper && game.player(*p).faction != Faction::Fremen).unwrap();
    let e = bot(&game, blocker).determine_karma_shipment_prevention(&game).expect("a Karama shipment prevention");
    assert_eq!(e.target, game.player(shipper).faction);
}

/// Shipping normal forces: the bot sizes the shipment with the price of the forces it adds, so it
/// never ships more than it can pay for. Shipment cost depends only on the number of forces, so
/// pricing a special force instead (the old code) gave the same sizes.
#[test]
fn shipment_sizing_prices_the_normal_forces_it_adds() {
    let mut game = load("ship");
    let me = player_of(&game, Faction::Atreides);
    game.player_mut(me).resources = 7;
    let sand = game.map.the_great_flat();
    let b = bot(&game, me);
    let size = b.shipment_size(&game, 20.0, false, sand, None, 10, 0, 0, 99, false);
    // 2 spice per force in the sand: 7 spice pays for 3.
    assert_eq!((size.forces, size.special_forces), (3, 0));
}

/// Bene Gesserit with only a poison weapon, against an opponent known to hold a Shield and one
/// unknown card: the Shield doesn't stop poison, so the voice forces it.
#[test]
fn bene_gesserit_voices_a_useless_defense_against_poison() {
    let mut game = load("battle");
    let bg = player_of(&game, Faction::BeneGesserit);
    let opponent = player_of(&game, Faction::Harkonnen);
    let poison = card(&game, T::Poison);
    game.player_mut(bg).treachery_cards = vec![poison];
    let shield = card(&game, T::Shield);
    let unknown = card(&game, T::Karma);
    game.player_mut(opponent).treachery_cards = vec![shield, unknown];
    game.player_mut(bg).known_cards.add(shield);

    let plan = bot(&game, bg).best_voice(&game, None, bg, opponent);
    assert_eq!(plan.weapon_to_use, Some(poison));
    assert!(plan.voice.must);
    assert_eq!(plan.voice.type_, T::Shield);
}

/// The dial an Ecaz bot estimates for its ally (BattleClaimed) is the ally's own estimate, not the
/// bot's.
#[test]
fn dial_needed_for_another_player_is_theirs() {
    let game = load("battle");
    let territory = game.current_battle.as_ref().unwrap().territory(&game).unwrap();
    let aggressor = player_of(&game, game.current_battle.as_ref().unwrap().aggressor(&game));
    let defender = player_of(&game, game.current_battle.as_ref().unwrap().defender(&game));
    let estimator = game.player_indices().find(|p| *p != aggressor && *p != defender && game.player(*p).faction != Faction::BeneGesserit).unwrap();

    let by_aggressor = bot(&game, aggressor).get_dial_needed(&game, aggressor, territory, defender, false);
    let by_other = bot(&game, estimator).get_dial_needed(&game, aggressor, territory, defender, false);
    let others_own = bot(&game, estimator).get_dial_needed(&game, estimator, territory, defender, false);
    assert_eq!(by_other, by_aggressor);
    assert_ne!(by_other, others_own, "the state doesn't tell the two players apart");
}

/// The opponent has told everyone that all my leaders are their traitors, so any leader I pick
/// betrays me and the dial I need is 99. Before the fix the bot looked at the traitors it had
/// told itself.
#[test]
fn traitors_told_by_the_opponent_count_as_known_traitors() {
    let mut game = load("battle");
    let battle = game.current_battle.clone().unwrap();
    let territory = battle.territory(&game).unwrap();
    let (mut me, mut opponent) = (player_of(&game, battle.aggressor(&game)), player_of(&game, battle.defender(&game)));
    if game.player(me).faction == Faction::Atreides {
        // Avoid the Kwisatz Haderach, who makes traitors harmless.
        std::mem::swap(&mut me, &mut opponent);
    }
    let without = bot(&game, me).get_dial_needed(&game, me, territory, opponent, false);
    for l in game.player(me).leaders.clone() {
        game.player_mut(opponent).told_traitors.add(Hero::Leader(l));
    }
    let with = bot(&game, me).get_dial_needed(&game, me, territory, opponent, false);
    assert_ne!(without, 99.0);
    assert_eq!(with, 99.0);
}

/// Without a human to pick, Bene Gesserit predicts a random other player with one draw, then
/// draws the turn. The coin flip that came first is gone.
#[test]
fn bene_gesserit_prediction_draws_no_coin() {
    let game = load("mc-prediction");
    let bg = player_of(&game, Faction::BeneGesserit);
    for seed in 1..20 {
        let b = ClassicBot::new(bg, BotParameters::for_faction(Faction::BeneGesserit), SharedRandom::from_seed(seed));
        let e = b.determine_bene_gesserit_prediction(&game);
        // The turn takes two draws for every faction but Tleilaxu (one), who isn't in this game.
        assert_ne!(e.to_win, Faction::Tleilaxu);
        assert_eq!(b.random.get_count(), 3, "seed {seed}: predicted {:?}", e.to_win);
    }
}
