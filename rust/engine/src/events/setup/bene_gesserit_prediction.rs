use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct BeneGesseritPrediction {
    pub to_win: Option<Faction>,
    pub turn: Turn,
    /// The fields of `Event` (Initiator, Time, Game, Player).
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
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if !game.is_playing(self.to_win) {
            return Some(express!("Invalid target"));
        }
        if self.turn < Turn::FIRST || self.turn > game.maximum_turns() {
            return Some(express!("Invalid turn"));
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_targets(g: &Game, p: PlayerId) -> impl Iterator<Item = Faction> {
        g.players_other_than(p)
    }

    pub fn valid_turns(g: &Game) -> Vec<Turn> {
        (1..=g.maximum_turns().number()).map(Turn::new).collect()
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let me = self.player();
        game.player_mut(me).predicted_faction = self.to_win;
        game.player_mut(me).predicted_turn = Some(self.turn);
        self.log(game);
        let condition = game.treachery_cards_before_traitors();
        if condition { game.deal_starting_treachery_cards() } else { game.deal_traitors() }
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " predict who will win and when")
    }
}

impl GameEventMethods for BeneGesseritPrediction {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> EventType {
        EventType::BeneGesseritPrediction
    }

    fn into_event(self: Rc<Self>) -> Event {
        Event::BeneGesseritPrediction(self)
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
