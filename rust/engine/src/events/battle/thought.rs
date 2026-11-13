use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct Thought {
    #[canon(name = "_cardId")]
    pub card_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for Thought {
    fn default() -> Self {
        Thought {
            card_id: Default::default(),
            base: Default::default(),
        }
    }
}

macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(ExpressionElement::from($e)),*] };
}

impl Thought {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn card(&self) -> Option<TreacheryCard> {
        TreacheryCardManager::get(self.card_id)
    }

    pub fn set_card(&mut self, value: Option<TreacheryCard>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.card_id = TreacheryCardManager::get_id(value);
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        // Enumerable.Contains(null) on a sequence of cards is false.
        let card = self.card();
        if !Self::valid_cards(game).into_iter().any(|c| Some(c) == card) {
            return Some(express!("Invalid card"));
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_cards(g: &Game) -> impl Iterator<Item = TreacheryCard> {
        TreacheryCardManager::get_cards_in_play(g)
            .into_iter()
            .filter(move |c| c.is_weapon() && c.type_() != TreacheryCardType::Chemistry)
    }

    pub fn may_be_used(game: &Game, player: PlayerIndex) -> bool {
        game.skilled_as_player_leader_skill(player, LeaderSkill::Thinker)
            && game.current_battle.is_some()
            && game.current_thought.is_none()
            && game.current_battle.as_ref().nre().is_aggressor_or_defender(game, player)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        game.current_thought = Some(self.clone());
        let battle = game.current_battle.clone().nre();
        let opponent = game.player(battle.opponent_of_faction(game, self.initiator()).nre()).faction;
        game.log(elems![self.initiator(), " use their ", LeaderSkill::Thinker, " skill to ask ", opponent, " if they have a ", self.card()]);
        game.stone(Milestone::Prescience);
        game.enter(Phase::Thought, true);
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " use their ", LeaderSkill::Thinker, " skill to ask a question")
    }
}

impl GameEventMethods for Thought {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::Thought
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::Thought(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        Thought::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        Thought::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        Thought::get_message(self, game)
    }

}
