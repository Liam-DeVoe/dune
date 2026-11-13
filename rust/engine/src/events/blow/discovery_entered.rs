use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct DiscoveryEntered {
    // ---- inherited from PlacementEvent ----
    pub to_id: i32,
    pub force_locations: Option<String>,
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for DiscoveryEntered {
    fn default() -> Self {
        DiscoveryEntered {
            to_id: Default::default(),
            force_locations: Some("".to_string()),
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl DiscoveryEntered {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if !self.passed {
            // IEnumerable<DiscoveredLocation>.Contains(Location): Location equality.
            let to = self.to(game);
            if !Self::valid_targets(game, self.player()).any(|ds| Some(ds.location()) == to) {
                return Some(express!("You can't move there"));
            }
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_targets(g: &Game, p: PlayerIndex) -> impl Iterator<Item = DiscoveredLocation> {
        g.just_revealed_discovery_strongholds
            .iter()
            .copied()
            .filter(move |ds| g.player(p).any_forces_in_territory(Some(ds.attached_to_location(&g.map).expect("attached to location").territory())) > 0)
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_sources(game: &Game, p: PlayerIndex, ds: DiscoveredLocation) -> impl Iterator<Item = Location> {
        ds.attached_to_location(&game.map)
            .expect("attached to location")
            .territory()
            .locations()
            .iter()
            .copied()
            .filter(move |l| game.player(p).any_forces_in(Some(*l)) > 0)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let Some(discovered_location) = self.to(game).and_then(|l| l.as_discovered_location()) else { return Ok(()) };

        if let Some(i) = game.just_revealed_discovery_strongholds.iter().position(|d| d.location() == discovered_location.location()) {
            game.just_revealed_discovery_strongholds.remove(i);
        }
        game.stone(Milestone::Move);

        self.log(game);

        if !self.passed {
            // ForceLocations is re-parsed from the event's string at every access in C#; the
            // parse reads only the static lookup and the player's faction, so each access
            // yields the same dictionary.
            let from_territories = self.force_locations(game).keys().map(|l| l.territory()).distinct();
            for from_territory in from_territories {
                let mut total_number_of_forces = 0;
                let mut total_number_of_special_forces = 0;

                for (key, value) in self.force_locations(game).iter().filter(|(k, _)| k.territory() == from_territory) {
                    if value.total_amount_of_forces() > 0 {
                        let me = self.player();
                        let to = self.to(game).required("to")?;
                        game.perform_move_from_location(me, *key, value, to, &mut total_number_of_forces, &mut total_number_of_special_forces, true);
                    }
                }

                if total_number_of_forces > 0 || total_number_of_special_forces > 0 {
                    let from_locations: Vec<Location> = self
                        .force_locations(game)
                        .iter()
                        .filter(|(k, v)| k.territory() == from_territory && v.total_amount_of_forces() > 0)
                        .map(|(k, _)| *k)
                        .collect();
                    let me = self.player();
                    let to = self.to(game).required("to")?;
                    game.log_move(me, from_territory, &from_locations, to, total_number_of_forces, total_number_of_special_forces, false, false, None);
                    game.flip_bene_gesserit_advisors_when_alone();
                }
            }
        }
        Ok(())
    }

    pub fn get_message(&self, game: &Game) -> Message {
        if !self.passed {
            return express!(self.initiator(), " enter ", self.to(game));
        }
        express!(self.initiator(), " don't enter ")
    }

}

impl GameEventMethods for DiscoveryEntered {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::DiscoveryEntered
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::DiscoveryEntered(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        DiscoveryEntered::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        DiscoveryEntered::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        DiscoveryEntered::get_message(self, game)
    }

}

impl PassableGameEventMethods for DiscoveryEntered {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}

impl PlacementEventMethods for DiscoveryEntered {
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

impl crate::events::location_event::ILocationEvent for DiscoveryEntered {
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

impl crate::events::placement::IPlacement for DiscoveryEntered {
    fn initiator(&self) -> Faction {
        GameEventMethods::initiator(self)
    }

    fn force_locations(&self, game: &Game) -> IndexMap<Location, Battalion> {
        <Self as PlacementEventMethods>::force_locations(self, game)
    }

    fn to(&self, game: &Game) -> Option<Location> {
        <Self as PlacementEventMethods>::to(self, game)
    }

}
