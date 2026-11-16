use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct KarmaHandSwapInitiated {
    pub target: Option<Faction>,
}

impl EventRules for KarmaHandSwapInitiated {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        game.bidding.karma_hand_swap_paused_phase = Some(game.turn.current_phase);
        game.enter(Phase::PerformingKarmaHandSwap, true);
        let victim = game.get_player(self.target);
        let me = by.player();
        let karama = Karma::valid_karma_cards(game, me).into_iter().next();
        game.player_mut(me).special_karma_power_used = true;
        let victim = victim.required("victim")?;
        game.bidding.karma_hand_swap_number_of_cards = game.player(victim).treachery_cards.len() as i32;
        game.bidding.karma_hand_swap_target = self.target;
        let mut cards_to_draw_from = Deck::new(game.player(victim).treachery_cards.clone());
        game.stone(Milestone::Shuffled);
        cards_to_draw_from.shuffle(&mut game.rng);

        let mut i = 0;
        while i < game.bidding.karma_hand_swap_number_of_cards {
            let card = cards_to_draw_from.try_draw()?;
            Game::register_known_by_player(game, me, Some(card));
            crate::game::remove_card(&mut game.player_mut(victim).treachery_cards, Some(card));
            game.player_mut(me).treachery_cards.push(card);
            i += 1;
        }

        game.play_karma(me, karama, vec!["swap cards with ".into(), self.target.into()]);
        game.stone(Milestone::Karma);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!("Using ", TreacheryCardType::Karma, ", ", by.faction, " swap cards with ", self.target)
    }
}
