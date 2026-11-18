use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct PerformSetup {
    pub resources: u16,
    // ---- inherited from PlacementEvent ----
    pub to: Option<Location>,
    #[serde(with = "crate::events::wire::force_list")]
    #[ts(as = "Option<Vec<crate::events::wire::ForceEntry>>")]
    pub force_locations: Option<String>,
    #[serde(skip)]
    pub passed: bool,
}

impl Default for PerformSetup {
    fn default() -> Self {
        PerformSetup {
            resources: Default::default(),
            to: Location::from_id(0),
            force_locations: Some("".to_string()),
            passed: Default::default(),
        }
    }
}

impl EventRules for PerformSetup {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        let faction = game.next_faction_to_perform_custom_setup();
        let p = game.get_player(faction);
        let number_of_special_forces =
            self.force_locations(game, by).values().map(|b| b.special_forces()).sum::<i32>();
        let Some(p) = p else { return Err(Rejection(express!("No faction is setting up"))) };
        if number_of_special_forces > game.player(p).special_forces_in_reserve() {
            return Err(Rejection(express!("Too many ", game.player(p).special_force())));
        }

        let number_of_forces = self.force_locations(game, by).values().map(|b| b.forces()).sum::<i32>();
        if number_of_forces > game.player(p).forces_in_reserve() {
            return Err(Rejection(express!("Too many ", game.player(p).force())));
        }

        Ok(())
    }

    fn try_validate(&self, game: &Game, by: By) -> GameResult<Result<(), Rejection>> {
        self.force_locations.as_ref().required("force locations")?;
        Ok(self.validate(game, by))
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let faction = game.next_faction_to_perform_custom_setup();
        let player = game.get_player(faction);

        for (location, battalion) in self.force_locations(game, by).iter() {
            let location = *location;
            Player::ship_forces(game, player.required("player")?, location, battalion.forces())?;
            Player::ship_special_forces(game, player.required("player")?, location, battalion.special_forces())?;
        }

        game.player_mut(player.required("player")?).resources = u16::try_from(i32::from(self.resources)).ok().required("an amount of spice from 0 to 65535")?;

        game.log(vec![faction.into(), " initial positions set, starting with ".into(), Payment::of(i32::from(self.resources)).into()],
        );
        game.turn.has_acted_or_passed.push(faction.required("faction")?);

        if game.players.len() == game.turn.has_acted_or_passed.len() {
            if game.treachery_cards_before_traitors() {
                game.enter_storm_phase()?;
            } else {
                game.deal_starting_treachery_cards()?;
            }
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " initial positions and ", Concept::Resource, " (", Payment::of(i32::from(self.resources)), ") determined")
    }
}

/// The force list, read as every placement event reads it ([`PlacementEvent`]).
impl PerformSetup {
    /// The battalions in the force list, of the submitter `by`.
    pub fn force_locations(&self, game: &Game, by: By) -> IndexMap<Location, Battalion> {
        PlacementEvent::force_locations_by(game, by, &self.force_locations)
    }

    pub fn set_force_locations(&mut self, game: &Game, value: &IndexMap<Location, Battalion>) {
        self.force_locations = Some(PlacementEvent::force_locations_string(game, value));
    }
}

impl crate::events::location_event::LocationEvent for PerformSetup {
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
