use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct CardsDetermined {
    pub treachery_card_ids: Option<String>,
    pub richese_card_ids: Option<String>,
}

impl Default for CardsDetermined {
    fn default() -> Self {
        CardsDetermined {
            treachery_card_ids: Some(String::new()),
            richese_card_ids: Some(String::new()),
        }
    }
}

impl CardsDetermined {
    pub fn treachery_cards(&self) -> Vec<TreacheryCard> {
        Event::id_string_to_objects(self.treachery_card_ids.as_deref(), &TreacheryCardManager::lookup())
    }

    pub fn set_treachery_cards(&mut self, value: &[TreacheryCard]) {
        self.treachery_card_ids = Some(Event::objects_to_id_string(value, &TreacheryCardManager::lookup()));
    }

    pub fn richese_cards(&self) -> Vec<TreacheryCard> {
        Event::id_string_to_objects(self.richese_card_ids.as_deref(), &TreacheryCardManager::lookup())
    }

    pub fn set_richese_cards(&mut self, value: &[TreacheryCard]) {
        self.richese_card_ids = Some(Event::objects_to_id_string(value, &TreacheryCardManager::lookup()));
    }

    pub fn get_verbose_message(&self) -> Message {
        if !self.richese_cards().is_empty() {
            return express!("Treachery Cards: ", self.treachery_cards(), ". ", Faction::Richese, " Cards: ", self.richese_cards());
        }
        express!("Treachery Cards: ", self.treachery_cards())
    }
}

impl EventRules for CardsDetermined {
    fn validate(&self, game: &Game, _by: By) -> Result<(), Rejection> {
        let selected = (self.treachery_cards().len() as i32) + (self.richese_cards().len() as i32);
        let needed = game.players.iter().map(|p| p.maximum_number_of_cards(game)).sum::<i32>();
        if selected < (needed + 1) {
            return Err(Rejection(express!("Not enough cards selected")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, _by: By) -> GameResult {
        game.decks.treachery_deck = Some(Deck::new(self.treachery_cards()));
        game.decks.treachery_deck.as_mut().required("treachery deck")?.shuffle(&mut game.rng);
        game.stone(Milestone::Shuffled);
        game.decks.richese_cache = self.richese_cards();
        let message = self.get_verbose_message();
        game.log(vec![message.into()]);
        game.enter_setup_phase()?;
        Ok(())
    }

    fn describe(&self, _game: &Game, _by: By) -> Message {
        express!("Card decks were customized.")
    }
}
