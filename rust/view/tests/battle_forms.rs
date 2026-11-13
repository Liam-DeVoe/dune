//! The battle action forms (`src/actions/battle.rs`): in every fixture state and every state of
//! the sample bot games where one of the 25 battle event types is applicable to a player, the
//! form's options equal the results of the engine's helpers, its draft is valid event JSON, and
//! the dependent options (the battle plan's
//! weapons and defenses per leader and card, Clairvoyance's questions per target) follow the
//! draft in a draft query.

mod common;

use std::collections::BTreeMap;
use std::path::PathBuf;

use treachery_engine::compat::cs::Shared;
use treachery_engine::events::GameEventType;
use treachery_engine::json;
use treachery_engine::compat::net_enum::NetEnum;
use treachery_engine::game::Participation;
use treachery_engine::prelude::*;
use treachery_view::actions::{self, battle};
use treachery_view::build::{event, ResolvedViewer};
use treachery_view::describe::{default_describer, Describer, InGame};
use treachery_view::replica::ViewReplica;
use treachery_view::schema::*;

fn viewer(game: &Game, p: PlayerIndex) -> ResolvedViewer {
    ResolvedViewer { player: Some(p), faction: Some(game.player(p).faction), is_host: p.index == 0, is_observer: false }
}

fn ordinal(a: &str, b: &str) -> std::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

fn ids<T: IIdentifiable>(xs: impl IntoIterator<Item = T>) -> Vec<i32> {
    xs.into_iter().map(|x| x.id()).collect()
}

fn values<T: NetEnum>(xs: impl IntoIterator<Item = T>) -> Vec<i32> {
    xs.into_iter().map(|x| x.value()).collect()
}

/// `OrderBy(Name)` over cards.
fn cards_by_name(cards: Vec<Option<TreacheryCard>>, d: &dyn Describer) -> Vec<i32> {
    let mut named: Vec<(String, i32)> = cards.into_iter().flatten().map(|c| (d.describe(&c.into()), c.id())).collect();
    named.sort_by(|a, b| ordinal(&a.0, &b.0));
    named.into_iter().map(|(_, id)| id).collect()
}

/// The draft carried in the options: valid event JSON of the form's type, which the engine reads
/// back to the same JSON.
fn check_draft(game: &Game, faction: Faction, draft: &serde_json::Value, t: GameEventType, at: &str) -> GameEvent {
    let text = draft.to_string();
    let e: GameEvent = json::deserialize(&text).unwrap_or_else(|err| panic!("{at}: draft of {} is not event JSON: {err:?}", t.name()));
    assert_eq!(e.event_type(), t, "{at}");
    let again: serde_json::Value = serde_json::from_str(&json::serialize(&e)).unwrap();
    assert_eq!(&again, draft, "{at}: the draft doesn't survive the engine's JSON");
    // As a draft query and a submission do: the server sets the initiator.
    e.initialize(game, faction);
    e
}

/// Every battle form carries its draft; take it from the serialized options.
fn draft_of(o: &ActionOptions) -> serde_json::Value {
    serde_json::to_value(o).unwrap()["draft"].clone()
}

/// What the forms reached, for the summary and the coverage asserts.
#[derive(Default)]
struct Stats(BTreeMap<String, usize>, BTreeMap<String, String>);

impl Stats {
    fn add(&mut self, what: impl Into<String>) {
        *self.0.entry(what.into()).or_default() += 1;
    }

    /// Counts a form and remembers where it was first reached.
    fn reached(&mut self, what: &str, at: &str) {
        self.add(what);
        self.1.entry(what.to_string()).or_insert_with(|| at.to_string());
    }
}

/// Checks one applicable battle form of player `p`.
fn check(game: &Game, p: PlayerIndex, t: GameEventType, d: &dyn Describer, stats: &mut Stats, at: &str) {
    let v = viewer(game, p);
    let view = actions::action_view(game, &v, t, d);
    let options = view.options.clone().unwrap_or_else(|| panic!("{at}: no options for {}", t.name()));
    let at = format!("{at} {} {}", game.player(p).faction, t.name());
    let at = at.as_str();
    let draft = check_draft(game, game.player(p).faction, &draft_of(&options), t, at);
    // The view's validation is the draft's.
    assert_eq!(view.validation, treachery_view::build::validate(game, draft.clone(), d), "{at}: validation");
    stats.reached(t.name(), at);
    let me = game.player(p);
    match (&options, &draft) {
        (ActionOptions::AuditCancelled(o), _) => {
            assert_eq!((o.cost, o.cards), (AuditCancelled::cost_game(game), AuditCancelled::get_number_of_cards_that_may_be_audited(game)), "{at}");
        }
        (ActionOptions::Audited(o), _) => assert_eq!(o.cards, ids(game.audited_cards.iter().copied()), "{at}"),
        (ActionOptions::Battle(o), GameEvent::Battle(b)) => check_battle(game, p, o, b, d, stats, at),
        (ActionOptions::BattleConcluded(o), _) => {
            let winner = game.battle_winner == Some(me.faction);
            assert_eq!(o.winner, winner, "{at}");
            if winner {
                let supplier = Battle::determine_force_supplier(game, p);
                assert_eq!(o.max_replacement_forces, BattleConcluded::valid_max_replacement_force_amount(game, supplier), "{at}");
                assert_eq!(o.capture.is_some(), BattleConcluded::may_capture_or_kill(game, Some(p)), "{at}");
                if let Some(c) = &o.capture {
                    assert_eq!(c.decisions.iter().map(|x| x.value).collect::<Vec<_>>(), values(BattleConcluded::valid_capture_decisions(game)), "{at}");
                    stats.add("BattleConcluded capture");
                }
                let loser = game.get_player(game.battle_loser).unwrap();
                assert_eq!(o.tech_tokens.iter().map(|x| x.value).collect::<Vec<_>>(), values(game.player(loser).tech_tokens.iter().copied()), "{at}");
                assert_eq!(o.decipherer_traitors.iter().map(|h| h.id).collect::<Vec<_>>(), ids(game.traitors_decipherer_can_look_at.iter().copied()), "{at}");
                assert_eq!(o.add_extra_force_in.is_some(), BattleConcluded::may_add_extra_force(game, p), "{at}");
                if !BattleConcluded::may_choose_to_discard_cards(game) {
                    assert!(o.discardable_weapon.is_none() && o.discardable_defense.is_none() && o.discardable_mercenary.is_none(), "{at}");
                } else if o.discardable_weapon.is_some() || o.discardable_defense.is_some() {
                    stats.add("BattleConcluded discard");
                }
                stats.add("BattleConcluded winner");
            }
        }
        (ActionOptions::BattleInitiated(o), GameEvent::BattleInitiated(e)) => {
            let fights = Battle::battles_to_be_fought(game, p, false);
            let got: Vec<(i32, i32)> = o.fights.iter().map(|f| (f.faction.value, f.territory)).collect();
            let want: Vec<(i32, i32)> = fights.iter().map(|f| (f.faction.value(), f.territory.id())).collect();
            assert_eq!(got, want, "{at}");
            // The default is the first opponent and its first territory.
            assert_eq!(e.target, fights[0].faction, "{at}");
            assert_eq!(e.territory(game), Some(fights[0].territory), "{at}");
            assert_eq!(view.validation, None, "{at}: the first fight is a valid battle");
        }
        (ActionOptions::BeneGesseritBattleAnnouncement(o), _) => {
            assert_eq!(o.territories, ids(BeneGesseritBattleAnnouncement::valid_territories(game, p)), "{at}")
        }
        (ActionOptions::CardsDetermined(o), _) => {
            assert_eq!(o.treachery_cards, ids(game.treachery_deck.as_ref().unwrap().items.iter().copied()), "{at}");
            assert_eq!(o.richese_cards, ids(game.richese_cache.iter().copied()), "{at}");
        }
        (ActionOptions::ClairVoyanceAnswered(o), _) => {
            let latest = game.latest_clairvoyance.as_ref().unwrap();
            assert_eq!(o.questioner, latest.initiator().to_string(), "{at}");
        }
        (ActionOptions::ClairVoyancePlayed(o), GameEvent::ClairVoyancePlayed(e)) => {
            let targets: Vec<Faction> = ClairVoyancePlayed::valid_targets(game, p).collect();
            assert_eq!(o.targets.iter().map(|x| x.value).collect::<Vec<_>>(), values(targets.iter().copied()), "{at}");
            assert_eq!(o.questions.iter().map(|x| x.value).collect::<Vec<_>>(), values(ClairVoyancePlayed::valid_questions(game, e.target)), "{at}");
            assert_eq!(o.card_types.iter().map(|x| x.value.clone()).collect::<Vec<_>>(), Voice::valid_types(game).iter().map(|t| t.to_string()).collect::<Vec<_>>(), "{at}");
            assert_eq!(o.turns.iter().map(|x| x.value.clone()).collect::<Vec<_>>(), BeneGesseritPrediction::valid_turns(game).iter().map(|t| t.to_string()).collect::<Vec<_>>(), "{at}");
            assert_eq!(o.leaders.len(), game.traitors_in_play().len(), "{at}");
            // Dependent: the questions follow the draft's target.
            for target in targets {
                let mut q = ClairVoyancePlayed::new(game, me.faction);
                q.target = target;
                let (opts, _) = battle::options_and_validation(game, &v, &event(q), d).unwrap();
                let Some(ActionOptions::ClairVoyancePlayed(o)) = opts else { panic!("{at}") };
                assert_eq!(o.questions.iter().map(|x| x.value).collect::<Vec<_>>(), values(ClairVoyancePlayed::valid_questions(game, target)), "{at}");
            }
        }
        (ActionOptions::Diplomacy(o), _) => assert_eq!(o.cards, ids(Diplomacy::valid_cards(game, p)), "{at}"),
        (ActionOptions::FaceDanced(o), _) => {
            assert_eq!(o.maximum_forces, FaceDanced::maximum_number_of_forces(game, p), "{at}");
            assert_eq!(o.sources.iter().map(|s| s.location).collect::<Vec<_>>(), ids(FaceDanced::valid_source_locations(game, p)), "{at}");
            assert_eq!(o.targets.iter().map(|s| s.location).collect::<Vec<_>>(), ids(FaceDanced::valid_target_locations(game)), "{at}");
        }
        (ActionOptions::FaceDancerReplaced(o), _) => {
            assert_eq!(o.heroes.iter().map(|h| h.id).collect::<Vec<_>>(), ids(FaceDancerReplaced::valid_face_dancers(game, p)), "{at}")
        }
        (ActionOptions::FaceDancerRevealed(o), _) => {
            assert_eq!(o.may_call, FaceDanced::may_call_face_dancer(game, p), "{at}")
        }
        (ActionOptions::HMSAdvantageChosen(o), _) => {
            assert_eq!(o.choices.iter().map(|x| x.value).collect::<Vec<_>>(), values(HMSAdvantageChosen::valid_advantages(game, p)), "{at}")
        }
        (ActionOptions::JuicePlayed(o), _) => assert_eq!(o.choices.iter().map(|x| x.value).collect::<Vec<_>>(), values(JuicePlayed::valid_types(game, p)), "{at}"),
        (ActionOptions::LoserConcluded(o), GameEvent::LoserConcluded(e)) => {
            assert_eq!(e.kept_card(), None, "{at}: default kept card");
            assert_eq!(o.cards_to_keep, ids(LoserConcluded::cards_loser_may_keep(game)), "{at}");
            assert_eq!(o.assassination.is_some(), LoserConcluded::can_assassinate(game, p), "{at}");
            assert_eq!(o.may_use_karma, LoserConcluded::may_use_karma_to_force_kept_cards(game, p), "{at}");
            if o.may_use_karma {
                assert_eq!(o.cards_kept_or_discarded, ids(LoserConcluded::cards_that_may_be_kept_or_discarded(game)), "{at}");
            }
        }
        (ActionOptions::Planetology(_), _) | (ActionOptions::RockWasMelted(_), _) => {}
        (ActionOptions::Prescience(o), _) => assert_eq!(o.aspects.iter().map(|x| x.value).collect::<Vec<_>>(), values(Prescience::valid_aspects(game, p)), "{at}"),
        (ActionOptions::Retreat(o), _) => {
            assert_eq!(o.targets, ids(Retreat::valid_targets(game, p)), "{at}");
            assert_eq!(
                (o.max_forces, o.max_special_forces, o.max_total_forces),
                (Retreat::max_forces(game, p), Retreat::max_special_forces(game, p), Retreat::max_total_forces(game, p)),
                "{at}"
            );
        }
        (ActionOptions::TakeLosses(o), _) => {
            let loss = TakeLosses::losses_to_take(game);
            assert_eq!((o.amount, o.location), (loss.amount, loss.location.map(|l| l.id())), "{at}");
            assert_eq!(o.max_forces, TakeLosses::valid_max_force_amount(game, p), "{at}");
            assert_eq!(o.max_special_forces, TakeLosses::valid_max_special_force_amount(game, p), "{at}");
        }
        (ActionOptions::ThoughtAnswered(o), _) => assert_eq!(o.cards, ids(ThoughtAnswered::valid_cards(game, p)), "{at}"),
        (ActionOptions::Thought(o), _) => assert_eq!(o.cards, ids(Thought::valid_cards(game)), "{at}"),
        (ActionOptions::TreacheryCalled(o), _) => assert_eq!(o.may_call, TreacheryCalled::may_call_treachery(game, p), "{at}"),
        (ActionOptions::Voice(o), _) => {
            let mut want: Vec<(String, i32)> = Voice::valid_types(game).into_iter().map(|x| (d.describe(&x.into()), x.value())).collect();
            want.sort_by(|a, b| ordinal(&a.0, &b.0));
            assert_eq!(o.types.iter().map(|x| x.value).collect::<Vec<_>>(), want.into_iter().map(|x| x.1).collect::<Vec<_>>(), "{at}");
        }
        (o, _) => panic!("{at}: unexpected options {o:?}"),
    }
}

/// The battle plan: heroes, cards per draft, the mixes, the token rows and the steppers' data.
fn check_battle(game: &Game, p: PlayerIndex, o: &BattleOptions, draft: &Battle, d: &dyn Describer, stats: &mut Stats, at: &str) {
    let v = viewer(game, p);
    let battle = game.current_battle.as_ref().unwrap();
    let territory = battle.territory(game);
    let supplier = Battle::determine_force_supplier(game, p);
    let heroes = Battle::valid_battle_heroes(game, p);
    assert_eq!(o.heroes.iter().map(|h| h.id).collect::<Vec<_>>(), ids(heroes.iter().copied()), "{at}");
    assert_eq!(o.normal_tokens, Battle::max_forces(game, supplier, false), "{at}");
    assert_eq!(o.special_tokens, Battle::max_forces(game, supplier, true), "{at}");
    assert_eq!(o.must_pay, Battle::must_pay_for_any_forces_in_battle(game, p), "{at}");
    assert_eq!(o.max_banker_boost, Battle::max_banker_boost(game, p, draft.hero()), "{at}");
    // Without a previous plan, the form starts with no cards (ids 0 are real cards).
    if battle.previous_plan_of(game, game.player(p).faction).is_none() {
        assert!(draft.weapon().is_none() && draft.defense().is_none(), "{at}: default cards");
    }
    // The default draft takes the first leader when one is required, and its cards are valid for it.
    if !game.applicable(Rule::BattleWithoutLeader) && !heroes.is_empty() {
        assert!(draft.hero().is_some(), "{at}: a leader is required");
    }
    assert_eq!(o.weapons, cards_by_name(Battle::valid_weapons(game, p, draft.defense(), draft.hero(), territory, false), d), "{at}");
    assert_eq!(o.defenses, cards_by_name(Battle::valid_defenses(game, p, draft.weapon(), territory, false), d), "{at}");
    if !o.weapons.is_empty() || !o.defenses.is_empty() {
        stats.add("Battle with cards");
    }

    // Mixes: each is Battle.Dial and Battle.Cost of its forces, and they are every affordable one.
    let opponent = game.player(battle.opponent_of(game, p).unwrap()).faction;
    let available = game.player(p).resources + o.ally_cap;
    let mut expected = 0;
    for fs in 0..=o.special_tokens {
        for fn_ in 0..=o.normal_tokens {
            if Battle::cost_game_player_int32_int32_int32(game, p, fn_, fs, &mut 0) <= available {
                expected += (if o.half_special { o.special_tokens - fs } else { 0 } + 1) * (if o.half_normal { o.normal_tokens - fn_ } else { 0 } + 1);
            }
        }
    }
    assert_eq!(o.mixes.len() as i32, expected, "{at}: mixes");
    for &(fs, hs, fn_, hn, dial, cost) in &o.mixes {
        assert_eq!(dial, Battle::dial_game_player_faction_int32_int32_int32_int32(game, p, opponent, fn_, fs, hn, hs), "{at}");
        assert_eq!(cost, Battle::cost_game_player_int32_int32_int32(game, p, fn_, fs, &mut 0), "{at}");
    }
    if o.half_normal || o.half_special {
        stats.add("Battle with half strength");
    }

    // Dependent options: every leader, with every weapon and defense it allows, in a query.
    let mut choices: Vec<Option<Hero>> = heroes.iter().map(|h| Some(*h)).collect();
    choices.push(None);
    for hero in choices {
        let mut q = Battle::new(game, game.player(p).faction);
        q.set_hero(hero);
        let (opts, validation) = battle::options_and_validation(game, &v, &event(q), d).unwrap();
        let Some(ActionOptions::Battle(o)) = opts else { panic!("{at}") };
        let normalized = check_draft(game, game.player(p).faction, &o.draft, GameEventType::Battle, at);
        let GameEvent::Battle(n) = &normalized else { unreachable!() };
        // A hero that isn't valid, or none when one is required, is replaced.
        if hero.is_some() || game.applicable(Rule::BattleWithoutLeader) || heroes.is_empty() {
            assert_eq!(n.hero(), hero, "{at}: the chosen leader stays");
        }
        assert_eq!(validation, treachery_view::build::validate(game, normalized.clone(), d), "{at}");
        assert_eq!(o.weapons, cards_by_name(Battle::valid_weapons(game, p, None, n.hero(), territory, false), d), "{at}");
        for &w in &o.weapons {
            let mut q = Battle::new(game, game.player(p).faction);
            q.set_hero(n.hero());
            q.set_weapon(TreacheryCardManager::lookup().find(w));
            let (opts, _) = battle::options_and_validation(game, &v, &event(q), d).unwrap();
            let Some(ActionOptions::Battle(o)) = opts else { panic!("{at}") };
            assert_eq!(o.defenses, cards_by_name(Battle::valid_defenses(game, p, TreacheryCardManager::lookup().find(w), territory, false), d), "{at}");
            stats.add("Battle weapon query");
        }
        if let Some(l) = &o.leader {
            let h = n.hero().unwrap();
            assert_eq!(l.value, h.value_in_combat_against(None), "{at}");
        }
    }
}

fn applicable_mine(game: &mut Game, p: PlayerIndex) -> Vec<GameEventType> {
    game.get_applicable_events(Some(p), p.index == 0).into_iter().filter(|t| battle::TYPES.contains(t)).collect()
}

fn check_all_players(game: &mut Game, d: &dyn Describer, stats: &mut Stats, at: &str) {
    for p in game.player_indices().collect::<Vec<_>>() {
        let types = applicable_mine(game, p);
        for t in types {
            let g: &Game = game;
            let in_game = InGame::new(d, g);
            check(g, p, t, &in_game, stats, at);
        }
    }
}

#[test]
fn battle_forms_in_fixtures() {
    let d = default_describer();
    let mut stats = Stats::default();
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/fixtures");
    let mut names: Vec<String> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().trim_end_matches(".json").to_string()).collect();
    names.sort();
    for name in names {
        let text = common::fixture(&name);
        let participation = Shared::new(Participation::default());
        let Ok(mut r) = ViewReplica::load(&text, &participation) else { continue };
        check_all_players(&mut r.game, &*d, &mut stats, &name);
    }
    eprintln!("fixtures: {:?}", stats.0);
    for t in ["Battle", "BattleConcluded", "TreacheryCalled", "Voice", "Prescience", "ClairVoyancePlayed", "Retreat", "TakeLosses", "ClairVoyanceAnswered", "FaceDanced"] {
        assert!(stats.0.contains_key(t), "no fixture reaches {t}");
    }
}

#[test]
fn battle_forms_in_bot_games() {
    let d = default_describer();
    let mut stats = Stats::default();
    for path in common::sample_bot_traces() {
        let trace = common::read_trace(&path);
        let participation = Shared::new(Participation::default());
        let mut r = ViewReplica::new(trace.version, &participation);
        for (n, e) in trace.events.iter().enumerate() {
            r.apply(e).unwrap();
            check_all_players(&mut r.game, &*d, &mut stats, &format!("{} step {n}", trace.name));
        }
    }
    eprintln!("bot games: {:?}", stats.0);
    for t in ["Battle", "BattleConcluded", "BattleInitiated", "TreacheryCalled", "LoserConcluded"] {
        assert!(stats.0.contains_key(t), "no bot game reaches {t}");
    }
}

/// A trace's accepted events up to its first undo (fuzz traces undo; bot traces don't).
fn read_until_undo(path: &std::path::Path) -> common::Trace {
    use std::io::{BufRead, BufReader};
    let file = std::fs::File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let reader = BufReader::new(flate2::read::GzDecoder::new(file));
    let mut version = 0;
    let mut events = vec![];
    for line in reader.lines() {
        let v: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        match v["kind"].as_str() {
            Some("header") => version = v["gameVersion"].as_i64().unwrap() as i32,
            Some("step") if v["result"] == "accepted" => events.push(v["event"].to_string()),
            Some("undo" | "restoreRecentlyUndone" | "dismissRecentlyUndone") => break,
            _ => {}
        }
    }
    common::Trace { name: path.file_name().unwrap().to_string_lossy().to_string(), version, events }
}

fn traces_in(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|r| r.map(|e| e.unwrap().path()).filter(|p| p.to_string_lossy().ends_with(".jsonl.gz")).collect())
        .unwrap_or_default();
    files.sort();
    files
}

/// The battle forms no fixture or sample game reaches, in `tests/data/battle`: oracle bot and
/// fuzz games (accepted events up to the first undo), cut off after the state that reaches the
/// form. With `BATTLE_FORMS_CORPUS_DIR`, the traces of that directory instead, to find such games
/// in a fresh oracle run; the summary names the first trace and event that reaches each form.
#[test]
fn battle_forms_in_rare_traces() {
    let d = default_describer();
    let mut stats = Stats::default();
    let scan = std::env::var("BATTLE_FORMS_CORPUS_DIR").ok();
    let dir = match &scan {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/battle"),
    };
    for path in traces_in(&dir) {
        let trace = read_until_undo(&path);
        let participation = Shared::new(Participation::default());
        let mut r = ViewReplica::new(trace.version, &participation);
        for (n, e) in trace.events.iter().enumerate() {
            if r.apply(e).is_err() {
                break;
            }
            check_all_players(&mut r.game, &*d, &mut stats, &format!("{} event {n}", trace.name));
        }
    }
    eprintln!("rare traces: {:?}", stats.0);
    for (form, at) in &stats.1 {
        eprintln!("  {form}: first at {at}");
    }
    if scan.is_some() {
        return;
    }
    let missing: Vec<&str> = RARE.iter().copied().filter(|t| !stats.0.contains_key(*t)).collect();
    assert!(missing.is_empty(), "tests/data/battle reaches no form of {missing:?}");
}

/// The forms only `tests/data/battle` reaches. The traces are `Treachery.Oracle bots --seed 7701
/// --dump none --hash none --gzip true` games, each cut after the event that reaches its last form
/// here. `HMSAdvantageChosen` is covered by `hms_advantage_form_in_a_built_state` instead.
const RARE: &[&str] = &[
    "AuditCancelled",
    "Audited",
    "CardsDetermined",
    "ClairVoyanceAnswered",
    "Diplomacy",
    "FaceDanced",
    "FaceDancerReplaced",
    "JuicePlayed",
    "Planetology",
    "RockWasMelted",
    "TakeLosses",
];

/// A battle in the Hidden Mobile Stronghold fought by its owner, who also owns Arrakeen, so the
/// owner may choose an HMS advantage. Bot and fuzz games almost never reach it (none of 106
/// latest-version fuzz games did), so the state is built from the `battle` fixture: the HMS is
/// pointed at the Great Flat, the first two factions get forces in it, the first owns it and
/// Arrakeen, and the battle between them starts there.
fn hms_battle_state() -> ViewReplica {
    let participation = Shared::new(Participation::default());
    let mut r = ViewReplica::load(&common::fixture("battle"), &participation).expect("battle fixture loads");
    let g = &mut r.game;
    let flat = g.map.the_great_flat();
    let hms = g.map.hidden_mobile_stronghold();
    hms.point_at(&mut g.map, flat);
    let (a, b) = (g.player_at(0), g.player_at(1));
    let (fa, fb) = (g.player(a).faction, g.player(b).faction);
    Player::add_forces(g, a, hms.location(), 3, false);
    Player::add_forces(g, b, hms.location(), 2, false);
    g.stronghold_ownership.insert(hms.location(), fa);
    g.stronghold_ownership.insert(g.map.arrakeen(), fa);
    let mut battle = BattleInitiated::new(g, fa);
    battle.target = fb;
    battle.territory_id = hms.location().territory().id();
    g.current_battle = Some(std::rc::Rc::new(battle));
    g.current_phase = Phase::BattlePhase;
    g.chosen_hms_advantage = Some(StrongholdAdvantage::None);
    r
}

#[test]
fn hms_advantage_form_in_a_built_state() {
    let d = default_describer();
    let mut stats = Stats::default();
    let mut r = hms_battle_state();
    check_all_players(&mut r.game, &*d, &mut stats, "hms battle");
    assert!(stats.0.contains_key("HMSAdvantageChosen"), "{:?}", stats.0);

    // The form's choice is a valid event that the engine executes.
    let owner = r.game.player_at(0);
    let faction = r.game.player(owner).faction;
    let advantage = HMSAdvantageChosen::valid_advantages(&r.game, owner)[0];
    let mut e = HMSAdvantageChosen::new(&r.game, faction);
    e.advantage = advantage;
    let e = std::rc::Rc::new(e).into_event();
    assert!(e.validate(&r.game).is_none(), "{:?}", e.validate(&r.game).map(|m| m.to_cs_string()));
    assert!(e.execute(&mut r.game, true, true).is_none());
    assert_eq!(r.game.chosen_hms_advantage, Some(advantage));
}
