use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
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
    fn validate(&self, game: &Game, _by: By) -> Result<(), Rejection> {
        let offered = game.decks.starting_treachery_cards.as_ref().is_some_and(|d| self.card.is_some_and(|c| d.items.contains(&c)));
        if !offered {
            return Err(Rejection(express!("Invalid card")));
        }
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        // Only reached outside the phase when events are replayed without validation.
        if game.setup().is_none() {
            game_error!("Not in the setup phase");
        }
        let me = game.get_player(by.faction).required("get player")?;
        let card = self.card.required("card")?;
        game.player_mut(me).treachery_cards.push(card);
        crate::iter::remove_first(&mut game.decks.starting_treachery_cards.as_mut().required("starting treachery cards")?.items, &card);
        let m = self.describe(game, by);
        game.log_event(m);
        game.decks.starting_treachery_cards.as_mut().required("starting treachery cards")?.shuffle(&mut game.rng);
        game.stone(Milestone::Shuffled);
        game.deal_remaining_starting_treachery_cards_to_non_ixian()?;
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " pick their starting treachery card")
    }
}
