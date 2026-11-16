use crate::log::blow::{NexusLog, NexusRide};
use crate::game::MonsterAppearance;
use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct FremenRidesMonster {
    pub forces_from_reserves: u16,
    pub special_forces_from_reserves: u16,
    // ---- inherited from PlacementEvent ----
    pub to: Option<Location>,
    pub force_locations: Option<String>,
    pub passed: bool,
}

impl Default for FremenRidesMonster {
    fn default() -> Self {
        FremenRidesMonster {
            forces_from_reserves: Default::default(),
            special_forces_from_reserves: Default::default(),
            to: Location::from_id(0),
            force_locations: Some("".to_string()),
            passed: Default::default(),
        }
    }
}

impl FremenRidesMonster {
    pub fn total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        // base.TotalAmountOfForcesAddedToLocation is ForcesAddedToLocation + SpecialForcesAddedToLocation.
        let base = { let (forces, special_forces) = PlacedForces::amounts(game, &self.force_locations); forces + special_forces };
        (base + i32::from(self.forces_from_reserves)) + i32::from(self.special_forces_from_reserves)
    }

    pub fn is_applicable(g: &Game) -> bool {
        Self::to_ride(g).is_some()
    }

    pub fn to_ride(g: &Game) -> Option<MonsterAppearance> {
        Self::to_ride_index(g).map(|i| g.blow.monsters[i].clone())
    }

    /// Index in `Game.Monsters` of the monster `ToRide` returns (Rust-only: `List.Remove` in
    /// `ExecuteConcreteEvent` removes that object by reference).
    fn to_ride_index(g: &Game) -> Option<usize> {
        let fremen = g.get_player(Some(Faction::Fremen));
        if let Some(fremen) = fremen {
            return g.blow.monsters.iter().position(|m| {
                (m.is_great_monster && g.player(fremen).any_forces_in_reserves() > 0 && !g.prevented(FactionAdvantage::FremenRidesMonster))
                    || (!m.is_great_monster && Self::locations_with_forces_that_can_ride(g, fremen, m.territory).next().is_some())
            });
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn locations_with_forces_that_can_ride<'a>(g: &'a Game, fremen: PlayerId, territory: Territory) -> Box<dyn Iterator<Item = Location> + 'a> {
        let may_ride_from_storm = g.applicable(Rule::FremenMayMoveIntoStorm);

        if NexusPlayed::can_use_cunning(g, fremen) && g.player(fremen).any_forces_in_territory(Some(territory)) == 0 {
            // ForcesOnPlanet builds its dictionary at the call; the Where runs as the caller pulls.
            let keys: Vec<Location> = g.player(fremen).forces_on_planet().keys().copied().collect();
            return Box::new(
                keys.into_iter()
                    .filter(move |l| !l.is_protected_from_storm() && !l.is_stronghold() && (may_ride_from_storm || !g.is_in_storm(*l))),
            );
        }
        if !g.prevented(FactionAdvantage::FremenRidesMonster) {
            return Box::new(
                territory
                    .locations()
                    .iter()
                    .copied()
                    .filter(move |l| (may_ride_from_storm || !g.is_in_storm(*l)) && g.player(fremen).any_forces_in(Some(*l)) > 0),
            );
        }
        Box::new(std::iter::empty())
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_sources<'a>(g: &'a Game) -> Box<dyn Iterator<Item = Location> + 'a> {
        let fremen = g.get_player(Some(Faction::Fremen));
        if let Some(fremen) = fremen {
            let to_ride = Self::to_ride(g);
            let to_ride = to_ride.expect("to ride");
            if !to_ride.is_great_monster {
                return Self::locations_with_forces_that_can_ride(g, fremen, to_ride.territory);
            }
        }

        Box::new(std::iter::empty())
    }

    pub fn max_forces_from_reserves(g: &Game, p: PlayerId, special: bool) -> i32 {
        let ride_from = Self::to_ride(g);
        if ride_from.is_some() && Self::to_ride(g).expect("to ride").is_great_monster {
            return if special { g.player(p).special_forces_in_reserve() } else { g.player(p).forces_in_reserve() };
        }

        0
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_targets(g: &Game, p: PlayerId) -> impl Iterator<Item = Location> {
        let may_move_into_storm = g.player(p).faction == Some(Faction::Fremen)
            && g.applicable(Rule::FremenMayMoveIntoStorm)
            && g.applicable(Rule::FremenStormLosses);

        g.board.map
            .locations(false)
            .filter(move |l| {
                (may_move_into_storm || !Sector::in_storm(l.sector(&g.board.map), g.board.sector_in_storm))
                    && (match l.as_attached_location() {
                        None => true,
                        // Reads the sector of the location it is attached to, not al.Sector.
                        Some(al) => match al.attached_to_location(&g.board.map) {
                            Some(a) => !Sector::in_storm(a.sector(&g.board.map), g.board.sector_in_storm),
                            None => false,
                        },
                    })
                    && (!l.territory().is_stronghold() || g.nr_of_occupants_excluding_faction(*l, g.player(p).faction) < 2)
                    && (!g.player(p).has_ally() || *l == g.board.map.polar_sink() || !g.player(g.player(p).allied_player(g).expect("allied player")).occupies(g, Some(*l)))
            })
    }

    pub fn is_either_valid_discovery_or_no_discovery(l: Location, map: &Map) -> bool {
        // C# parity: never called in C#.
        match l.as_discovered_location() {
            None => true,
            Some(ds) => ds.location().visible(map),
        }
    }
}

impl EventRules for FremenRidesMonster {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if self.passed {
            return Ok(());
        }

        if Self::total_amount_of_forces_added_to_location(self, game) == 0 {
            return Err(Rejection(express!("No forces selected")));
        }
        if self.to.is_none() {
            return Err(Rejection(express!("Target location not selected")));
        }
        if !Self::valid_targets(game, by.player()).any(|v| v == self.to.expect("destination")) {
            return Err(Rejection(express!("Invalid target location")));
        }

        let too_many_forces = self.force_locations(game, by.player())
            .iter()
            .any(|(l, b)| b.forces() > game.player(by.player()).forces_in(Some(*l)));
        if too_many_forces {
            return Err(Rejection(express!("Invalid amount of forces")));
        }

        let too_many_special_forces = self.force_locations(game, by.player())
            .iter()
            .any(|(l, b)| b.special_forces() > game.player(by.player()).special_forces_in(Some(*l)));
        if too_many_special_forces {
            return Err(Rejection(express!("Invalid amount of special forces")));
        }

        if i32::from(self.forces_from_reserves) > Self::max_forces_from_reserves(game, by.player(), false)
            || i32::from(self.special_forces_from_reserves) > Self::max_forces_from_reserves(game, by.player(), true)
        {
            return Err(Rejection(express!("You can't ride with that many reserves")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let to_ride_index = Self::to_ride_index(game);
        let to_ride = to_ride_index.map(|i| game.blow.monsters[i].clone());

        if let Some(i) = to_ride_index {
            // List.Remove compares by reference; ToRide returned the element at this index.
            // Remove(null) finds nothing and does nothing.
            game.blow.monsters.remove(i);
        }

        if !self.passed {
            if self.force_locations(game, by.player()).keys().any(|l| l.territory() != to_ride.as_ref().expect("to ride").territory) {
                let me = by.player();
                game.play_nexus_card(me, parts!["ride from any territory on the planet"]);
            }

            let initiator = game.get_player(by.faction);
            game.ship_move.last_shipment_or_movement = Some(Event::from(self.clone()));
            for (from, value) in self.force_locations(game, by.player()).iter() {
                let from = *from;
                let ini = initiator.required("initiator")?;
                let to = self.to.required("to")?;
                Player::move_forces(game, ini, from, to, value.forces())?;
                let to = self.to.required("to")?;
                Player::move_special_forces(game, ini, from, to, value.special_forces())?;
                let msg = express!(
                    by.faction,
                    " ride ",
                    express_part_if!(value.amount_of_forces > 0; value.amount_of_forces, game.player(ini).force()),
                    express_part_if!(value.amount_of_special_forces > 0; value.amount_of_special_forces, game.player(ini).special_force()),
                    " from ",
                    from,
                    " to ",
                    self.to
                )
                .with_route(by.faction.required("faction")?, [from], self.to)
                .with_log(NexusLog::new(
                    Some(NexusRide::new(by.faction.required("faction")?, value.forces(), value.special_forces(), Some(from), self.to.required("to")?)),
                    None,
                ));
                game.log(parts![msg]);
            }

            if i32::from(self.forces_from_reserves) > 0 || i32::from(self.special_forces_from_reserves) > 0 {
                if i32::from(self.forces_from_reserves) > 0 {
                    let ini = initiator.required("initiator")?;
                    let to = self.to.required("to")?;
                    Player::ship_forces(game, ini, to, i32::from(self.forces_from_reserves))?;
                }
                if i32::from(self.special_forces_from_reserves) > 0 {
                    let ini = initiator.required("initiator")?;
                    let to = self.to.required("to")?;
                    Player::ship_special_forces(game, ini, to, i32::from(self.special_forces_from_reserves))?;
                }
                let ini = initiator.required("initiator")?;
                let msg = express!(
                    by.faction,
                    " ride ",
                    express_part_if!(i32::from(self.forces_from_reserves) > 0; i32::from(self.forces_from_reserves), game.player(ini).force()),
                    express_part_if!(i32::from(self.special_forces_from_reserves) > 0; i32::from(self.special_forces_from_reserves), game.player(ini).special_force()),
                    " from their reserves to ",
                    self.to
                )
                .with_route(by.faction.required("faction")?, [], self.to)
                .with_log(NexusLog::new(
                    Some(NexusRide::new(by.faction.required("faction")?, i32::from(self.forces_from_reserves), i32::from(self.special_forces_from_reserves), None, self.to.required("to")?)),
                    None,
                ));
                game.log(parts![msg]);
            }

            game.flip_bene_gesserit_advisors_when_alone()?;
            game.check_intrusion(&Event::from(self.clone()), by.faction.expect("faction"))?;
        } else {
            let msg = express!(by.faction, " pass a ride on ", Concept::Monster).with_log(NexusLog::new(None, None));
            game.log(parts![msg]);
        }

        game.determine_next_shipment_and_move_sub_phase()?;
        Ok(())
    }

    fn describe(&self, game: &Game, by: By) -> Message {
        if self.passed {
            return express!(by.faction, " don't ride");
        }

        let from = self.force_locations(game, by.player()).keys().next().map(|l| l.territory());
        if let Some(from) = from {
            return express!(by.faction, " ride from ", from, " to ", self.to)
                .with_route(by.faction.expect("faction"), self.force_locations(game, by.player()).keys().copied(), self.to);
        }
        express!(by.faction, " ride from reserves to ", self.to)
    }
}

/// The force list, as placement events read it (`PlacementEvent` in C#).
impl FremenRidesMonster {
    /// The battalions in the force list, of player `p`, the submitter.
    pub fn force_locations(&self, game: &Game, p: PlayerId) -> IndexMap<Location, Battalion> {
        PlacedForces::of(game, p, &self.force_locations)
    }

    pub fn set_force_locations(&mut self, game: &Game, value: &IndexMap<Location, Battalion>) {
        self.force_locations = PlacedForces::text(game, value);
    }
}

impl crate::events::location_event::LocationEvent for FremenRidesMonster {
    fn to(&self, _game: &Game) -> Option<Location> {
        self.to
    }

    fn total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        FremenRidesMonster::total_amount_of_forces_added_to_location(self, game)
    }

    fn forces_added_to_location(&self, game: &Game) -> i32 {
        PlacedForces::amounts(game, &self.force_locations).0
    }

    fn special_forces_added_to_location(&self, game: &Game) -> i32 {
        PlacedForces::amounts(game, &self.force_locations).1
    }
}
