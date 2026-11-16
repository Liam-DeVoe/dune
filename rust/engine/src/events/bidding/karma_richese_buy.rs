use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct KarmaRicheseBuy {
    pub card: Option<TreacheryCard>,
}

impl Default for KarmaRicheseBuy {
    fn default() -> Self {
        KarmaRicheseBuy { card: TreacheryCardManager::get(0) }
    }
}

impl EventRules for KarmaRicheseBuy {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if i32::from(game.player(by.player()).resources) < 3 {
            return Err(Rejection(express!("You can't pay ", Payment::of(3))));
        }
        if !game.decks.richese_cache.iter().any(|c| Some(*c) == self.card) {
            return Err(Rejection(express!("Invalid card")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let me = by.player();
        let karma_card = Karma::valid_karma_cards(game, me).into_iter().next();
        game.play_karma(me, karma_card, vec!["buy a card from their cache for ".into(), Payment::of(3).into()]);
        let card = self.card;
        game.player_mut(me).treachery_cards.push(card.required("card")?);
        crate::game::remove_card(&mut game.decks.richese_cache, card);
        if let Some(card) = card {
            game.drop_copy_in_treachery_deck(card);
        }
        let p = game.player_mut(me);
        p.resources -= 3;
        p.special_karma_power_used = true;
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " use ", TreacheryCardType::Karma, " to buy a card from their cache for ", Payment::of(3))
    }
}
