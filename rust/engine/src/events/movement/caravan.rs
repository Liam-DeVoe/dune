use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct Caravan {
    pub as_advisors: bool,
    // ---- inherited from PlacementEvent ----
    pub to: Option<Location>,
    #[serde(with = "crate::events::wire::force_list")]
    #[ts(as = "Option<Vec<crate::events::wire::ForceEntry>>")]
    pub force_locations: Option<String>,
    // ---- inherited from PassableGameEvent ----
    #[serde(skip)]
    pub passed: bool,
}

impl Default for Caravan {
    fn default() -> Self {
        Caravan {
            as_advisors: Default::default(),
            to: Location::from_id(0),
            force_locations: Some("".to_string()),
            passed: Default::default(),
        }
    }
}

impl Caravan {
    /// `ForceLocations`, parsed for the faction of `by`.
    pub fn force_locations(&self, game: &Game, by: By) -> IndexMap<Location, Battalion> {
        PlacementEvent::force_locations_by(game, by, &self.force_locations)
    }

    pub fn set_force_locations(&mut self, game: &Game, value: &IndexMap<Location, Battalion>) {
        self.force_locations = Some(PlacementEvent::force_locations_string(game, value));
    }
}

impl EventRules for Caravan {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        // A pass skipped every check, but `execute` still made the move: from anywhere, with
        // forces the player didn't have (escapes.md, "Forces below zero"). Nobody passes a
        // Caravan: the client offers no pass and the card is simply not played.
        if self.passed {
            return Err(Rejection(express!("You can't pass a ", TreacheryCardType::Caravan)));
        }
        PlacementEvent::validate_move(game, by, self.to, &self.force_locations(game, by), self.as_advisors)
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let force_locations = self.force_locations(game, by);
        game.ship_move.recent_moves.push(RecentMove { initiator: by.faction.expect("faction"), to: self.to, force_locations: force_locations.clone() });

        game.storm.storm_losses_to_take.clear();

        let me = by.player();
        game.discard(me, TreacheryCardType::Caravan)?;
        let as_advisors = !by.is(Faction::BeneGesserit) || self.as_advisors;
        let event = Event::from(self.clone());
        game.perform_move_from_locations(me, &force_locations, &event, as_advisors, true)?;

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

        if game.check_intrusion(&event, by.faction.expect("faction"))? {
            game.show_first_intrusion(IntrusionCause::Caravan)?;
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        if self.passed {
            return express!(by.faction, " pass ", TreacheryCardType::Caravan);
        }
        express!(by.faction, " move to ", self.to, " by ", TreacheryCardType::Caravan)
    }
}

impl crate::events::location_event::LocationEvent for Caravan {
    fn to(&self, _game: &Game) -> Option<Location> {
        self.to
    }

    fn total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        self.forces_added_to_location(game) + self.special_forces_added_to_location(game)
    }

    fn forces_added_to_location(&self, game: &Game) -> i32 {
        PlacementEvent::force_amounts(game, &self.force_locations).0
    }

    fn special_forces_added_to_location(&self, game: &Game) -> i32 {
        PlacementEvent::force_amounts(game, &self.force_locations).1
    }
}
