//! Move decisions. They read and reset the shipment the bot decided on earlier in the turn
//!, and many of the helpers they call draw random numbers, so the order of the
//! checks matters.

use crate::prelude::*;

impl ClassicBot {
    pub fn determine_move(&mut self, game: &Game) -> Option<Rc<Move>> {
        let moved = if game.in_guild_cunning_shipment { None } else { self.determine_moved_battalion(game, true) };

        let Some(moved) = moved else {
            let mut m = Move::new(game, self.faction(game));
            m.passed = true;
            return Some(Rc::new(m));
        };

        let (forces, as_advisors) = self.move_details(game, &moved);
        let mut result = Move::new(game, self.faction(game));
        result.passed = false;
        result.set_to(game, Some(moved.to));
        result.set_force_locations(game, &forces);
        result.as_advisors = as_advisors;

        result.is_valid(game).then(|| Rc::new(result))
    }

    pub fn determine_caravan(&mut self, game: &Game) -> Option<Rc<Caravan>> {
        if self.forces_on_planet(game).keys().filter(|l| !l.is_stronghold()).count() <= 1 {
            return None;
        }

        let moved = self.determine_moved_battalion(game, false)?;

        let (forces, as_advisors) = self.move_details(game, &moved);
        let mut result = Caravan::new(game, self.faction(game));
        result.passed = false;
        result.set_to(game, Some(moved.to));
        result.set_force_locations(game, &forces);
        result.as_advisors = as_advisors;

        self.decided_shipment_action = ShipmentDecision::None;

        result.is_valid(game).then(|| Rc::new(result))
    }

    /// The forces to move, keyed by where they come from, and whether Bene Gesserit forces
    /// arrive as advisors (they must where advisors already are).
    fn move_details(&self, game: &Game, moved: &MovedBattalion) -> (IndexMap<Location, Battalion>, bool) {
        let mut forces: IndexMap<Location, Battalion> = IndexMap::new();
        forces.insert_new(moved.from, moved.battalion.clone());

        let as_advisors =
            self.faction(game) == Faction::BeneGesserit && game.player(self.player).special_forces_in_territory(Some(moved.to.territory())) > 0;
        (forces, as_advisors)
    }

    /// The battalion to move this turn, in order of priority. `include_low_priority_moves` adds the
    /// moves that only improve an already safe position.
    pub fn determine_moved_battalion(&mut self, game: &Game, include_low_priority_moves: bool) -> Option<MovedBattalion> {
        let winning = self.i_am_winning(game);

        let me = self.player;
        let faction = self.faction(game);

        // Finish a shipment to a stronghold near spice by moving on to the spice.
        if self.decided_shipment_action == ShipmentDecision::StrongholdNearResources {
            // The shipment's location ids are the same as those of `game`.
            let decided = self.decided_shipment.clone().expect("decided shipment");
            let decided_to = decided.to(game).expect("to");
            if game.player(me).forces_in_locations.contains_key(&decided_to) {
                let to_move = game
                    .player(me)
                    .forces_in_locations
                    .index(&decided_to)
                    .take(decided.force_amount + decided.special_force_amount, faction == Faction::Ixian);

                if let Some(final_destination) = self.final_destination
                    && self.within_range(game, decided_to, final_destination, &to_move)
                    && let Some(mv) = self.construct_move(game, final_destination, decided_to, &to_move)
                {
                    return Some(mv);
                }
            }
        }

        // The biggest battalion threatened by ally presence
        if let Some(b) = self.battalion_that_should_be_moved_due_to_ally_presence(game)
            && let Some(to) = self.most_suitable_nearby_location(game, b.location, &b.battalion, true, true)
            && let Some(mv) = self.construct_move(game, to, b.location, &b.battalion)
        {
            return Some(mv);
        }

        // The biggest battalion threatened by the storm
        if !self.last_turn(game)
            && !winning
            && let Some(b) = self.biggest_battalion_threatened_by_storm_without_spice(game)
            && let Some(to) = self.most_suitable_nearby_location(game, b.location, &b.battalion, true, false)
            && let Some(mv) = self.construct_move(game, to, b.location, &b.battalion)
        {
            return Some(mv);
        }

        // From a spiceless non-stronghold location on rock to a safe stronghold, spice or rock
        if let Some(b) = self.biggest_battalion_in_spiceless_non_stronghold_location_on_rock(game)
            && let Some(to) = self.most_suitable_nearby_location(game, b.location, &b.battalion, false, false)
            && let Some(mv) = self.construct_move(game, to, b.location, &b.battalion)
        {
            return Some(mv);
        }

        // From a useless location to a safe stronghold, spice or rock
        if let Some(b) = self.biggest_battalion_in_spiceless_non_stronghold_location_in_sand_or_not_near_stronghold(game)
            && let Some(to) = self.most_suitable_nearby_location(game, b.location, &b.battalion, true, false)
            && let Some(mv) = self.construct_move(game, to, b.location, &b.battalion)
        {
            return Some(mv);
        }

        // A stack of advisors from a stronghold to a nearby vacant stronghold
        if faction == Faction::BeneGesserit
            && let Some(b) = self.biggest_movable_stack_of_advisors_in_stronghold_near_vacant_stronghold(game)
            && let Some(to) = self.vacant_and_safe_nearby_stronghold(game, b.location, &b.battalion)
            && let Some(mv) = self.construct_move(game, to, b.location, &b.battalion)
        {
            return Some(mv);
        }

        if !include_low_priority_moves {
            return None;
        }

        // Half of a big, unthreatened stronghold garrison to a nearby vacant stronghold
        if !winning
            && let Some(b) = self.biggest_large_unthreatened_movable_battalion_in_stronghold_near_vacant_stronghold(game)
            && let Some(to) = self.vacant_and_safe_nearby_stronghold(game, b.location, &b.battalion)
            && let Some(mv) = self.construct_move(game, to, b.location, &b.battalion.take_half())
        {
            return Some(mv);
        }

        // Part of a big, unthreatened stronghold garrison to nearby spice
        if !self.last_turn(game)
            && !winning
            && let Some(b) = self.biggest_large_unthreatened_movable_battalion_in_stronghold_near_spice(game)
            && let Some(to) = self
                .best_safe_and_nearby_resources(game, b.location, &b.battalion, false)
                .or_else(|| self.best_safe_and_nearby_resources(game, b.location, &b.battalion, true))
        {
            let forces_for_collection = std::cmp::max(
                2,
                std::cmp::min(self.determine_forces_needed_for_collection(game, to), b.battalion.total_amount_of_forces() / 2),
            );
            if let Some(mv) = self.construct_move(game, to, b.location, &b.battalion.take(forces_for_collection, faction == Faction::Ixian)) {
                return Some(mv);
            }
        }

        // Towards the Shield Wall, to blow it up with Family Atomics
        if !self.last_turn(game)
            && !winning
            && (0..=8).contains(&game.sector_in_storm)
            && game.player(me).treachery_cards.iter().any(|c| c.type_() == TreacheryCardType::Metheor)
            && !MetheorPlayed::has_forces_at_or_near_shield_wall(game, me)
            && let Some(m) = self.battalion_that_will_destroy_shield_wall(game)
            && let Some(mv) = self.construct_move(game, m.to, m.from, &m.battalion)
        {
            return Some(mv);
        }

        self.decided_shipment_action = ShipmentDecision::None;
        None
    }

    /// The move, unless it would take Richese special forces somewhere at low threshold.
    fn construct_move(&self, game: &Game, to: Location, from: Location, battalion: &Battalion) -> Option<MovedBattalion> {
        if self.faction(game) == Faction::Richese && game.has_low_threshold(Faction::Richese) && battalion.amount_of_special_forces != 0 {
            return None;
        }
        Some(MovedBattalion { to, from, battalion: battalion.clone() })
    }

    /// One force that can move next to the Shield Wall, tried from the Polar Sink, Tuek's Sietch,
    /// Arrakeen and Carthag in that order.
    fn battalion_that_will_destroy_shield_wall(&self, game: &Game) -> Option<MovedBattalion> {
        let map = &game.map;
        let false_wall_east = |sector: i32| map.false_wall_east().locations().iter().copied().find(|l| l.sector(map) == sector).expect("a location in that sector");
        let false_wall_east_target = || if game.sector_in_storm != 8 { false_wall_east(8) } else { false_wall_east(7) };
        let pasty_mesa_target = || map.pasty_mesa().locations().iter().copied().find(|l| l.sector(map) == 7).expect("a location in sector 7");

        let attempts: [(Location, &dyn Fn() -> Location); 4] = [
            (map.polar_sink(), &false_wall_east_target),
            (map.tueks_sietch(), &pasty_mesa_target),
            (map.arrakeen(), &false_wall_east_target),
            (map.carthag(), &false_wall_east_target),
        ];
        attempts.into_iter().find_map(|(from, target)| {
            let to = target();
            self.find_one_troop_that_can_safely_move(game, from, to).map(|battalion| MovedBattalion { from, to, battalion })
        })
    }

    pub fn determine_flight_used(&self, game: &Game) -> Option<Rc<FlightUsed>> {
        let outside_strongholds = self.force_locations_outside_strongholds(game);
        if outside_strongholds.len() > 1 && outside_strongholds.iter().map(|(_, b)| b.total_amount_of_forces()).sum::<i32>() > 3 {
            let mut f = FlightUsed::new(game, self.faction(game));
            f.move_three_territories = false;
            return Some(Rc::new(f));
        }

        None
    }

    pub fn determine_flight_discovery_used(&self, game: &Game) -> Option<Rc<FlightDiscoveryUsed>> {
        if !game.has_ornithopters(self.player)
            && (self.biggest_battalion_in_spiceless_non_stronghold_location_in_sand_or_not_near_stronghold(game).is_some()
                || self.biggest_battalion_in_spiceless_non_stronghold_location_not_near_stronghold_and_spice(game).is_some()
                || self.biggest_battalion_threatened_by_storm_without_spice(game).is_some()
                || self.biggest_battalion_in_spiceless_non_stronghold_location_on_rock(game).is_some())
        {
            return Some(Rc::new(FlightDiscoveryUsed::new(game, self.faction(game))));
        }

        None
    }

    /// The best place within reach for `battalion` to move to from `location`, in order of
    /// preference. Second-best places are the Polar Sink and nearby rock. With `must_move`, any
    /// valid target will do as a last resort.
    pub fn most_suitable_nearby_location(&self, game: &Game, location: Location, battalion: &Battalion, include_second_best_locations: bool, must_move: bool) -> Option<Location> {
        let best = self
            .nearby_stronghold_of_winning_opponent(game, location, battalion, false)
            .or_else(|| self.nearby_stronghold_of_winning_opponent(game, location, battalion, true))
            .or_else(|| self.vacant_and_safe_nearby_stronghold(game, location, battalion))
            .or_else(|| self.nearby_stronghold_of_almost_winning_opponent(game, location, battalion, false))
            .or_else(|| self.nearby_stronghold_of_almost_winning_opponent(game, location, battalion, true))
            .or_else(|| self.weak_and_safe_nearby_stronghold(game, location, battalion))
            .or_else(|| if self.last_turn(game) { None } else { self.best_safe_and_nearby_resources(game, location, battalion, false) })
            .or_else(|| self.unthreatened_and_safe_nearby_stronghold(game, location, battalion))
            .or_else(|| self.winnable_nearby_stronghold(game, location, battalion))
            .or_else(|| {
                if self.last_turn(game) {
                    None
                } else {
                    self.best_safe_and_nearby_discovery(game, location, battalion, false)
                        .or_else(|| self.best_safe_and_nearby_resources(game, location, battalion, true))
                }
            });
        if best.is_some() {
            return best;
        }

        if include_second_best_locations {
            if !self.last_turn(game) && self.within_range(game, location, game.map.polar_sink(), battalion) {
                return Some(game.map.polar_sink());
            }

            if location != game.map.polar_sink() {
                let rock = game.map.locations(false).find(|&l| {
                    game.is_protected_from_storm(l)
                        && self.within_range(game, location, l, battalion)
                        && self.not_occupied_by_others(game, l.territory())
                        && l.territory() != location.territory()
                });
                if rock.is_some() {
                    return rock;
                }
            }
        }

        if !must_move {
            return None;
        }

        PlacementEvent::valid_targets(game, self.player, location, battalion)
            .into_iter()
            .find(|l| self.ally_doesnt_block(game, l.territory()) && *l != location)
    }
}

/// Forces the bot decided to move, and where from and to.
pub struct MovedBattalion {
    pub from: Location,
    pub to: Location,
    pub battalion: Battalion,
}
