//! Hazards: the move decisions read and reset the shipment state (`DecidedShipmentAction`,
//! `DecidedShipment`, `FinalDestination`, hazards 16.8), and the Knowledge helpers they call draw,
//! so the order of the checks is kept exactly (hazards 16.5).

use crate::prelude::*;

impl ClassicBot {
    pub fn determine_move(&mut self, game: &Game) -> Option<Rc<Move>> {

        if game.in_guild_cunning_shipment {
            let mut m = Move::new(game, self.faction(game));
            m.passed = true;
            return Some(Rc::new(m));
        }

        let moved = self.determine_moved_battalion(game, true);

        let Some(moved) = moved else {
            let mut m = Move::new(game, self.faction(game));
            m.passed = true;
            return Some(Rc::new(m));
        };

        let mut forces: NetDictionary<Location, Battalion> = NetDictionary::new();
        forces.add_with(moved.from, moved.battalion.clone(), |k| ExpressionElement::from(*k).to_cs_string());

        let as_advisors =
            self.faction(game) == Faction::BeneGesserit && game.player(self.player).special_forces_in_territory(Some(moved.to.territory())) > 0;
        // Object initializer, in C# order.
        let mut result = Move::new(game, self.faction(game));
        result.passed = false;
        result.set_to(game, Some(moved.to));
        result.set_force_locations(game, &forces);
        result.as_advisors = as_advisors;

        let validation_error = result.validate(game);
        if validation_error.is_some() {
            Self::log_info_message(validation_error.as_ref());
            return None;
        }

        Some(Rc::new(result))
    }

    pub fn determine_caravan(&mut self, game: &Game) -> Option<Rc<Caravan>> {

        if self.forces_on_planet(game).keys().filter(|l| !l.is_stronghold()).count() <= 1 {
            return None;
        }

        let moved = self.determine_moved_battalion(game, false)?;

        let mut forces: NetDictionary<Location, Battalion> = NetDictionary::new();
        forces.add_with(moved.from, moved.battalion.clone(), |k| ExpressionElement::from(*k).to_cs_string());

        let as_advisors =
            self.faction(game) == Faction::BeneGesserit && game.player(self.player).special_forces_in_territory(Some(moved.to.territory())) > 0;
        // Object initializer, in C# order.
        let mut result = Caravan::new(game, self.faction(game));
        result.passed = false;
        result.set_to(game, Some(moved.to));
        result.set_force_locations(game, &forces);
        result.as_advisors = as_advisors;

        self.decided_shipment_action = ShipmentDecision::None;

        // IsValid and the log argument both run Validate.
        if !result.is_valid(game) {
            let error = result.validate(game);
            Self::log_info_message(error.as_ref());
            return None;
        }

        Some(Rc::new(result))
    }

    pub fn determine_moved_battalion(&mut self, game: &Game, include_low_priority_moves: bool) -> Option<MovedBattalion> {
        let winning = self.i_am_winning(game);
        // LogInfo("DetermineMove(). AllIn: {0}, Winning: {1}.", LastTurn, winning): LastTurn is pure.
        let _ = self.last_turn(game);

        let me = self.player;
        let faction = self.faction(game);

        if self.decided_shipment_action == ShipmentDecision::StrongholdNearResources {
            // DecidedShipment!.To: a lookup in the shipment's own game, which holds the same location ids.
            let decided = self.decided_shipment.clone().nre();
            let decided_to = decided.to(game).or_null();
            if game.player(me).forces_in_locations.contains_key(&decided_to) {
                let to_move = game
                    .player(me)
                    .forces_in_locations
                    .index(&decided_to)
                    .take(decided.force_amount.wrapping_add(decided.special_force_amount), faction == Faction::Ixian);

                if let Some(final_destination) = self.final_destination
                    && self.within_range(game, decided_to, final_destination, &to_move)
                {
                    let mv = self.construct_move(game, final_destination, decided_to, &to_move);
                    if mv.is_some() {
                        return mv;
                    }
                }
            }
        }

        //Move the biggest battalion threatened by ally presence
        let battalion_threatened_by_ally_presence = self.battalion_that_should_be_moved_due_to_ally_presence(game);
        if let Some(b) = battalion_threatened_by_ally_presence {
            let move_to = self.determine_most_suitable_nearby_location(game, &b, true, true);
            if let Some(move_to) = move_to {
                let mv = self.construct_move(game, move_to, b.location, &b.battalion);
                if mv.is_some() {
                    return mv;
                }
            }
        }

        //Move the biggest battalion threatened by storm
        if !self.last_turn(game) && !winning {
            let biggest_battalion_threatened_by_storm = self.biggest_battalion_threatened_by_storm_without_spice(game);
            if let Some(b) = biggest_battalion_threatened_by_storm {
                let move_to = self.determine_most_suitable_nearby_location(game, &b, true, false);
                if let Some(move_to) = move_to {
                    let mv = self.construct_move(game, move_to, b.location, &b.battalion);
                    if mv.is_some() {
                        return mv;
                    }
                }
            }
        }

        //Move from spiceless non-stronghold location (if not needed for atomics) to safe stronghold or spice or rock
        let biggest_battalion_in_spiceless_non_stronghold = self.biggest_battalion_in_spiceless_non_stronghold_location_on_rock(game);
        if let Some(b) = biggest_battalion_in_spiceless_non_stronghold {
            let move_to = self.determine_most_suitable_nearby_location(game, &b, false, false);
            if let Some(move_to) = move_to {
                let mv = self.construct_move(game, move_to, b.location, &b.battalion);
                if mv.is_some() {
                    return mv;
                }
            }
        }

        //Move from useless location (if not needed for atomics) to safe stronghold or spice or rock
        let biggest_battalion_in_spiceless_non_stronghold_not_near_stronghold =
            self.biggest_battalion_in_spiceless_non_stronghold_location_in_sand_or_not_near_stronghold(game);
        if let Some(b) = biggest_battalion_in_spiceless_non_stronghold_not_near_stronghold {
            let move_to = self.determine_most_suitable_nearby_location(game, &b, true, false);
            if let Some(move_to) = move_to {
                let mv = self.construct_move(game, move_to, b.location, &b.battalion);
                if mv.is_some() {
                    return mv;
                }
            }
        }

        if faction == Faction::BeneGesserit {
            //Move a stack of advisors from a stronghold to a nearby vacant stronghold
            let biggest_stack_of_advisors_in_stronghold_near_vacant_stronghold =
                self.biggest_movable_stack_of_advisors_in_stronghold_near_vacant_stronghold(game);
            if let Some(b) = biggest_stack_of_advisors_in_stronghold_near_vacant_stronghold {
                let move_to = self.vacant_and_safe_nearby_stronghold(game, b.location, &b.battalion);
                if let Some(move_to) = move_to {
                    let mv = self.construct_move(game, move_to, b.location, &b.battalion);
                    if mv.is_some() {
                        return mv;
                    }
                }
            }
        }

        if !include_low_priority_moves {
            return None;
        }

        //Move partly from unthreatened stronghold location with enough forces to nearby vacant stronghold
        if !winning {
            let big_unthreatened_battalion_near_vacant_stronghold =
                self.biggest_large_unthreatened_movable_battalion_in_stronghold_near_vacant_stronghold(game);
            if let Some(b) = big_unthreatened_battalion_near_vacant_stronghold {
                let move_to = self.vacant_and_safe_nearby_stronghold(game, b.location, &b.battalion);
                if let Some(move_to) = move_to {
                    let mv = self.construct_move(game, move_to, b.location, &b.battalion.take_half());
                    if mv.is_some() {
                        return mv;
                    }
                }
            }
        }

        //Move partly from unthreatened stronghold location with enough forces to nearby spice
        if !self.last_turn(game) && !winning {
            let big_unthreatened_battalion_near_spice = self.biggest_large_unthreatened_movable_battalion_in_stronghold_near_spice(game);
            if let Some(b) = big_unthreatened_battalion_near_spice {
                let move_to = self
                    .best_safe_and_nearby_resources(game, b.location, &b.battalion, false)
                    .or_else(|| self.best_safe_and_nearby_resources(game, b.location, &b.battalion, true));

                if let Some(move_to) = move_to {
                    let forces_for_collection = std::cmp::max(
                        2,
                        std::cmp::min(self.determine_forces_needed_for_collection(game, move_to), b.battalion.total_amount_of_forces() / 2),
                    );
                    let mv = self.construct_move(game, move_to, b.location, &b.battalion.take(forces_for_collection, faction == Faction::Ixian));
                    if mv.is_some() {
                        return mv;
                    }
                }
            }
        }

        //Move towards shield wall to detonate family atomics if needed
        // C# parity: `!LastTurn` is tested twice (hazards 18).
        if !self.last_turn(game) && !winning {
            //var locationsNearShieldWall = Map.FindNeighbours(Game.Map.ShieldWall.MiddleLocation, 1, false, Faction, Game.SectorInStorm, null);
            if !self.last_turn(game)
                && (0..=8).contains(&game.sector_in_storm)
                && game.player(me).treachery_cards.iter().any(|c| c.type_() == TreacheryCardType::Metheor)
                && !MetheorPlayed::has_forces_at_or_near_shield_wall(game, me)
            {
                let mut from = Location::NULL;
                let mut to: Option<Location> = None;
                let troop_that_will_blow_shield_wall = self.determine_battalion_that_will_destroy_shield_wall(game, &mut from, &mut to);
                if let Some(troop) = troop_that_will_blow_shield_wall
                    && let Some(to) = to
                {
                    let mv = self.construct_move(game, to, from, &troop);
                    if mv.is_some() {
                        return mv;
                    }
                }
            }
        }

        self.decided_shipment_action = ShipmentDecision::None;
        None
    }

    pub fn construct_move(&self, game: &Game, to: Location, from: Location, battalion: &Battalion) -> Option<MovedBattalion> {
        if self.faction(game) == Faction::Richese && game.has_low_threshold(Faction::Richese) && battalion.amount_of_special_forces != 0 {
            return None;
        }
        Some(MovedBattalion { to, from, battalion: battalion.clone() })
    }

    pub fn determine_battalion_that_will_destroy_shield_wall(&self, game: &Game, from: &mut Location, to: &mut Option<Location>) -> Option<Battalion> {
        let map = &game.map;
        let false_wall_east = |sector: i32| map.false_wall_east().locations().iter().copied().cs_first_where(|l| l.sector(map) == sector);
        let false_wall_east_target = || if game.sector_in_storm != 8 { false_wall_east(8) } else { false_wall_east(7) };

        *from = map.polar_sink();
        *to = Some(false_wall_east_target());
        let mut result = self.find_one_troop_that_can_safely_move(game, *from, to.nre());

        if result.is_none() {
            *from = map.tueks_sietch();
            *to = Some(map.pasty_mesa().locations().iter().copied().cs_first_where(|l| l.sector(map) == 7));
            result = self.find_one_troop_that_can_safely_move(game, *from, to.nre());
        }

        if result.is_none() {
            *from = map.arrakeen();
            *to = Some(false_wall_east_target());
            result = self.find_one_troop_that_can_safely_move(game, *from, to.nre());
        }

        if result.is_none() {
            *from = map.carthag();
            *to = Some(false_wall_east_target());
            result = self.find_one_troop_that_can_safely_move(game, *from, to.nre());
        }

        result
    }

    pub fn determine_flight_used(&self, game: &Game) -> Option<Rc<FlightUsed>> {

        if self.forces_on_planet(game).keys().filter(|l| !l.is_stronghold()).count() > 1
            && dotnet_compat::linq::sum_i32(
                self.forces_on_planet(game).iter().filter(|kvp| !kvp.0.is_stronghold()).map(|lwf| lwf.1.total_amount_of_forces()),
            ) > 3
        {
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

    pub fn determine_most_suitable_nearby_location(&self, game: &Game, battalion_at_location: &BattalionInLocation, include_second_best_locations: bool, must_move: bool) -> Option<Location> {
        self.most_suitable_nearby_location(
            game,
            battalion_at_location.location,
            &battalion_at_location.battalion,
            include_second_best_locations,
            must_move,
        )
    }

    /// `$"{message}: {returnValue}"` formats a location, which has no side effects.
    pub fn log<T>(&self, _game: &Game, message: &str, return_value: T) -> T {
        Self::log_info(message, vec![]);
        return_value
    }

    pub fn most_suitable_nearby_location(&self, game: &Game, location: Location, battalion: &Battalion, include_second_best_locations: bool, must_move: bool) -> Option<Location> {
        let mut result = self.nearby_stronghold_of_winning_opponent(game, location, battalion, false);
        if result.is_some() {
            return self.log(game, "Suitable NearbyStrongholdOfWinningOpponent excluding bots", result);
        }

        result = self.nearby_stronghold_of_winning_opponent(game, location, battalion, true);
        if result.is_some() {
            return self.log(game, "Suitable NearbyStrongholdOfWinningOpponent including bots", result);
        }

        result = self.vacant_and_safe_nearby_stronghold(game, location, battalion);
        if result.is_some() {
            return self.log(game, "Suitable VacantAndSafeNearbyStronghold", result);
        }

        result = self.nearby_stronghold_of_almost_winning_opponent(game, location, battalion, false);
        if result.is_some() {
            return self.log(game, "Suitable NearbyStrongholdOfAlmostWinningOpponent excluding bots", result);
        }

        result = self.nearby_stronghold_of_almost_winning_opponent(game, location, battalion, true);
        if result.is_some() {
            return self.log(game, "Suitable NearbyStrongholdOfAlmostWinningOpponent including bots", result);
        }

        result = self.weak_and_safe_nearby_stronghold(game, location, battalion);
        if result.is_some() {
            return self.log(game, "Suitable WeakAndSafeNearbyStronghold", result);
        }

        if !self.last_turn(game) {
            result = self.best_safe_and_nearby_resources(game, location, battalion, false);
            if result.is_some() {
                return self.log(game, "Suitable BestSafeAndNearbyResources without fighting", result);
            }
        }

        result = self.unthreatened_and_safe_nearby_stronghold(game, location, battalion);
        if result.is_some() {
            return self.log(game, "Suitable UnthreatenedAndSafeNearbyStronghold", result);
        }

        result = self.winnable_nearby_stronghold(game, location, battalion);
        if result.is_some() {
            return self.log(game, "Suitable WinnableNearbyStronghold", result);
        }

        if !self.last_turn(game) {
            result = self.best_safe_and_nearby_discovery(game, location, battalion, false);
            if result.is_some() {
                return self.log(game, "Nearby Discovery", result);
            }

            result = self.best_safe_and_nearby_resources(game, location, battalion, true);
            if result.is_some() {
                return self.log(game, "Suitable BestSafeAndNearbyResources with fighting", result);
            }
        }

        if include_second_best_locations {
            if !self.last_turn(game) && self.within_range(game, location, game.map.polar_sink(), battalion) {
                result = Some(game.map.polar_sink());
                if result.is_some() {
                    return self.log(game, "Suitable - Polar Sink nearby?", result);
                }
            }

            if location != game.map.polar_sink() {
                result = game.map.locations(false).cs_first_or_default_where(|l| {
                    let l = *l;
                    game.is_protected_from_storm(l)
                        && self.within_range(game, location, l, battalion)
                        && self.not_occupied_by_others(game, l.territory())
                        && l.territory() != location.territory()
                });
                if result.is_some() {
                    return self.log(game, "Suitable nearby Rock", result);
                }
            }
        }

        if !must_move {
            return result;
        }

        result = PlacementEvent::valid_targets(game, self.player, location, battalion)
            .into_iter()
            .cs_first_or_default_where(|l| self.ally_doesnt_block(game, l.territory()) && *l != location);

        result
    }
}

pub struct MovedBattalion {
    pub from: Location,
    pub to: Location,
    pub battalion: Battalion,
}
