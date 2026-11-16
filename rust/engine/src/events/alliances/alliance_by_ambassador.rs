use crate::prelude::*;
use crate::messages::part::Part;

#[derive(Clone, Debug)]
pub struct AllianceByAmbassador {
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
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
    ($($e:expr),* $(,)?) => { vec![$(Part::from($e)),*] };
}

impl AllianceByAmbassador {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let paused = game.ship_move.paused_ambassador_phase.required("paused ambassador phase")?;
        game.enter(paused, true);

        if !self.passed {
            let activated = game.ship_move.current_ambassador_activated.clone().required("current ambassador activated")?;
            game.make_alliance(self.initiator().required("initiator")?, activated.initiator().required("ambassador owner")?);

            if activated.ecaz_give_vidal_to_ally {
                game.take_vidal(self.player(), VidalMoment::EndOfTurn);
            }

            if game.turn.has_acted_or_passed.contains_opt(self.initiator())
                && game.turn.has_acted_or_passed.contains_opt(activated.initiator())
            {
                game.check_if_forces_should_be_destroyed_by_ally_presence(self.player());
            }
            game.flip_bene_gesserit_advisors_when_alone();
        } else {
            let activated = game.ship_move.current_ambassador_activated.clone().required("current ambassador activated")?;
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
