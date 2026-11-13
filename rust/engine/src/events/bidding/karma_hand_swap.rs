use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct KarmaHandSwap {
    #[canon(name = "_cardIds")]
    pub card_ids: Option<String>,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for KarmaHandSwap {
    fn default() -> Self {
        KarmaHandSwap {
            card_ids: Default::default(),
            base: Default::default(),
        }
    }
}

impl KarmaHandSwap {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn returned_cards(&self) -> Vec<TreacheryCard> {
        GameEvent::id_string_to_objects(self.card_ids.as_deref(), &TreacheryCardManager::lookup())
    }

    pub fn set_returned_cards(&mut self, value: &[TreacheryCard]) {
        self.card_ids = Some(GameEvent::objects_to_id_string(value, &TreacheryCardManager::lookup()));
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if self.returned_cards().len() as i32 != game.karma_hand_swap_number_of_cards {
            return Some(express!("Select ", game.karma_hand_swap_number_of_cards, " cards to return"));
        }
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        let victim = self.get_player(game, Some(game.karma_hand_swap_target));
        let me = self.player_opt();
        let others: Vec<PlayerIndex> = game.player_indices().filter(|p| Some(*p) != me && Some(*p) != victim).collect();
        for p in others {
            let my_cards = game.player(me.nre()).treachery_cards.clone();
            Game::unregister_known_player_i_enumerable_treachery_card(game, p, &my_cards);
            let victim_cards = game.player(victim.nre()).treachery_cards.clone();
            Game::unregister_known_player_i_enumerable_treachery_card(game, p, &victim_cards);
        }

        // ReturnedCards parses the id string again on every enumeration, as in C#.
        for returned in self.returned_cards() {
            game.player_mut(victim.nre()).treachery_cards.push(returned);
            crate::game::remove_card(&mut game.player_mut(me.nre()).treachery_cards, Some(returned));
        }

        for returned in self.returned_cards() {
            Game::register_known_player_treachery_card(game, me.nre(), Some(returned));
        }
        self.log(game);
        let paused = game.karma_hand_swap_paused_phase;
        game.enter(paused, false);
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " return ", self.returned_cards().len() as i32, " cards")
    }

}

impl GameEventMethods for KarmaHandSwap {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::KarmaHandSwap
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::KarmaHandSwap(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        KarmaHandSwap::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        KarmaHandSwap::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        KarmaHandSwap::get_message(self, game)
    }

}
