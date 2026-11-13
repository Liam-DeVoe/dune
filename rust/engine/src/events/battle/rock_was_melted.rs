use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct RockWasMelted {
    pub kill: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for RockWasMelted {
    fn default() -> Self {
        RockWasMelted {
            kill: Default::default(),
            base: Default::default(),
        }
    }
}

impl RockWasMelted {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn can_be_played(g: &Game, p: PlayerIndex) -> bool {
        let plan = g.current_battle.as_ref().nre().plan_of(g, Some(p));
        match plan {
            Some(plan) => match plan.weapon() {
                Some(weapon) => weapon.is_rock_melter(),
                None => false,
            },
            None => false,
        }
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        self.log(game);
        game.current_rock_was_melted = Some(self.clone());
        game.enter(Phase::CallTraitorOrPass, true);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        if self.kill {
            return express!(self.initiator(), " use their ", TreacheryCardType::Rockmelter, " to kill both leaders");
        }
        express!(self.initiator(), " use their ", TreacheryCardType::Rockmelter, " to reduce both leaders to 0 strength")
    }
}

impl GameEventMethods for RockWasMelted {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::RockWasMelted
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::RockWasMelted(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        RockWasMelted::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        RockWasMelted::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        RockWasMelted::get_message(self, game)
    }

}
