use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct DiscardedSearched {
    pub card: Option<TreacheryCard>,
}

impl Default for DiscardedSearched {
    fn default() -> Self {
        DiscardedSearched {
            card: TreacheryCardManager::get(0),
        }
    }
}

impl DiscardedSearched {
    pub fn valid_cards(g: &Game) -> Vec<TreacheryCard> {
        g.decks.treachery_discard_pile.as_ref().expect("treachery discard pile").items.clone()
    }

    pub fn can_be_played(game: &Game, p: PlayerId) -> bool {
        game.player(p).has_treachery_card_type(TreacheryCardType::SearchDiscarded)
    }
}

impl EventRules for DiscardedSearched {
    fn validate(&self, game: &Game, _by: By) -> Result<(), Rejection> {
        // List.Contains(null) is false.
        let valid_cards = DiscardedSearched::valid_cards(game);
        if !self.card.is_some_and(|x| valid_cards.contains(&x)) {
            return Err(Rejection(express!("Invalid card")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);

        for i in 0..game.players.len() {
            let items = game.decks.treachery_discard_pile.as_ref().required("treachery discard pile")?.items.clone();
            Game::unregister_all_known_by_player(game, game.player_at(i), &items);
        }

        let card = self.card.required("card")?;
        crate::iter::remove_first(&mut game.decks.treachery_discard_pile.as_mut().required("treachery discard pile")?.items, &card);
        let me = by.player();
        game.player_mut(me).treachery_cards.push(card);
        game.decks.treachery_discard_pile.as_mut().required("treachery discard pile")?.shuffle(&mut game.rng);
        game.discard(me, TreacheryCardType::SearchDiscarded)?;
        game.resume(Interrupt::SearchingDiscarded, true)?;
        game.stone(Milestone::Shuffled);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " take one card and the Treachery Discard Pile is then shuffled")
    }
}
