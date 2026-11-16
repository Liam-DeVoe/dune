use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct Caravan {
    pub as_advisors: bool,
    // ---- inherited from PlacementEvent ----
    pub to: Option<Location>,
    pub force_locations: Option<String>,
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for Caravan {
    fn default() -> Self {
        Caravan {
            as_advisors: Default::default(),
            to: Location::from_id(0),
            force_locations: Some("".to_string()),
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl Caravan {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        self.validate_move(game, self.as_advisors)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.ship_move.recent_moves.push(self.clone().into_event());

        game.storm.storm_losses_to_take.clear();

        let me = self.player();
        game.discard(me, TreacheryCardType::Caravan)?;
        let force_locations = PlacementEventMethods::force_locations(&**self, game);
        let as_advisors = self.initiator() != Some(Faction::BeneGesserit) || self.as_advisors;
        game.perform_move_from_locations(me, &force_locations, &self.clone().into_event(), as_advisors, true);

        let to = self.to;
        if crate::events::contains_conflicting_ally_opt(game, me, to)? {
            game.ship_move.chosen_destinations_with_allies.push(to.required("to")?.territory());
        }

        game.ship_move.current_flight_used = None;
        game.ship_move.current_flight_discovery_used = None;

        if !game.applicable(Rule::FullPhaseKarma) {
            game.allow(FactionAdvantage::FremenExtraMove);
        }
        if !game.applicable(Rule::FullPhaseKarma) {
            game.allow(FactionAdvantage::IxianCyborgExtraMove);
        }

        if game.check_intrusion(&self.clone().into_event())? {
            game.ship_move.phase_before_caravan_caused_intrusion = Some(game.turn.current_phase);
            let bene_gesserit = game.last_bene_gesserit_intrusion().is_some();
            let terror = game.last_terror_trigger().is_some();
            game.enter_first(&[(bene_gesserit, Step::Enter(Phase::BeneGesseritIntrudedByCaravan)), (terror, Step::Enter(Phase::TerrorTriggeredByCaravan))], Step::Enter(Phase::AmbassadorTriggeredByCaravan));
        }
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        if self.passed {
            return express!(self.initiator(), " pass ", TreacheryCardType::Caravan);
        }
        express!(self.initiator(), " move to ", self.to, " by ", TreacheryCardType::Caravan)
    }

}

impl GameEventMethods for Caravan {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::Caravan
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::Caravan(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        Caravan::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        Caravan::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        Caravan::get_message(self, game)
    }

}

impl PassableGameEventMethods for Caravan {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}

impl PlacementEventMethods for Caravan {
    fn field_to(&self) -> &Option<Location> {
        &self.to
    }

    fn field_force_locations(&self) -> &Option<String> {
        &self.force_locations
    }

    fn field_to_mut(&mut self) -> &mut Option<Location> {
        &mut self.to
    }

    fn field_force_locations_mut(&mut self) -> &mut Option<String> {
        &mut self.force_locations
    }

}

impl crate::events::location_event::LocationEvent for Caravan {
    fn initiator(&self) -> Faction {
        GameEventMethods::initiator(self).expect("faction")
    }

    fn to(&self, _game: &Game) -> Option<Location> {
        self.to
    }

    fn total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        <Self as PlacementEventMethods>::total_amount_of_forces_added_to_location(self, game)
    }

    fn forces_added_to_location(&self, game: &Game) -> i32 {
        <Self as PlacementEventMethods>::forces_added_to_location(self, game)
    }

    fn special_forces_added_to_location(&self, game: &Game) -> i32 {
        <Self as PlacementEventMethods>::special_forces_added_to_location(self, game)
    }

}

impl crate::events::placement::Placement for Caravan {
    fn initiator(&self) -> Faction {
        GameEventMethods::initiator(self).expect("faction")
    }

    fn force_locations(&self, game: &Game) -> IndexMap<Location, Battalion> {
        <Self as PlacementEventMethods>::force_locations(self, game)
    }

    fn to(&self, _game: &Game) -> Option<Location> {
        self.to
    }

}
