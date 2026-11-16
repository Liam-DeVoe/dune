use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct EmperorDiscarded {
    pub card: Option<TreacheryCard>,
}

impl Default for EmperorDiscarded {
    fn default() -> Self {
        EmperorDiscarded { card: TreacheryCardManager::get(0) }
    }
}

impl EventRules for EmperorDiscarded {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if self.card.is_none() {
            return Err(Rejection(express!("Choose a card to discard")));
        }
        if !game.player(by.player()).has(self.card) {
            return Err(Rejection(express!("Invalid card")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);
        let me = by.player();
        {
            let p = game.player_mut(me);
            p.resources -= 2;
        }
        game.discard_card_of(Some(me), self.card, true);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " pay ", Payment::of(2), " to discard ", self.card)
    }
}
