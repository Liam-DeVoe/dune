use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct ThoughtAnswered {
    #[canon(name = "_cardId")]
    pub card_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for ThoughtAnswered {
    fn default() -> Self {
        ThoughtAnswered {
            card_id: Default::default(),
            base: Default::default(),
        }
    }
}

macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(ExpressionElement::from($e)),*] };
}

impl ThoughtAnswered {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn card(&self) -> Option<TreacheryCard> {
        TreacheryCardManager::get(self.card_id)
    }

    pub fn set_card(&mut self, value: Option<TreacheryCard>) {
        self.card_id = TreacheryCardManager::get_id(value);
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if Self::valid_cards_opt(game, self.player_opt()).is_empty() {
            return None;
        }
        // Enumerable.Contains(null) on a sequence of cards is false.
        let card = self.card();
        if !Self::valid_cards_opt(game, self.player_opt()).into_iter().any(|c| Some(c) == card) {
            return Some(express!("Select a valid card to show"));
        }

        None
    }

    pub fn valid_cards(g: &Game, p: PlayerIndex) -> Vec<TreacheryCard> {
        Self::valid_cards_opt(g, Some(p))
    }

    /// `ValidCards` with a possibly null `p`: `g.CurrentThought.Card` is read before `p.Has`.
    fn valid_cards_opt(g: &Game, p: Option<PlayerIndex>) -> Vec<TreacheryCard> {
        let card = g.current_thought.as_ref().expect("current thought").card();
        let p = p.expect("player");
        if g.player(p).has(card) {
            return vec![card.expect("card")];
        }

        g.player(p).treachery_cards.clone()
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        match self.card() {
            None => {
                game.log(elems![self.initiator(), " don't own any cards"]);
            }
            Some(card) => {
                let thought_initiator = game.current_thought.as_ref().required("current thought")?.initiator();
                game.log_to(thought_initiator, elems!["In response, ", self.initiator(), " show you: ", card]);
                let thought_initiator = game.current_thought.as_ref().required("current thought")?.initiator();
                game.log(elems!["In response, ", self.initiator(), " show ", thought_initiator, " a card"]);
                let thought_initiator = game.current_thought.as_ref().required("current thought")?.initiator();
                game.register_known_faction_treachery_card(thought_initiator, card);
            }
        }

        game.enter(Phase::BattlePhase, true);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        if self.card().is_none() {
            return express!(self.initiator(), " don't have a card to show");
        }
        express!(self.initiator(), " show one of their cards")
    }
}

impl GameEventMethods for ThoughtAnswered {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::ThoughtAnswered
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::ThoughtAnswered(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        ThoughtAnswered::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        ThoughtAnswered::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        ThoughtAnswered::get_message(self, game)
    }

}
