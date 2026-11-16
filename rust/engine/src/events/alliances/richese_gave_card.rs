use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct RicheseGaveCard {
    pub card: Option<TreacheryCard>,
}

impl Default for RicheseGaveCard {
    fn default() -> Self {
        RicheseGaveCard {
            card: None,
        }
    }
}

impl RicheseGaveCard {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_cards(game: &Game, p: PlayerId) -> impl Iterator<Item = TreacheryCard> {
        game.player(p).treachery_cards.iter().copied().filter(move |c| c.rules().contains(&Rule::RicheseTreacheryCards))
    }
}

impl EventRules for RicheseGaveCard {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let me = by.player();
        let target = game.player(me).allied_player(game);

        let card = self.card;
        crate::game::remove_card(&mut game.player_mut(me).treachery_cards, card);
        Game::register_known_by_player(game, me, card);
        let target = target.required("target")?;
        game.player_mut(target).treachery_cards.push(card.required("card")?);

        let others: Vec<PlayerId> = game.player_indices().filter(|p| *p != me && *p != target).collect();
        for p in others {
            let my_cards = game.player(me).treachery_cards.clone();
            Game::unregister_all_known_by_player(game, p, &my_cards);
            let target_cards = game.player(target).treachery_cards.clone();
            Game::unregister_all_known_by_player(game, p, &target_cards);
        }

        let m = self.describe(game, by);
        game.log_event(m);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " give a card to their ally")
    }
}
