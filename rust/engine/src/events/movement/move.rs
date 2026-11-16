use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct Move {
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

impl Default for Move {
    fn default() -> Self {
        Move {
            as_advisors: Default::default(),
            to: Location::from_id(0),
            force_locations: Some("".to_string()),
            passed: Default::default(),
        }
    }
}

impl Move {
    /// `ForceLocations`, parsed for the faction of `by`.
    pub fn force_locations(&self, game: &Game, by: By) -> IndexMap<Location, Battalion> {
        PlacementEvent::force_locations_by(game, by, &self.force_locations)
    }

    pub fn set_force_locations(&mut self, game: &Game, value: &IndexMap<Location, Battalion>) {
        self.force_locations = Some(PlacementEvent::force_locations_string(game, value));
    }
}

impl EventRules for Move {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if !self.passed && game.in_guild_cunning_shipment() {
            return Err(Rejection(express!("You cannot move after Cunning shipment")));
        }
        if self.passed {
            return Ok(());
        }

        PlacementEvent::validate_move(game, by, self.to, &self.force_locations(game, by), self.as_advisors)
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        game.storm.storm_losses_to_take.clear();

        let extra_move = match &game.turn.current_flight_used {
            Some(fu) => fu.by.faction == by.faction && fu.extra_move(),
            None => false,
        };
        let state = game.ship_move_mut().required("the shipment and move phase")?;
        state.current_player_may_perform_extra_move = extra_move;
        let cunning = state.in_guild_cunning_shipment;

        if !extra_move && !cunning {
            game.turn.has_acted_or_passed.push(by.required_faction()?);

            if game.phase() == Phase::NonGuildMove {
                game.next_shipment_and_move_player();
                game.skip_guild_in_turn_order();
            }
        }

        if cunning {
            game.turn.current_guild_cunning = None;
            game.ship_move_mut().required("the shipment and move phase")?.in_guild_cunning_shipment = false;
        }

        if !self.passed {
            let force_locations = self.force_locations(game, by);
            game.turn.recent_moves.push(RecentMove { initiator: by.faction.expect("faction"), to: self.to, force_locations: force_locations.clone() });

            let me = by.player();
            let to = self.to;
            if contains_conflicting_ally_opt(game, me, to)? {
                game.turn.chosen_destinations_with_allies.push(to.required("to")?.territory());
            }

            let as_advisors = !by.is(Faction::BeneGesserit) || self.as_advisors;
            let event = Event::from(self.clone());
            game.perform_move_from_locations(me, &force_locations, &event, as_advisors, false)?;
            game.check_intrusion(&event, by.faction.expect("faction"))?;
        } else {
            let m = express!(by.faction, " pass movement").with_log(MovePassed { faction: by.required_faction()? });
            game.log(parts![m]);
        }

        game.determine_next_shipment_and_move_sub_phase()?;
        game.check_if_forces_should_be_destroyed_by_ally_presence(by.player())?;
        game.flip_bene_gesserit_advisors_when_alone()?;

        if !game.applicable(Rule::FullPhaseKarma) {
            game.allow(FactionAdvantage::FremenExtraMove);
        }
        if !game.applicable(Rule::FullPhaseKarma) {
            game.allow(FactionAdvantage::IxianCyborgExtraMove);
        }

        game.turn.current_flight_used = None;
        game.turn.current_flight_discovery_used = None;
        game.turn.current_planetology = None;
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        if self.passed {
            return express!(by.faction, " pass move");
        }
        express!(by.faction, " move to ", self.to)
    }
}

/// `Game.ContainsConflictingAlly(Player, To)` where `To` may be null.
///
/// `to` is only dereferenced after the early returns, so a null
/// `To` gives `false` there and fails the event after them (C#: NullReferenceException).
pub(crate) fn contains_conflicting_ally_opt(game: &Game, initiator: PlayerId, to: Option<Location>) -> GameResult<bool> {
    match to {
        Some(to) => Ok(game.contains_conflicting_ally(initiator, to)),
        None => {
            let ini = game.player(initiator);
            if !ini.has_ally() || ini.faction == Some(Faction::Ecaz) || ini.ally == Some(Faction::Ecaz) {
                return Ok(false);
            }
            game_error!("There is no destination")
        }
    }
}

impl crate::events::location_event::LocationEvent for Move {
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
