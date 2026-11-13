use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct Planetology {
    pub add_one_to_movement: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for Planetology {
    fn default() -> Self {
        Planetology {
            add_one_to_movement: Default::default(),
            base: Default::default(),
        }
    }
}

impl Planetology {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn move_from_two_territories(&self) -> bool {
        !self.add_one_to_movement
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn can_be_played(g: &Game, p: PlayerIndex) -> bool {
        g.skilled_as_player_leader_skill(p, LeaderSkill::Planetologist) && g.current_planetology.is_none()
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        self.log(game);
        game.current_planetology = Some(self.clone());
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        if self.add_one_to_movement {
            return express!(LeaderSkill::Planetologist, " adds ", 1, " to force movement");
        }
        express!(LeaderSkill::Planetologist, " allows movement from ", 2, " different territories")
    }
}

impl GameEventMethods for Planetology {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::Planetology
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::Planetology(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        Planetology::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        Planetology::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        Planetology::get_message(self, game)
    }

}
