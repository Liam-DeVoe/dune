use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct DiscoveryEntered {
    // ---- inherited from PlacementEvent ----
    pub to: Option<Location>,
    pub force_locations: Option<String>,
    pub passed: bool,
}

impl Default for DiscoveryEntered {
    fn default() -> Self {
        DiscoveryEntered {
            to: Location::from_id(0),
            force_locations: Some("".to_string()),
            passed: Default::default(),
        }
    }
}

impl DiscoveryEntered {
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
}

impl EventRules for DiscoveryEntered {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if !self.passed {
            // IEnumerable<DiscoveredLocation>.Contains(Location): Location equality.
            let to = self.to;
            if !Self::valid_targets(game, by.player()).any(|ds| Some(ds.location()) == to) {
                return Err(Rejection(express!("You can't move there")));
            }
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let Some(discovered_location) = self.to.and_then(|l| l.as_discovered_location()) else { return Ok(()) };

        if let Some(i) = game.collection.just_revealed_discovery_strongholds.iter().position(|d| d.location() == discovered_location.location()) {
            game.collection.just_revealed_discovery_strongholds.remove(i);
        }
        game.stone(Milestone::Move);

        let m = self.describe(game, by);
        game.log_event(m);

        if !self.passed {
            let from_territories = self.force_locations(game, by).keys().map(|l| l.territory()).distinct();
            for from_territory in from_territories {
                let mut total_number_of_forces = 0;
                let mut total_number_of_special_forces = 0;

                for (key, value) in self.force_locations(game, by).iter().filter(|(k, _)| k.territory() == from_territory) {
                    if value.total_amount_of_forces() > 0 {
                        let me = by.player();
                        let to = self.to.required("to")?;
                        game.perform_move_from_location(me, *key, value, to, &mut total_number_of_forces, &mut total_number_of_special_forces, true)?;
                    }
                }

                if total_number_of_forces > 0 || total_number_of_special_forces > 0 {
                    let from_locations: Vec<Location> = self.force_locations(game, by)
                        .iter()
                        .filter(|(k, v)| k.territory() == from_territory && v.total_amount_of_forces() > 0)
                        .map(|(k, _)| *k)
                        .collect();
                    let me = by.player();
                    let to = self.to.required("to")?;
                    game.log_move(me, from_territory, &from_locations, to, total_number_of_forces, total_number_of_special_forces, false, false, None);
                    game.flip_bene_gesserit_advisors_when_alone()?;
                }
            }
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        if !self.passed {
            return express!(by.faction, " enter ", self.to);
        }
        express!(by.faction, " don't enter ")
    }
}

/// The force list, as placement events read it (`PlacementEvent` in C#).
impl DiscoveryEntered {
    /// The battalions in the force list, of the submitter `by`.
    pub fn force_locations(&self, game: &Game, by: By) -> IndexMap<Location, Battalion> {
        PlacementEvent::force_locations_by(game, by, &self.force_locations)
    }

    pub fn set_force_locations(&mut self, game: &Game, value: &IndexMap<Location, Battalion>) {
        self.force_locations = Some(PlacementEvent::force_locations_string(game, value));
    }
}

impl crate::events::location_event::LocationEvent for DiscoveryEntered {
    fn to(&self, _game: &Game) -> Option<Location> {
        self.to
    }

    fn total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        let (forces, special_forces) = PlacementEvent::force_amounts(game, &self.force_locations);
        forces + special_forces
    }

    fn forces_added_to_location(&self, game: &Game) -> i32 {
        PlacementEvent::force_amounts(game, &self.force_locations).0
    }

    fn special_forces_added_to_location(&self, game: &Game) -> i32 {
        PlacementEvent::force_amounts(game, &self.force_locations).1
    }
}
