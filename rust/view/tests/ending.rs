//! The `ending` field: the winners and the story. `mc-ended` is a finished two-turn game.

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
    // Cards were held at the start of turn 2's bidding, and every faction's spice is shown.
    assert!(rows(story).iter().any(|r| !r.cards.is_empty()));
    for b in story {
        if let StoryBlock::Setup { table, .. } | StoryBlock::Situation { table, .. } = b {
            assert!(table.rows.iter().all(|r| r.resources.is_some() == table.show_resources));
        }
    }
}

#[test]
fn the_rules_can_disable_the_story() {
    let mut r = load_text(&with_rules("mc-ended", &[1015]), 0);
    let e = r.view(OBSERVER).ending.expect("ending");
    assert!(e.story.is_none());
    assert!(!e.winners.is_empty());
}
