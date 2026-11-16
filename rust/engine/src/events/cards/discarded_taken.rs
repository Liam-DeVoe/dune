use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct DiscardedTaken {
    pub card: Option<TreacheryCard>,
}

impl Default for DiscardedTaken {
    fn default() -> Self {
        DiscardedTaken {
            card: TreacheryCardManager::get(0),
        }
    }
}

impl DiscardedTaken {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_cards(g: &Game, p: PlayerId) -> impl Iterator<Item = TreacheryCard> {
        g.decks.recently_discarded
            .iter()
            .filter(move |(k, v)| Some(**v) != g.player(p).faction && g.decks.treachery_discard_pile.as_ref().expect("treachery discard pile").items.contains(*k))
            .map(move |(k, _)| *k)
    }

    pub fn can_be_played(g: &Game, p: PlayerId) -> bool {
        !g.current_phase_cannot_be_interrupted()
            && g.player(p).has_treachery_card_type(TreacheryCardType::TakeDiscarded)
            && DiscardedTaken::valid_cards(g, p).next().is_some()
    }
}

impl EventRules for DiscardedTaken {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        // ValidCards(Game, Player) is lazy: a null Player and a null discard pile are
        // dereferenced inside the predicate, per element, up to the first match.
        if !DiscardedTaken::valid_cards(game, by.player()).any(|v| Some(v) == self.card) {
            return Err(Rejection(express!("Invalid card")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);
        // Dictionary.Remove(null) throws.
        let Some(card) = self.card else {
            game_error!("Value cannot be null. (Parameter 'key')")
        };
        game.decks.recently_discarded.shift_remove(&card);
        {
            let items = &mut game.decks.treachery_discard_pile.as_mut().required("treachery discard pile")?.items;
            if let Some(i) = items.iter().position(|c| *c == card) {
                items.remove(i);
            }
        }
        let me = by.player();
        game.player_mut(me).treachery_cards.push(card);
        game.discard(me, TreacheryCardType::TakeDiscarded)?;
        game.stone(Milestone::CardWonSwapped);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " use ", TreacheryCardType::TakeDiscarded, " to acquire the discarded ", self.card)
    }
}
