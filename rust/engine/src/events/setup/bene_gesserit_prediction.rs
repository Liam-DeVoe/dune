use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct BeneGesseritPrediction {
    pub to_win: Faction,
    pub turn: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for BeneGesseritPrediction {
    fn default() -> Self {
        BeneGesseritPrediction {
            to_win: Default::default(),
            turn: Default::default(),
            base: Default::default(),
        }
    }
}

impl BeneGesseritPrediction {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if !game.is_playing(self.to_win) {
            return Some(express!("Invalid target"));
        }
        if self.turn < 1 || self.turn > game.maximum_turns() {
            return Some(express!("Invalid turn"));
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_targets(g: &Game, p: PlayerIndex) -> impl Iterator<Item = Faction> {
        g.players_other_than(p)
    }

    pub fn valid_turns(g: &Game) -> Vec<i32> {
        let count = g.maximum_turns();
        assert!(count >= 0, "negative number of turns: {count}");
        (0..count).map(|i| 1i32 + i).collect()
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let me = self.player();
        game.player_mut(me).predicted_faction = self.to_win;
        game.player_mut(me).predicted_turn = self.turn;
        self.log(game);
        let condition = game.treachery_cards_before_traitors();
        Game::enter_boolean_action_action(
            game,
            condition,
            |g: &mut Game| g.deal_starting_treachery_cards(),
            |g: &mut Game| g.deal_traitors(),
        );
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " predict who will win and when")
    }
}

impl GameEventMethods for BeneGesseritPrediction {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::BeneGesseritPrediction
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::BeneGesseritPrediction(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        BeneGesseritPrediction::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        BeneGesseritPrediction::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        BeneGesseritPrediction::get_message(self, game)
    }

}
