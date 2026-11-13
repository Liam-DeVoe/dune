use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct DivideResourcesAccepted {
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for DivideResourcesAccepted {
    fn default() -> Self {
        DivideResourcesAccepted {
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl DivideResourcesAccepted {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn is_applicable(g: &Game, p: PlayerIndex) -> bool {
        g.current_phase == Phase::AcceptingResourceDivision
            && Self::get_resources_to_be_divided(g).map(|t| t.other_faction) == Some(g.player(p).faction)
    }

    pub fn get_resources_to_be_divided(g: &Game) -> Option<ResourcesToBeDivided> {
        g.collected_resources_to_be_divided.first().cloned()
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.divide_resources_from_collection(!self.passed);
        let any = !game.collected_resources_to_be_divided.is_empty();
        game.enter_boolean_phase_action(any, Phase::DividingCollectedResources, Game::end_collection_main_phase);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), if !self.passed { "" } else { " don't" }, " agree with the proposed division")
    }

}

impl GameEventMethods for DivideResourcesAccepted {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::DivideResourcesAccepted
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::DivideResourcesAccepted(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        DivideResourcesAccepted::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        DivideResourcesAccepted::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        DivideResourcesAccepted::get_message(self, game)
    }

}

impl PassableGameEventMethods for DivideResourcesAccepted {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}
