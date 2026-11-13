//! The revival forms (`src/actions/revival.rs`): for every fixture state and every bot-game
//! state where one of the eight revival events is applicable, the form's options equal the
//! engine's helpers, the validation is the engine's for the adjusted
//! draft, and draft queries recompute the dependent options.

mod common;

use common::{load, read_trace, sample_bot_traces, viewer_of};
use treachery_engine::shared::Shared;
use treachery_engine::events::GameEventType;
use treachery_engine::json;
use treachery_engine::compat::net_enum::NetEnum;
use treachery_engine::game::Participation;
use treachery_engine::prelude::*;
use treachery_view::actions::{self, revival};
use treachery_view::build::{validate, ResolvedViewer};
use treachery_view::describe::{default_describer, Describer};
use treachery_view::replica::ViewReplica;
use treachery_view::schema::*;
use treachery_view::shield::hero_ref;

fn heroes(hs: impl IntoIterator<Item = Hero>) -> Vec<HeroOption> {
    hs.into_iter().map(|h| HeroOption { id: h.id(), hero: hero_ref(h) }).collect()
}

fn factions(fs: impl IntoIterator<Item = Faction>) -> Vec<FactionOption> {
    fs.into_iter().map(|f| FactionOption { faction: f.to_string(), value: f.value() }).collect()
}

fn draft_of(options: &ActionOptions) -> &serde_json::Value {
    match options {
        ActionOptions::Revival(o) => &o.draft,
        ActionOptions::RaiseDeadPlayed(o) => &o.draft,
        ActionOptions::KarmaFreeRevival(o) => &o.draft,
        ActionOptions::KarmaRevivalPrevention(o) => &o.draft,
        ActionOptions::ChoamFreeRevivalPrevention(o) => &o.draft,
        ActionOptions::RequestTleilaxuRevival(o) => &o.draft,
        ActionOptions::AcceptOrCancelTleilaxuRevival(o) => &o.draft,
        ActionOptions::SetIncreasedRevivalLimits(o) => &o.draft,
        other => panic!("not a revival form: {other:?}"),
    }
}

fn event_of(v: &serde_json::Value, game: &Game) -> GameEvent {
    let e: GameEvent = json::deserialize(&v.to_string()).expect("the draft is an event");
    e.initialize_game(game);
    e
}

fn int(v: &serde_json::Value, k: &str) -> i32 {
    v[k].as_i64().unwrap_or_else(|| panic!("{k} in {v}")) as i32
}

fn hero_of(v: &serde_json::Value) -> Option<Hero> {
    LeaderManager::hero_lookup().find(int(v, "_heroId"))
}

/// Checks one form against the engine: the options are the helpers' results for the form's
/// draft, the draft is within the ranges the component allows, and the validation is the
/// engine's for that draft.
fn check(game: &Game, p: PlayerIndex, options: &ActionOptions, validation: &Option<RichText>, d: &dyn Describer, at: &str) {
    let pl = game.player(p);
    let draft = draft_of(options);
    let e = event_of(draft, game);
    assert_eq!(*validation, validate(game, e.clone(), d), "{at}: validation");
    match options {
        ActionOptions::Revival(o) => {
            let sa = draft["UsesEmperorSecretAlly"].as_bool().unwrap();
            assert_eq!(o.may_use_secret_ally, pl.nexus == Faction::Emperor && NexusPlayed::can_use_secret_ally(game, p), "{at}");
            assert!(o.may_use_secret_ally || !sa, "{at}: secret ally without the checkbox");
            assert_eq!(o.max_forces, Revival::valid_max_revivals(game, p, false, sa), "{at}");
            assert_eq!(o.max_special_forces, Revival::valid_max_revivals(game, p, true, sa), "{at}");
            assert_eq!(o.max_by_emperor, Revival::valid_max_revivals_by_emperor(game, p), "{at}");
            let locations: Vec<i32> = Revival::valid_revived_force_locations(game, p).map(|l| l.id()).collect();
            assert_eq!(o.locations, locations, "{at}");
            let (f, sf) = (int(draft, "AmountOfForces"), int(draft, "AmountOfSpecialForces"));
            let (ef, esf) = (int(draft, "ExtraForcesPaidByEmperor"), int(draft, "ExtraSpecialForcesPaidByEmperor"));
            let on_planet = if locations.is_empty() { 0 } else { Revival::number_of_forces_that_may_be_placed_on_planet(game, p, sa, f + ef) };
            let special_on_planet = if locations.is_empty() { 0 } else { Revival::number_of_special_forces_that_may_be_placed_on_planet(game, p, sf + esf) };
            assert_eq!((o.max_forces_on_planet, o.max_special_forces_on_planet), (on_planet, special_on_planet), "{at}");
            assert_eq!(o.heroes, heroes(Revival::valid_revival_heroes(game, p)), "{at}");
            let hero = hero_of(draft);
            assert_eq!(o.may_assign_skill, hero.is_some_and(|h| Revival::may_assign_skill(game, p, Some(h))), "{at}");
            let cost = Revival::determine_cost(game, p, hero, f, sf, ef, esf, sa);
            assert_eq!(o.cost_for_player, cost.total_cost_for_player(), "{at}");
            assert_eq!(o.cost_for_emperor, (pl.ally == Faction::Emperor).then_some(cost.cost_for_emperor), "{at}");
            // The draft is what the component's widgets allow.
            assert!((0..=o.max_forces.max(0)).contains(&f) && (0..=o.max_special_forces.max(0)).contains(&sf), "{at}: {draft}");
            assert!((0..=o.max_by_emperor.max(0)).contains(&ef) && (0..=o.max_by_emperor.max(0)).contains(&esf), "{at}: {draft}");
            assert!(int(draft, "NumberOfForcesInLocation") <= on_planet.max(0) && int(draft, "NumberOfSpecialForcesInLocation") <= special_on_planet.max(0), "{at}");
            assert!(o.may_assign_skill || !draft["AssignSkill"].as_bool().unwrap(), "{at}");
            let loc = int(draft, "_locationId");
            assert!(loc == -1 || locations.contains(&loc), "{at}");
        }
        ActionOptions::RaiseDeadPlayed(o) => {
            assert_eq!(o.card_name, d.describe_in(&TreacheryCardType::RaiseDead.into(), None));
            assert_eq!(o.max_forces, RaiseDeadPlayed::valid_max_amount(game, p, false), "{at}");
            assert_eq!(o.max_special_forces, RaiseDeadPlayed::valid_max_amount(game, p, true), "{at}");
            let locations: Vec<i32> = Revival::valid_revived_force_locations(game, p).map(|l| l.id()).collect();
            assert_eq!(o.locations, locations, "{at}");
            let sf = int(draft, "AmountOfSpecialForces");
            let special_on_planet = if locations.is_empty() { 0 } else { Revival::number_of_special_forces_that_may_be_placed_on_planet(game, p, sf) };
            assert_eq!(o.max_special_forces_on_planet, special_on_planet, "{at}");
            assert_eq!(o.heroes, heroes(RaiseDeadPlayed::valid_heroes(game, p)), "{at}");
            let hero = hero_of(draft);
            assert_eq!(o.may_assign_skill, hero.is_some_and(|h| Revival::may_assign_skill(game, p, Some(h))), "{at}");
            assert!((0..=o.max_forces.max(0)).contains(&int(draft, "AmountOfForces")) && (0..=o.max_special_forces.max(0)).contains(&sf), "{at}");
        }
        ActionOptions::KarmaFreeRevival(o) => {
            assert_eq!(o.card_name, d.describe_in(&TreacheryCardType::Karma.into(), None));
            assert_eq!(o.max_forces, KarmaFreeRevival::valid_max_amount(game, p, false), "{at}");
            assert_eq!(o.max_special_forces, KarmaFreeRevival::valid_max_amount(game, p, true), "{at}");
            assert_eq!(o.heroes, heroes(game.killed_heroes(p)), "{at}");
            let hero = hero_of(draft);
            assert_eq!(o.may_assign_skill, hero.is_some_and(|h| Revival::may_assign_skill(game, p, Some(h))), "{at}");
        }
        ActionOptions::KarmaRevivalPrevention(o) => {
            let targets: Vec<Faction> = KarmaRevivalPrevention::get_valid_targets(game, p).collect();
            assert_eq!(o.targets, factions(targets.clone()), "{at}");
            let t = Faction::from_value(int(draft, "Target"));
            assert_eq!(t, targets.first().copied().filter(|_| !targets.contains(&t)).unwrap_or(t), "{at}: required choice");
        }
        ActionOptions::ChoamFreeRevivalPrevention(o) => {
            let targets: Vec<Faction> = game.players_other_than(p).collect();
            assert_eq!(o.targets, factions(targets.clone()), "{at}");
            assert_eq!(o.card.is_some(), ChoamFreeRevivalPrevention::card_to_use(game, p).is_some(), "{at}");
            assert!(targets.is_empty() || targets.contains(&Faction::from_value(int(draft, "Target"))), "{at}");
        }
        ActionOptions::RequestTleilaxuRevival(o) => {
            let targets: Vec<Hero> = RequestTleilaxuRevival::valid_targets(game, p).collect();
            assert_eq!(o.heroes, heroes(targets.clone()), "{at}");
            assert_eq!(hero_of(draft), targets.first().copied().filter(|_| !hero_of(draft).is_some_and(|h| targets.contains(&h))).or(hero_of(draft)), "{at}");
        }
        ActionOptions::AcceptOrCancelTleilaxuRevival(o) => {
            let requests: Vec<Hero> = game.current_revival_requests.iter().filter_map(|r| r.hero()).collect();
            assert_eq!(o.requests, heroes(requests.clone()), "{at}");
            let offers: Vec<(i32, Option<i32>)> = game.early_revivals_offers.iter().map(|(h, v)| (h.id(), (*v != i32::MAX).then_some(*v))).collect();
            assert_eq!(o.offers.iter().map(|x| (x.hero.id, x.price)).collect::<Vec<_>>(), offers, "{at}");
            assert_eq!(o.bot_awaits, game.current_revival_requests.iter().any(|r| game.is_bot(r.player())), "{at}");
            if !requests.is_empty() {
                assert!(hero_of(draft).is_some_and(|h| requests.contains(&h)), "{at}: a request is picked");
                assert!((0..=50).contains(&int(draft, "Price")), "{at}");
            }
            assert_eq!(draft["Cancel"], serde_json::json!(false));
        }
        ActionOptions::SetIncreasedRevivalLimits(o) => {
            assert_eq!(o.targets, factions(SetIncreasedRevivalLimits::valid_targets(game, p)), "{at}");
        }
        other => panic!("{at}: not a revival form: {other:?}"),
    }
}

fn resolved(game: &Game, p: PlayerIndex) -> ResolvedViewer {
    ResolvedViewer { player: Some(p), faction: Some(game.player(p).faction), is_host: false, is_observer: false }
}

/// Drafts a player could make from the default one: each input moved through its range.
fn variations(default: &serde_json::Value, options: &ActionOptions) -> Vec<serde_json::Value> {
    let mut out = vec![];
    let mut with = |changes: &[(&str, serde_json::Value)]| {
        let mut v = default.clone();
        for (k, x) in changes {
            v[*k] = x.clone();
        }
        out.push(v);
    };
    match options {
        ActionOptions::Revival(o) => {
            for sa in [false, true] {
                for f in [0, 1, o.max_forces, o.max_forces + 3] {
                    for sf in [0, o.max_special_forces] {
                        with(&[
                            ("UsesEmperorSecretAlly", sa.into()),
                            ("AmountOfForces", f.into()),
                            ("AmountOfSpecialForces", sf.into()),
                            ("ExtraForcesPaidByEmperor", o.max_by_emperor.into()),
                            ("NumberOfForcesInLocation", 20.into()),
                            ("NumberOfSpecialForcesInLocation", 20.into()),
                            ("_locationId", o.locations.first().copied().unwrap_or(-1).into()),
                        ]);
                    }
                }
            }
            for h in &o.heroes {
                with(&[("_heroId", h.id.into()), ("AssignSkill", true.into())]);
            }
        }
        ActionOptions::RaiseDeadPlayed(o) => {
            for f in [0, o.max_forces, o.max_forces + 1] {
                for sf in [0, o.max_special_forces] {
                    with(&[
                        ("AmountOfForces", f.into()),
                        ("AmountOfSpecialForces", sf.into()),
                        ("NumberOfSpecialForcesInLocation", 5.into()),
                        ("_locationId", o.locations.first().copied().unwrap_or(-1).into()),
                    ]);
                }
            }
            for h in &o.heroes {
                with(&[("_heroId", h.id.into()), ("AssignSkill", true.into())]);
            }
        }
        ActionOptions::KarmaFreeRevival(o) => {
            with(&[("AmountOfForces", (o.max_forces + 1).into()), ("AmountOfSpecialForces", o.max_special_forces.into())]);
            for h in &o.heroes {
                with(&[("_heroId", h.id.into()), ("AssignSkill", true.into())]);
            }
        }
        ActionOptions::KarmaRevivalPrevention(o) => o.targets.iter().for_each(|t| with(&[("Target", t.value.into())])),
        ActionOptions::ChoamFreeRevivalPrevention(o) => o.targets.iter().for_each(|t| with(&[("Target", t.value.into())])),
        ActionOptions::RequestTleilaxuRevival(o) => o.heroes.iter().for_each(|h| with(&[("_heroId", h.id.into())])),
        ActionOptions::AcceptOrCancelTleilaxuRevival(o) => {
            for h in &o.requests {
                for price in [-1, 3, 51] {
                    with(&[("_heroId", h.id.into()), ("Price", price.into()), ("Cancel", true.into())]);
                }
            }
        }
        ActionOptions::SetIncreasedRevivalLimits(o) => {
            with(&[("Factions", serde_json::json!(o.targets.iter().map(|t| t.value).collect::<Vec<_>>()))]);
            with(&[("Factions", serde_json::json!([]))]);
        }
        _ => {}
    }
    out
}

/// Checks every applicable revival form of player `p` in the current state, its default and
/// the drafts in [`variations`]. Returns the event types checked.
fn check_state(game: &Game, p: PlayerIndex, types: &[GameEventType], d: &dyn Describer, at: &str) -> Vec<GameEventType> {
    let v = resolved(game, p);
    let mut checked = vec![];
    for t in types.iter().filter(|t| revival::TYPES.contains(t)) {
        let a = actions::action_view(game, &v, *t, d);
        let options = a.options.as_ref().unwrap_or_else(|| panic!("{at}: {t:?} has options"));
        check(game, p, options, &a.validation, d, &format!("{at} {} default", t.name()));
        // Adjusting the adjusted draft changes nothing.
        let again = revival::options_and_validation(game, &v, &event_of(draft_of(options), game), d).unwrap();
        assert_eq!(again, (a.options.clone(), a.validation.clone()), "{at} {}: normalizing is stable", t.name());
        for draft in variations(draft_of(options), options) {
            let (o, val) = revival::options_and_validation(game, &v, &event_of(&draft, game), d).unwrap();
            check(game, p, o.as_ref().unwrap(), &val, d, &format!("{at} {} draft {draft}", t.name()));
        }
        checked.push(*t);
    }
    checked
}

/// Every fixture, every faction: the views' revival forms, and the draft-query path through the
/// replica for one varied draft per form.
#[test]
fn fixture_forms_match_the_engine() {
    let d = default_describer();
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/fixtures");
    let mut names: Vec<String> = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()?.file_name().to_str()?.strip_suffix(".json").map(str::to_string)).collect();
    names.sort();
    let mut seen = std::collections::BTreeSet::new();
    for name in names {
        // Skip a fixture the engine can't load.
        let Ok(mut r) = std::panic::catch_unwind(|| load(&name, 0)) else { continue };
        let fs: Vec<Faction> = r.game.players.iter().map(|p| p.faction).collect();
        for f in fs {
            let viewer = viewer_of(&r, f);
            let view = r.view(viewer);
            let p = r.game.get_player(Some(f)).unwrap();
            let types: Vec<GameEventType> = view.actions.iter().filter_map(|a| GameEventType::ALL.iter().copied().find(|t| t.name() == a.event_type)).collect();
            for a in view.actions.iter().filter(|a| revival::TYPES.iter().any(|t| t.name() == a.event_type)) {
                let at = format!("{name}/{f} {}", a.event_type);
                check(&r.game, p, a.options.as_ref().unwrap(), &a.validation, d.as_ref(), &at);
                // The server's query path gives the same answer as the builder.
                let draft = draft_of(a.options.as_ref().unwrap()).to_string();
                let q = r.query(viewer, &draft).unwrap();
                assert_eq!((q.options, q.validation), (a.options.clone(), a.validation.clone()), "{at}: query of the default draft");
                seen.insert(a.event_type.clone());
            }
            check_state(&r.game, p, &types, d.as_ref(), &format!("{name}/{f}"));
        }
    }
    assert!(seen.contains("Revival"), "the revival fixture offers Revival: {seen:?}");
}

/// The `revival` fixture: Revival's dependent options follow the draft. More forces revived
/// cost more, and asking for more than the maximum is clamped to it.
#[test]
fn revival_draft_queries() {
    let mut r = load("revival", 0);
    let fs: Vec<Faction> = r.game.players.iter().map(|p| p.faction).collect();
    let mut any = false;
    for f in fs {
        let viewer = viewer_of(&r, f);
        let view = r.view(viewer);
        let Some(a) = view.actions.iter().find(|a| a.event_type == "Revival") else { continue };
        let Some(ActionOptions::Revival(o)) = &a.options else { panic!("Revival options") };
        assert_eq!(int(&o.draft, "AmountOfForces"), 0);
        assert_eq!(int(&o.draft, "_heroId"), -1, "no hero picked by default");
        if o.max_forces == 0 {
            continue;
        }
        any = true;
        let mut draft = o.draft.clone();
        draft["AmountOfForces"] = (o.max_forces + 5).into();
        let q = r.query(viewer, &draft.to_string()).unwrap();
        let Some(ActionOptions::Revival(q)) = q.options else { panic!("Revival options") };
        assert_eq!(int(&q.draft, "AmountOfForces"), o.max_forces, "{f}: clamped to the maximum");
        let p = r.game.get_player(Some(f)).unwrap();
        assert_eq!(q.cost_for_player, Revival::determine_cost(&r.game, p, None, o.max_forces, 0, 0, 0, false).total_cost_for_player());
    }
    assert!(any, "some faction in the revival fixture can revive forces");
}

/// Every state of every sample bot game: the revival forms of every player for whom one is
/// applicable.
#[test]
fn bot_game_forms_match_the_engine() {
    let d = default_describer();
    let mut seen = std::collections::BTreeMap::<&str, usize>::new();
    for path in sample_bot_traces() {
        let trace = read_trace(&path);
        let participation = Shared::new(Participation::default());
        let mut r = ViewReplica::new(trace.version, &participation);
        for (i, e) in trace.events.iter().enumerate() {
            r.apply(e).unwrap();
            let fs: Vec<Faction> = r.game.players.iter().map(|p| p.faction).collect();
            for f in fs {
                let p = r.game.get_player(Some(f)).unwrap();
                let types = r.game.get_applicable_events(Some(p), false);
                if !types.iter().any(|t| revival::TYPES.contains(t)) {
                    continue;
                }
                for t in check_state(&r.game, p, &types, d.as_ref(), &format!("{} event {i} {f}", trace.name)) {
                    *seen.entry(t.name()).or_default() += 1;
                }
            }
        }
    }
    eprintln!("revival forms checked in bot games: {seen:?}");
    assert!(seen.contains_key("Revival"));
}
