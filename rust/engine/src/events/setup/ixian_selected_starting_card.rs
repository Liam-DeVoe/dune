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
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        // Only reached outside the phase when events are replayed without validation.
        if game.setup().is_none() {
            game_error!("Not in the setup phase");
        }
        let me = game.get_player(by.faction).required("get player")?;
        // C# parity: Validate accepts the id of any card, not only one of the starting cards.
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
