use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct DiscardedSearchedAnnounced {}

impl DiscardedSearchedAnnounced {
    pub fn can_be_played(g: &Game, p: PlayerId) -> bool {
        !g.current_phase_cannot_be_interrupted()
            && i32::from(g.player(p).resources) >= 2
            && g.player(p).has_treachery_card_type(TreacheryCardType::SearchDiscarded)
            && DiscardedSearched::valid_cards(g).len() > 0
    }
}

impl EventRules for DiscardedSearchedAnnounced {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);
        game.turn.phase_before_searching_discarded = Some(game.turn.current_phase);
        let me = by.player();
        let p = game.player_mut(me);
        p.resources -= 2;
        game.enter(Phase::SearchingDiscarded, true);
        game.stone(Milestone::CardWonSwapped);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(
            by.faction,
            " use ",
            TreacheryCardType::SearchDiscarded,
            " and pay ",
            Payment::of(2),
            " to search a card in the Treachery Discard Pile"
        )
    }
}
