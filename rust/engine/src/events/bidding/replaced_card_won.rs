use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReplacedCardWon {
    pub passed: bool,
}

impl EventRules for ReplacedCardWon {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        if !self.passed {
            let card_just_won = game.bidding.card_just_won;
            game.discard_card(card_just_won, true)?;
            let initiator = game.get_player(by.faction);
            let new_card = game.draw_treachery_card();
            let initiator = initiator.required("initiator")?;
            game.player_mut(initiator).treachery_cards.push(new_card.required("new_card")?);
            game.stone(Milestone::CardWonSwapped);

            if game.bidding.replacing_bought_card_using_nexus {
                game.play_nexus_card(by.player(), vec!["replace the card they just bought".into()]);
                game.bidding.replacing_bought_card_using_nexus = false;
            } else {
                let m = self.describe(game, by);
                game.log_event(m);
            }

            let initiator_faction = game.player(initiator).faction;
            let card_just_won = game.bidding.card_just_won;
            game.log_to(initiator_faction.required("faction")?, vec!["You replaced your ".into(), card_just_won.into(), " with a ".into(), new_card.into()]);
        }

        if game.bidding.card_just_won == game.bidding.card_sold_on_black_market {
            game.enter_richese_bidding();
        } else {
            game.determine_next_step_after_card_was_sold();
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        if self.passed {
            return express!(by.faction, " keep the card they just won");
        }
        express!(by.faction, " discard the card they just won and draw a new card")
    }
}
