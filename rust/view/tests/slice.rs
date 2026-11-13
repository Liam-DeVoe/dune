//! The first vertical slice: views of the alliance and storm phases from real savegames
//! (`scripts/fixtures`), checked against the engine's helpers, plus the visibility rules every
//! view must keep.

mod common;

use common::{load, load_text, truncated, viewer_of, Viewer, OBSERVER};
use treachery_engine::events::GameEventType;
use treachery_engine::identity::IIdentifiable;
use treachery_engine::prelude::*;
use treachery_view::actions::QueryError;
use treachery_view::schema::*;

fn factions(r: &treachery_view::replica::ViewReplica) -> Vec<Faction> {
    r.game.players.iter().map(|p| p.faction).collect()
}

fn action<'a>(view: &'a GameView, t: &str) -> Option<&'a ActionView> {
    view.actions.iter().find(|a| a.event_type == t)
}

fn text(s: &str) -> RichText {
    vec![Part::Text { v: s.to_string() }]
}

/// The `AllianceOffered` form: the targets are `AllianceOffered::valid_targets`, and
/// the form's default (no faction picked) fails validation, which disables "Offer Alliance".
#[test]
fn alliance_offer_targets_are_the_engines() {
    let mut r = load("alliance", 0);
    for f in factions(&r) {
        let view = r.view(viewer_of(&r, f));
        assert_eq!(view.phase, "AllianceA");
        let me = r.game.get_player(Some(f)).unwrap();
        let expected: Vec<String> = AllianceOffered::valid_targets(&r.game, me).map(|t| t.to_string()).collect();
        let offer = action(&view, "AllianceOffered").expect("AllianceOffered applicable");
        assert_eq!(offer.options, Some(ActionOptions::AllianceOffered { targets: expected.clone() }));
        assert!(!expected.contains(&f.to_string()), "a faction can't ally with itself");
        // The required image choice sets its value to the first target, so the default offer
        // goes to that faction and is valid.
        assert_eq!(offer.validation, None);
        // Without an ally, AlliancePassed asks "Done making alliances?".
        let pass = action(&view, "AlliancePassed").expect("AlliancePassed applicable");
        assert_eq!(
            pass.options,
            Some(ActionOptions::Simple { title: Some(text("Done making alliances?")), confirm_label: "Pass".into(), alert: true, allow_auto_confirm: false })
        );
        assert_eq!(pass.validation, None);
        assert!(action(&view, "AllianceBroken").is_none());
    }
}

/// With an ally: break or keep, and the keep question names the ally.
#[test]
fn allied_players_may_break_or_keep() {
    let mut r = load("alliance-allied", 0);
    for f in factions(&r) {
        let view = r.view(viewer_of(&r, f));
        let ally = r.game.player(r.game.get_player(Some(f)).unwrap()).ally;
        assert_ne!(ally, Faction::None);
        assert!(action(&view, "AllianceOffered").is_none());
        let keep = action(&view, "AlliancePassed").unwrap();
        assert_eq!(
            keep.options,
            Some(ActionOptions::Simple {
                title: Some(vec![
                    Part::Text { v: "Keep your alliance with ".into() },
                    Part::Faction { v: ally.to_string(), label: treachery_skin::default_skin().describe_faction(ally) },
                    Part::Text { v: "?".into() }
                ]),
                confirm_label: "Keep alliance".into(),
                alert: true,
                allow_auto_confirm: false
            })
        );
        assert!(action(&view, "AllianceBroken").is_some());
        assert_eq!(view.players.iter().find(|p| p.faction == f.to_string()).unwrap().ally, ally.to_string());
    }
}

/// Everyone is awaited in the alliance phase; observers get the waiting text and no actions.
#[test]
fn status_and_observer() {
    let mut r = load("alliance", 0);
    let view = r.view(OBSERVER);
    assert!(view.viewer.is_observer);
    assert!(view.me.is_none());
    assert!(view.actions.is_empty());
    assert!(!view.status.waiting_for_me);
    assert_eq!(view.status.waiting_for.len(), r.game.players.len());
    assert_eq!(view.status.message, text("Factions are making and breaking alliances..."));

    let emperor = viewer_of(&r, Faction::Emperor);
    let view = r.view(emperor);
    assert!(view.status.waiting_for_me);
}

/// The hidden-information rules every view keeps. `me` is the viewer's own shield and nobody
/// else's; other players show only counts; private log lines reach only their target.
#[test]
fn views_hold_no_one_elses_secrets() {
    for fixture in ["alliance", "alliance-allied", "battle", "bid", "mc-traitors", "mc-prediction", "revival"] {
        let mut r = load(fixture, 0);
        let mut viewers: Vec<(Viewer, Option<Faction>)> = factions(&r).into_iter().map(|f| (viewer_of(&r, f), Some(f))).collect();
        viewers.push((OBSERVER, None));
        for (viewer, faction) in viewers {
            let view = r.view(viewer);
            let json = serde_json::to_value(&view).unwrap();
            match faction {
                None => assert!(view.me.is_none(), "{fixture}: observer sees a shield"),
                Some(f) => {
                    let me = view.me.as_ref().unwrap();
                    let p = r.game.player(r.game.get_player(Some(f)).unwrap());
                    assert_eq!(me.faction, f.to_string());
                    assert_eq!(me.hand.iter().map(|c| c.id).collect::<Vec<_>>(), p.treachery_cards.iter().map(|c| c.id()).collect::<Vec<_>>());
                    // PlayerPanel.StillPickingTraitor hides the dealt candidates until the pick.
                    let picking = r.game.current_phase == Phase::SelectingTraitors
                        && f != Faction::Harkonnen
                        && f != Faction::Tleilaxu
                        && !r.game.has_acted_or_passed.contains(&f);
                    assert_eq!(me.traitors.len(), if picking { 0 } else { p.traitors.len() }, "{fixture}");
                    if f != Faction::BeneGesserit {
                        assert!(me.predicted_faction.is_none(), "{fixture}: only Bene Gesserit has a prediction");
                    }
                }
            }
            // Other players: counts only. The public player record has no field that names a card,
            // a traitor or an amount of spice behind the shield.
            for p in json["players"].as_array().unwrap() {
                for secret in ["resources", "hand", "traitors", "faceDancers", "predictedFaction", "nexus"] {
                    assert!(p.get(secret).is_none(), "{fixture}: public player has {secret}");
                }
            }
            for report in &view.log.latest {
                for line in &report.lines {
                    assert!(!line.private || faction.is_some(), "{fixture}: observer reads a private line");
                }
            }
        }
    }
}

/// Private log lines are those whose `Message.Target` is the viewer, and only those. Checked
/// over every report of a few games (the history request renders older reports the same way).
#[test]
fn private_log_lines_reach_only_their_target() {
    let mut private_total = 0;
    for fixture in ["mc-traitors", "bid", "battle", "alliance"] {
        let r = load(fixture, 0);
        for f in factions(&r) {
            let v = viewer_of(&r, f).resolve(&r.game);
            for (i, report) in r.game.reports().iter().enumerate() {
                let lines = treachery_view::build::report_lines(i, report, &v, &treachery_skin::default_skin()).lines;
                assert_eq!(lines.len(), report.messages.iter().filter(|m| m.target == Faction::None || m.target == f).count());
                let private = report.messages.iter().filter(|m| m.target == f).count();
                assert_eq!(lines.iter().filter(|l| l.private).count(), private);
                private_total += private;
            }
        }
        let o = OBSERVER.resolve(&r.game);
        for (i, report) in r.game.reports().iter().enumerate() {
            assert!(treachery_view::build::report_lines(i, report, &o, &treachery_skin::default_skin()).lines.iter().all(|l| !l.private));
        }
    }
    assert!(private_total > 0, "the fixtures have private lines to check");
}

/// The `StormDialled` form: a number between `StormDialled::valid_min_amount` and `valid_max_amount`,
/// starting at the minimum (the number picker clamps its initial 0).
#[test]
fn storm_dial_range() {
    // The alliance fixture's last storm: events 606 and 607 are the two dials.
    let mut r = load_text(&truncated("alliance", 606), 0);
    let mut dialers = 0;
    for f in factions(&r) {
        let view = r.view(viewer_of(&r, f));
        assert_eq!(view.phase, "DiallingStorm");
        if let Some(dial) = action(&view, "StormDialled") {
            dialers += 1;
            let (min, max) = (StormDialled::valid_min_amount(&r.game), StormDialled::valid_max_amount(&r.game));
            assert!(min <= max);
            assert_eq!(dial.options, Some(ActionOptions::StormDialled { min, max }));
            assert_eq!(dial.validation, None, "the minimum is a valid dial");
            assert!(view.status.waiting_for_me);
        } else {
            assert!(!view.status.waiting_for_me);
        }
    }
    assert_eq!(dialers, 2);

    // After one dial, only the other dialer is awaited.
    let mut r = load_text(&truncated("alliance", 607), 0);
    let waiting: Vec<String> = r.view(OBSERVER).status.waiting_for;
    assert_eq!(waiting.len(), 1);
    let dialed = r.game.history.last().unwrap().initiator().to_string();
    assert!(!waiting.contains(&dialed));
}

/// Draft queries: the client sends the form's inputs as an event in savegame JSON, and gets the
/// options and validation for them. The server sets the initiator.
#[test]
fn draft_queries() {
    let mut r = load("alliance", 0);
    let emperor = viewer_of(&r, Faction::Emperor);
    let _ = r.view(emperor);
    let ev = |t: &str, extra: &str| format!(r#"{{"$type":"{t}"{extra},"Initiator":0,"Time":"2026-01-01T00:00:00+00:00"}}"#);

    let ok = r.query(emperor, &ev("AllianceOffered", &format!(r#","Target":{}"#, Faction::BeneGesserit.0))).unwrap();
    assert_eq!(ok.event_type, "AllianceOffered");
    assert_eq!(ok.validation, None);
    assert!(matches!(ok.options, Some(ActionOptions::AllianceOffered { .. })));

    let own = r.query(emperor, &ev("AllianceOffered", &format!(r#","Target":{}"#, Faction::Emperor.0))).unwrap();
    assert_eq!(own.validation, Some(text("Invalid faction")));

    assert_eq!(r.query(emperor, &ev("StormDialled", r#","Amount":3"#)), Err(QueryError::NotApplicable));
    assert_eq!(r.query(OBSERVER, &ev("AllianceOffered", "")), Err(QueryError::NotAPlayer));
    assert!(matches!(r.query(emperor, "{not json"), Err(QueryError::BadJson(_))));
    assert!(GameEventType::ALL.contains(&GameEventType::AllianceOffered));
}

/// Applying an accepted event to the replica moves every view on, and the event number follows
/// `History.Count`.
#[test]
fn replica_follows_events() {
    let mut r = load("alliance", 0);
    let before = r.view(OBSERVER).event_number;
    let emperor = Faction::Emperor.0;
    r.apply(&format!(r#"{{"$type":"AlliancePassed","Initiator":{emperor},"Time":"2026-01-01T00:00:00+00:00"}}"#)).unwrap();
    let view = r.view(OBSERVER);
    assert_eq!(view.event_number, before + 1);
    assert!(!view.status.waiting_for.contains(&"Emperor".to_string()));
    let emperor_view = r.view(viewer_of(&r, Faction::Emperor));
    assert!(action(&emperor_view, "AllianceOffered").is_none());
}

/// `ViewReplica::scene` seats one user for the `/scene` route: host, named "Scene", and every
/// other seat without a user (a bot).
#[test]
fn scene_matches_the_csharp_scene_page() {
    let (mut r, viewer) = treachery_view::replica::ViewReplica::scene(&common::fixture("alliance"), Faction::Atreides).unwrap();
    let view = r.view(viewer);
    assert_eq!(view.viewer.faction.as_deref(), Some("Atreides"));
    assert!(view.viewer.is_host && !view.viewer.is_observer);
    for p in &view.players {
        assert_eq!(p.is_bot, p.faction != "Atreides", "{}", p.faction);
        if p.faction == "Atreides" {
            assert_eq!(p.name, "Scene");
        }
    }
    assert!(treachery_view::replica::ViewReplica::scene(&common::fixture("alliance"), Faction::Ecaz).is_err());
}

/// One replica serves every viewer (client.md, The view replica), so views must not depend on
/// the order in which viewers' views are built.
#[test]
fn views_do_not_depend_on_build_order() {
    for fixture in ["battle", "mc-voice", "retreat", "mc-concluded", "alliance", "bid", "move"] {
        let mut forward = load(fixture, 0);
        let mut backward = load(fixture, 0);
        let mut viewers: Vec<Viewer> = factions(&forward).into_iter().map(|f| viewer_of(&forward, f)).collect();
        viewers.push(OBSERVER);
        let a: Vec<String> = viewers.iter().map(|v| serde_json::to_string(&forward.view(*v)).unwrap()).collect();
        let mut b: Vec<String> = viewers.iter().rev().map(|v| serde_json::to_string(&backward.view(*v)).unwrap()).collect();
        b.reverse();
        assert_eq!(a, b, "{fixture}");
        // And building a view again in the same state gives the same view.
        let again: Vec<String> = viewers.iter().map(|v| serde_json::to_string(&forward.view(*v)).unwrap()).collect();
        assert_eq!(a, again, "{fixture}");
    }
}
