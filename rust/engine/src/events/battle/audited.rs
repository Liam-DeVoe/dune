use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct Audited {
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for Audited {
    fn default() -> Self {
        Audited {
            base: Default::default(),
        }
    }
}

impl Audited {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        self.log(game);
        game.stone(Milestone::Audited);
        // RegisterKnown only adds to the player's KnownCards, so the list does not change.
        let audited = game.audited_cards.clone();
        for card in audited {
            Game::register_known_player_treachery_card(game, self.player(), Some(card));
        }
        let winner_is_none = game.battle_winner == Some(Faction::None);
        let harkonnen_must_decide = game.harkonnen_must_decide_to_capture();
        // Enter(condition, FinishBattle, ...), with FinishBattle's failure passed on.
        if winner_is_none {
            game.finish_battle()?;
        } else {
            game.enter_boolean_phase_phase(harkonnen_must_decide, Phase::CaptureDecision, Phase::BattleConclusion);
        }
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(Faction::Choam, " finish their audit")
    }
}

impl GameEventMethods for Audited {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::Audited
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::Audited(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        Audited::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        Audited::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        Audited::get_message(self, game)
    }

}
