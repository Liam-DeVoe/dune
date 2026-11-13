use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct BeneGesseritFlip {
    pub as_advisors: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for BeneGesseritFlip {
    fn default() -> Self {
        BeneGesseritFlip {
            as_advisors: Default::default(),
            base: Default::default(),
        }
    }
}

impl BeneGesseritFlip {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        if self.initiator() != Faction::BeneGesserit {
            return Some(express!("Your faction can't flip"));
        }

        None
    }

    pub fn get_territory(g: &Game) -> Territory {
        g.last_bene_gesserit_intrusion().nre().territory(g)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        let m = self.get_dynamic_message(game).with_log(self.log_record(game));
        self.log_objects(game, vec![m.into()]);

        let territory = game.last_shipment_or_movement.as_ref().nre().to_location(game).nre().territory();
        Player::flip_forces_territory_boolean(game, self.player(), territory, self.as_advisors);

        game.flip_bene_gesserit_advisors_when_alone();

        game.dequeue_intrusion(IntrusionType::BeneGesseritIntrusion);
        game.determine_next_shipment_and_move_sub_phase();
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        if self.as_advisors {
            express!(self.initiator(), " flip to ", FactionSpecialForce::BeneGesserit)
        } else {
            express!(self.initiator(), " flip to ", FactionForce::BeneGesserit)
        }
    }

    pub fn log_record(&self, game: &Game) -> BeneGesseritReaction {
        let territory = BeneGesseritFlip::get_territory(game);
        let become_ = (game.player(self.player()).special_forces_in_territory(Some(territory)) > 0) ^ self.as_advisors;
        BeneGesseritReaction::new(
            if become_ { BeneGesseritReactionKind::Become } else { BeneGesseritReactionKind::StayAs },
            territory.into(),
            self.as_advisors,
        )
    }

    pub fn get_dynamic_message(&self, game: &Game) -> Message {
        let territory = BeneGesseritFlip::get_territory(game);
        let has_advisors_there = game.player(self.player()).special_forces_in_territory(Some(territory)) > 0;

        let force: ExpressionElement = if self.as_advisors { FactionSpecialForce::BeneGesserit.into() } else { FactionForce::BeneGesserit.into() };
        express!(
            self.initiator(),
            if has_advisors_there ^ self.as_advisors { " become " } else { " stay as " },
            force,
            " in ",
            territory
        )
    }

}

impl GameEventMethods for BeneGesseritFlip {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::BeneGesseritFlip
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::BeneGesseritFlip(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        BeneGesseritFlip::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        BeneGesseritFlip::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        BeneGesseritFlip::get_message(self, game)
    }

}
