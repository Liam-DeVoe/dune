use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct IxianSelectedStartingCard {
    pub card: Option<TreacheryCard>,
}

impl Default for IxianSelectedStartingCard {
    fn default() -> Self {
        IxianSelectedStartingCard {
            card: TreacheryCardManager::get(0),
        }
    }
}

impl EventRules for IxianSelectedStartingCard {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let me = game.get_player(by.faction).required("get player")?;
        // C# parity: Validate accepts the id of any card, not only one of the starting cards.
        let card = self.card.required("card")?;
        game.player_mut(me).treachery_cards.push(card);
        crate::iter::remove_first(&mut game.setup.starting_treachery_cards.as_mut().required("starting treachery cards")?.items, &card);
        let m = self.describe(game, by);
        game.log_event(m);
        game.setup.starting_treachery_cards.as_mut().required("starting treachery cards")?.shuffle();
        game.stone(Milestone::Shuffled);
        game.deal_remaining_starting_treachery_cards_to_non_ixian()?;
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " pick their starting treachery card")
    }
}
