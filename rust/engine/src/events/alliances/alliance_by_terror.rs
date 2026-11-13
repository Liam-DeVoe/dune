use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct AllianceByTerror {
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for AllianceByTerror {
    fn default() -> Self {
        AllianceByTerror {
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(ExpressionElement::from($e)),*] };
}

impl AllianceByTerror {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let paused = game.paused_terror_phase;
        game.enter(paused, true);

        if !self.passed {
            if game.player(self.player()).has_ally() {
                let ally = game.player(self.player()).ally;
                self.log_objects(game, elems![self.initiator(), " and ", ally, " end their alliance"]);
                game.break_alliance(self.initiator());
            }

            let moritani = game.get_player(Some(Faction::Moritani));
            if let Some(moritani) = moritani {
                if game.player(moritani).has_ally() {
                    let ally = game.player(moritani).ally;
                    self.log_objects(game, elems![Faction::Moritani, " and ", ally, " end their alliance"]);
                    game.break_alliance(Faction::Moritani);
                }
            }

            game.make_alliance(self.initiator(), Faction::Moritani);

            if game.has_acted_or_passed.contains(&self.initiator()) && game.has_acted_or_passed.contains(&Faction::Moritani) {
                game.check_if_forces_should_be_destroyed_by_ally_presence(self.player());
            }

            let territory = game.last_terror_trigger().required("last terror trigger")?.territory(game);
            self.log_objects(game, elems!["Terror in ", territory, " is returned to supplies"]);
            // ToList(): a snapshot before the dictionary changes
            for t in game.terror_in(territory).collect::<Vec<_>>() {
                game.terror_on_planet.remove(&t);
                game.unplaced_terror_tokens.push(t);
            }

            game.alliance_by_terror_was_offered = false;
            game.dequeue_intrusion(IntrusionType::Terror)?;
            game.determine_next_shipment_and_move_sub_phase()?;
        } else {
            self.log_objects(game, elems![self.initiator(), " don't ally with ", Faction::Moritani]);
        }

        game.let_factions_discard_surplus_cards();
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), if !self.passed { "" } else { " don't" }, " agree to ally")
    }
}

impl GameEventMethods for AllianceByTerror {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::AllianceByTerror
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::AllianceByTerror(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        AllianceByTerror::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        AllianceByTerror::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        AllianceByTerror::get_message(self, game)
    }

}

impl PassableGameEventMethods for AllianceByTerror {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}
