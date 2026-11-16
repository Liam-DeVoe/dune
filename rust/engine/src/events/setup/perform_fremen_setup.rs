use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct PerformFremenSetup {
    // ---- inherited from PlacementEvent ----
    pub to: Option<Location>,
    pub force_locations: Option<String>,
    pub passed: bool,
}

impl Default for PerformFremenSetup {
    fn default() -> Self {
        PerformFremenSetup {
            to: Location::from_id(0),
            force_locations: Some("".to_string()),
            passed: Default::default(),
        }
    }
}

impl EventRules for PerformFremenSetup {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        let p = by.player;
        let number_of_special_forces =
            self.force_locations(game, by).values().map(|b| b.special_forces()).sum::<i32>();
        let p = p.expect("player");
        if number_of_special_forces > game.player(p).special_forces_in_reserve() {
            return Err(Rejection(express!("You only have ", game.player(p).special_forces_in_reserve(), " ", FactionSpecialForce::Fremen)));
        }

        let number_of_forces = self.force_locations(game, by).values().map(|b| b.forces()).sum::<i32>();
        if (number_of_forces + number_of_special_forces) != 10 {
            return Err(Rejection(express!("Distribute 10 forces")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        for (location, battalion) in self.force_locations(game, by).iter() {
            let location = *location;
            Player::ship_forces(game, by.player(), location, battalion.forces())?;
            Player::ship_special_forces(game, by.player(), location, battalion.special_forces())?;
        }

        let m = self.describe(game, by);
        game.log_event(m);

        // All conditions are evaluated before the call.
        let c1 = game.is_playing(Faction::BeneGesserit) && PerformBeneGesseritPlacement::bene_gesserit_may_place_first_force_in_any_territory(game);
        let c2 = game.is_playing(Faction::Moritani);
        let c3 = game.treachery_cards_before_traitors();
        game.enter_first(&[(c1, Step::Enter(Phase::BeneGesseritSettingUp)), (c2, Step::Enter(Phase::MoritaniSettingUp)), (c3, Step::Run(Game::enter_storm_phase))], Step::Run(Game::deal_starting_treachery_cards))?;
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " have set up forces")
    }
}

/// The force list, as placement events read it (`PlacementEvent` in C#).
impl PerformFremenSetup {
    /// The battalions in the force list, of the submitter `by`.
    pub fn force_locations(&self, game: &Game, by: By) -> IndexMap<Location, Battalion> {
        PlacementEvent::force_locations_by(game, by, &self.force_locations)
    }

    pub fn set_force_locations(&mut self, game: &Game, value: &IndexMap<Location, Battalion>) {
        self.force_locations = Some(PlacementEvent::force_locations_string(game, value));
    }
}

impl crate::events::location_event::LocationEvent for PerformFremenSetup {
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
