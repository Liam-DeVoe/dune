use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct KarmaHandSwapInitiated {
    pub target: Faction,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for KarmaHandSwapInitiated {
    fn default() -> Self {
        KarmaHandSwapInitiated {
            target: Default::default(),
            base: Default::default(),
        }
    }
}

impl KarmaHandSwapInitiated {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.karma_hand_swap_paused_phase = game.current_phase;
        game.enter(Phase::PerformingKarmaHandSwap, true);
        let victim = self.get_player(game, Some(self.target));
        let me = self.player();
        let karama = Karma::valid_karma_cards(game, me).into_iter().next();
        game.player_mut(me).special_karma_power_used = true;
        let victim = victim.required("victim")?;
        game.karma_hand_swap_number_of_cards = game.player(victim).treachery_cards.len() as i32;
        game.karma_hand_swap_target = self.target;
        let mut cards_to_draw_from = Deck::new(game.player(victim).treachery_cards.clone(), game.random.clone());
        game.stone(Milestone::Shuffled);
        cards_to_draw_from.shuffle();

        let mut i = 0;
        while i < game.karma_hand_swap_number_of_cards {
            let card = cards_to_draw_from.try_draw()?;
            Game::register_known_player_treachery_card(game, me, Some(card));
            crate::game::remove_card(&mut game.player_mut(victim).treachery_cards, Some(card));
            game.player_mut(me).treachery_cards.push(card);
            i += 1;
        }

        game.play_karma(me, karama, vec!["swap cards with ".into(), self.target.into()]);
        game.stone(Milestone::Karma);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!("Using ", TreacheryCardType::Karma, ", ", self.initiator(), " swap cards with ", self.target)
    }

}

impl GameEventMethods for KarmaHandSwapInitiated {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::KarmaHandSwapInitiated
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::KarmaHandSwapInitiated(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        KarmaHandSwapInitiated::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        KarmaHandSwapInitiated::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        KarmaHandSwapInitiated::get_message(self, game)
    }

}
