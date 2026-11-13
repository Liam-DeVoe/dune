use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct AllianceBroken {
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for AllianceBroken {
    fn default() -> Self {
        AllianceBroken {
            base: Default::default(),
        }
    }
}

impl AllianceBroken {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if !game.player(self.player()).has_ally() {
            return Some(express!("You currently have no ally"));
        }

        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let ally = game.player(self.player()).ally;
        let m = self
            .get_message(game)
            .with_log(NexusLog::new(None, Some(NexusAlliance::new(NexusAllianceChange::Ended, self.initiator(), ally))));
        self.log_objects(game, vec![m.into()]);

        // The former ally gets to act again in the alliance phase, even if they already passed.
        if matches!(game.current_phase, Phase::AllianceA | Phase::AllianceB) {
            // List.Remove: the first occurrence, if any.
            if let Some(i) = game.has_acted_or_passed.iter().position(|f| *f == ally) {
                game.has_acted_or_passed.remove(i);
            }
        }

        game.break_alliance(self.initiator());
        game.let_factions_discard_surplus_cards();
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " end their alliance")
    }

}

impl GameEventMethods for AllianceBroken {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::AllianceBroken
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::AllianceBroken(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        AllianceBroken::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        AllianceBroken::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        AllianceBroken::get_message(self, game)
    }

}
