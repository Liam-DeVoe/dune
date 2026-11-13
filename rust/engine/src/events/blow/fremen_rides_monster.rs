use crate::log::blow::{NexusLog, NexusRide};
use crate::game::MonsterAppearence;
use crate::prelude::*;

/// `params object?[]` element list for `Log(...)`.
macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(ExpressionElement::from($e)),*] };
}

#[derive(Clone, Debug, Canon)]
pub struct FremenRidesMonster {
    pub forces_from_reserves: i32,
    pub special_forces_from_reserves: i32,
    // ---- inherited from PlacementEvent ----
    #[canon(name = "_toId")]
    pub to_id: i32,
    #[canon(name = "_forceLocations")]
    pub force_locations: Option<String>,
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for FremenRidesMonster {
    fn default() -> Self {
        FremenRidesMonster {
            forces_from_reserves: Default::default(),
            special_forces_from_reserves: Default::default(),
            to_id: Default::default(),
            force_locations: Some("".to_string()),
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl FremenRidesMonster {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        // base.TotalAmountOfForcesAddedToLocation is ForcesAddedToLocation + SpecialForcesAddedToLocation.
        let base = <Self as PlacementEventMethods>::forces_added_to_location(self, game) + (<Self as PlacementEventMethods>::special_forces_added_to_location(self, game));
        (base + self.forces_from_reserves) + self.special_forces_from_reserves
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if self.passed {
            return None;
        }

        if Self::total_amount_of_forces_added_to_location(self, game) == 0 {
            return Some(express!("No forces selected"));
        }
        if self.to(game).is_none() {
            return Some(express!("Target location not selected"));
        }
        if !Self::valid_targets(game, self.player()).any(|v| v == self.to(game).nre()) {
            return Some(express!("Invalid target location"));
        }

        let too_many_forces = self
            .force_locations(game)
            .iter()
            .any(|(l, b)| b.amount_of_forces > game.player(self.player()).forces_in(Some(*l)));
        if too_many_forces {
            return Some(express!("Invalid amount of forces"));
        }

        let too_many_special_forces = self
            .force_locations(game)
            .iter()
            .any(|(l, b)| b.amount_of_special_forces > game.player(self.player()).special_forces_in(Some(*l)));
        if too_many_special_forces {
            return Some(express!("Invalid amount of special forces"));
        }

        if self.forces_from_reserves > Self::max_forces_from_reserves(game, self.player(), false)
            || self.special_forces_from_reserves > Self::max_forces_from_reserves(game, self.player(), true)
        {
            return Some(express!("You can't ride with that many reserves"));
        }

        None
    }

    pub fn is_applicable(g: &Game) -> bool {
        Self::to_ride(g).is_some()
    }

    pub fn to_ride(g: &Game) -> Option<MonsterAppearence> {
        Self::to_ride_index(g).map(|i| g.monsters[i].clone())
    }

    /// Index in `Game.Monsters` of the monster `ToRide` returns (Rust-only: `List.Remove` in
    /// `ExecuteConcreteEvent` removes that object by reference).
    fn to_ride_index(g: &Game) -> Option<usize> {
        let fremen = g.get_player(Some(Faction::Fremen));
        if let Some(fremen) = fremen {
            return g.monsters.iter().position(|m| {
                (m.is_great_monster && g.player(fremen).any_forces_in_reserves() > 0 && !g.prevented(FactionAdvantage::FremenRidesMonster))
                    || (!m.is_great_monster && Self::locations_with_forces_that_can_ride(g, fremen, m.territory).next().is_some())
            });
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn locations_with_forces_that_can_ride<'a>(g: &'a Game, fremen: PlayerIndex, territory: Territory) -> Box<dyn Iterator<Item = Location> + 'a> {
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

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_sources<'a>(g: &'a Game) -> Box<dyn Iterator<Item = Location> + 'a> {
        let fremen = g.get_player(Some(Faction::Fremen));
        if let Some(fremen) = fremen {
            let to_ride = Self::to_ride(g);
            let to_ride = to_ride.nre();
            if !to_ride.is_great_monster {
                return Self::locations_with_forces_that_can_ride(g, fremen, to_ride.territory);
            }
        }

        Box::new(std::iter::empty())
    }

    pub fn max_forces_from_reserves(g: &Game, p: PlayerIndex, special: bool) -> i32 {
        let ride_from = Self::to_ride(g);
        if ride_from.is_some() && Self::to_ride(g).nre().is_great_monster {
            return if special { g.player(p).special_forces_in_reserve() } else { g.player(p).forces_in_reserve() };
        }

        0
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_targets(g: &Game, p: PlayerIndex) -> impl Iterator<Item = Location> {
        let may_move_into_storm = g.player(p).faction == Faction::Fremen
            && g.applicable(Rule::FremenMayMoveIntoStorm)
            && g.applicable(Rule::FremenStormLosses);

        g.map
            .locations(false)
            .filter(move |l| {
                (may_move_into_storm || l.sector(&g.map) != g.sector_in_storm)
                    && (match l.as_attached_location() {
                        None => true,
                        // Reads the sector of the location it is attached to, not al.Sector (hazards).
                        Some(al) => match al.attached_to_location(&g.map) {
                            Some(a) => a.sector(&g.map) != g.sector_in_storm,
                            None => false,
                        },
                    })
                    && (!l.territory().is_stronghold() || g.nr_of_occupants_excluding_faction(*l, g.player(p).faction) < 2)
                    && (!g.player(p).has_ally() || *l == g.map.polar_sink() || !g.player(g.player(p).allied_player(g).nre()).occupies(g, Some(*l)))
            })
    }

    pub fn is_either_valid_discovery_or_no_discovery(l: Location, map: &Map) -> bool {
        // C# parity: never called in C# (hazards: uncalled helpers are ported anyway).
        match l.as_discovered_location() {
            None => true,
            Some(ds) => ds.location().visible(map),
        }
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let to_ride_index = Self::to_ride_index(game);
        let to_ride = to_ride_index.map(|i| game.monsters[i].clone());

        if let Some(i) = to_ride_index {
            // List.Remove compares by reference; ToRide returned the element at this index.
            // Remove(null) finds nothing and does nothing.
            game.monsters.remove(i);
        }

        if !self.passed {
            if self.force_locations(game).keys().any(|l| l.territory() != to_ride.as_ref().nre().territory) {
                let me = self.player();
                game.play_nexus_card(me, elems!["cunning", "to ride from any territory on the planet"]);
            }

            let initiator = self.get_player(game, Some(self.initiator()));
            game.last_shipment_or_movement = Some(self.clone().into_event());
            for (from, value) in self.force_locations(game).iter() {
                let from = *from;
                // C# passes To (possibly null) into MoveForces, which removes the forces from
                // `from` and then throws ArgumentNullException on the null key.
                let ini = initiator.required("initiator")?;
                let to = self.to(game).or_null();
                Player::move_forces(game, ini, from, to, value.amount_of_forces);
                let to = self.to(game).or_null();
                Player::move_special_forces(game, ini, from, to, value.amount_of_special_forces);
                let msg = express!(
                    self.initiator(),
                    " ride ",
                    express_part_if!(value.amount_of_forces > 0; value.amount_of_forces, game.player(ini).force()),
                    express_part_if!(value.amount_of_special_forces > 0; value.amount_of_special_forces, game.player(ini).special_force()),
                    " from ",
                    from,
                    " to ",
                    self.to(game)
                )
                .with_route(self.initiator(), [from], self.to(game))
                .with_log(NexusLog::new(
                    Some(NexusRide::new(self.initiator(), value.amount_of_forces, value.amount_of_special_forces, Some(from), self.to(game).or_null())),
                    None,
                ));
                self.log_objects(game, elems![msg]);
            }

            if self.forces_from_reserves > 0 || self.special_forces_from_reserves > 0 {
                if self.forces_from_reserves > 0 {
                    let ini = initiator.required("initiator")?;
                    let to = self.to(game).or_null();
                    Player::ship_forces(game, ini, to, self.forces_from_reserves);
                }
                if self.special_forces_from_reserves > 0 {
                    let ini = initiator.required("initiator")?;
                    let to = self.to(game).or_null();
                    Player::ship_special_forces(game, ini, to, self.special_forces_from_reserves);
                }
                let ini = initiator.required("initiator")?;
                let msg = express!(
                    self.initiator(),
                    " ride ",
                    express_part_if!(self.forces_from_reserves > 0; self.forces_from_reserves, game.player(ini).force()),
                    express_part_if!(self.special_forces_from_reserves > 0; self.special_forces_from_reserves, game.player(ini).special_force()),
                    " from their reserves to ",
                    self.to(game)
                )
                .with_route(self.initiator(), [], self.to(game))
                .with_log(NexusLog::new(
                    Some(NexusRide::new(self.initiator(), self.forces_from_reserves, self.special_forces_from_reserves, None, self.to(game).or_null())),
                    None,
                ));
                self.log_objects(game, elems![msg]);
            }

            game.flip_bene_gesserit_advisors_when_alone();
            game.check_intrusion(&self.clone().into_event())?;
        } else {
            let msg = express!(self.initiator(), " pass a ride on ", Concept::Monster).with_log(NexusLog::new(None, None));
            self.log_objects(game, elems![msg]);
        }

        game.determine_next_shipment_and_move_sub_phase()?;
        Ok(())
    }

    pub fn get_message(&self, game: &Game) -> Message {
        if self.passed {
            return express!(self.initiator(), " don't ride");
        }

        let from = self.force_locations(game).keys().next().map(|l| l.territory());
        if let Some(from) = from {
            return express!(self.initiator(), " ride from ", from, " to ", self.to(game))
                .with_route(self.initiator(), self.force_locations(game).keys().copied(), self.to(game));
        }
        express!(self.initiator(), " ride from reserves to ", self.to(game))
    }

}

impl GameEventMethods for FremenRidesMonster {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::FremenRidesMonster
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::FremenRidesMonster(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        FremenRidesMonster::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        FremenRidesMonster::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        FremenRidesMonster::get_message(self, game)
    }

}

impl PassableGameEventMethods for FremenRidesMonster {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}

impl PlacementEventMethods for FremenRidesMonster {
    fn field_to_id(&self) -> &i32 {
        &self.to_id
    }

    fn field_force_locations(&self) -> &Option<String> {
        &self.force_locations
    }

    fn field_to_id_mut(&mut self) -> &mut i32 {
        &mut self.to_id
    }

    fn field_force_locations_mut(&mut self) -> &mut Option<String> {
        &mut self.force_locations
    }

    fn total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        FremenRidesMonster::total_amount_of_forces_added_to_location(self, game)
    }

}

impl crate::events::location_event::ILocationEvent for FremenRidesMonster {
    fn initiator(&self) -> Faction {
        GameEventMethods::initiator(self)
    }

    fn to(&self, game: &Game) -> Option<Location> {
        <Self as PlacementEventMethods>::to(self, game)
    }

    fn total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        <Self as PlacementEventMethods>::total_amount_of_forces_added_to_location(self, game)
    }

    fn forces_added_to_location(&self, game: &Game) -> i32 {
        <Self as PlacementEventMethods>::forces_added_to_location(self, game)
    }

    fn special_forces_added_to_location(&self, game: &Game) -> i32 {
        <Self as PlacementEventMethods>::special_forces_added_to_location(self, game)
    }

}

impl crate::events::placement::IPlacement for FremenRidesMonster {
    fn initiator(&self) -> Faction {
        GameEventMethods::initiator(self)
    }

    fn force_locations(&self, game: &Game) -> NetDictionary<Location, Battalion> {
        <Self as PlacementEventMethods>::force_locations(self, game)
    }

    fn to(&self, game: &Game) -> Option<Location> {
        <Self as PlacementEventMethods>::to(self, game)
    }

}
