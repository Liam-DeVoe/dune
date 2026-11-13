use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct Caravan {
    pub as_advisors: bool,
    // ---- inherited from PlacementEvent ----
    #[canon(name = "_toId")]
    pub to_id: i32,
    #[canon(name = "_forceLocations")]
    pub force_locations: Option<String>,
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for Caravan {
    fn default() -> Self {
        Caravan {
            as_advisors: Default::default(),
            to_id: Default::default(),
            force_locations: Some("".to_string()),
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl Caravan {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        self.validate_move(game, self.as_advisors)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        game.recent_moves.push(self.clone().into_event());

        game.storm_losses_to_take.clear();

        let me = self.player();
        game.discard(me, TreacheryCardType::Caravan);
        let force_locations = PlacementEventMethods::force_locations(&**self, game);
        let as_advisors = self.initiator() != Faction::BeneGesserit || self.as_advisors;
        game.perform_move_from_locations(me, &force_locations, &self.clone().into_event(), as_advisors, true);

        let to = PlacementEventMethods::to(&**self, game);
        if crate::events::contains_conflicting_ally_opt(game, me, to) {
            game.chosen_destinations_with_allies.push(to.nre().territory());
        }

        game.current_flight_used = None;
        game.current_flight_discovery_used = None;

        if !game.applicable(Rule::FullPhaseKarma) {
            game.allow(FactionAdvantage::FremenExtraMove);
        }
        if !game.applicable(Rule::FullPhaseKarma) {
            game.allow(FactionAdvantage::IxianCyborgExtraMove);
        }

        if game.check_intrusion(&self.clone().into_event()) {
            game.phase_before_caravan_caused_intrusion = game.current_phase;
            let bene_gesserit = game.last_bene_gesserit_intrusion().is_some();
            let terror = game.last_terror_trigger().is_some();
            game.enter_boolean_phase_boolean_phase_phase(bene_gesserit, Phase::BeneGesseritIntrudedByCaravan, terror, Phase::TerrorTriggeredByCaravan, Phase::AmbassadorTriggeredByCaravan);
        }
    }

    pub fn get_message(&self, game: &Game) -> Message {
        if self.passed {
            return express!(self.initiator(), " pass ", TreacheryCardType::Caravan);
        }
        express!(self.initiator(), " move to ", PlacementEventMethods::to(self, game), " by ", TreacheryCardType::Caravan)
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

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
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
    fn field_to_id(&self) -> &i32 {
        &self.to_id
    }

    fn field_force_locations(&self) -> &Option<String> {
        &self.force_locations
    }

    fn field_to_id_mut(&mut self) -> &mut i32 {
        &mut self.to_id
    }

    fn field_force_locations_mut(&mut self) -> &mut Option<String> {
        &mut self.force_locations
    }

}

impl crate::events::location_event::ILocationEvent for Caravan {
    fn initiator(&self) -> Faction {
        GameEventMethods::initiator(self)
    }

    fn to(&self, game: &Game) -> Option<Location> {
        <Self as PlacementEventMethods>::to(self, game)
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

impl crate::events::placement::IPlacement for Caravan {
    fn initiator(&self) -> Faction {
        GameEventMethods::initiator(self)
    }

    fn force_locations(&self, game: &Game) -> NetDictionary<Location, Battalion> {
        <Self as PlacementEventMethods>::force_locations(self, game)
    }

    fn to(&self, game: &Game) -> Option<Location> {
        <Self as PlacementEventMethods>::to(self, game)
    }

}
