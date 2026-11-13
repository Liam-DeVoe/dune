use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct NexusVoted {
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for NexusVoted {
    fn default() -> Self {
        NexusVoted {
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl NexusVoted {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        self.log(game);
        game.nexus_votes.push(self.clone());
        if game.nexus_votes.len() == game.players.len() {
            let passed = game.nexus_votes.iter().filter(|v| v.passed).count() as i32;
            if 2i32.wrapping_mul(passed) >= game.players.len() as i32 {
                let condition = game.current_phase == Phase::VoteAllianceA && game.applicable(Rule::IncreasedResourceFlow);
                Game::enter_boolean_action_action(game, condition, Game::enter_blow_b, Game::start_nexus_card_phase);
            } else {
                if game.applicable(Rule::BreakAlliancesOnAlliancePhase) {
                    game.break_all_alliances();
                }
                let condition = game.current_phase == Phase::VoteAllianceA;
                game.enter_alliance_phase(condition);
            }
        }
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " vote ", if self.passed { "No" } else { "Yes" })
    }

}

impl GameEventMethods for NexusVoted {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::NexusVoted
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::NexusVoted(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        NexusVoted::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        NexusVoted::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        NexusVoted::get_message(self, game)
    }

}

impl PassableGameEventMethods for NexusVoted {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}
