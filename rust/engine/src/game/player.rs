use crate::prelude::*;
use crate::game::amounts;

#[derive(Clone, Debug)]
pub struct Player {
    /// `None` until the player has a faction.
    pub faction: Option<Faction>,
    /// `None` until `EstablishPlayers` draws the seats, right after it creates the players.
    pub seat: Option<Seat>,
    pub resources: u16,
    pub extortion: u16,
    pub bribes: u16,
    pub resources_after_bidding: u16,
    pub banked_resources: u16,
    pub transferable_resources: u16,
    pub treachery_cards: Vec<TreacheryCard>,
    // In the order the player learned of them.
    pub known_cards: IndexSet<TreacheryCard>,
    pub traitors: Vec<Hero>,
    // In the order they were revealed.
    pub revealed_traitors: IndexSet<Hero>,
    pub told_traitors: IndexSet<Hero>,
    pub told_non_traitors: IndexSet<Hero>,
    pub known_non_traitors: IndexSet<Hero>,
    pub discarded_traitors: IndexSet<Hero>,
    pub face_dancers: Vec<Hero>,
    pub revealed_face_dancers: IndexSet<Hero>,
    pub told_face_dancers: IndexSet<Hero>,
    pub told_non_face_dancers: IndexSet<Hero>,
    pub leaders: Vec<Leader>,
    // In map order (by location id), like spice on the board.
    pub forces_in_locations: BTreeMap<Location, Battalion>,
    pub ambassadors: Vec<Ambassador>,
    pub predicted_faction: Option<Faction>,
    pub predicted_turn: Option<Turn>,
    pub forces_killed: u16,
    pub special_forces_killed: u16,
    pub total_forces_killed_in_battle: u16,
    pub ally: Option<Faction>,
    pub special_karma_power_used: bool,
    pub tech_tokens: Vec<TechToken>,
    pub most_recently_revived_leader: Option<Leader>,
    pub skills_to_choose_from: Vec<LeaderSkill>,
    pub home_worlds: Vec<Homeworld>,
    pub nexus: Option<Faction>,
    pub salusa_secundus_homeworld_is_on_low_threshold: bool,
}

impl Player {
    pub fn new(faction: Option<Faction>) -> Self {
        Player {
            faction,
            seat: None,
            resources: 0,
            extortion: 0,
            bribes: 0,
            resources_after_bidding: 0,
            banked_resources: 0,
            transferable_resources: 0,
            treachery_cards: Vec::new(),
            known_cards: IndexSet::new(),
            traitors: Vec::new(),
            revealed_traitors: IndexSet::new(),
            told_traitors: IndexSet::new(),
            told_non_traitors: IndexSet::new(),
            known_non_traitors: IndexSet::new(),
            discarded_traitors: IndexSet::new(),
            face_dancers: Vec::new(),
            revealed_face_dancers: IndexSet::new(),
            told_face_dancers: IndexSet::new(),
            told_non_face_dancers: IndexSet::new(),
            leaders: Vec::new(),
            forces_in_locations: BTreeMap::new(),
            ambassadors: Vec::new(),
            predicted_faction: None,
            predicted_turn: None,
            forces_killed: 0,
            special_forces_killed: 0,
            total_forces_killed_in_battle: 0,
            ally: None,
            special_karma_power_used: false,
            tech_tokens: Vec::new(),
            most_recently_revived_leader: None,
            skills_to_choose_from: Vec::new(),
            home_worlds: Vec::new(),
            nexus: None,
            salusa_secundus_homeworld_is_on_low_threshold: false,
        }
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
        self.forces_in_locations.iter().filter(|(k, _)| k.as_homeworld().is_none()).map(|(k, v)| (*k, v.clone())).collect()
    }

    pub fn no_field_is_active(&self) -> bool {
        self.faction == Some(Faction::Richese) && self.forces_in_locations.values().any(|b| b.amount_of_special_forces > 0)
    }

    /// Returns the live battalion, not a copy.
    pub fn get_and_create_if_needed(&mut self, location: Location) -> &mut Battalion {
        let faction = self.faction;
        self.forces_in_locations.entry(location).or_insert_with(|| Battalion::new(faction.expect("faction"), 0, 0, location))
    }

    /// Adds `nr_of_forces` (negative to remove) at `location`. A Game Error if fewer than zero
    /// would be left, a change C# ignored.
    pub fn change_forces(game: &mut Game, me: PlayerId, location: Location, nr_of_forces: i32) -> GameResult {
        let me_ = game.player_mut(me);
        let result = me_.get_and_create_if_needed(location);
        let changed = result.change_forces(nr_of_forces);
        if result.total_amount_of_forces() == 0 {
            me_.forces_in_locations.remove(&location);
        }
        changed
    }

    /// Adds `nr_of_forces` special forces (negative to remove) at `location`. A Game Error if
    /// fewer than zero would be left, a change C# ignored.
    pub fn change_special_forces(game: &mut Game, me: PlayerId, location: Location, nr_of_forces: i32) -> GameResult {
        let me_ = game.player_mut(me);
        let result = me_.get_and_create_if_needed(location);
        let changed = result.change_special_forces(nr_of_forces);
        if result.total_amount_of_forces() == 0 {
            me_.forces_in_locations.remove(&location);
        }
        changed?;
        Player::check_if_emperor_star_threshold_was_passed(game, me);
        Ok(())
    }

    /// From reserves, the forces come from the first homeworld that holds all of them. A Game
    /// Error if none does, where C# placed nothing.
    pub fn add_forces(game: &mut Game, me: PlayerId, location: Location, nr_of_forces: i32, from_reserves: bool) -> GameResult {
        if from_reserves {
            let p = game.player(me);
            let source_world = p.home_worlds.iter().copied().find(|w| p.forces_in(Some(w.location())) >= nr_of_forces);
            match source_world {
                Some(source_world) => Player::move_forces(game, me, source_world.location(), location, nr_of_forces),
                None => game_error!("No homeworld holds {nr_of_forces} forces"),
            }
        } else {
            Player::change_forces(game, me, location, nr_of_forces)
        }
    }

    /// As [`Player::add_forces`], for special forces.
    pub fn add_special_forces(game: &mut Game, me: PlayerId, location: Location, nr_of_forces: i32, from_reserves: bool) -> GameResult {
        if from_reserves {
            let p = game.player(me);
            let source_world = p.home_worlds.iter().copied().find(|w| p.special_forces_in(Some(w.location())) >= nr_of_forces);
            match source_world {
                Some(source_world) => Player::move_special_forces(game, me, source_world.location(), location, nr_of_forces),
                None => game_error!("No homeworld holds {nr_of_forces} special forces"),
            }
        } else {
            Player::change_special_forces(game, me, location, nr_of_forces)
        }
    }

    /// Adds `nr_of_forces` (negative to remove) to the homeworld of the player's normal forces.
    /// A Game Error if it holds fewer than it should give up, or if the player has no such
    /// homeworld and the amount isn't 0.
    pub fn add_forces_to_reserves(game: &mut Game, me: PlayerId, nr_of_forces: i32) -> GameResult {
        let source_world = game.player(me).home_worlds.iter().copied().find(|w| w.is_home_of_normal_forces());
        match source_world {
            Some(source_world) => Player::change_forces(game, me, source_world.location(), nr_of_forces),
            None if nr_of_forces == 0 => Ok(()),
            None => game_error!("No homeworld for forces"),
        }
    }

    pub fn check_if_emperor_star_threshold_was_passed(game: &mut Game, me: PlayerId) {
        let p = game.player(me);
        if p.faction == Some(Faction::Emperor) && game.applicable(Rule::EmperorSpecialForces) {
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

    /// As [`Player::add_forces_to_reserves`], for the homeworld of the special forces.
    pub fn add_special_forces_to_reserves(game: &mut Game, me: PlayerId, nr_of_forces: i32) -> GameResult {
        let source_world = game.player(me).home_worlds.iter().copied().find(|w| w.is_home_of_special_forces());
        match source_world {
            Some(source_world) => Player::change_special_forces(game, me, source_world.location(), nr_of_forces),
            None if nr_of_forces == 0 => Ok(()),
            None => game_error!("No homeworld for special forces"),
        }
    }

    pub fn remove_forces(game: &mut Game, me: PlayerId, location: Location, nr_of_forces: i32) -> GameResult {
        Player::change_forces(game, me, location, -nr_of_forces)
    }

    pub fn remove_special_forces(game: &mut Game, me: PlayerId, location: Location, nr_of_forces: i32) -> GameResult {
        Player::change_special_forces(game, me, location, -nr_of_forces)
    }

    pub fn forces_in(&self, location: Option<Location>) -> i32 {
        location.and_then(|l| self.forces_in_locations.get(&l)).map_or(0, |b| b.forces())
    }

    pub fn special_forces_in(&self, location: Option<Location>) -> i32 {
        location.and_then(|l| self.forces_in_locations.get(&l)).map_or(0, |b| b.special_forces())
    }

    pub fn any_forces_in(&self, location: Option<Location>) -> i32 {
        location.and_then(|l| self.forces_in_locations.get(&l)).map_or(0, |b| b.total_amount_of_forces())
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

    pub fn forces_to_reserves(game: &mut Game, me: PlayerId, location: Location) -> GameResult {
        let amount_of_forces = game.player(me).forces_in_locations[&location].forces();

        Player::add_forces_to_reserves(game, me, amount_of_forces)?;

        let live_special = |game: &Game| game.player(me).special_forces_in(Some(location));
        let faction = game.player(me).faction;
        if faction == Some(Faction::BeneGesserit) {
            let n = live_special(game);
            Player::add_forces_to_reserves(game, me, n)?;
        } else if faction != Some(Faction::Richese) {
            let n = live_special(game);
            Player::add_special_forces_to_reserves(game, me, n)?;
        }

        game.player_mut(me).forces_in_locations.remove(&location);
        Player::check_if_emperor_star_threshold_was_passed(game, me);
        Ok(())
    }

    pub fn some_forces_to_reserves(game: &mut Game, me: PlayerId, location: Location, amount: i32) -> GameResult {
        Player::add_forces_to_reserves(game, me, amount)?;
        Player::change_forces(game, me, location, -amount)
    }

    pub fn special_forces_to_reserves(game: &mut Game, me: PlayerId, location: Location, amount: i32) -> GameResult {
        let faction = game.player(me).faction;
        if faction == Some(Faction::BeneGesserit) {
            Player::add_forces_to_reserves(game, me, amount)?;
        } else if faction != Some(Faction::Richese) {
            Player::add_special_forces_to_reserves(game, me, amount)?;
        }

        Player::change_special_forces(game, me, location, -amount)
    }

    /// Special forces shipped from `location` back to the reserves on `homeworld`. Bene Gesserit
    /// advisors return as fighters, as in [`Player::special_forces_to_reserves`]. Richese's
    /// special force is its No-Field token: it leaves the planet unrevealed.
    pub fn special_forces_back_to_reserves(game: &mut Game, me: PlayerId, location: Location, homeworld: Homeworld, amount: i32) -> GameResult {
        if amount <= 0 {
            return Ok(());
        }
        match game.player(me).faction {
            Some(Faction::BeneGesserit) => {
                Player::change_special_forces(game, me, location, -amount)?;
                Player::change_forces(game, me, homeworld.location(), amount)
            }
            Some(Faction::Richese) => {
                Player::change_special_forces(game, me, location, -amount)?;
                game.ship_move.current_no_field_value = -1;
                Ok(())
            }
            _ => Player::move_special_forces(game, me, location, homeworld.location(), amount),
        }
    }

    /// Up to `amount` forces (or special forces) in `t` back to reserves, from the locations
    /// with the least spice first. If `t` holds fewer, all of them go.
    pub fn some_forces_in_territory_to_reserves(game: &mut Game, me: PlayerId, t: Territory, amount: i32, special: bool) -> GameResult {
        if amount > 0 {
            let mut to_remove_in_total = amount;
            let mut locations: Vec<Location> = t.locations().to_vec();
            locations.sort_by_cached_key(|l| l.spice_blow_amount()); // OrderBy: stable
            for l in locations {
                let p = game.player(me);
                let forces_in = if special { p.special_forces_in(Some(l)) } else { p.forces_in(Some(l)) };
                if forces_in > 0 {
                    let to_remove_in_this_location = std::cmp::min(forces_in, to_remove_in_total);

                    // C# sent all of `to_remove_in_total` to the reserves here, and skipped the
                    // removal when this location held fewer (escapes.md, "Forces below zero").
                    let faction = game.player(me).faction;
                    if special && faction != Some(Faction::BeneGesserit) {
                        Player::add_special_forces_to_reserves(game, me, to_remove_in_this_location)?;
                    } else if !special || faction != Some(Faction::Richese) {
                        Player::add_forces_to_reserves(game, me, to_remove_in_this_location)?;
                    }

                    if special {
                        Player::change_special_forces(game, me, l, -to_remove_in_this_location)?;
                    } else {
                        Player::change_forces(game, me, l, -to_remove_in_this_location)?;
                    }

                    to_remove_in_total -= to_remove_in_this_location;
                }

                if to_remove_in_total == 0 {
                    break;
                }
            }
        }
        Ok(())
    }

    pub fn forces_in_territory_to_reserves(game: &mut Game, me: PlayerId, t: Territory) -> GameResult {
        // Lazy Where: the filter is re-checked after each body ran.
        for &l in t.locations() {
            if game.player(me).any_forces_in(Some(l)) > 0 {
                Player::forces_to_reserves(game, me, l)?;
            }
        }
        Ok(())
    }

    /// Kills every force at `location` and returns how many died.
    pub fn kill_all_forces(game: &mut Game, me: PlayerId, location: Location, in_battle: bool) -> GameResult<i32> {
        let p = game.player(me);
        if let Some(battalion) = p.forces_in_locations.get(&location) {
            let kill_count = battalion.forces();
            let special_kill_count = battalion.special_forces();
            Player::to_tanks(game, me, kill_count, special_kill_count, in_battle)?;
            game.player_mut(me).forces_in_locations.remove(&location);
            Player::check_if_emperor_star_threshold_was_passed(game, me);
            return Ok(kill_count + special_kill_count);
        }

        Ok(0)
    }

    pub fn kill_all_forces_in_territory(game: &mut Game, me: PlayerId, t: Territory, in_battle: bool) -> GameResult {
        // Lazy Where: the filter is re-checked after each body ran.
        for &l in t.locations() {
            if game.player(me).any_forces_in(Some(l)) > 0 {
                Player::kill_all_forces(game, me, l, in_battle)?;
            }
        }
        Ok(())
    }

    /// Kills forces at `location` and returns how many died. A Game Error if `location` holds
    /// fewer, where C# put them in the tanks without taking them off the board.
    pub fn kill_forces(game: &mut Game, me: PlayerId, location: Location, amount_of_forces: i32, amount_of_special_forces: i32, in_battle: bool) -> GameResult<i32> {
        Player::change_forces(game, me, location, -amount_of_forces)?;
        Player::change_special_forces(game, me, location, -amount_of_special_forces)?;
        Player::to_tanks(game, me, amount_of_forces, amount_of_special_forces, in_battle)?;
        Ok(amount_of_forces + amount_of_special_forces)
    }

    /// Counts forces that died in the tanks. Bene Gesserit advisors die as fighters.
    fn to_tanks(game: &mut Game, me: PlayerId, forces: i32, special_forces: i32, in_battle: bool) -> GameResult {
        let p = game.player_mut(me);
        amounts::gain(&mut p.forces_killed, forces, "forces killed")?;

        if p.faction == Some(Faction::BeneGesserit) {
            amounts::gain(&mut p.forces_killed, special_forces, "forces killed")?;
        } else {
            amounts::gain(&mut p.special_forces_killed, special_forces, "special forces killed")?;
        }

        if in_battle {
            amounts::gain(&mut p.total_forces_killed_in_battle, forces + special_forces, "forces killed in battle")?;
        }
        Ok(())
    }

    pub fn kill_forces_in_territory(game: &mut Game, me: PlayerId, t: Territory, amount: i32, special: bool, in_battle: bool) -> GameResult {
        let mut to_kill = amount;
        let mut locations: Vec<Location> = t.locations().to_vec();
        locations.sort_by_cached_key(|l| l.spice_blow_amount()); // OrderBy: stable
        for l in locations {
            let p = game.player(me);
            let forces_in = if special { p.special_forces_in(Some(l)) } else { p.forces_in(Some(l)) };
            if forces_in > 0 {
                let to_be_killed = std::cmp::min(forces_in, to_kill);
                if special {
                    Player::kill_forces(game, me, l, 0, to_be_killed, in_battle)?;
                } else {
                    Player::kill_forces(game, me, l, to_be_killed, 0, in_battle)?;
                }
                to_kill -= to_be_killed;
            }

            if to_kill == 0 {
                break;
            }
        }
        Ok(())
    }

    /// Kills `amount` forces (or special forces) in reserve: first on the homeworld of that kind
    /// of force, then on the other homeworlds in order. A Game Error if the reserves hold fewer.
    /// C# took them only from the first homeworld, and put them in the tanks even when it held
    /// fewer.
    pub fn kill_forces_in_reserves(game: &mut Game, me: PlayerId, amount: i32, special: bool) -> GameResult {
        let p = game.player(me);
        let mut worlds: Vec<Homeworld> = p.home_worlds.iter().copied().filter(|w| if special { w.is_home_of_special_forces() } else { w.is_home_of_normal_forces() }).collect();
        worlds.extend(p.home_worlds.iter().copied().filter(|w| !worlds.contains(w)).collect::<Vec<_>>());
        let mut left = amount;
        for w in worlds {
            let p = game.player(me);
            let here = if special { p.special_forces_in(Some(w.location())) } else { p.forces_in(Some(w.location())) };
            let n = here.min(left);
            if n > 0 {
                if special {
                    Player::kill_forces(game, me, w.location(), 0, n, false)?;
                } else {
                    Player::kill_forces(game, me, w.location(), n, 0, false)?;
                }
                left -= n;
            }
        }
        if left > 0 {
            game_error!("Can't kill {amount} forces in reserve");
        }
        Ok(())
    }

    /// Forces from the tanks to reserves. A Game Error if fewer are in the tanks.
    pub fn revive_forces(game: &mut Game, me: PlayerId, amount: i32) -> GameResult {
        amounts::spend(&mut game.player_mut(me).forces_killed, amount, "forces killed")?;
        Player::add_forces_to_reserves(game, me, amount)
    }

    /// Special forces from the tanks to reserves. A Game Error if fewer are in the tanks.
    pub fn revive_special_forces(game: &mut Game, me: PlayerId, amount: i32) -> GameResult {
        amounts::spend(&mut game.player_mut(me).special_forces_killed, amount, "special forces killed")?;
        Player::add_special_forces_to_reserves(game, me, amount)
    }

    pub fn ship_forces(game: &mut Game, me: PlayerId, to: Location, amount: i32) -> GameResult {
        Player::add_forces(game, me, to, amount, true)
    }

    pub fn ship_forces_from_homeworld(game: &mut Game, me: PlayerId, to: Location, from: Homeworld, amount: i32) -> GameResult {
        Player::move_forces(game, me, from.location(), to, amount)
    }

    /// Richese's special force is its No-Field token, which comes from no reserves.
    pub fn ship_special_forces(game: &mut Game, me: PlayerId, to: Location, amount: i32) -> GameResult {
        if game.player(me).faction == Some(Faction::Richese) {
            Player::change_special_forces(game, me, to, amount)
        } else {
            Player::add_special_forces(game, me, to, amount, true)
        }
    }

    pub fn ship_special_forces_from_homeworld(game: &mut Game, me: PlayerId, to: Location, from: Homeworld, amount: i32) -> GameResult {
        if game.player(me).faction == Some(Faction::Richese) {
            Player::change_special_forces(game, me, to, amount)
        } else {
            Player::move_special_forces(game, me, from.location(), to, amount)
        }
    }

    /// Bene Gesserit forces from reserves to `l` as advisors. A Game Error if the reserves hold
    /// fewer, where C# placed the advisors anyway.
    pub fn ship_advisors(game: &mut Game, me: PlayerId, l: Location, amount: i32) -> GameResult {
        Player::add_forces_to_reserves(game, me, -amount)?;
        Player::change_special_forces(game, me, l, amount)
    }

    /// A Game Error if `from` holds fewer than `amount`, where C# added them to `to` anyway.
    pub fn move_forces(game: &mut Game, me: PlayerId, from: Location, to: Location, amount: i32) -> GameResult {
        Player::change_forces(game, me, from, -amount)?;
        Player::change_forces(game, me, to, amount)
    }

    /// As [`Player::move_forces`], for special forces.
    pub fn move_special_forces(game: &mut Game, me: PlayerId, from: Location, to: Location, amount: i32) -> GameResult {
        Player::change_special_forces(game, me, from, -amount)?;
        Player::change_special_forces(game, me, to, amount)
    }

    pub fn flip_forces(game: &mut Game, me: PlayerId, l: Location, as_advisors: bool) -> GameResult {
        if game.player(me).faction == Some(Faction::BeneGesserit) {
            if as_advisors {
                let number_of_forces = game.player(me).forces_in(Some(l));
                Player::change_forces(game, me, l, -number_of_forces)?;
                Player::change_special_forces(game, me, l, number_of_forces)?;
            } else {
                let number_of_special_forces = game.player(me).special_forces_in(Some(l));
                Player::change_forces(game, me, l, number_of_special_forces)?;
                Player::change_special_forces(game, me, l, -number_of_special_forces)?;
            }
        }
        Ok(())
    }

    pub fn flip_forces_in_territory(game: &mut Game, me: PlayerId, t: Territory, as_advisors: bool) -> GameResult {
        // `t.Locations` are objects of t's map: after an undo, t can come from an event of
        // the other game, and a battalion created for such a location keeps it as its key.
        for l in t.locations().iter().copied() {
            Player::flip_forces(game, me, l, as_advisors)?;
        }
        Ok(())
    }

    pub fn occupying_forces_in_location(&self, l: Location) -> i32 {
                self.forces_in(Some(l)) + (if self.faction == Some(Faction::BeneGesserit) { 0 } else { self.special_forces_in(Some(l)) })
    }

    pub fn occupies(&self, game: &Game, l: Option<Location>) -> bool {
        match l {
            None => false,
            Some(l) => {
                self.occupying_forces_in_location(l) > 0
                    || (self.ally == Some(Faction::Ecaz)
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
        game.board.map.locations(true).filter(|l| self.occupies(game, Some(*l))).collect()
    }

    pub fn occupied_territories(&self, game: &Game) -> Vec<Territory> {
        game.board.map.territories(true).filter(|t| self.occupies_territory(game, Some(*t))).collect()
    }

    pub fn controls(&self, g: &Game, l: Location, contested_strongholds_count_as_controlled: bool) -> bool {
        if contested_strongholds_count_as_controlled {
            return self.occupies(g, Some(l));
        }
        self.occupies(g, Some(l)) && g.nr_of_occupants_excluding_faction(l, self.faction) == 0
    }

    pub fn controls_territory(&self, g: &Game, t: Territory, contested_strongholds_count_as_occupied: bool) -> bool {
        if contested_strongholds_count_as_occupied {
            return self.occupies_territory(g, Some(t));
        }
        self.occupies_territory(g, Some(t)) && g.nr_of_occupants_of_territory_excluding(t, self.faction) == 0
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn territories_with_forces(&self, game: &Game) -> impl Iterator<Item = Territory> {
        game.board.map.territories(true).filter(move |t| self.any_forces_in_territory(Some(*t)) > 0)
    }

    pub fn locations_with_any_forces(&self) -> Vec<Location> {
                self.forces_in_locations.keys().copied().collect()
    }

    pub fn force(&self) -> Option<FactionForce> {
                match self.faction {
            Some(Faction::Atreides) => Some(FactionForce::Atreides),
            Some(Faction::Harkonnen) => Some(FactionForce::Harkonnen),
            Some(Faction::Fremen) => Some(FactionForce::Fremen),
            Some(Faction::Emperor) => Some(FactionForce::Emperor),
            Some(Faction::Guild) => Some(FactionForce::Guild),
            Some(Faction::BeneGesserit) => Some(FactionForce::BeneGesserit),
            Some(Faction::Ixian) => Some(FactionForce::Ixian),
            Some(Faction::Tleilaxu) => Some(FactionForce::Tleilaxu),
            Some(Faction::Choam) => Some(FactionForce::Choam),
            Some(Faction::Richese) => Some(FactionForce::Richese),
            Some(Faction::Ecaz) => Some(FactionForce::Ecaz),
            Some(Faction::Moritani) => Some(FactionForce::Moritani),
            _ => None,
        }
    }

    pub fn special_force(&self) -> Option<FactionSpecialForce> {
                match self.faction {
            Some(Faction::Emperor) => Some(FactionSpecialForce::Emperor),
            Some(Faction::Fremen) => Some(FactionSpecialForce::Fremen),
            Some(Faction::BeneGesserit) => Some(FactionSpecialForce::BeneGesserit),
            Some(Faction::Ixian) => Some(FactionSpecialForce::Ixian),
            Some(Faction::Richese) => Some(FactionSpecialForce::Richese),
            _ => None,
        }
    }

    pub fn has_special_forces(&self, game: &Game) -> bool {
        (game.applicable(Rule::FremenSpecialForces) && self.is(Faction::Fremen))
            || (game.applicable(Rule::EmperorSpecialForces) && self.is(Faction::Emperor))
            || self.is(Faction::Ixian)
    }

    pub fn allied_player(&self, game: &Game) -> Option<PlayerId> {
        game.get_player(self.ally)
    }

    pub fn allied_resources(&self, game: &Game) -> i32 {
        match self.allied_player(game) {
            Some(a) => i32::from(game.player(a).resources),
            None => 0,
        }
    }

    pub fn has_ally(&self) -> bool {
                self.ally != None
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

    pub fn is(&self, f: impl Into<Option<Faction>>) -> bool {
        let f: Option<Faction> = f.into();
                self.faction == f
    }

    pub fn or_ally_is(&self, f: Faction) -> bool {
                self.faction == Some(f) || self.ally == Some(f)
    }

    pub fn maximum_number_of_cards(&self, game: &Game) -> i32 {
        let occupier_of_choam_homeworld = game.occupier_of(World::Choam);
        let occupation_bonus = match occupier_of_choam_homeworld {
            Some(o) if Some(o) == game.index_of(self) || game.player(o).faction == self.ally => 1,
            _ => 0,
        };
        let atomics_penalty =
            if game.board.atomics_aftermath.is_some() && (self.faction == Some(Faction::Moritani) || self.ally == Some(Faction::Moritani)) { 1 } else { 0 };

        let amount = match self.faction {
            Some(Faction::Harkonnen) => 8,
            Some(Faction::Choam) => 5,
            _ => 4,
        };

        amount + occupation_bonus - atomics_penalty
    }

    pub fn has_no_field_in(&self, territory: Territory) -> bool {
                self.faction == Some(Faction::Richese) && self.special_forces_in_territory(Some(territory)) > 0
    }

    pub fn has_room_for_cards(&self, game: &Game) -> bool {
        (self.treachery_cards.len() as i32) < self.maximum_number_of_cards(game)
    }

    pub fn hand_size_exceeded(&self, game: &Game) -> bool {
        (self.treachery_cards.len() as i32) > self.maximum_number_of_cards(game)
    }

    pub fn number_of_traitors(&self) -> i32 {
                if self.faction == Some(Faction::Harkonnen) { 4 } else { 1 }
    }

    pub fn number_of_face_dancers(&self) -> i32 {
                if self.faction == Some(Faction::Tleilaxu) { 3 } else { 0 }
    }

    pub fn battalions_in(&self, territory: Territory) -> IndexMap<Location, Battalion> {
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
        let mut result = None;
        if self.battalion_in(location, &mut result) { result } else { Some(Battalion::new(self.faction.expect("faction"), 0, 0, location)) }
    }

    pub fn assign_leaders(g: &mut Game, me: PlayerId) {
        let faction = g.player(me).faction;
        let leaders: Vec<Leader> = match faction {
            Some(Faction::Choam) => LeaderManager::get_leaders(Faction::Choam)
                .filter(|l| g.applicable(Rule::ChoamAuditor) || l.hero_type() != HeroType::Auditor)
                .collect(),
            Some(Faction::Ecaz) => LeaderManager::get_leaders(Faction::Ecaz).filter(|l| l.hero_type() != HeroType::Vidal).collect(),
            _ => faction.map_or(Vec::new(), |f| LeaderManager::get_leaders(f).collect()),
        };
        g.player_mut(me).leaders = leaders;
    }

    pub fn card(&self, type_: TreacheryCardType) -> Option<TreacheryCard> {
                self.treachery_cards.iter().copied().find(|c| c.type_() == type_)
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn unrevealed_traitors(&self) -> impl Iterator<Item = Hero> {
                self.traitors.iter().copied().filter(move |f| !self.revealed_traitors.contains(f))
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
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

    pub fn initialize_homeworld(game: &mut Game, me: PlayerId, world: Homeworld, initial_normal_forces: i32, initial_special_forces: i32) -> GameResult {
        if initial_normal_forces > 0 {
            Player::add_forces(game, me, world.location(), initial_normal_forces, false)?;
        }
        if initial_special_forces > 0 {
            Player::add_special_forces(game, me, world.location(), initial_special_forces, false)?;
        }

        let p = game.player_mut(me);
        p.salusa_secundus_homeworld_is_on_low_threshold = world.world() == World::SalusaSecundus && initial_special_forces < world.threshold();

        p.home_worlds.push(world);
        Ok(())
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

    pub fn has_any_high_threshold(&self, game: &Game) -> bool {
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

    pub fn has_any_low_threshold(&self, game: &Game) -> bool {
        game.applicable(Rule::Homeworlds) && self.home_worlds.iter().any(|w| self.has_low_threshold(game, w.world()))
    }

    /// Whether this player's faction played the Nexus card `nexus`.
    pub fn initiated(&self, nexus: Option<&NexusCardPlayed>) -> bool {
        nexus.is_some_and(|n| n.initiator() == self.faction)
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

    pub fn have_forces_on_each_others_home_worlds(&self, game: &Game, other: PlayerId) -> bool {
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlayerId(u8);

impl PlayerId {
    /// The player's position in `Game.players`.
    pub fn index(self) -> usize {
        self.0 as usize
    }

    pub(crate) fn from_index(index: usize) -> PlayerId {
        PlayerId(u8::try_from(index).expect("a game has at most 255 players"))
    }
}

