use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct AllianceByAmbassador {
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for AllianceByAmbassador {
    fn default() -> Self {
        AllianceByAmbassador {
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(ExpressionElement::from($e)),*] };
}

impl AllianceByAmbassador {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let paused = game.paused_ambassador_phase;
        game.enter(paused, true);

        if !self.passed {
            let activated = game.current_ambassador_activated.clone().required("current ambassador activated")?;
            game.make_alliance(self.initiator(), activated.initiator());

            if activated.ecaz_give_vidal_to_ally {
                game.take_vidal(self.player(), VidalMoment::EndOfTurn);
            }

            if game.has_acted_or_passed.contains(&self.initiator())
                && game.has_acted_or_passed.contains(&activated.initiator())
            {
                game.check_if_forces_should_be_destroyed_by_ally_presence(self.player());
            }
            game.flip_bene_gesserit_advisors_when_alone();
        } else {
            let activated = game.current_ambassador_activated.clone().required("current ambassador activated")?;
            self.log_objects(game, elems![self.initiator(), " don't ally with ", activated.initiator()]);
            if activated.ecaz_take_vidal {
                game.take_vidal(activated.player(), VidalMoment::AfterUsedInBattle);
            }
        }

        game.determine_next_shipment_and_move_sub_phase()?;
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), if !self.passed { "" } else { " don't" }, " agree to ally")
    }
}

impl GameEventMethods for AllianceByAmbassador {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::AllianceByAmbassador
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::AllianceByAmbassador(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        AllianceByAmbassador::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        AllianceByAmbassador::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        AllianceByAmbassador::get_message(self, game)
    }

}

impl PassableGameEventMethods for AllianceByAmbassador {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}
