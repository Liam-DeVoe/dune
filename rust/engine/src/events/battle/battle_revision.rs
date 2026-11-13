use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct BattleRevision {
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for BattleRevision {
    fn default() -> Self {
        BattleRevision {
            base: Default::default(),
        }
    }
}

impl BattleRevision {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        let Some(current_battle) = game.current_battle.clone() else {
            throw!(Exception, "BattleRevision event executed when there is no current battle");
        };

        let withdrawn = current_battle.plan_of(game, self.player_opt());
        if let Some(withdrawn) = withdrawn {
            for aspect in Battle::aspects_seen_by_opponent(game, self.player()) {
                game.prescience_seen_plans.insert(aspect, withdrawn.clone());
            }
        }

        if self.by(current_battle.aggressor(game)) {
            game.previous_aggressor_plan = game.aggressor_plan.take();
        } else if self.by(current_battle.defender(game)) {
            game.previous_defender_plan = game.defender_plan.take();
        }
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " revise their battle plan")
    }

}

impl GameEventMethods for BattleRevision {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::BattleRevision
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::BattleRevision(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        BattleRevision::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        BattleRevision::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        BattleRevision::get_message(self, game)
    }

}
