use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct DiscoveryEntered {
    // ---- inherited from PlacementEvent ----
    pub to: Option<Location>,
    pub force_locations: Option<String>,
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `Event` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for DiscoveryEntered {
    fn default() -> Self {
        DiscoveryEntered {
            to: Location::from_id(0),
            force_locations: Some("".to_string()),
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl DiscoveryEntered {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if !self.passed {
            // IEnumerable<DiscoveredLocation>.Contains(Location): Location equality.
            let to = self.to;
            if !Self::valid_targets(game, self.player()).any(|ds| Some(ds.location()) == to) {
                return Some(express!("You can't move there"));
            }
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_targets(g: &Game, p: PlayerId) -> impl Iterator<Item = DiscoveredLocation> {
        g.collection.just_revealed_discovery_strongholds
            .iter()
            .copied()
            .filter(move |ds| g.player(p).any_forces_in_territory(Some(ds.attached_to_location(&g.board.map).expect("attached to location").territory())) > 0)
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_sources(game: &Game, p: PlayerId, ds: DiscoveredLocation) -> impl Iterator<Item = Location> {
        ds.attached_to_location(&game.board.map)
            .expect("attached to location")
            .territory()
            .locations()
            .iter()
            .copied()
            .filter(move |l| game.player(p).any_forces_in(Some(*l)) > 0)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let Some(discovered_location) = self.to.and_then(|l| l.as_discovered_location()) else { return Ok(()) };

        if let Some(i) = game.collection.just_revealed_discovery_strongholds.iter().position(|d| d.location() == discovered_location.location()) {
            game.collection.just_revealed_discovery_strongholds.remove(i);
        }
        game.stone(Milestone::Move);

        self.log(game);

        if !self.passed {
            let from_territories = self.force_locations(game).keys().map(|l| l.territory()).distinct();
            for from_territory in from_territories {
                let mut total_number_of_forces = 0;
                let mut total_number_of_special_forces = 0;

                for (key, value) in self.force_locations(game).iter().filter(|(k, _)| k.territory() == from_territory) {
                    if value.total_amount_of_forces() > 0 {
                        let me = self.player();
                        let to = self.to.required("to")?;
                        game.perform_move_from_location(me, *key, value, to, &mut total_number_of_forces, &mut total_number_of_special_forces, true)?;
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
                    let to = self.to.required("to")?;
                    game.log_move(me, from_territory, &from_locations, to, total_number_of_forces, total_number_of_special_forces, false, false, None);
                    game.flip_bene_gesserit_advisors_when_alone()?;
                }
            }
        }
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        if !self.passed {
            return express!(self.initiator(), " enter ", self.to);
        }
        express!(self.initiator(), " don't enter ")
    }

}

impl GameEventMethods for DiscoveryEntered {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> EventType {
        EventType::DiscoveryEntered
    }

    fn into_event(self: Rc<Self>) -> Event {
        Event::DiscoveryEntered(self)
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

impl crate::events::location_event::LocationEvent for DiscoveryEntered {
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

impl crate::events::placement::Placement for DiscoveryEntered {
    fn force_locations(&self, game: &Game) -> IndexMap<Location, Battalion> {
        <Self as PlacementEventMethods>::force_locations(self, game)
    }

    fn to(&self, _game: &Game) -> Option<Location> {
        self.to
    }

}
