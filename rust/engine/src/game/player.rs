use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
#[canon(game_field = "Game")]
pub struct Player {
    pub faction: Faction,
    pub seat: i32,
    pub resources: i32,
    pub extortion: i32,
    pub bribes: i32,
    pub resources_after_bidding: i32,
    pub banked_resources: i32,
    pub transferable_resources: i32,
    pub treachery_cards: Vec<TreacheryCard>,
    pub known_cards: NetHashSet<TreacheryCard>,
    pub traitors: Vec<Hero>,
    pub revealed_traitors: NetHashSet<Hero>,
    pub told_traitors: IndexSet<Hero>,
    pub told_non_traitors: IndexSet<Hero>,
    pub known_non_traitors: IndexSet<Hero>,
    pub discarded_traitors: IndexSet<Hero>,
    pub face_dancers: Vec<Hero>,
    pub revealed_face_dancers: IndexSet<Hero>,
    pub told_face_dancers: IndexSet<Hero>,
    pub told_non_face_dancers: IndexSet<Hero>,
    pub leaders: Vec<Leader>,
    pub forces_in_locations: NetDictionary<Location, Battalion>,
    pub ambassadors: Vec<Ambassador>,
    pub predicted_faction: Faction,
    pub predicted_turn: i32,
    pub forces_killed: i32,
    pub special_forces_killed: i32,
    pub total_forces_killed_in_battle: i32,
    pub ally: Faction,
    pub special_karma_power_used: bool,
    pub tech_tokens: Vec<TechToken>,
    pub most_recently_revived_leader: Option<Leader>,
    pub skills_to_choose_from: Vec<LeaderSkill>,
    pub home_worlds: Vec<Homeworld>,
    pub nexus: Faction,
    pub salusa_secundus_homeworld_is_on_low_threshold: bool,
}

impl Player {
    pub fn new(faction: Faction) -> Self {
        Player {
            faction,
            seat: -1,
            resources: 0,
            extortion: 0,
            bribes: 0,
            resources_after_bidding: 0,
            banked_resources: 0,
            transferable_resources: 0,
            treachery_cards: Vec::new(),
            known_cards: NetHashSet::new(),
            traitors: Vec::new(),
            revealed_traitors: NetHashSet::new(),
            told_traitors: IndexSet::new(),
            told_non_traitors: IndexSet::new(),
            known_non_traitors: IndexSet::new(),
            discarded_traitors: IndexSet::new(),
            face_dancers: Vec::new(),
            revealed_face_dancers: IndexSet::new(),
            told_face_dancers: IndexSet::new(),
            told_non_face_dancers: IndexSet::new(),
            leaders: Vec::new(),
            forces_in_locations: NetDictionary::new(),
            ambassadors: Vec::new(),
            predicted_faction: Faction::default(),
            predicted_turn: 0,
            forces_killed: 0,
            special_forces_killed: 0,
            total_forces_killed_in_battle: 0,
            ally: Faction::default(),
            special_karma_power_used: false,
            tech_tokens: Vec::new(),
            most_recently_revived_leader: None,
            skills_to_choose_from: Vec::new(),
            home_worlds: Vec::new(),
            nexus: Faction::None,
            salusa_secundus_homeworld_is_on_low_threshold: false,
        }
    }

    pub fn name(&self, game: &Game) -> String {
        // Game.GetPlayerName(this): `self` is always one of game.players.
        game.get_player_name_player(game.index_of(self).expect("a player of this game"))
    }

    pub fn forces_in_reserve(&self) -> i32 {
                self.home_worlds.iter().map(|hw| self.forces_in(Some(hw.location()))).sum::<i32>()
    }

    pub fn special_forces_in_reserve(&self) -> i32 {
                self.home_worlds.iter().map(|hw| self.special_forces_in(Some(hw.location()))).sum::<i32>()
    }

    pub fn any_forces_in_reserves(&self) -> i32 {
                self.forces_in_reserve() + self.special_forces_in_reserve()
    }

    pub fn forces_on_planet(&self) -> IndexMap<Location, Battalion> {
                // C# shares the live Battalion objects with ForcesInLocations; no caller mutates them,
        // so copies are equivalent.
        self.forces_in_locations.iter().filter(|(k, _)| k.as_homeworld().is_none()).map(|(k, v)| (*k, v.clone())).collect()
    }

    pub fn no_field_is_active(&self) -> bool {
                self.faction == Faction::Richese && self.forces_in_locations.values().any(|b| b.amount_of_special_forces > 0)
    }

    pub fn get_homeworld(&self, special_forces: bool) -> Homeworld {
                *self
            .home_worlds
            .iter()
            .find(|hw| if special_forces { hw.is_home_of_special_forces() } else { hw.is_home_of_normal_forces() })
            .expect("a homeworld for these forces")
    }

    /// Returns the live battalion, not a copy.
    pub fn get_and_create_if_needed(&mut self, location: Location) -> &mut Battalion {
                if !self.forces_in_locations.contains_key(&location) {
            let result = Battalion::new(self.faction, 0, 0, location);
            self.forces_in_locations.insert_new(location, result);
        }

        &mut self.forces_in_locations[&location]
    }

    pub fn change_forces(game: &mut Game, me: PlayerIndex, location: Location, nr_of_forces: i32) {
        let me_ = game.player_mut(me);
        let result = me_.get_and_create_if_needed(location);
        result.change_forces(nr_of_forces);
        if result.total_amount_of_forces() == 0 {
            me_.forces_in_locations.remove(&location);
        }
    }

    pub fn change_special_forces(game: &mut Game, me: PlayerIndex, location: Location, nr_of_forces: i32) {
        let me_ = game.player_mut(me);
        let result = me_.get_and_create_if_needed(location);
        result.change_special_forces(nr_of_forces);
        if result.total_amount_of_forces() == 0 {
            me_.forces_in_locations.remove(&location);
        }
        Player::check_if_emperor_star_threshold_was_passed(game, me);
    }

    pub fn add_forces(game: &mut Game, me: PlayerIndex, location: Location, nr_of_forces: i32, from_reserves: bool) {
        if from_reserves {
            let p = game.player(me);
            let source_world = p.home_worlds.iter().copied().find(|w| p.forces_in(Some(w.location())) >= nr_of_forces);
            if let Some(source_world) = source_world {
                Player::move_forces(game, me, source_world.location(), location, nr_of_forces);
            }
        } else {
            Player::change_forces(game, me, location, nr_of_forces);
        }
    }

    pub fn add_special_forces(game: &mut Game, me: PlayerIndex, location: Location, nr_of_forces: i32, from_reserves: bool) {
        if from_reserves {
            let p = game.player(me);
            let source_world = p.home_worlds.iter().copied().find(|w| p.special_forces_in(Some(w.location())) >= nr_of_forces);
            if let Some(source_world) = source_world {
                Player::move_special_forces(game, me, source_world.location(), location, nr_of_forces);
            }
        } else {
            Player::change_special_forces(game, me, location, nr_of_forces);
        }
    }

    pub fn add_forces_to_reserves(game: &mut Game, me: PlayerIndex, nr_of_forces: i32) {
        let source_world = game.player(me).home_worlds.iter().copied().find(|w| w.is_home_of_normal_forces());
        if let Some(source_world) = source_world {
            Player::change_forces(game, me, source_world.location(), nr_of_forces);
        }
    }

    pub fn check_if_emperor_star_threshold_was_passed(game: &mut Game, me: PlayerIndex) {
        let p = game.player(me);
        if p.faction == Faction::Emperor && game.applicable(Rule::EmperorSpecialForces) {
            let home_world = p.home_worlds.iter().copied().find(|hw| hw.world() == World::SalusaSecundus);

            let Some(home_world) = home_world else {
                return;
            };

            let has_less_than_threshold = p.special_forces_in(Some(home_world.location())) < home_world.threshold();

            if has_less_than_threshold && !p.salusa_secundus_homeworld_is_on_low_threshold {
                game.player_mut(me).salusa_secundus_homeworld_is_on_low_threshold = true;
            } else if !has_less_than_threshold && p.salusa_secundus_homeworld_is_on_low_threshold {
                game.player_mut(me).salusa_secundus_homeworld_is_on_low_threshold = false;
            }
        }
    }

    pub fn add_special_forces_to_reserves(game: &mut Game, me: PlayerIndex, nr_of_forces: i32) {
        let source_world = game.player(me).home_worlds.iter().copied().find(|w| w.is_home_of_special_forces());
        if let Some(source_world) = source_world {
            Player::change_special_forces(game, me, source_world.location(), nr_of_forces);
        }
    }

    pub fn remove_forces(game: &mut Game, me: PlayerIndex, location: Location, nr_of_forces: i32) {
        Player::change_forces(game, me, location, -nr_of_forces);
    }

    pub fn remove_special_forces(game: &mut Game, me: PlayerIndex, location: Location, nr_of_forces: i32) {
        Player::change_special_forces(game, me, location, -nr_of_forces);
    }

    pub fn forces_in(&self, location: Option<Location>) -> i32 {
                if let Some(location) = location {
            if let Some(battalion) = self.forces_in_locations.get(&location) {
                return battalion.amount_of_forces;
            }
        }

        0
    }

    pub fn special_forces_in(&self, location: Option<Location>) -> i32 {
                if let Some(location) = location {
            if let Some(battalion) = self.forces_in_locations.get(&location) {
                return battalion.amount_of_special_forces;
            }
        }

        0
    }

    pub fn any_forces_in(&self, location: Option<Location>) -> i32 {
                if let Some(location) = location {
            if let Some(battalion) = self.forces_in_locations.get(&location) {
                return battalion.total_amount_of_forces();
            }
        }

        0
    }

    pub fn occupying_forces_in(&self, t: Option<Territory>) -> i32 {
                match t {
            None => 0,
            Some(t) => t.locations().iter().map(|l| self.occupying_forces_in_location(*l)).sum::<i32>(),
        }
    }

    pub fn forces_in_territory(&self, t: Option<Territory>) -> i32 {
                match t {
            None => 0,
            Some(t) => t.locations().iter().map(|l| self.forces_in(Some(*l))).sum::<i32>(),
        }
    }

    pub fn special_forces_in_territory(&self, t: Option<Territory>) -> i32 {
                match t {
            None => 0,
            Some(t) => t.locations().iter().map(|l| self.special_forces_in(Some(*l))).sum::<i32>(),
        }
    }

    pub fn any_forces_in_territory(&self, t: Option<Territory>) -> i32 {
                match t {
            None => 0,
            Some(t) => t.locations().iter().map(|l| self.any_forces_in(Some(*l))).sum::<i32>(),
        }
    }

    pub fn forces_to_reserves(game: &mut Game, me: PlayerIndex, location: Location) {
        // C# holds the live Battalion object `ForcesInLocations[location]` and reads it after
        // AddForcesToReserves. That object stays in the dictionary in between (adding a
        // non-negative amount never empties a battalion), so reading the dictionary again is
        // the same; a missing entry would mean a detached, emptied battalion, hence the 0.
        let amount_of_forces = game.player(me).forces_in_locations[&location].amount_of_forces;

        Player::add_forces_to_reserves(game, me, amount_of_forces);

        let live_special = |game: &Game| game.player(me).forces_in_locations.get(&location).map_or(0, |b| b.amount_of_special_forces);
        let faction = game.player(me).faction;
        if faction == Faction::BeneGesserit {
            let n = live_special(game);
            Player::add_forces_to_reserves(game, me, n);
        } else if faction != Faction::Richese {
            let n = live_special(game);
            Player::add_special_forces_to_reserves(game, me, n);
        }

        game.player_mut(me).forces_in_locations.remove(&location);
        Player::check_if_emperor_star_threshold_was_passed(game, me);
    }

    pub fn forces_to_reserves_location_int32(game: &mut Game, me: PlayerIndex, location: Location, amount: i32) {
        Player::add_forces_to_reserves(game, me, amount);
        Player::change_forces(game, me, location, -amount);
    }

    pub fn special_forces_to_reserves(game: &mut Game, me: PlayerIndex, location: Location, amount: i32) {
        let faction = game.player(me).faction;
        if faction == Faction::BeneGesserit {
            Player::add_forces_to_reserves(game, me, amount);
        } else if faction != Faction::Richese {
            Player::add_special_forces_to_reserves(game, me, amount);
        }

        Player::change_special_forces(game, me, location, -amount);
    }

    /// Special forces shipped from `location` back to the reserves on `homeworld`. Bene Gesserit
    /// advisors return as fighters, as in [`Player::special_forces_to_reserves`]. Richese's
    /// special force is its No-Field token: it leaves the planet unrevealed.
    pub fn special_forces_back_to_reserves(game: &mut Game, me: PlayerIndex, location: Location, homeworld: Homeworld, amount: i32) {
        if amount <= 0 {
            return;
        }
        match game.player(me).faction {
            Faction::BeneGesserit => {
                Player::change_special_forces(game, me, location, -amount);
                Player::change_forces(game, me, homeworld.location(), amount);
            }
            Faction::Richese => {
                Player::change_special_forces(game, me, location, -amount);
                game.current_no_field_value = -1;
            }
            _ => Player::move_special_forces(game, me, location, homeworld.location(), amount),
        }
    }

    pub fn forces_to_reserves_territory_int32_boolean(game: &mut Game, me: PlayerIndex, t: Territory, amount: i32, special: bool) {
        if amount > 0 {
            let mut to_remove_in_total = amount;
            let mut locations: Vec<Location> = t.locations().to_vec();
            locations.sort_by_cached_key(|l| l.spice_blow_amount()); // OrderBy: stable
            for l in locations {
                let p = game.player(me);
                let forces_in = if special { p.special_forces_in(Some(l)) } else { p.forces_in(Some(l)) };
                if forces_in > 0 {
                    let to_remove_in_this_location = std::cmp::min(forces_in, to_remove_in_total);

                    // C# parity: uses toRemoveInTotal where toRemoveInThisLocation was meant
                    // (hazards §18): the whole remaining amount goes to reserves at every
                    // location, and the removal below is refused when it exceeds the forces there.
                    let faction = game.player(me).faction;
                    if special && faction != Faction::BeneGesserit {
                        Player::add_special_forces_to_reserves(game, me, to_remove_in_total);
                    } else if !special || faction != Faction::Richese {
                        Player::add_forces_to_reserves(game, me, to_remove_in_total);
                    }

                    if special {
                        Player::change_special_forces(game, me, l, -to_remove_in_total);
                    } else {
                        Player::change_forces(game, me, l, -to_remove_in_total);
                    }

                    to_remove_in_total -= to_remove_in_this_location;
                }

                if to_remove_in_total == 0 {
                    break;
                }
            }
        }
    }

    pub fn forces_to_reserves_territory(game: &mut Game, me: PlayerIndex, t: Territory) {
        // Lazy Where: the filter is re-checked after each body ran.
        for &l in t.locations() {
            if game.player(me).any_forces_in(Some(l)) > 0 {
                Player::forces_to_reserves(game, me, l);
            }
        }
    }

    pub fn kill_all_forces(game: &mut Game, me: PlayerIndex, location: Location, in_battle: bool) -> i32 {
        let p = game.player(me);
        if let Some(battalion) = p.forces_in_locations.get(&location) {
            let kill_count = battalion.amount_of_forces;
            let special_kill_count = battalion.amount_of_special_forces;
            let faction = p.faction;

            let p = game.player_mut(me);
            p.forces_killed += kill_count;

            if faction == Faction::BeneGesserit {
                p.forces_killed += special_kill_count;
            } else {
                p.special_forces_killed += special_kill_count;
            }

            p.forces_in_locations.remove(&location);
            Player::check_if_emperor_star_threshold_was_passed(game, me);

            if in_battle {
                let p = game.player_mut(me);
                p.total_forces_killed_in_battle += kill_count + special_kill_count;
            }

            return kill_count + special_kill_count;
        }

        0
    }

    pub fn kill_all_forces_territory_boolean(game: &mut Game, me: PlayerIndex, t: Territory, in_battle: bool) {
        // Lazy Where: the filter is re-checked after each body ran.
        for &l in t.locations() {
            if game.player(me).any_forces_in(Some(l)) > 0 {
                Player::kill_all_forces(game, me, l, in_battle);
            }
        }
    }

    pub fn kill_forces(game: &mut Game, me: PlayerIndex, location: Location, amount_of_forces: i32, amount_of_special_forces: i32, in_battle: bool) -> i32 {
        Player::change_forces(game, me, location, -amount_of_forces);
        Player::change_special_forces(game, me, location, -amount_of_special_forces);

        let p = game.player_mut(me);
        p.forces_killed += amount_of_forces;

        if p.faction == Faction::BeneGesserit {
            p.forces_killed += amount_of_special_forces;
        } else {
            p.special_forces_killed += amount_of_special_forces;
        }

        if in_battle {
            p.total_forces_killed_in_battle += amount_of_forces + amount_of_special_forces;
        }

        amount_of_forces + amount_of_special_forces
    }

    pub fn kill_forces_territory_int32_boolean_boolean(game: &mut Game, me: PlayerIndex, t: Territory, amount: i32, special: bool, in_battle: bool) {
        let mut to_kill = amount;
        let mut locations: Vec<Location> = t.locations().to_vec();
        locations.sort_by_cached_key(|l| l.spice_blow_amount()); // OrderBy: stable
        for l in locations {
            let p = game.player(me);
            let forces_in = if special { p.special_forces_in(Some(l)) } else { p.forces_in(Some(l)) };
            if forces_in > 0 {
                let to_be_killed = std::cmp::min(forces_in, to_kill);
                if special {
                    Player::kill_forces(game, me, l, 0, to_be_killed, in_battle);
                } else {
                    Player::kill_forces(game, me, l, to_be_killed, 0, in_battle);
                }
                to_kill -= to_be_killed;
            }

            if to_kill == 0 {
                break;
            }
        }
    }

    pub fn revive_forces(game: &mut Game, me: PlayerIndex, amount: i32) {
        let p = game.player_mut(me);
        p.forces_killed -= amount;
        Player::add_forces_to_reserves(game, me, amount);
    }

    pub fn revive_special_forces(game: &mut Game, me: PlayerIndex, amount: i32) {
        let p = game.player_mut(me);
        p.special_forces_killed -= amount;
        Player::add_special_forces_to_reserves(game, me, amount);
    }

    pub fn ship_forces(game: &mut Game, me: PlayerIndex, to: Location, amount: i32) {
        Player::add_forces(game, me, to, amount, true);
    }

    pub fn ship_forces_location_homeworld_int32(game: &mut Game, me: PlayerIndex, to: Location, from: Homeworld, amount: i32) {
        Player::move_forces(game, me, from.location(), to, amount);
    }

    pub fn ship_special_forces(game: &mut Game, me: PlayerIndex, to: Location, amount: i32) {
        if game.player(me).faction == Faction::Richese {
            Player::change_special_forces(game, me, to, amount);
        } else {
            Player::add_special_forces(game, me, to, amount, true);
        }
    }

    pub fn ship_special_forces_location_homeworld_int32(game: &mut Game, me: PlayerIndex, to: Location, from: Homeworld, amount: i32) {
        if game.player(me).faction == Faction::Richese {
            Player::change_special_forces(game, me, to, amount);
        } else {
            Player::move_special_forces(game, me, from.location(), to, amount);
        }
    }

    pub fn ship_advisors(game: &mut Game, me: PlayerIndex, l: Location, amount: i32) {
        Player::add_forces_to_reserves(game, me, -amount);
        Player::change_special_forces(game, me, l, amount);
    }

    pub fn move_forces(game: &mut Game, me: PlayerIndex, from: Location, to: Location, amount: i32) {
        Player::change_forces(game, me, from, -amount);
        Player::change_forces(game, me, to, amount);
    }

    pub fn move_special_forces(game: &mut Game, me: PlayerIndex, from: Location, to: Location, amount: i32) {
        Player::change_special_forces(game, me, from, -amount);
        Player::change_special_forces(game, me, to, amount);
    }

    pub fn flip_forces(game: &mut Game, me: PlayerIndex, l: Location, as_advisors: bool) {
        if game.player(me).faction == Faction::BeneGesserit {
            if as_advisors {
                let number_of_forces = game.player(me).forces_in(Some(l));
                Player::change_forces(game, me, l, -number_of_forces);
                Player::change_special_forces(game, me, l, number_of_forces);
            } else {
                let number_of_special_forces = game.player(me).special_forces_in(Some(l));
                Player::change_forces(game, me, l, number_of_special_forces);
                Player::change_special_forces(game, me, l, -number_of_special_forces);
            }
        }
    }

    pub fn flip_forces_territory_boolean(game: &mut Game, me: PlayerIndex, t: Territory, as_advisors: bool) {
        // `t.Locations` are objects of t's map: after an undo, t can come from an event of
        // the other game, and a battalion created for such a location keeps it as its key.
        for l in t.locations().iter().copied() {
            Player::flip_forces(game, me, l, as_advisors);
        }
    }

    pub fn occupying_forces_in_location(&self, l: Location) -> i32 {
                self.forces_in(Some(l)) + (if self.faction == Faction::BeneGesserit { 0 } else { self.special_forces_in(Some(l)) })
    }

    pub fn occupies(&self, game: &Game, l: Option<Location>) -> bool {
        match l {
            None => false,
            Some(l) => {
                self.occupying_forces_in_location(l) > 0
                    || (self.ally == Faction::Ecaz
                        && self.occupying_forces_in_location(l) > 0
                        // `AlliedPlayer?.OccupyingForcesIn(l) > 0`: null > 0 is false
                        && self.allied_player(game).is_some_and(|a| game.player(a).occupying_forces_in_location(l) > 0))
            }
        }
    }

    pub fn occupies_territory(&self, game: &Game, t: Option<Territory>) -> bool {
        match t {
            None => false,
            Some(t) => t.locations().iter().any(|l| self.occupies(game, Some(*l))),
        }
    }

    pub fn occupied_locations(&self, game: &Game) -> Vec<Location> {
        game.map.locations(true).filter(|l| self.occupies(game, Some(*l))).collect()
    }

    pub fn occupied_territories(&self, game: &Game) -> Vec<Territory> {
        game.map.territories(true).filter(|t| self.occupies_territory(game, Some(*t))).collect()
    }

    pub fn controls(&self, g: &Game, l: Location, contested_strongholds_count_as_controlled: bool) -> bool {
        if contested_strongholds_count_as_controlled {
            return self.occupies(g, Some(l));
        }
        self.occupies(g, Some(l)) && g.nr_of_occupants_excluding_faction(l, self.faction) == 0
    }

    pub fn controls_game_territory_boolean(&self, g: &Game, t: Territory, contested_strongholds_count_as_occupied: bool) -> bool {
        if contested_strongholds_count_as_occupied {
            return self.occupies_territory(g, Some(t));
        }
        self.occupies_territory(g, Some(t)) && g.nr_of_occupants_excluding_faction_territory_faction(t, self.faction) == 0
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn territories_with_forces(&self, game: &Game) -> impl Iterator<Item = Territory> {
        game.map.territories(true).filter(move |t| self.any_forces_in_territory(Some(*t)) > 0)
    }

    pub fn locations_with_any_forces(&self) -> Vec<Location> {
                self.forces_in_locations.keys().copied().collect()
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn locations_with_any_forces_in_territory(&self, t: Territory) -> impl Iterator<Item = Location> {
                t.locations().iter().copied().filter(move |l| self.any_forces_in(Some(*l)) > 0)
    }

    pub fn force(&self) -> FactionForce {
                match self.faction {
            Faction::Atreides => FactionForce::Atreides,
            Faction::Harkonnen => FactionForce::Harkonnen,
            Faction::Fremen => FactionForce::Fremen,
            Faction::Emperor => FactionForce::Emperor,
            Faction::Guild => FactionForce::Guild,
            Faction::BeneGesserit => FactionForce::BeneGesserit,
            Faction::Ixian => FactionForce::Ixian,
            Faction::Tleilaxu => FactionForce::Tleilaxu,
            Faction::Choam => FactionForce::Choam,
            Faction::Richese => FactionForce::Richese,
            Faction::Ecaz => FactionForce::Ecaz,
            Faction::Moritani => FactionForce::Moritani,
            _ => FactionForce::None,
        }
    }

    pub fn special_force(&self) -> FactionSpecialForce {
                match self.faction {
            Faction::Emperor => FactionSpecialForce::Emperor,
            Faction::Fremen => FactionSpecialForce::Fremen,
            Faction::BeneGesserit => FactionSpecialForce::BeneGesserit,
            Faction::Ixian => FactionSpecialForce::Ixian,
            Faction::Richese => FactionSpecialForce::Richese,
            _ => FactionSpecialForce::None,
        }
    }

    pub fn has_special_forces(&self, game: &Game) -> bool {
        (game.applicable(Rule::FremenSpecialForces) && self.is(Faction::Fremen))
            || (game.applicable(Rule::EmperorSpecialForces) && self.is(Faction::Emperor))
            || self.is(Faction::Ixian)
    }

    pub fn allied_player(&self, game: &Game) -> Option<PlayerIndex> {
        game.get_player(Some(self.ally))
    }

    pub fn allied_resources(&self, game: &Game) -> i32 {
        match self.allied_player(game) {
            Some(a) => game.player(a).resources,
            None => 0,
        }
    }

    pub fn has_ally(&self) -> bool {
                self.ally != Faction::None
    }

    pub fn has(&self, card: Option<TreacheryCard>) -> bool {
                match card {
            None => false,
            Some(card) => self.treachery_cards.contains(&card),
        }
    }

    pub fn has_treachery_card_type(&self, card_type: TreacheryCardType) -> bool {
                self.treachery_cards.iter().any(|c| c.type_() == card_type)
    }

    pub fn has_leader(&self, leader: Leader) -> bool {
                self.leaders.contains(&leader)
    }

    pub fn is(&self, f: Faction) -> bool {
                self.faction == f
    }

    pub fn or_ally_is(&self, f: Faction) -> bool {
                self.faction == f || self.ally == f
    }

    pub fn maximum_number_of_cards(&self, game: &Game) -> i32 {
        let occupier_of_choam_homeworld = game.occupier_of(World::Choam);
        let occupation_bonus = match occupier_of_choam_homeworld {
            Some(o) if Some(o) == game.index_of(self) || game.player(o).faction == self.ally => 1,
            _ => 0,
        };
        let atomics_penalty =
            if game.atomics_aftermath.is_some() && (self.faction == Faction::Moritani || self.ally == Faction::Moritani) { 1 } else { 0 };

        let amount = match self.faction {
            Faction::Harkonnen => 8,
            Faction::Choam => 5,
            _ => 4,
        };

        amount + occupation_bonus - atomics_penalty
    }

    pub fn has_no_field_in(&self, territory: Territory) -> bool {
                self.faction == Faction::Richese && self.special_forces_in_territory(Some(territory)) > 0
    }

    pub fn has_room_for_cards(&self, game: &Game) -> bool {
        (self.treachery_cards.len() as i32) < self.maximum_number_of_cards(game)
    }

    pub fn hand_size_exceeded(&self, game: &Game) -> bool {
        (self.treachery_cards.len() as i32) > self.maximum_number_of_cards(game)
    }

    pub fn number_of_traitors(&self) -> i32 {
                if self.faction == Faction::Harkonnen { 4 } else { 1 }
    }

    pub fn number_of_face_dancers(&self) -> i32 {
                if self.faction == Faction::Tleilaxu { 3 } else { 0 }
    }

    pub fn battalions_in(&self, territory: Territory) -> IndexMap<Location, Battalion> {
                // C# shares the live Battalion objects; no caller mutates them, so copies are equivalent.
        let mut result = IndexMap::new();
        for (k, v) in self.forces_in_locations.iter().filter(|(k, _)| k.territory() == territory) {
            result.insert_new(*k, v.clone());
        }
        result
    }

    pub fn battalion_in(&self, location: Location, battalion: &mut Option<Battalion>) -> bool {
                match self.forces_in_locations.get(&location) {
            Some(b) => {
                *battalion = Some(b.clone());
                true
            }
            None => {
                *battalion = None;
                false
            }
        }
    }

    pub fn battalion_in_location(&self, location: Location) -> Option<Battalion> {
                // C# returns the live battalion when there is one, else a new detached one. No caller
        // mutates the result, so a copy is equivalent.
        let mut result = None;
        if self.battalion_in(location, &mut result) { result } else { Some(Battalion::new(self.faction, 0, 0, location)) }
    }

    pub fn assign_leaders(g: &mut Game, me: PlayerIndex) {
        let faction = g.player(me).faction;
        let leaders: Vec<Leader> = match faction {
            Faction::Choam => LeaderManager::get_leaders(Faction::Choam)
                .filter(|l| g.applicable(Rule::ChoamAuditor) || l.hero_type() != HeroType::Auditor)
                .collect(),
            Faction::Ecaz => LeaderManager::get_leaders(Faction::Ecaz).filter(|l| l.hero_type() != HeroType::Vidal).collect(),
            _ => LeaderManager::get_leaders(faction).collect(),
        };
        g.player_mut(me).leaders = leaders;
    }

    pub fn card(&self, type_: TreacheryCardType) -> Option<TreacheryCard> {
                self.treachery_cards.iter().copied().find(|c| c.type_() == type_)
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn unrevealed_traitors(&self) -> impl Iterator<Item = Hero> {
                self.traitors.iter().copied().filter(move |f| !self.revealed_traitors.contains(f))
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn unrevealed_face_dancers(&self) -> impl Iterator<Item = Hero> {
                self.face_dancers.iter().copied().filter(move |f| !self.revealed_face_dancers.contains(f))
    }

    pub fn messiah_available(&self, game: &Game) -> bool {
        game.applicable(Rule::AtreidesMessiah)
            && self.is(Faction::Atreides)
            && self.total_forces_killed_in_battle >= 7
            && game.is_alive(LeaderManager::messiah())
    }

    pub fn is_bot(&self, game: &Game) -> bool {
        // Game.IsBot(this): `self` is always one of game.players.
        game.is_bot(game.index_of(self).expect("a player of this game"))
    }

    pub fn ally_is_bot(&self, game: &Game) -> bool {
        self.has_ally() && game.is_bot(self.allied_player(game).expect("allied player"))
    }

    pub fn primary_homeworld(&self) -> Homeworld {
                *self.home_worlds.first().expect("a homeworld")
    }

    pub fn has_karma(&self, g: &Game) -> bool {
        // Karma.ValidKarmaCards(g, this): `self` is always one of g.players.
        // Any() stops at the first match.
        crate::events::Karma::valid_karma_cards(g, g.index_of(self).expect("a player of this game")).next().is_some()
    }

    pub fn initialize_homeworld(game: &mut Game, me: PlayerIndex, world: Homeworld, initial_normal_forces: i32, initial_special_forces: i32) {
        if initial_normal_forces > 0 {
            Player::add_forces(game, me, world.location(), initial_normal_forces, false);
        }
        if initial_special_forces > 0 {
            Player::add_special_forces(game, me, world.location(), initial_special_forces, false);
        }

        let p = game.player_mut(me);
        p.salusa_secundus_homeworld_is_on_low_threshold = world.world() == World::SalusaSecundus && initial_special_forces < world.threshold();

        p.home_worlds.push(world);
    }

    pub fn has_high_threshold(&self, game: &Game, w: World) -> bool {
        if !game.applicable(Rule::Homeworlds) {
            return false;
        }

        let Some(homeworld) = self.home_worlds.iter().copied().find(|hw| hw.world() == w) else {
            return false;
        };

        if homeworld.world() == World::SalusaSecundus {
            return !self.salusa_secundus_homeworld_is_on_low_threshold;
        }

        self.any_forces_in(Some(homeworld.location())) >= homeworld.threshold()
    }

    pub fn has_high_threshold_0(&self, game: &Game) -> bool {
        if !game.applicable(Rule::Homeworlds) {
            return false;
        }
        self.home_worlds.iter().any(|w| self.has_high_threshold(game, w.world()))
    }

    pub fn has_low_threshold(&self, game: &Game, w: World) -> bool {
        if !game.applicable(Rule::Homeworlds) {
            return false;
        }

        let Some(homeworld) = self.home_worlds.iter().copied().find(|hw| hw.world() == w) else {
            return false;
        };

        if homeworld.world() == World::SalusaSecundus {
            return self.salusa_secundus_homeworld_is_on_low_threshold;
        }

        self.any_forces_in(Some(homeworld.location())) < homeworld.threshold()
    }

    pub fn has_low_threshold_0(&self, game: &Game) -> bool {
        game.applicable(Rule::Homeworlds) && self.home_worlds.iter().any(|w| self.has_low_threshold(game, w.world()))
    }

    pub fn initiated(&self, e: Option<&GameEvent>) -> bool {
                match e {
            Some(e) => e.initiator() == self.faction,
            None => false,
        }
    }

    pub fn is_native(&self, territory: Territory) -> bool {
                territory.locations().iter().any(|l| match l.as_homeworld() {
            Some(hw) => self.is_native_homeworld(hw),
            None => false,
        })
    }

    pub fn is_native_homeworld(&self, hw: Homeworld) -> bool {
                // List<Homeworld>.Contains uses Location.Equals (by Id).
        self.home_worlds.iter().any(|h| h.location() == hw.location())
    }

    pub fn have_forces_on_each_others_home_worlds(&self, game: &Game, other: PlayerIndex) -> bool {
        // HomeWorlds.Contains(l) with a Location argument: Enumerable.Contains, Location.Equals (by Id).
        let other = game.player(other);
        self.forces_in_locations.keys().any(|l| other.home_worlds.iter().any(|hw| hw.location() == *l))
            || other.forces_in_locations.keys().any(|l| self.home_worlds.iter().any(|hw| hw.location() == *l))
    }

    pub fn get_homeworld_battle_contribution_and_lasgun_shield_limit(&self, game: &Game, where_battle_happens: Territory) -> i32 {
        if !where_battle_happens.is_homeworld() || !self.is_native(where_battle_happens) {
            return 0;
        }

        let homeworld = where_battle_happens.locations().first().copied().expect("a location").as_homeworld();

        let Some(homeworld) = homeworld else {
            return 0;
        };

        if self.has_high_threshold(game, homeworld.world()) {
            homeworld.battle_bonus_and_lasgun_shield_limit_at_high_threshold()
        } else {
            homeworld.battle_bonus_and_lasgun_shield_limit_at_low_threshold()
        }
    }

}

/// A player of a game: its index in `Game.players`. Players are only ever appended,
/// so the index is stable. `Game::player` resolves it; `Game::player_at(i)` makes one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Canon)]
#[canon(manual)]
pub struct PlayerIndex {
    pub index: usize,
}

