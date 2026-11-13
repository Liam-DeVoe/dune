//! The "other events" forms (`src/actions/other.rs`).
//!
//! For every fixture state and every state of the sample bot and fuzz traces, for every player
//! with one of these forms applicable, the form's options must equal the engine's helpers, its
//! `draft` must be an event of the
//! form's type, and the validation must be that draft's. In the fixture states, querying the
//! form's own draft must return the same options and validation.

mod common;

use std::collections::BTreeMap;
use std::path::PathBuf;

use treachery_engine::shared::Shared;
use treachery_engine::events::GameEventType;
use treachery_engine::json;
use treachery_engine::enum_type::GameEnum;
use treachery_engine::game::Participation;
use treachery_engine::prelude::*;
use treachery_view::actions::{action_view, other};
use treachery_view::build::{validate, ResolvedViewer};
use treachery_view::describe::{default_describer, Describer, InGame};
use treachery_view::replica::ViewReplica;
use treachery_view::schema::*;

fn mine(t: GameEventType) -> bool {
    other::SIMPLE.contains(&t) || other::YES_NO.contains(&t) || other::FORMS.contains(&t)
}

fn factions(fs: impl IntoIterator<Item = Faction>) -> Vec<String> {
    fs.into_iter().map(|f| f.to_string()).collect()
}

fn targets(o: &[FactionOption]) -> Vec<String> {
    for f in o {
        assert_eq!(Faction::from_value(f.value).to_string(), f.faction, "faction value of {}", f.faction);
    }
    o.iter().map(|f| f.faction.clone()).collect()
}

fn ids(cards: impl IntoIterator<Item = TreacheryCard>) -> Vec<i32> {
    cards.into_iter().map(|c| c.id()).collect()
}

fn heroes(o: &[HeroOption]) -> Vec<i32> {
    o.iter().map(|h| h.id).collect()
}

fn names<T: ToString>(o: impl IntoIterator<Item = T>) -> Vec<String> {
    o.into_iter().map(|x| x.to_string()).collect()
}

fn option_names(o: &[EnumOption]) -> Vec<String> {
    o.iter().map(|x| x.name.clone()).collect()
}

/// The options must hold what the engine helpers return for this state.
fn check(game: &Game, p: PlayerIndex, v: &ResolvedViewer, o: &ActionOptions, at: &str) {
    let pl = game.player(p);
    let hand = ids(pl.treachery_cards.iter().copied());
    match o {
        ActionOptions::Simple { confirm_label, .. } => assert!(!confirm_label.is_empty(), "{at}"),
        ActionOptions::YesNo(o) => assert!(!o.title.is_empty(), "{at}"),
        ActionOptions::AllyPermission(o) => {
            assert_eq!(o.ally, pl.ally.to_string(), "{at}");
            assert_eq!(o.max_resources, pl.resources, "{at}");
            assert_eq!(o.current_resources, game.get_permitted_use_of_ally_resources(pl.ally), "{at}");
            assert_eq!(o.karma_cards, Karma::can_karma(game, p).then(|| ids(Karma::valid_karma_cards(game, p))), "{at}");
            assert_eq!(o.current_karma_card, game.get_permitted_use_of_ally_karma(pl.ally).map(|c| c.id()), "{at}");
        }
        ActionOptions::AutomationConfigured(o) => {
            assert_eq!(option_names(&o.rule_types), names(AutomationConfigured::get_available_rule_types(game, p)), "{at}");
            assert_eq!(targets(&o.bidding_factions), factions(AutomationConfigured::get_valid_bidding_factions(game, p)), "{at}");
        }
        ActionOptions::BeneGesseritPrediction(o) => {
            assert_eq!(targets(&o.targets), factions(BeneGesseritPrediction::valid_targets(game, p)), "{at}");
            assert_eq!(o.max_turn, game.maximum_turns(), "{at}");
        }
        ActionOptions::CardTraded(o) => assert_eq!(o.cards, ids(CardTraded::valid_cards(game, p)), "{at}"),
        ActionOptions::ChoamDiscarded(o) => assert_eq!(o.cards, ids(ChoamDiscarded::valid_cards(game, p)), "{at}"),
        ActionOptions::ChoamEconomics(o) => assert_eq!(option_names(&o.statuses), ["Double", "Cancel"], "{at}"),
        ActionOptions::ChoamKarmaPrevention(o) => {
            assert_eq!(targets(&o.targets), factions(game.players_other_than(p)), "{at}");
            assert_eq!(o.card.is_some(), ChoamKarmaPrevention::card_to_use(game, p).is_some(), "{at}");
        }
        ActionOptions::DealOffered(o) => {
            let types: Vec<i32> = DealOffered::get_standard_deal_types(game, p).into_iter().map(|t| t.value()).collect();
            assert_eq!(o.deal_types.iter().map(|t| t.value).collect::<Vec<_>>(), types, "{at}");
            assert_eq!(targets(&o.targets), factions(game.players_other_than(p).filter(|f| *f != pl.ally)), "{at}");
        }
        ActionOptions::Discarded(o) | ActionOptions::EmperorDiscarded(o) | ActionOptions::IxianSwappedCardOnBid(o) => assert_eq!(o.cards, hand, "{at}"),
        ActionOptions::DiscardedSearched(o) => assert_eq!(o.cards, ids(DiscardedSearched::valid_cards(game)), "{at}"),
        ActionOptions::DiscardedTaken(o) => assert_eq!(o.cards, ids(DiscardedTaken::valid_cards(game, p)), "{at}"),
        ActionOptions::DistransUsed(o) => {
            assert_eq!(targets(&o.targets), factions(DistransUsed::valid_targets(game, p)), "{at}");
            assert_eq!(o.cards, ids(DistransUsed::valid_cards(game, p)), "{at}");
        }
        ActionOptions::DivideResources(o) => {
            let tbd = DivideResources::get_resources_to_be_divided(game).expect("something to divide");
            assert_eq!((o.amount, o.territory, o.other_faction.clone()), (tbd.amount, tbd.territory.id(), tbd.other_faction.to_string()), "{at}");
        }
        ActionOptions::Donated(o) => {
            // The default draft does not give from the bank.
            assert_eq!(targets(&o.targets), factions(Donated::valid_targets(game, p)), "{at}");
            assert_eq!((o.min, o.max), (Donated::min_amount(game, p), Donated::max_amount(game, p)), "{at}");
            assert_eq!(o.cards, game.applicable(Rule::CardsCanBeTraded).then(|| hand.clone()), "{at}");
            assert_eq!(o.is_host, v.is_host, "{at}");
        }
        ActionOptions::EmperorBidSupport(o) => {
            assert_eq!(targets(&o.targets), factions(game.players_other_than(p).filter(|f| *f != pl.ally)), "{at}");
            assert_eq!(o.max, pl.resources, "{at}");
        }
        ActionOptions::FactionSelected(o) => assert_eq!(targets(&o.targets), factions(FactionSelected::valid_factions(game)), "{at}"),
        ActionOptions::FactionTradeOffered(o) => assert_eq!(targets(&o.targets), factions(FactionTradeOffered::valid_targets(game, p)), "{at}"),
        ActionOptions::IxianRemovedCardFromAuction(o) => {
            assert_eq!(o.cards, ids(game.cards_on_auction.as_ref().map(|d| d.items.clone()).unwrap_or_default()), "{at}")
        }
        ActionOptions::IxianSelectedStartingCard(o) => {
            assert_eq!(o.cards, ids(game.starting_treachery_cards.as_ref().map(|d| d.items.clone()).unwrap_or_default()), "{at}")
        }
        ActionOptions::Karma(o) => {
            assert_eq!(o.cards, ids(Karma::valid_karma_cards(game, p)), "{at}");
            assert_eq!(option_names(&o.advantages), names(Karma::valid_faction_advantages(game, p)), "{at}");
        }
        ActionOptions::KarmaChoamDiscard(o) => assert_eq!(o.karma_cards, ids(Karma::valid_karma_cards(game, p)), "{at}"),
        ActionOptions::KarmaHandSwap(o) => {
            assert_eq!((o.number, o.target.clone(), o.cards.clone()), (game.karma_hand_swap_number_of_cards, game.karma_hand_swap_target.to_string(), hand), "{at}")
        }
        ActionOptions::KarmaHandSwapInitiated(o) => assert_eq!(targets(&o.targets), factions(game.players_other_than(p)), "{at}"),
        ActionOptions::KarmaMonster(o) => assert_eq!(o.territories, KarmaMonster::valid_targets(game).map(|t| t.id()).collect::<Vec<_>>(), "{at}"),
        ActionOptions::KarmaRicheseBuy(o) => assert_eq!(o.cards, ids(game.richese_cache.iter().copied()), "{at}"),
        ActionOptions::MulliganPerformed(o) => assert_eq!(o.may, MulliganPerformed::may_mulligan(game, p), "{at}"),
        ActionOptions::NexusCardDrawn(o) => assert_eq!(o.may, NexusCardDrawn::may_draw(game, p), "{at}"),
        ActionOptions::NexusPlayed(o) => {
            assert_eq!(o.nexus, pl.nexus.to_string(), "{at}");
            if let Some(a) = &o.aspects {
                assert_eq!(option_names(a), names(Prescience::valid_aspects(game, p)), "{at}");
            }
            if let Some(t) = &o.tleilaxu {
                assert_eq!(t.max_forces, NexusPlayed::valid_tleilaxu_max_amount(game, p, false), "{at}");
                assert_eq!(t.max_special_forces, NexusPlayed::valid_tleilaxu_max_amount(game, p, true), "{at}");
                assert_eq!(heroes(&t.heroes), NexusPlayed::valid_tleilaxu_heroes(game, p).iter().map(|h| h.id()).collect::<Vec<_>>(), "{at}");
            }
            if let Some(c) = &o.choam_cards {
                assert_eq!(c, &ids(NexusPlayed::valid_choam_cards(game, p)), "{at}");
            }
            if let Some(f) = &o.ecaz_factions {
                assert_eq!(targets(f), factions(NexusPlayed::valid_ecaz_factions(game)), "{at}");
            }
        }
        ActionOptions::ReplacedCardWon(_) | ActionOptions::StormSpellPlayed(_) | ActionOptions::RicheseAnnouncesAuction(_) | ActionOptions::RicheseKeepsUnsoldCard(_) => {}
        ActionOptions::ResourcesAudited(o) => assert_eq!(targets(&o.targets), factions(ResourcesAudited::valid_factions(game, p)), "{at}"),
        ActionOptions::ResourcesTransferred(o) => assert_eq!(o.max, ResourcesTransferred::max_amount(game, p), "{at}"),
        ActionOptions::SkillAssigned(o) => {
            assert_eq!(option_names(&o.skills), names(SkillAssigned::valid_skills(game, p)), "{at}");
            let leaders: Vec<i32> = SkillAssigned::valid_leaders(game, p).into_iter().flatten().map(|l| l.id()).collect();
            assert_eq!(heroes(&o.leaders), leaders, "{at}");
        }
        ActionOptions::TraitorDiscarded(o) => {
            assert_eq!(heroes(&o.heroes), TraitorDiscarded::valid_traitors(game, p).iter().map(|h| h.id()).collect::<Vec<_>>(), "{at}")
        }
        ActionOptions::TraitorsSelected(o) => assert_eq!(heroes(&o.heroes), pl.traitors.iter().map(|h| h.id()).collect::<Vec<_>>(), "{at}"),
        ActionOptions::RicheseAnnouncesBlackMarket(o) => assert_eq!(o.cards, ids(RicheseAnnouncesBlackMarket::valid_cards(game, p)), "{at}"),
        ActionOptions::RicheseGaveCard(o) => assert_eq!(o.cards, ids(RicheseGaveCard::valid_cards(game, p)), "{at}"),
        ActionOptions::RicheseSpecifiesAuction(o) => {
            assert_eq!(o.may_specify_card, RicheseSpecifiesAuction::may_specify_card(game, p), "{at}");
            assert_eq!(o.may_specify_type, RicheseSpecifiesAuction::may_specify_type_of_auction(game, p), "{at}");
            assert_eq!(o.cards, ids(RicheseSpecifiesAuction::valid_cards(game)), "{at}");
        }
        other => panic!("{at}: unexpected form {other:?}"),
    }
}

/// The `draft` of the options, when the form has one.
fn draft_of(o: &ActionOptions) -> Option<serde_json::Value> {
    let v = serde_json::to_value(o).unwrap();
    v.get("draft").cloned()
}

/// How often each form was reached, and where first.
#[derive(Default)]
struct Seen(BTreeMap<String, usize>, BTreeMap<String, String>, BTreeMap<String, String>);

impl Seen {
    fn add(&mut self, key: String, at: &str) {
        *self.0.entry(key.clone()).or_default() += 1;
        self.1.entry(key.clone()).or_insert_with(|| at.to_string());
        // `at` starts with "<trace> event <i>".
        let mut words = at.split(' ');
        if let (Some(trace), Some("event"), Some(i)) = (words.next(), words.next(), words.next()) {
            self.2.entry(format!("{key}\t{trace}")).or_insert_with(|| i.to_string());
        }
    }
}

/// Checks every applicable form of this package for every player of `game`.
fn check_state(game: &mut Game, d: &dyn Describer, at: &str, seen: &mut Seen) {
    let players: Vec<PlayerIndex> = game.player_indices().collect();
    for p in players {
        let v = ResolvedViewer { player: Some(p), faction: Some(game.player(p).faction), is_host: p.index == 0, is_observer: false };
        let types = game.get_applicable_events(Some(p), v.is_host);
        let game: &Game = game;
        let in_game = InGame::new(d, game);
        for t in types.into_iter().filter(|t| mine(*t)) {
            let view = action_view(game, &v, t, &in_game);
            let at = format!("{at} {} {}", game.player(p).faction, t.name());
            let options = view.options.as_ref().unwrap_or_else(|| panic!("{at}: no options"));
            check(game, p, &v, options, &at);
            if let Some(draft) = draft_of(options) {
                let e: GameEvent = json::deserialize(&draft.to_string()).unwrap_or_else(|e| panic!("{at}: draft {draft}: {e:?}"));
                assert_eq!(e.event_type(), t, "{at}");
                e.initialize(game, game.player(p).faction);
                assert_eq!(validate(game, e, &in_game), view.validation, "{at}: validation is the draft's");
            }
            if let ActionOptions::NexusPlayed(o) = options {
                seen.add(format!("NexusPlayed/{}", o.nexus), &at);
                seen.add(format!("NexusPlayed/{}/{}", o.nexus, o.kind), &at);
                if let Some(m) = &o.map_member {
                    seen.add(format!("NexusPlayed/{}/{m}", o.nexus), &at);
                }
                if o.tleilaxu.as_ref().is_some_and(|t| t.max_special_forces_on_planet > 0) {
                    seen.add("NexusPlayed/Tleilaxu/TleilaxuLocation".to_string(), &at);
                }
            }
            seen.add(t.name().to_string(), &at);
        }
    }
}

/// A trace's accepted events up to its first undo (fuzz traces undo; bot traces don't).
fn read_until_undo(path: &std::path::Path) -> common::Trace {
    use std::io::{BufRead, BufReader};
    let file = std::fs::File::open(path).unwrap();
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

/// The sample bot and fuzz traces, and `tests/data/other`: short bot and fuzz games cut off
/// after the state that reaches a form nothing else reaches. With `OTHER_FORMS_CORPUS_DIR`, the
/// traces of that directory instead (to find such games in a larger set of recorded games).
fn traces() -> Vec<PathBuf> {
    if let Ok(dir) = std::env::var("OTHER_FORMS_CORPUS_DIR") {
        return traces_in(&PathBuf::from(dir));
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files = common::sample_bot_traces();
    files.extend(traces_in(&root.join("../testdata/games")).into_iter().filter(|p| p.file_name().unwrap().to_string_lossy().starts_with("fuzz-")));
    files.extend(traces_in(&root.join("tests/data/other")));
    files
}

#[test]
fn options_equal_the_engine_helpers_over_the_corpus() {
    let d = default_describer();
    let mut seen = Seen::default();
    for path in traces() {
        let trace = read_until_undo(&path);
        let participation = Shared::new(Participation::default());
        let mut r = ViewReplica::new(trace.version, &participation);
        for (i, e) in trace.events.iter().enumerate() {
            if r.apply(e).is_err() {
                break;
            }
            check_state(&mut r.game, d.as_ref(), &format!("{} event {i}", trace.name), &mut seen);
        }
    }
    println!("{:#?}", seen.0);
    println!("{:#?}", seen.1);
    if std::env::var("OTHER_FORMS_CORPUS_DIR").is_ok() {
        // Every trace's first state per form, as `form<TAB>trace<TAB>event index`, to pick short traces.
        for (key, at) in &seen.2 {
            println!("first\t{key}\t{at}");
        }
        return;
    }
    // The corpus reaches these; a form that stops being reached means the corpus changed.
    // `tests/data/other` holds short games for the rest (recorded bot games, seeds 7701
    // and 7702 and fuzz seeds 7701 and 7703, cut after the state that reaches the form; bots-7701-*
    // are `--versions latest` games for the Ecaz Secret Ally and Tleilaxu Cunning Nexus cards).
    let reached = [
        "AllyPermission", "AutomationConfigured", "Donated", "DealOffered", "Karma", "TraitorsSelected", "NexusPlayed", "SkillAssigned",
        "AllianceByTerror", "AmalPlayed", "BattleClaimed", "Captured", "ChoamExtraMove", "Discarded", "DiscardedSearched",
        "DiscardedSearchedAnnounced", "DiscardedTaken", "DivideResources", "DivideResourcesAccepted", "ExtortionPrevented", "FlightDiscoveryUsed",
        "HarvesterPlayed", "KarmaHandSwap", "KarmaHandSwapInitiated", "KarmaMonster", "KarmaPrescience", "KarmaRicheseBuy", "NexusVoted",
        "PoisonToothCancelled", "PortableAntidoteUsed", "RecruitsPlayed", "ReplacedCardWon", "ResidualPlayed", "RicheseGaveCard", "ThumperPlayed",
        "TraitorDiscarded", "NexusPlayed/Atreides/Betrayal", "NexusPlayed/BeneGesserit/Betrayal", "NexusPlayed/BeneGesserit/Cunning",
        "NexusPlayed/Choam/Betrayal", "NexusPlayed/Ecaz/Secret Ally", "NexusPlayed/Emperor/Betrayal", "NexusPlayed/Fremen/Betrayal",
        "NexusPlayed/Fremen/Secret Ally", "NexusPlayed/Harkonnen/Secret Ally", "NexusPlayed/Moritani/MoritaniTerritory",
        "NexusPlayed/Tleilaxu/Cunning", "NexusPlayed/Tleilaxu/Secret Ally",
    ];
    let missing: Vec<&str> = reached.iter().copied().filter(|t| !seen.0.contains_key(*t)).collect();
    assert!(missing.is_empty(), "the corpus reaches no form of {missing:?}");
}

fn fixtures() -> Vec<String> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/fixtures");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .map(|p| p.file_stem().unwrap().to_string_lossy().to_string())
        .collect();
    names.sort();
    names
}

/// The fixtures, seated: the options of the views, and draft queries of each form's own draft.
#[test]
fn fixture_forms_and_queries() {
    let mut seen = Seen::default();
    for name in fixtures() {
        let participation = Shared::new(Participation::default());
        let Ok(mut r) = ViewReplica::load(&common::fixture(&name), &participation) else { continue };
        drop(participation);
        let mut r2 = common::load(&name, 0);
        let d = default_describer();
        check_state(&mut r.game, d.as_ref(), &name, &mut seen);
        let factions: Vec<Faction> = r2.game.players.iter().map(|p| p.faction).collect();
        for f in factions {
            let viewer = common::viewer_of(&r2, f);
            let view = r2.view(viewer);
            for a in view.actions.iter().filter(|a| GameEventType::ALL.iter().any(|t| t.name() == a.event_type && mine(*t))) {
                let Some(draft) = a.options.as_ref().and_then(draft_of) else { continue };
                let answer = r2.query(viewer, &draft.to_string()).unwrap_or_else(|e| panic!("{name} {f} {}: {e:?}", a.event_type));
                assert_eq!(answer.options, a.options, "{name} {f} {}: query of the form's own draft", a.event_type);
                assert_eq!(answer.validation, a.validation, "{name} {f} {}", a.event_type);
            }
        }
    }
    println!("{:#?}", seen.0);
    for t in ["AllyPermission", "BeneGesseritPrediction", "CharityClaimed", "Discarded", "TraitorsSelected", "KarmaHandSwapInitiated", "NexusPlayed", "KarmaMonster", "NexusPlayed/Moritani/MoritaniTerritory"] {
        assert!(seen.0.contains_key(t), "no fixture reaches {t}");
    }
}

/// Dependent inputs: the forms whose options follow other inputs of the draft.
#[test]
fn dependent_inputs() {
    // Donated: giving from the bank (host only) offers every player and -99..99.
    let mut r = common::load("alliance-allied", 0);
    let host = common::viewer_of(&r, r.game.players[0].faction);
    let view = r.view(host);
    let donated = view.actions.iter().find(|a| a.event_type == "Donated").expect("Donated applicable");
    let mut draft = draft_of(donated.options.as_ref().unwrap()).unwrap();
    draft["FromBank"] = serde_json::json!(true);
    let answer = r.query(host, &draft.to_string()).unwrap();
    let Some(ActionOptions::Donated(o)) = answer.options else { panic!("Donated options") };
    assert_eq!((o.min, o.max), (-99, 99));
    assert_eq!(o.targets.len(), r.game.players.len());
    // A non-host can't give from the bank: the form resets the box.
    let others: Vec<Faction> = r.game.players.iter().skip(1).map(|p| p.faction).collect();
    let viewers: Vec<_> = others.into_iter().map(|f| common::viewer_of(&r, f)).collect();
    let other = viewers
        .into_iter()
        .find(|v| r.view(*v).actions.iter().any(|a| a.event_type == "Donated"))
        .expect("a non-host who may donate");
    let answer = r.query(other, &draft.to_string()).unwrap();
    let Some(ActionOptions::Donated(o)) = answer.options else { panic!("Donated options") };
    assert_eq!(o.draft["FromBank"], serde_json::json!(false));
    assert!(o.min >= 0);

    // A required image choice falls back to the first value; a number picker clamps.
    let mut r = common::load("mc-prediction", 0);
    let bg = common::viewer_of(&r, Faction::BeneGesserit);
    // Queries are answered for the actions of the viewer's latest view.
    let _ = r.view(bg);
    let draft = format!(r#"{{"$type":"BeneGesseritPrediction","Initiator":{},"ToWin":{},"Turn":99}}"#, Faction::BeneGesserit.value(), Faction::BeneGesserit.value());
    let answer = r.query(bg, &draft).unwrap();
    let Some(ActionOptions::BeneGesseritPrediction(o)) = answer.options else { panic!("prediction options") };
    assert_eq!(o.draft["ToWin"], serde_json::json!(o.targets[0].value));
    assert_eq!(o.draft["Turn"], serde_json::json!(r.game.maximum_turns()));
    assert_eq!(answer.validation, None);

    // Map picks: nothing picked is invalid; a picked territory makes the draft valid, and one
    // the form doesn't offer is dropped (MapPicker keeps only targets).
    let mut r = common::load("karma-monster", 0);
    let fremen = common::viewer_of(&r, Faction::Fremen);
    let view = r.view(fremen);
    let a = view.actions.iter().find(|a| a.event_type == "KarmaMonster").expect("KarmaMonster applicable");
    let Some(ActionOptions::KarmaMonster(o)) = &a.options else { panic!("KarmaMonster options") };
    // KarmaMonster's validation accepts anything, even no territory.
    assert_eq!(a.validation, None);
    let mut draft = o.draft.clone();
    draft["_territoryId"] = serde_json::json!(o.territories[0]);
    let answer = r.query(fremen, &draft.to_string()).unwrap();
    assert_eq!(answer.validation, None);
    let Some(ActionOptions::KarmaMonster(o2)) = answer.options else { panic!("KarmaMonster options") };
    assert_eq!(o2.draft["_territoryId"], serde_json::json!(o.territories[0]));
    let mut r = common::load("nexus-moritani", 0);
    let actor = common::viewer_of(&r, Faction::Harkonnen);
    let view = r.view(actor);
    let a = view.actions.iter().find(|a| a.event_type == "NexusPlayed").expect("NexusPlayed applicable");
    let Some(ActionOptions::NexusPlayed(o)) = &a.options else { panic!("NexusPlayed options") };
    assert_eq!(o.map_member.as_deref(), Some("MoritaniTerritory"));
    assert!(a.validation.is_some(), "no stronghold picked yet");
    let territories = o.territories.clone().unwrap();
    let mut draft = o.draft.clone();
    draft["_moritaniTerritoryId"] = serde_json::json!(territories[0]);
    let answer = r.query(actor, &draft.to_string()).unwrap();
    assert_eq!(answer.validation, None);
    let Some(ActionOptions::NexusPlayed(o)) = answer.options else { panic!("NexusPlayed options") };
    assert_eq!(o.draft["_moritaniTerritoryId"], serde_json::json!(territories[0]));
    let not_offered = r.game.map.territories(false).map(|t| t.id()).find(|t| !territories.contains(t)).unwrap();
    draft["_moritaniTerritoryId"] = serde_json::json!(not_offered);
    let answer = r.query(actor, &draft.to_string()).unwrap();
    assert!(answer.validation.is_some());
}
