//! The `ending` field (package 9): `WinnerPanel` and the story, with `HideSecrets` applied.
//!
//! `mc-ended` is a finished two-turn game. `mc-ended-hidden` is the same game with Guild and
//! Bene Gesserit hiding their secrets afterwards (HideSecrets events appended by hand; the event
//! is applicable only up to version 123).

mod common;

use common::*;
use treachery_engine::prelude::*;
use treachery_view::ending::{EndingView, StoryBlock, StoryRow};

fn endings(name: &str) -> Vec<EndingView> {
    let mut r = load(name, 0);
    let factions: Vec<Faction> = r.game.players.iter().map(|p| p.faction).collect();
    let mut viewers: Vec<Viewer> = factions.iter().map(|f| viewer_of(&r, *f)).collect();
    viewers.push(OBSERVER);
    viewers.into_iter().map(|v| r.view(v).ending.expect("ending after GameEnded")).collect()
}

fn rows(story: &[StoryBlock]) -> Vec<&StoryRow> {
    story
        .iter()
        .flat_map(|b| match b {
            StoryBlock::Setup { table, .. } | StoryBlock::Situation { table, .. } => table.rows.iter().collect::<Vec<_>>(),
            _ => vec![],
        })
        .collect()
}

#[test]
fn no_ending_before_the_game_ends() {
    let mut r = load("mc-discard", 0);
    assert!(r.view(OBSERVER).ending.is_none());
    let mut r = load_text(&truncated("mc-ended", -4), 0);
    assert_ne!(r.game.current_phase, Phase::GameEnded);
    assert!(r.view(OBSERVER).ending.is_none());
}

#[test]
fn everyone_sees_the_same_story() {
    let all = endings("mc-ended");
    assert!(all.windows(2).all(|w| w[0] == w[1]), "the ending differs between viewers");
    let e = &all[0];
    let r = load("mc-ended", 0);
    let winners: Vec<String> = r.game.winners.iter().map(|p| r.game.player(*p).faction.to_string()).collect();
    assert_eq!(e.winners.iter().map(|w| w.faction.clone()).collect::<Vec<_>>(), winners);

    let story = e.story.as_ref().expect("story");
    assert!(matches!(story.first(), Some(StoryBlock::Setup { .. })), "the story starts with the setup");
    assert!(matches!(story.last(), Some(StoryBlock::Ended { .. })));
    let turns: Vec<i32> = story.iter().filter_map(|b| if let StoryBlock::Turn { turn } = b { Some(*turn) } else { None }).collect();
    assert_eq!(turns, vec![1, 2]);
    assert!(story.iter().any(|b| matches!(b, StoryBlock::Purchase { card: Some(_), .. })));
    assert!(story.iter().any(|b| matches!(b, StoryBlock::Battle { .. })));
    assert!(story.iter().any(|b| matches!(b, StoryBlock::Prediction { .. })));
    assert!(rows(story).iter().all(|r| !r.secret));
    // Cards were held at the start of turn 2's bidding, and every faction's spice is shown.
    assert!(rows(story).iter().any(|r| !r.cards.is_empty()));
}

#[test]
fn hidden_secrets_stay_hidden() {
    let all = endings("mc-ended-hidden");
    assert!(all.windows(2).all(|w| w[0] == w[1]));
    let story = all[0].story.as_ref().expect("story");
    let hid = ["Guild", "BeneGesserit"];
    let mut seen_hidden = false;
    for b in story {
        match b {
            StoryBlock::Setup { table, .. } | StoryBlock::Situation { table, .. } => {
                for r in &table.rows {
                    let hides = hid.contains(&r.faction.as_str());
                    assert_eq!(r.secret, hides, "{}", r.faction);
                    if hides {
                        seen_hidden = true;
                        assert!(r.cards.is_empty() && r.traitors.is_empty() && r.resources.is_none() && r.nexus.is_none(), "{:?}", r);
                    } else if table.show_resources {
                        assert!(r.resources.is_some());
                    }
                }
            }
            StoryBlock::Purchase { buyer, card, .. } => assert_eq!(card.is_none(), hid.contains(&buyer.as_str()), "{buyer}"),
            StoryBlock::Prediction { .. } => panic!("Bene Gesserit's prediction is a secret"),
            _ => {}
        }
    }
    assert!(seen_hidden);

    // Apart from the secrets, the story is the one without HideSecrets.
    let open = endings("mc-ended").remove(0).story.unwrap();
    let kinds = |s: &[StoryBlock]| s.iter().filter(|b| !matches!(b, StoryBlock::Prediction { .. })).count();
    assert_eq!(kinds(story), kinds(&open));
}

#[test]
fn the_rules_can_disable_the_story() {
    let mut r = load_text(&with_rules("mc-ended", &[1015]), 0);
    let e = r.view(OBSERVER).ending.expect("ending");
    assert!(e.story.is_none());
    assert!(!e.winners.is_empty());
}
