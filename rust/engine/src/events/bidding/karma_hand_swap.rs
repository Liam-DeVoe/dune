use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct KarmaHandSwap {
    pub card_ids: Option<String>,
}

impl KarmaHandSwap {
    pub fn returned_cards(&self) -> Vec<TreacheryCard> {
        Event::id_string_to_objects(self.card_ids.as_deref(), &TreacheryCardManager::lookup())
    }

    pub fn set_returned_cards(&mut self, value: &[TreacheryCard]) {
        self.card_ids = Some(Event::objects_to_id_string(value, &TreacheryCardManager::lookup()));
    }
}

impl EventRules for KarmaHandSwap {
    fn validate(&self, game: &Game, _by: By) -> Result<(), Rejection> {
        if self.returned_cards().len() as i32 != game.bidding.karma_hand_swap_number_of_cards {
            return Err(Rejection(express!("Select ", game.bidding.karma_hand_swap_number_of_cards, " cards to return")));
        }
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let victim = game.get_player(game.bidding.karma_hand_swap_target);
        let me = by.player;
        let others: Vec<PlayerId> = game.player_indices().filter(|p| Some(*p) != me && Some(*p) != victim).collect();
        for p in others {
            let my_cards = game.player(me.required("me")?).treachery_cards.clone();
            Game::unregister_all_known_by_player(game, p, &my_cards);
            let victim_cards = game.player(victim.required("victim")?).treachery_cards.clone();
            Game::unregister_all_known_by_player(game, p, &victim_cards);
        }

        // ReturnedCards parses the id string again on every enumeration.
        for returned in self.returned_cards() {
            game.player_mut(victim.required("victim")?).treachery_cards.push(returned);
            crate::game::remove_card(&mut game.player_mut(me.required("me")?).treachery_cards, Some(returned));
        }

        for returned in self.returned_cards() {
            Game::register_known_by_player(game, me.required("me")?, Some(returned));
        }
        let m = self.describe(game, by);
        game.log_event(m);
        let paused = game.bidding.karma_hand_swap_paused_phase.required("karma hand swap paused phase")?;
        game.enter(paused, false);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " return ", self.returned_cards().len() as i32, " cards")
    }
}
