use crate::prelude::*;

/// The rules shared by the events that place forces: their force lists and move checks.
pub struct PlacementEvent;

impl PlacementEvent {
    pub fn force_locations_string(g: &Game, force_locations: &IndexMap<Location, Battalion>) -> String {
        force_locations
            .iter()
            .map(|(k, v)| format!("{}:{}|{}", g.board.map.location_lookup().get_id(Some(*k)), v.amount_of_forces, v.amount_of_special_forces))
            .collect::<Vec<_>>()
            .join(",")
    }

    /// An amount in a force list. Panics on text that isn't a number, as C# threw. The JSON
    /// reader has refused amounts outside 0 to 65535 (`wire::force_list`).
    pub fn parse_force_amount(s: &str) -> u16 {
        let n = crate::parse::int_or_panic(s);
        u16::try_from(n).expect("an amount of forces the event JSON reader accepted")
    }

    pub fn parse_force_locations(g: &Game, f: Faction, force_locations: &str) -> IndexMap<Location, Battalion> {
        let mut result: IndexMap<Location, Battalion> = IndexMap::new();
        for (location, amount_of_normal_forces, amount_of_normal_special_forces) in PlacementEvent::parse_entries(g, force_locations) {
            result.insert_new(location, Battalion::new(f, amount_of_normal_forces, amount_of_normal_special_forces, location));
        }

        result
    }

    /// The entries of a force list, skipping unknown locations.
    fn parse_entries(g: &Game, force_locations: &str) -> Vec<(Location, u16, u16)> {
        let mut result = Vec::new();
        // forceLocations.Length <= 0 (null is handled by the caller, which throws there)
        if force_locations.len() == 0 {
            return result;
        }
        for location_amount_pair in force_locations.split(',') {
            let location_and_amounts: Vec<&str> = location_amount_pair.split(':').collect();
            let location = g.board.map.location_lookup().find(crate::parse::int_or_panic(location_and_amounts[0]));
            let Some(location) = location else { continue };
            let amounts: Vec<&str> = location_and_amounts[1].split('|').collect();
            result.push((location, PlacementEvent::parse_force_amount(amounts[0]), PlacementEvent::parse_force_amount(amounts[1])));
        }

        result
    }

    /// `ForceLocations` of a plain placement event submitted by `by`: the list `s` parsed for
    /// the submitter's faction. Panics without a player or a list, as C# threw.
    pub fn force_locations_by(game: &Game, by: By, s: &Option<String>) -> IndexMap<Location, Battalion> {
        let faction = game.player(by.player()).faction;
        PlacementEvent::parse_force_locations(game, faction.expect("faction"), s.as_deref().expect("field force locations"))
    }

    /// The forces and special forces in the list `s`, whoever's they are. Panics without a list.
    pub fn force_amounts(game: &Game, s: &Option<String>) -> (i32, i32) {
        let mut amounts: IndexMap<Location, (u16, u16)> = IndexMap::new();
        for (location, amount, special_amount) in PlacementEvent::parse_entries(game, s.as_deref().expect("field force locations")) {
            amounts.insert_new(location, (amount, special_amount));
        }
        (amounts.values().map(|a| i32::from(a.0)).sum(), amounts.values().map(|a| i32::from(a.1)).sum())
    }

    /// The checks of a move (`PlacementEvent.ValidateMove`) by `by` of `force_locations` to `to`.
    /// A pass is checked by the caller.
    pub fn validate_move(game: &Game, by: By, to: Option<Location>, force_locations: &IndexMap<Location, Battalion>, as_advisors: bool) -> Result<(), Rejection> {
        // The player may be unset; it is read only where needed.
        let p = by.player;
        let pl = || game.player(p.expect("player"));

        let force_amount = force_locations.values().map(|b| b.forces()).sum::<i32>();
        let too_many_forces = force_locations.iter().any(|(l, b)| b.forces() > pl().forces_in(Some(*l)));
        if too_many_forces {
            return Err(Rejection(express!("Invalid number of ", pl().force())));
        }

        let special_force_amount = force_locations.values().map(|b| b.special_forces()).sum::<i32>();
        let too_many_special_forces = force_locations.iter().any(|(l, b)| b.special_forces() > pl().special_forces_in(Some(*l)));
        if too_many_special_forces {
            return Err(Rejection(express!("Invalid number of ", pl().special_force())));
        }

        if force_amount == 0 && special_force_amount == 0 {
            return Err(Rejection(express!("No forces selected")));
        }

        let Some(to) = to else {
            return Err(Rejection(express!("To not selected")));
        };
        let valid_targets = PlacementEvent::valid_targets_for_forces(game, p.expect("player"), force_locations);
        if !valid_targets.contains(&to) {
            return Err(Rejection(express!("Invalid To location")));
        }
        if as_advisors && !(pl().is(Faction::BeneGesserit) && game.applicable(Rule::BeneGesseritAdvisors)) {
            return Err(Rejection(express!("You can't move as advisors")));
        }

        if by.is(Faction::BeneGesserit) {
            if as_advisors && !PlacementEvent::may_move_as_advisors(game, p.expect("player"), to.territory(), special_force_amount > 0) {
                return Err(Rejection(express!("You can't be advisors there")));
            }
            if !as_advisors && !PlacementEvent::may_move_as_bene_gesserit_fighters(game, p.expect("player"), to.territory()) {
                return Err(Rejection(express!("You can't be fighters there")));
            }
        }

        let can_move_from_two_territories = game.turn.current_planetology.is_some_and(|pl| pl.move_from_two_territories() && pl.initiator == by.faction);
        let number_of_selected_territories = force_locations
            .iter()
            .filter(|kvp| kvp.1.total_amount_of_forces() > 0)
            .map(|fl| fl.0.territory())
            .distinct()
            .len();
        if number_of_selected_territories > 1 && !can_move_from_two_territories {
            return Err(Rejection(express!("You can't move from two territories at the same time")));
        }
        if number_of_selected_territories > 2 {
            return Err(Rejection(express!("You can't move from more than two territories at the same time")));
        }

        if force_locations.keys().any(|l| game.is_in_storm(*l)) {
            return Err(Rejection(express!("Cannot move out of a storm")));
        }

        if by.is(Faction::Richese)
            && force_locations.values().any(|v| v.amount_of_special_forces != 0)
            && game.has_low_threshold(Faction::Richese)
        {
            return Err(Rejection(express!("Low Threshold prevents you from moving your No-Field")));
        }

        Ok(())
    }

    pub fn may_move_as_advisors(g: &Game, p: PlayerId, to: Territory, are_advisors: bool) -> bool {
        let player = g.player(p);
        player.is(Faction::BeneGesserit)
            && g.applicable(Rule::BeneGesseritAdvisors)
            && (player.special_forces_in_territory(Some(to)) > 0
                || (are_advisors && player.forces_in_territory(Some(to)) == 0 && g.any_forces_in(to)))
    }

    pub fn may_move_as_bene_gesserit_fighters(game: &Game, p: PlayerId, to: Territory) -> bool {
        let player = game.player(p);
        player.is(Faction::BeneGesserit) && player.special_forces_in_territory(Some(to)) == 0
    }

    pub fn valid_targets(g: &Game, p: PlayerId, location: Location, battalion: &Battalion) -> Vec<Location> {
        let mut dict: IndexMap<Location, Battalion> = IndexMap::new();
        dict.insert_new(location, battalion.clone());

        PlacementEvent::valid_targets_for_forces(g, p, &dict)
    }

    pub fn valid_targets_for_forces(g: &Game, p: PlayerId, forces: &IndexMap<Location, Battalion>) -> Vec<Location> {
        let mut result: Option<Vec<Location>> = None;

        // The targets are computed in the loop; the Intersect chain is lazy, but it
        // only reads lists that nothing changes afterwards.
        let moved: Vec<Battalion> = forces.values().cloned().collect();
        for (key, _) in forces.iter().filter(|b| (b.1.amount_of_forces + b.1.amount_of_special_forces) > 0) {
            let target = PlacementEvent::valid_move_to_targets(g, p, *key, &moved);

            result = Some(match result {
                None => target,
                Some(r) => crate::iter::unique(r.into_iter().filter(|l| target.contains(l))).collect(),
            });
        }

        match result {
            Some(r) => r,
            None => Vec::new(),
        }
    }

    pub fn valid_move_to_targets(g: &Game, p: PlayerId, from: Location, moved: &[Battalion]) -> Vec<Location> {
        let player = g.player(p);
        if player.is(Faction::Emperor) && from.as_homeworld().is_some() && g.applicable(Rule::EmperorSpecialForces) {
            let hw = from.as_homeworld().expect("as homeworld");
            if hw.world() == World::Emperor {
                let salusa_secundus = g.board.map.get_home_world(World::SalusaSecundus);
                if let Some(salusa_secundus) = salusa_secundus {
                    return vec![salusa_secundus.location()];
                }
            }

            if hw.world() == World::SalusaSecundus {
                let emperor = g.board.map.get_home_world(World::Emperor);
                if let Some(emperor) = emperor {
                    return vec![emperor.location()];
                }
            }
        } else {
            let maximum_distance = g.determine_maximum_move_distance(p, moved);
            let may_move_into_storm =
                player.faction == Some(Faction::Fremen) && g.applicable(Rule::FremenMayMoveIntoStorm) && g.applicable(Rule::FremenStormLosses);
            return g.board.map.find_neighbours(from, maximum_distance, may_move_into_storm, player.faction.expect("faction"), g, true);
        }

        Vec::new()
    }

    pub fn valid_movement_sources(g: &Game, p: PlayerId) -> Vec<Territory> {
        let player = g.player(p);
        let mut res: Vec<Territory> = g
            .board.map
            .territories(false)
            .filter(|t| t.locations().iter().any(|l| !Sector::in_storm(l.sector(&g.board.map), g.board.sector_in_storm) && player.any_forces_in(Some(*l)) > 0))
            .collect();
        if player.is(Faction::Emperor) && g.applicable(Rule::EmperorSpecialForces) {
            let salusa_secundus = g.board.map.get_home_world(World::SalusaSecundus);
            if let Some(salusa_secundus) = salusa_secundus {
                if player.any_forces_in(Some(salusa_secundus.location())) > 0 {
                    res.push(salusa_secundus.location().territory());
                }
            }

            let emperor = g.board.map.get_home_world(World::Emperor);
            if let Some(emperor) = emperor {
                if player.any_forces_in(Some(emperor.location())) > 0 {
                    res.push(emperor.location().territory());
                }
            }
        }

        res
    }

}
