use crate::prelude::*;

/// Holds the static members of `PlacementEvent`. Its fields are inlined into every derived event struct;
/// its instance members are the default methods of [`PlacementEventMethods`].
pub struct PlacementEvent;

/// The instance members of placement events. Every placement event implements this trait
/// (generated in its module); members an event overrides are delegated to its inherent method.
pub trait PlacementEventMethods: PassableGameEventMethods {
    /// The inherited field `_toId` (`self.to_id` on the event struct).
    fn field_to_id(&self) -> &i32;

    /// The inherited field `_forceLocations` (`self.force_locations` on the event struct).
    fn field_force_locations(&self) -> &Option<String>;

    /// The inherited field `_toId`, for the `To` setter.
    fn field_to_id_mut(&mut self) -> &mut i32;

    /// The inherited field `_forceLocations`, for the `ForceLocations` setter.
    fn field_force_locations_mut(&mut self) -> &mut Option<String>;

    fn to(&self, game: &Game) -> Option<Location> {
        game.map.location_lookup().find(*self.field_to_id())
    }

    fn set_to(&mut self, game: &Game, value: Option<Location>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        *self.field_to_id_mut() = game.map.location_lookup().get_id(value);
    }

    fn force_locations(&self, game: &Game) -> NetDictionary<Location, Battalion> {
        // ParseForceLocations(Game, Player.Faction, _forceLocations): Player is dereferenced
        // first; a null string throws at `.Length`.
        let faction = game.player(self.player()).faction;
        let s = self.field_force_locations().as_deref().nre();
        PlacementEvent::parse_force_locations(game, faction, s)
    }

    fn set_force_locations(&mut self, game: &Game, value: &NetDictionary<Location, Battalion>) {
        *self.field_force_locations_mut() = Some(PlacementEvent::force_locations_string(game, value));
    }

    fn total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        self.forces_added_to_location(game) + self.special_forces_added_to_location(game)
    }

    fn forces_added_to_location(&self, game: &Game) -> i32 {
        // ForceLocations != null ? ... : 0 (the dictionary is never null; parsed twice as in C#)
        let _ = self.force_locations(game);
        dotnet_compat::linq::sum_i32(self.force_locations(game).values().map(|b| b.amount_of_forces))
    }

    fn special_forces_added_to_location(&self, game: &Game) -> i32 {
        let _ = self.force_locations(game);
        dotnet_compat::linq::sum_i32(self.force_locations(game).values().map(|b| b.amount_of_special_forces))
    }

    fn validate_move(&self, game: &Game, as_advisors: bool) -> Option<Message> {
        if *self.field_passed() {
            return None;
        }

        // var p = Player (may be null; dereferenced only where C# does)
        let p = self.player_opt();
        let pl = || game.player(p.nre());

        // ForceLocations is re-parsed on every read in C#; parsing has no side effects,
        // and an exception would come from the first read, so one parse per read site is
        // kept only where the first read happens.
        let force_locations = self.force_locations(game);

        let force_amount = dotnet_compat::linq::sum_i32(force_locations.values().map(|b| b.amount_of_forces));
        let too_many_forces = force_locations.iter().any(|(l, b)| b.amount_of_forces > pl().forces_in(Some(*l)));
        if too_many_forces {
            return Some(express!("Invalid number of ", pl().force()));
        }

        let special_force_amount = dotnet_compat::linq::sum_i32(force_locations.values().map(|b| b.amount_of_special_forces));
        let too_many_special_forces = force_locations.iter().any(|(l, b)| b.amount_of_special_forces > pl().special_forces_in(Some(*l)));
        if too_many_special_forces {
            return Some(express!("Invalid number of ", pl().special_force()));
        }

        if force_amount == 0 && special_force_amount == 0 {
            return Some(express!("No forces selected"));
        }

        if self.to(game).is_none() {
            return Some(express!("To not selected"));
        }
        let valid_targets = PlacementEvent::valid_targets_game_player_dictionary_location_battalion(game, p.nre(), &force_locations);
        if !valid_targets.contains(&self.to(game).nre()) {
            return Some(express!("Invalid To location"));
        }
        if as_advisors && !(pl().is(Faction::BeneGesserit) && game.applicable(Rule::BeneGesseritAdvisors)) {
            return Some(express!("You can't move as advisors"));
        }

        if self.initiator() == Faction::BeneGesserit {
            if as_advisors && !PlacementEvent::may_move_as_advisors(game, p.nre(), self.to(game).nre().territory(), special_force_amount > 0) {
                return Some(express!("You can't be advisors there"));
            }
            if !as_advisors && !PlacementEvent::may_move_as_bene_gesserit_fighters(game, p.nre(), self.to(game).nre().territory()) {
                return Some(express!("You can't be fighters there"));
            }
        }

        let can_move_from_two_territories = game.current_planetology.is_some()
            && game.current_planetology.as_ref().nre().move_from_two_territories()
            && game.current_planetology.as_ref().nre().initiator() == self.initiator();
        let number_of_selected_territories = force_locations
            .iter()
            .filter(|kvp| kvp.1.total_amount_of_forces() > 0)
            .map(|fl| fl.0.territory())
            .cs_distinct()
            .len();
        if number_of_selected_territories > 1 && !can_move_from_two_territories {
            return Some(express!("You can't move from two territories at the same time"));
        }
        if number_of_selected_territories > 2 {
            return Some(express!("You can't move from more than two territories at the same time"));
        }

        if force_locations.keys().any(|l| game.is_in_storm(*l)) {
            return Some(express!("Cannot move out of a storm"));
        }

        if self.initiator() == Faction::Richese
            && force_locations.values().any(|v| v.amount_of_special_forces != 0)
            && game.has_low_threshold(Faction::Richese)
        {
            return Some(express!("Low Threshold prevents you from moving your No-Field"));
        }

        None
    }

}

impl PlacementEvent {
    pub fn force_locations_string(g: &Game, force_locations: &NetDictionary<Location, Battalion>) -> String {
        force_locations
            .iter()
            .map(|(k, v)| format!("{}:{}|{}", g.map.location_lookup().get_id(Some(*k)), v.amount_of_forces, v.amount_of_special_forces))
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn parse_force_locations(g: &Game, f: Faction, force_locations: &str) -> NetDictionary<Location, Battalion> {
        let mut result: NetDictionary<Location, Battalion> = NetDictionary::new();
        // forceLocations.Length <= 0 (null is handled by the caller, which throws there)
        if force_locations.len() == 0 {
            return result;
        }
        for location_amount_pair in force_locations.split(',') {
            let location_and_amounts: Vec<&str> = location_amount_pair.split(':').collect();
            let location = g.map.location_lookup().find(dotnet_compat::parse::convert_to_int32(Some(*cs::index_array(&location_and_amounts, 0))));
            let Some(location) = location else { continue };
            let amounts: Vec<&str> = cs::index_array(&location_and_amounts, 1).split('|').collect();
            let amount_of_normal_forces = dotnet_compat::parse::convert_to_int32(Some(*cs::index_array(&amounts, 0)));
            let amount_of_normal_special_forces = dotnet_compat::parse::convert_to_int32(Some(*cs::index_array(&amounts, 1)));
            result.add_with(
                location,
                Battalion::new(f, amount_of_normal_forces, amount_of_normal_special_forces, location),
                |k| ExpressionElement::from(*k).to_cs_string(),
            );
        }

        result
    }

    pub fn may_move_as_advisors(g: &Game, p: PlayerIndex, to: Territory, are_advisors: bool) -> bool {
        let player = g.player(p);
        player.is(Faction::BeneGesserit)
            && g.applicable(Rule::BeneGesseritAdvisors)
            && (player.special_forces_in_territory(Some(to)) > 0
                || (are_advisors && player.forces_in_territory(Some(to)) == 0 && g.any_forces_in(to)))
    }

    pub fn may_move_as_bene_gesserit_fighters(game: &Game, p: PlayerIndex, to: Territory) -> bool {
        let player = game.player(p);
        player.is(Faction::BeneGesserit) && player.special_forces_in_territory(Some(to)) == 0
    }

    pub fn valid_targets(g: &Game, p: PlayerIndex, location: Location, battalion: &Battalion) -> Vec<Location> {
        let mut dict: NetDictionary<Location, Battalion> = NetDictionary::new();
        dict.add_with(location, battalion.clone(), |k| ExpressionElement::from(*k).to_cs_string());

        PlacementEvent::valid_targets_game_player_dictionary_location_battalion(g, p, &dict)
    }

    pub fn valid_targets_game_player_dictionary_location_battalion(g: &Game, p: PlayerIndex, forces: &NetDictionary<Location, Battalion>) -> Vec<Location> {
        let mut result: Option<Vec<Location>> = None;

        // The targets are computed in the loop; the Intersect chain is lazy, but it
        // only reads lists that nothing changes afterwards.
        let moved: Vec<Battalion> = forces.values().cloned().collect();
        for (key, _) in forces.iter().filter(|b| (b.1.amount_of_forces + b.1.amount_of_special_forces) > 0) {
            let target = PlacementEvent::valid_move_to_targets(g, p, *key, &moved);

            result = Some(match result {
                None => target,
                Some(r) => dotnet_compat::linq::intersect(r, target),
            });
        }

        match result {
            Some(r) => r,
            None => Vec::new(),
        }
    }

    pub fn valid_move_to_targets(g: &Game, p: PlayerIndex, from: Location, moved: &[Battalion]) -> Vec<Location> {
        let player = g.player(p);
        if player.is(Faction::Emperor) && from.as_homeworld().is_some() && g.applicable(Rule::EmperorSpecialForces) {
            let hw = from.as_homeworld().nre();
            if hw.world() == World::Emperor {
                let salusa_secundus = g.map.get_home_world(World::SalusaSecundus);
                if let Some(salusa_secundus) = salusa_secundus {
                    return vec![salusa_secundus.location()];
                }
            }

            if hw.world() == World::SalusaSecundus {
                let emperor = g.map.get_home_world(World::Emperor);
                if let Some(emperor) = emperor {
                    return vec![emperor.location()];
                }
            }
        } else {
            let maximum_distance = g.determine_maximum_move_distance(p, moved);
            let may_move_into_storm =
                player.faction == Faction::Fremen && g.applicable(Rule::FremenMayMoveIntoStorm) && g.applicable(Rule::FremenStormLosses);
            return g.map.find_neighbours(from, maximum_distance, may_move_into_storm, player.faction, g, true);
        }

        Vec::new()
    }

    pub fn valid_movement_sources(g: &Game, p: PlayerIndex) -> Vec<Territory> {
        let player = g.player(p);
        let mut res: Vec<Territory> = g
            .map
            .territories(false)
            .filter(|t| t.locations().iter().any(|l| l.sector(&g.map) != g.sector_in_storm && player.any_forces_in(Some(*l)) > 0))
            .collect();
        if player.is(Faction::Emperor) && g.applicable(Rule::EmperorSpecialForces) {
            let salusa_secundus = g.map.get_home_world(World::SalusaSecundus);
            if let Some(salusa_secundus) = salusa_secundus {
                if player.any_forces_in(Some(salusa_secundus.location())) > 0 {
                    res.push(salusa_secundus.location().territory());
                }
            }

            let emperor = g.map.get_home_world(World::Emperor);
            if let Some(emperor) = emperor {
                if player.any_forces_in(Some(emperor.location())) > 0 {
                    res.push(emperor.location().territory());
                }
            }
        }

        res
    }

}
