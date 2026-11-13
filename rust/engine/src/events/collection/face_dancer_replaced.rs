use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct FaceDancerReplaced {
    #[canon(name = "dancerId")]
    pub dancer_id: i32,
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for FaceDancerReplaced {
    fn default() -> Self {
        FaceDancerReplaced {
            dancer_id: Default::default(),
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl FaceDancerReplaced {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn selected_dancer(&self) -> Option<Hero> {
        LeaderManager::hero_lookup().find(self.dancer_id)
    }

    pub fn set_selected_dancer(&mut self, value: Option<Hero>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.dancer_id = LeaderManager::hero_lookup().get_id(value);
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if !self.passed {
            let Some(selected_dancer) = self.selected_dancer() else {
                return Some(express!("Select a leader"));
            };
            let p = game.player(self.player());
            if p.revealed_face_dancers.contains(&selected_dancer) {
                return Some(express!("You can't replace a revealed Face Dancer"));
            }
            if !p.face_dancers.contains(&selected_dancer) {
                return Some(express!("Invalid Face Dancer"));
            }
        }

        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        if !self.passed {
            if let Some(selected_dancer) = self.selected_dancer() {
                let player = self.player();
                // List.Remove: the first match, if any.
                if let Some(i) = game.player(player).face_dancers.iter().position(|d| *d == selected_dancer) {
                    game.player_mut(player).face_dancers.remove(i);
                }
                game.traitor_deck.as_mut().required("traitor deck")?.put_on_top(selected_dancer);
                game.traitor_deck.as_mut().required("traitor deck")?.shuffle();
                game.stone(Milestone::Shuffled);
                let leader = game.traitor_deck.as_mut().required("traitor deck")?.draw();
                game.player_mut(player).face_dancers.push(leader);
                game.player_mut(player).known_non_traitors.add(leader);
            }
        }

        self.log(game);
        game.enter(Phase::TurnConcluded, true);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), express_part_if!(self.passed; " don't"), " replace a Face Dancer")
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_face_dancers(game: &Game, p: PlayerIndex) -> impl Iterator<Item = Hero> {
        game.player(p).unrevealed_face_dancers()
    }
}

impl GameEventMethods for FaceDancerReplaced {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::FaceDancerReplaced
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::FaceDancerReplaced(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        FaceDancerReplaced::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        FaceDancerReplaced::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        FaceDancerReplaced::get_message(self, game)
    }

}

impl PassableGameEventMethods for FaceDancerReplaced {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}
