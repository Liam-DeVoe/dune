use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct PerformSetup {
    pub resources: i32,
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

impl Default for PerformSetup {
    fn default() -> Self {
        PerformSetup {
            resources: Default::default(),
            to_id: Default::default(),
            force_locations: Some("".to_string()),
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl PerformSetup {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let faction = game.next_faction_to_perform_custom_setup();
        let p = game.get_player(Some(faction));
        let number_of_special_forces =
            self.force_locations(game).values().map(|b| b.amount_of_special_forces).sum::<i32>();
        let Some(p) = p else { return Some(express!("No faction is setting up")) };
        if number_of_special_forces > game.player(p).special_forces_in_reserve() {
            return Some(express!("Too many ", game.player(p).special_force()));
        }

        let number_of_forces = self.force_locations(game).values().map(|b| b.amount_of_forces).sum::<i32>();
        if number_of_forces > game.player(p).forces_in_reserve() {
            return Some(express!("Too many ", game.player(p).force()));
        }

        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let faction = game.next_faction_to_perform_custom_setup();
        let player = self.get_player(game, Some(faction));

        for (location, battalion) in self.force_locations(game).iter() {
            let location = *location;
            Player::ship_forces(game, player.required("player")?, location, battalion.amount_of_forces);
            Player::ship_special_forces(game, player.required("player")?, location, battalion.amount_of_special_forces);
        }

        game.player_mut(player.required("player")?).resources = self.resources;

        self.log_objects(
            game,
            vec![faction.into(), " initial positions set, starting with ".into(), Payment::of(self.resources).into()],
        );
        game.has_acted_or_passed.push(faction);

        if game.players.len() == game.has_acted_or_passed.len() {
            // Game.Enter(bool, Action, Action) is static in C#; inlined here.
            if game.treachery_cards_before_traitors() {
                game.enter_storm_phase();
            } else {
                game.deal_starting_treachery_cards();
            }
        }
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " initial positions and ", Concept::Resource, " (", Payment::of(self.resources), ") determined")
    }

}

impl GameEventMethods for PerformSetup {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::PerformSetup
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::PerformSetup(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        PerformSetup::validate(self, game)
    }

    fn try_validate(&self, game: &Game) -> GameResult<Option<Message>> {
        self.field_force_locations().as_ref().required("force locations")?;
        Ok(PerformSetup::validate(self, game))
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        PerformSetup::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        PerformSetup::get_message(self, game)
    }

}

impl PassableGameEventMethods for PerformSetup {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}

impl PlacementEventMethods for PerformSetup {
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

impl crate::events::location_event::ILocationEvent for PerformSetup {
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

impl crate::events::placement::IPlacement for PerformSetup {
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
