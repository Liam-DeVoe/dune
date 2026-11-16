use crate::prelude::*;
use crate::messages::part::Part;

#[derive(Clone, Debug)]
pub struct Shipment {
    pub shipment_type: ShipmentType,
    pub to: Option<Location>,
    pub from: Option<Location>,
    pub force_amount: i32,
    pub special_force_amount: i32,
    pub smuggled_amount: i32,
    pub smuggled_special_amount: i32,
    pub no_field_value: i32,
    pub cunning_no_field_value: i32,
    pub ally_contribution_amount: i32,
    pub karma_card: Option<TreacheryCard>,
    pub use_richese_secret_ally: bool,
    pub force_locations: Option<String>,
    /// Interior mutability: written after the event is stored in game state.
    pub guild_income_prevented: Cell<bool>,
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for Shipment {
    fn default() -> Self {
        Shipment {
            shipment_type: Default::default(),
            to: Location::from_id(0),
            from: Location::from_id(0),
            force_amount: Default::default(),
            special_force_amount: Default::default(),
            smuggled_amount: Default::default(),
            smuggled_special_amount: Default::default(),
            no_field_value: Default::default(),
            cunning_no_field_value: Default::default(),
            ally_contribution_amount: Default::default(),
            karma_card: TreacheryCardManager::get(0),
            use_richese_secret_ally: Default::default(),
            force_locations: Some("".to_string()),
            guild_income_prevented: Cell::new(Default::default()),
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

/// Builds a list of message elements from mixed arguments.
macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(Part::from($e)),*] };
}

impl Shipment {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn source_location_for_shipment_to_reserves(&self) -> Option<Location> {
        self.to
    }

    pub fn target_world_for_shipment_to_reserves(&self) -> Option<Homeworld> {
        self.from.and_then(|l| l.as_homeworld())
    }

    pub fn no_field_value(&self) -> i32 {
        self.no_field_value - 1
    }

    pub fn set_no_field_value(&mut self, value: i32) {
        self.no_field_value = value + 1;
    }

    pub fn cunning_no_field_value(&self) -> i32 {
        self.cunning_no_field_value - 1
    }

    pub fn set_cunning_no_field_value(&mut self, value: i32) {
        self.cunning_no_field_value = value + 1;
    }

    pub fn is_using_karma(&self) -> bool {
        self.karma_card.is_some()
    }

    pub fn is_back_to_reserves(&self) -> bool {
        // ForceAmount + SpecialForceAmount is an unchecked int addition.
        self.shipment_type == ShipmentType::ShipmentBack
            || (self.shipment_type == ShipmentType::Unknown && (self.force_amount + self.special_force_amount) < 0)
    }

    pub fn is_site_to_site(&self) -> bool {
        self.shipment_type == ShipmentType::ShipmentSiteToSite
            || (self.shipment_type == ShipmentType::Unknown && self.from.is_some())
    }

    pub fn is_no_field(&self) -> bool {
        self.no_field_value() >= 0
    }

    pub fn total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        self.forces_added_to_location(game) + self.special_forces_added_to_location(game)
    }

    pub fn forces_added_to_location(&self, _game: &Game) -> i32 {
        if self.use_richese_secret_ally { 1 } else { self.force_amount + self.smuggled_amount }
    }

    pub fn special_forces_added_to_location(&self, _game: &Game) -> i32 {
        if self.use_richese_secret_ally { 0 } else { self.special_force_amount + self.smuggled_special_amount }
    }

    pub fn force_locations(&self, game: &Game) -> IndexMap<Location, Battalion> {
        // Player.Faction: NullReferenceException when Player is null, before the string is parsed.
        let f = game.player(self.player()).faction;
        Shipment::parse_force_locations(game, f.expect("faction"), self.force_locations.as_deref())
    }

    pub fn set_force_locations(&mut self, game: &Game, value: &IndexMap<Location, Battalion>) {
        self.force_locations = Some(Shipment::force_locations_string(game, value));
    }

    pub fn determine_guild_profits(&self, game: &Game) -> i32 {
        let initiator = self.player();
        let cost = self.determine_cost_to_initiator(game);
        let extra = if game.player(initiator).ally != Some(Faction::Guild) || game.applicable(Rule::GuildShipmentContributionsFlowBack) {
            self.ally_contribution_amount
        } else {
            0
        };
        cost + extra
    }

    pub fn determine_cost_to_initiator(&self, g: &Game) -> i32 {
        let Some(_to) = self.to else { return 0 };

        Shipment::determine_cost(g, self.player(), self) - self.ally_contribution_amount
    }

    pub fn force_locations_string(g: &Game, force_locations: &IndexMap<Location, Battalion>) -> String {
        force_locations
            .iter()
            .map(|(k, v)| format!("{}:{}|{}", g.board.map.location_lookup().get_id(Some(*k)), v.amount_of_forces, v.amount_of_special_forces))
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn parse_force_locations(g: &Game, f: Faction, force_locations: Option<&str>) -> IndexMap<Location, Battalion> {
        let mut result = IndexMap::new();
        if let Some(s) = force_locations {
            if !s.is_empty() {
                for location_amount_pair in s.split(',') {
                    let location_and_amounts: Vec<&str> = location_amount_pair.split(':').collect();
                    let location = g
                        .board.map
                        .location_lookup()
                        .find(dotnet_compat::parse::convert_to_int32(Some(location_and_amounts[0])));

                    let Some(location) = location else { continue };

                    let amounts: Vec<&str> = location_and_amounts[1].split('|').collect();
                    let amount_of_normal_forces = dotnet_compat::parse::convert_to_int32(Some(amounts[0]));
                    let amount_of_normal_special_forces = dotnet_compat::parse::convert_to_int32(Some(amounts[1]));
                    result.insert_new(location, Battalion::new(f, amount_of_normal_forces, amount_of_normal_special_forces, location));
                }
            }
        }

        result
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if self.passed {
            return None;
        }

        // `var p = Player;` may be null: it throws only where C# dereferences it.
        let p_opt = self.player_opt();
        let p = || p_opt.expect("player");

        if !self.is_site_to_site() && game.prevented_from_shipping(self.initiator()) {
            return Some(express!(TreacheryCardType::Karma, " prevents you from shipping"));
        }

        if self.is_back_to_reserves() && !Shipment::may_ship_to_reserves(game, p()) {
            return Some(express!("You can't ship back to reserves"));
        }
        if self.is_back_to_reserves() {
            let source = self.source_location_for_shipment_to_reserves();
            let mut valid = Shipment::valid_source_locations(game, p());
            if !valid.any(|v| Some(v) == source) {
                return Some(express!("You can't ship back to reserves from there"));
            }
        }
        if self.is_back_to_reserves() {
            if let Some(target) = self.target_world_for_shipment_to_reserves() {
                // List<Homeworld>.Contains: Location.Equals, by Id.
                if !game.player(p()).home_worlds.iter().any(|hw| hw.location() == target.location()) {
                    return Some(express!("You can't ship back to that home world"));
                }
            }
        }

        if self.is_site_to_site() && !Shipment::may_ship_cross_planet(game, p()) {
            return Some(express!("You can't ship site-to-site"));
        }
        if self.is_site_to_site() {
            let from = self.from;
            let mut valid = Shipment::valid_source_locations(game, p());
            if !valid.any(|v| Some(v) == from) {
                return Some(express!("You can't ship site-to-site from there"));
            }
        }

        if !self.is_back_to_reserves() && (self.force_amount < 0 || self.special_force_amount < 0) {
            return Some(express!("Can't ship less than zero forces"));
        }
        if self.force_amount == 0 && self.special_force_amount == 0 {
            return Some(express!("Select forces to ship"));
        }
        let Some(to) = self.to else {
            return Some(express!("Target location not selected"));
        };

        // A shipment back to reserves has negative amounts: the forces leave `to`.
        if self.is_back_to_reserves() {
            let pl = game.player(p());
            if self.force_amount > 0 || self.special_force_amount > 0 {
                return Some(express!("Can't ship forces from reserves when shipping back to reserves"));
            }
            if -(self.force_amount as i64) > pl.forces_in(Some(to)) as i64 {
                return Some(express!("Not enough ", pl.force(), " to ship back to reserves"));
            }
            if -(self.special_force_amount as i64) > pl.special_forces_in(Some(to)) as i64 {
                return Some(express!("Not enough ", pl.special_force(), " to ship back to reserves"));
            }
        }

        let cost = Shipment::determine_cost(game, p(), self);
        if cost > (game.player(p()).resources + self.ally_contribution_amount) {
            return Some(express!("You can't pay that much"));
        }
        if cost < self.ally_contribution_amount {
            return Some(express!("Your ally is paying more than needed"));
        }
        // Not in C#, which let an ally pay spice it didn't have or hadn't offered
        // (behavior-changes.md). Bid and Battle check the same limit.
        if self.ally_contribution_amount > 0 && self.ally_contribution_amount > game.resources_your_ally_can_pay(p()) {
            return Some(express!("Your ally won't pay that much"));
        }
        let cross_planet = self.is_back_to_reserves() || (self.is_site_to_site() && !self.from.is_some_and(|f| f.as_homeworld().is_some()));
        if !Shipment::valid_shipment_locations(game, p(), cross_planet, self.use_richese_secret_ally).any(|v| v == to) {
            return Some(express!("Cannot ship there"));
        }

        //no field checks, requires refactoring
        if self.no_field_value() >= 0 && !Shipment::may_use_no_field(game, p()) {
            return Some(express!("You can't use a No-Field"));
        }

        let is_richese_no_field_shipment = game.player(p()).faction == Some(Faction::Richese) && self.no_field_value() >= 0;
        if is_richese_no_field_shipment && self.force_amount > 0 && !game.player(p()).has_any_high_threshold(game) {
            return Some(express!("You can't do both normal and No-Field shipment"));
        }
        if is_richese_no_field_shipment && self.special_force_amount != 1 {
            return Some(express!("Invalid special force value for No-Field shipment"));
        }

        if game.player(p()).faction == Some(Faction::Richese)
            && self.special_force_amount > 0
            && !Shipment::valid_no_field_values(game, p()).contains(&self.no_field_value())
        {
            return Some(express!("Invalid No-Field value"));
        }

        if self.cunning_no_field_value() >= 0 && !Shipment::may_use_cunning_no_field(game, p()) {
            return Some(express!("You cannot use cunning to ship a second No-Field"));
        }
        if self.no_field_value() < 0 && self.cunning_no_field_value() >= 0 {
            return Some(express!("You can only use cunning when you also ship a regular No-Field"));
        }
        if self.cunning_no_field_value() >= 0 && self.force_amount > 0 && !game.player(p()).has_any_high_threshold(game) {
            return Some(express!("You can't do both normal and No-Field shipment"));
        }
        if self.cunning_no_field_value() >= 0
            && !Shipment::valid_cunning_no_field_values(game, p(), self.no_field_value()).contains(&self.cunning_no_field_value())
        {
            return Some(express!("Invalid Cunning No-Field value"));
        }

        // C# parity: the sums wrap, so int.MaxValue + 1 passes the check.
        if !self.is_site_to_site() && (self.force_amount + self.smuggled_amount) > game.player(p()).forces_in_reserve() {
            return Some(express!("Not enough ", game.player(p()).force(), " in reserve"));
        }
        if !self.is_site_to_site()
            && !is_richese_no_field_shipment
            && (self.special_force_amount + self.smuggled_special_amount) > game.player(p()).special_forces_in_reserve()
        {
            return Some(express!("Not enough ", game.player(p()).special_force(), " in reserve"));
        }

        let is_smuggling = self.smuggled_amount > 0 || self.smuggled_special_amount > 0;
        let is_shipping_from_off_planet = !(self.is_back_to_reserves() || self.is_site_to_site())
            && self.initiator() != Some(Faction::Fremen)
            && (self.force_amount > 0 || self.special_force_amount > 0);
        if is_smuggling && (!is_shipping_from_off_planet || !Shipment::may_smuggle(game, p(), Some(to))) {
            return Some(express!("You can't smuggle forces here"));
        }
        if (self.smuggled_amount + self.smuggled_special_amount) > 1 {
            return Some(express!("You can't smuggle more than 1 force"));
        }

        if self.is_site_to_site() && self.force_amount > game.player(p()).forces_in(self.from) {
            return Some(express!("Not enough ", game.player(p()).force(), " for site-to-site shipment"));
        }
        if self.is_site_to_site() && self.special_force_amount > game.player(p()).special_forces_in(self.from) {
            return Some(express!("Not enough ", game.player(p()).special_force(), " for site-to-site shipment"));
        }

        if self.is_no_field() && game.player(p()).faction != Some(Faction::Richese) {
            let pl = game.player(p());
            let forces_to_ship = std::cmp::min(self.no_field_value(), pl.forces_in_reserve() + pl.special_forces_in_reserve());
            if (self.force_amount + self.special_force_amount) != forces_to_ship {
                return Some(express!("Using a No-Field of ", self.no_field_value(), ", you must select ", forces_to_ship, " forces to ship"));
            }
        }

        if let Some(karma_card) = self.karma_card {
            if !Shipment::valid_karma_cards(game, p()).contains(&karma_card) {
                return Some(express!("Invalid ", TreacheryCardType::Karma, " card: ", self.karma_card));
            }
        }

        if self.use_richese_secret_ally && !Shipment::may_use_richese_secret_ally(game, p()) {
            return Some(express!(Faction::Richese, " are not your secret ally"));
        }
        if self.use_richese_secret_ally && (self.force_amount + self.special_force_amount) > 5 {
            return Some(express!("You can't ship that much using ", Faction::Richese, " as your secret ally"));
        }

        // Sum is checked: OverflowException.
        let number_of_forces_selected = self.force_locations(game).values().map(|b| b.amount_of_forces).sum::<i32>();
        if self.forces_added_to_location(game) > 0
            && Shipment::homeworlds_to_ship_from(game, p(), false).len() > 1
            && number_of_forces_selected <= 0
        {
            return Some(express!("Please specify where to ship your ", game.player(p()).force(), " from"));
        }
        if number_of_forces_selected > 0 && number_of_forces_selected != self.forces_added_to_location(game) {
            return Some(express!("Select the sources of ", self.forces_added_to_location(game), game.player(p()).force(), " in total"));
        }

        let number_of_special_forces_selected =
            self.force_locations(game).values().map(|b| b.amount_of_special_forces).sum::<i32>();
        if self.special_forces_added_to_location(game) > 0
            && Shipment::homeworlds_to_ship_from(game, p(), true).len() > 1
            && number_of_special_forces_selected <= 0
        {
            return Some(express!("Please specify where to ship your ", game.player(p()).special_force(), " from"));
        }
        if number_of_special_forces_selected > 0
            && number_of_special_forces_selected != self.special_forces_added_to_location(game)
        {
            return Some(express!(
                "Select the source of in total ",
                self.special_forces_added_to_location(game),
                game.player(p()).special_force()
            ));
        }

        None
    }

    pub fn homeworlds_to_ship_from(game: &Game, p: PlayerId, special: bool) -> Vec<Homeworld> {
        let pl = game.player(p);
        if special {
            pl.home_worlds.iter().copied().filter(|hw| pl.special_forces_in(Some(hw.location())) > 0).collect()
        } else {
            pl.home_worlds.iter().copied().filter(|hw| pl.forces_in(Some(hw.location())) > 0).collect()
        }
    }

    pub fn fremen_spawn_locations(g: &Game, p: PlayerId) -> Vec<Location> {
        let m = &g.board.map;
        g.board.map
            .locations(g.applicable(Rule::Homeworlds))
            .filter(|l| {
                let l = *l;
                g.is_not_full(p, l)
                    && (l == m.the_great_flat()
                        || l.id() == m.the_greater_flat().id()
                        || l == m.funeral_plain()
                        || l.territory() == m.bight_of_the_cliff()
                        || l == m.sietch_tabr()
                        || l.territory() == m.plastic_basin()
                        || l.territory() == m.rock_outcroppings()
                        || l.territory() == m.broken_land()
                        || l.territory() == m.tsimpo()
                        || l.territory() == m.hagga_basin()
                        || l.id() == m.polar_sink().id()
                        || l.territory() == m.wind_pass()
                        || l.territory() == m.wind_pass_north()
                        || l.territory() == m.cielago_west()
                        || l.territory() == m.false_wall_west()
                        || l.territory() == m.habbanya_erg()
                        || l.as_discovered_location().is_some_and(|ds| {
                            ds.visible(m) && ds.attached_to_location(m).map(|a| a.territory()) == Some(m.pasty_mesa())
                        })
                        || l.as_discovered_location().is_some_and(|ds| {
                            ds.visible(m) && ds.attached_to_location(m).map(|a| a.territory()) == Some(m.false_wall_west())
                        }))
            })
            .collect()
    }

    pub fn determine_cost(g: &Game, p: PlayerId, s: &Shipment) -> i32 {
        match s.to {
            None => 0,
            Some(to) => {
                let forces = s.force_amount.abs();
                let special_forces = s.special_force_amount.abs();
                Shipment::determine_cost_of_forces(
                    g,
                    p,
                    forces,
                    special_forces,
                    to,
                    s.is_using_karma(),
                    s.is_back_to_reserves(),
                    s.is_site_to_site(),
                    s.is_richese_no_field(),
                    s.is_non_richese_no_field(),
                )
            }
        }
    }

    pub fn is_richese_no_field(&self) -> bool {
        self.initiator() == Some(Faction::Richese) && self.is_no_field()
    }

    pub fn is_non_richese_no_field(&self) -> bool {
        (self.initiator() != Some(Faction::Richese) && self.is_no_field()) || self.use_richese_secret_ally
    }

    pub fn determine_cost_of_forces(g: &Game, p: PlayerId, forces: i32, special_forces: i32, to: Location, karama_shipment: bool, back_to_reserves: bool, site_to_site: bool, no_field_by_richese: bool, no_field_by_non_richese: bool) -> i32 {
        let mut total_forces = forces + special_forces;
        let no_field_or_secret_ally = no_field_by_richese || no_field_by_non_richese;

        if no_field_by_non_richese {
            total_forces = std::cmp::min(forces + special_forces, 1);
        } else if no_field_by_richese {
            total_forces = std::cmp::max(forces, 1);
        }

        // (int)Math.Ceiling(0.5f * totalForces): float product, widened to double for Math.Ceiling.
        if back_to_reserves {
            return ((0.5f32 * total_forces as f32) as f64).ceil() as i32;
        }

        if !site_to_site
            && g.player(p).is(Faction::Fremen)
            && Shipment::fremen_spawn_locations(g, p).contains(&to)
            && !no_field_or_secret_ally
        {
            return 0;
        }

        // The int product can wrap before it is widened to double.
        let mut cost_of_shipment: f64 =
            (total_forces.abs() * if to.territory().has_reduced_shipping_cost() { 1 } else { 2 }) as f64;

        if Shipment::may_ship_with_discount(g, p) || karama_shipment || (site_to_site && g.has_shipment_permission(p, ShipmentPermission::GUILD_RATE)) {
            cost_of_shipment /= 2.0;
        }

        cost_of_shipment.ceil() as i32
    }

    pub fn valid_max_normal_shipment_forces(game: &Game, p: PlayerId, special_forces: bool, used_no_field: i32) -> i32 {
        let pl = game.player(p);
        let no_field_max = if used_no_field < 0 || pl.faction == Some(Faction::Richese) { i32::MAX } else { used_no_field };
        if special_forces {
            std::cmp::min(pl.special_forces_in_reserve(), no_field_max)
        } else {
            std::cmp::min(pl.forces_in_reserve(), no_field_max)
        }
    }

    pub fn default_no_field_special_force_amount(faction: Faction, no_field_value: i32) -> i32 {
        if faction == Faction::Richese && no_field_value >= 0 { 1 } else { 0 }
    }

    pub fn valid_max_secret_ally_shipment_forces(game: &Game, p: PlayerId, special_forces: bool) -> i32 {
        let pl = game.player(p);
        if special_forces {
            std::cmp::min(pl.special_forces_in_reserve(), 5)
        } else {
            std::cmp::min(pl.forces_in_reserve(), 5)
        }
    }

    pub fn valid_max_shipment_back_forces(game: &Game, p: PlayerId, special_forces: bool, source: Option<Location>) -> i32 {
        if source.is_none() {
            return 0;
        }
        let pl = game.player(p);
        if special_forces { pl.special_forces_in(source) } else { pl.forces_in(source) }
    }

    pub fn valid_max_shipment_site_to_site_forces(game: &Game, p: PlayerId, special_forces: bool, source: Option<Location>) -> i32 {
        if source.is_none() {
            return 0;
        }

        let pl = game.player(p);
        if special_forces { pl.special_forces_in(source) } else { pl.forces_in(source) }
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_shipment_locations(g: &Game, p: PlayerId, cross_planet: bool, richese_secret_ally: bool) -> impl Iterator<Item = Location> {
        use either::Either;
        if !g.player(p).is(Faction::Fremen) || richese_secret_ally {
            return Either::Left(Shipment::regular_shipment_locations(g, p, cross_planet));
        }

        if Shipment::may_ship_cross_planet(g, p) {
            // FremenSpawnLocations(g, p).Union(RegularShipmentLocations(...)).Distinct(): both calls run now,
            // the Where inside RegularShipmentLocations and the Union run as the caller pulls. The
            // Distinct after the Union passes everything through.
            let spawn = Shipment::fremen_spawn_locations(g, p);
            let regular = Shipment::regular_shipment_locations(g, p, cross_planet);
            Either::Right(Either::Left(crate::iter::unique(spawn.into_iter().chain(regular))))
        } else {
            Either::Right(Either::Right(Shipment::fremen_spawn_locations(g, p).into_iter()))
        }
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn regular_shipment_locations(g: &Game, p: PlayerId, cross_planet: bool) -> impl Iterator<Item = Location> {
        g.board.map
            .locations(g.applicable(Rule::Homeworlds))
            .filter(move |l| {
                let l = *l;
                Some(l.territory()) != g.board.atomics_aftermath
                    && !Sector::in_storm(l.sector(&g.board.map), g.board.sector_in_storm)
                    && (l.id() != g.board.map.hidden_mobile_stronghold().location().id() || g.player(p).is(Faction::Ixian))
                    && Shipment::is_either_valid_homeworld_or_no_homeworld(g, p, l, cross_planet)
                    && Shipment::is_either_valid_discovery_or_no_discovery(&g.board.map, l)
                    && g.is_not_full(p, l)
            })
    }

    pub fn is_either_valid_homeworld_or_no_homeworld(g: &Game, p: PlayerId, l: Location, from_planet: bool) -> bool {
        let Some(hw) = l.as_homeworld() else { return true };
        let pl = g.player(p);

        g.applicable(Rule::Homeworlds)
            && (!from_planet
                || pl.is(Faction::Guild)
                || !pl.is_native_homeworld(hw)
                || (pl.is_native_homeworld(hw) && Shipment::may_ship_to_reserves(g, p))
                || (pl.is_native_homeworld(hw) && g.has_shipment_permission(p, ShipmentPermission::TO_HOMEWORLD))) // from planet to OWN homeworld
            && (!from_planet
                || pl.is(Faction::Guild)
                || pl.is_native_homeworld(hw)
                || (!pl.is_native_homeworld(hw) && g.has_shipment_permission(p, ShipmentPermission::CROSS))) // from planet to OTHER homeworld
            && g.players.iter().any(|native| native.is_native_homeworld(hw))
            && (!pl.has_ally() || {
                // p.AlliedPlayer may be null when the ally is not in the game: NullReferenceException.
                let ally = g.player(pl.allied_player(g).expect("allied player"));
                !ally.is_native_homeworld(hw) && ally.any_forces_in(Some(hw.location())) == 0
            })
    }

    /// Takes the map because `ds.Visible` reads it.
    pub fn is_either_valid_discovery_or_no_discovery(map: &Map, l: Location) -> bool {
        match l.as_discovered_location() {
            None => true,
            Some(ds) => ds.visible(map),
        }
    }

    pub fn valid_no_field_values(g: &Game, p: PlayerId) -> Vec<i32> {
        let pl = g.player(p);
        if (pl.faction != Some(Faction::Richese) || g.prevented(FactionAdvantage::RicheseNofield)) && (pl.ally != Some(Faction::Richese) || !g.alliances.richese_allows_use_of_no_field) {
            return vec![];
        }

        let mut result = Vec::new();
        if pl.faction == Some(Faction::Richese) && g.ship_move.latest_revealed_no_field_value != 0 && g.ship_move.current_no_field_value != 0 {
            result.push(0);
        }
        if g.ship_move.latest_revealed_no_field_value != 3 && g.ship_move.current_no_field_value != 3 {
            result.push(3);
        }
        if g.ship_move.latest_revealed_no_field_value != 5 && g.ship_move.current_no_field_value != 5 {
            result.push(5);
        }
        result
    }

    pub fn valid_cunning_no_field_values(g: &Game, p: PlayerId, value_of_normal_no_field: i32) -> Vec<i32> {
        let pl = g.player(p);
        if (pl.faction != Some(Faction::Richese) || g.prevented(FactionAdvantage::RicheseNofield)) && (pl.ally != Some(Faction::Richese) || !g.alliances.richese_allows_use_of_no_field) {
            return vec![];
        }

        let mut result = Vec::new();
        if pl.faction == Some(Faction::Richese) && g.ship_move.latest_revealed_no_field_value != 0 && g.ship_move.current_no_field_value != 0 && value_of_normal_no_field != 0 {
            result.push(0);
        }
        if g.ship_move.latest_revealed_no_field_value != 3 && g.ship_move.current_no_field_value != 3 && value_of_normal_no_field != 3 {
            result.push(3);
        }
        if g.ship_move.latest_revealed_no_field_value != 5 && g.ship_move.current_no_field_value != 5 && value_of_normal_no_field != 5 {
            result.push(5);
        }
        result
    }

    pub fn valid_karma_cards(g: &Game, p: PlayerId) -> Vec<TreacheryCard> {
        let mut result = Shipment::valid_own_karma_cards(g, p);

        let ally_card = Shipment::valid_ally_karma_card(g, p);
        if let Some(c) = ally_card {
            result.push(c);
        }

        result
    }

    pub fn valid_own_karma_cards(g: &Game, p: PlayerId) -> Vec<TreacheryCard> {
        Karma::valid_karma_cards(g, p).collect()
    }

    pub fn valid_ally_karma_card(g: &Game, p: PlayerId) -> Option<TreacheryCard> {
        g.get_permitted_use_of_ally_karma(g.player(p).faction.expect("faction"))
    }

    pub fn can_karma(g: &Game, p: PlayerId) -> bool {
        !Shipment::valid_karma_cards(g, p).is_empty()
    }

    pub fn may_smuggle(g: &Game, p: PlayerId, l: Option<Location>) -> bool {
        match l {
            None => false,
            Some(l) => {
                !g.any_forces_in(l.territory())
                    && g.player(p).faction != Some(Faction::Fremen)
                    && g.has_skill_in_front_of_shield(p, LeaderSkill::Smuggler)
            }
        }
    }

    pub fn may_use_no_field(g: &Game, p: PlayerId) -> bool {
        let pl = g.player(p);
        (pl.faction == Some(Faction::Richese) && !g.prevented(FactionAdvantage::RicheseNofield)) || (pl.ally == Some(Faction::Richese) && g.alliances.richese_allows_use_of_no_field)
    }

    pub fn may_use_cunning_no_field(game: &Game, p: PlayerId) -> bool {
        game.player(p).faction == Some(Faction::Richese) && NexusPlayed::can_use_cunning(game, p)
    }

    pub fn may_use_richese_secret_ally(g: &Game, p: PlayerId) -> bool {
        g.player(p).nexus == Some(Faction::Richese) && NexusPlayed::can_use_secret_ally(g, p)
    }

    fn initiated_current_guild_secret_ally(g: &Game, p: PlayerId) -> bool {
        let e = g.turn.current_guild_secret_ally.clone().map(|e| e.into_event());
        g.player(p).initiated(e.as_ref())
    }

    pub fn may_ship_cross_planet(g: &Game, p: PlayerId) -> bool {
        let pl = g.player(p);
        (pl.is(Faction::Guild) && !g.prevented(FactionAdvantage::GuildSpecialShipments))
            || (pl.ally == Some(Faction::Guild) && g.alliances.guild_allows_shipping_discount)
            || Shipment::initiated_current_guild_secret_ally(g, p)
            || g.has_shipment_permission(p, ShipmentPermission::CROSS)
    }

    pub fn may_ship_to_reserves(g: &Game, p: PlayerId) -> bool {
        let pl = g.player(p);
        (pl.is(Faction::Guild) && !g.prevented(FactionAdvantage::GuildSpecialShipments))
            || Shipment::initiated_current_guild_secret_ally(g, p)
            || g.has_shipment_permission(p, ShipmentPermission::TO_HOMEWORLD)
    }

    pub fn may_ship_with_discount(g: &Game, p: PlayerId) -> bool {
        let pl = g.player(p);
        (pl.is(Faction::Guild) && !g.prevented(FactionAdvantage::GuildShipmentsDiscount))
            || (pl.ally == Some(Faction::Guild) && g.alliances.guild_allows_shipping_discount && !g.prevented(FactionAdvantage::GuildShipmentsDiscountAlly))
            || Shipment::initiated_current_guild_secret_ally(g, p)
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_source_locations(g: &Game, p: PlayerId) -> impl Iterator<Item = Location> {
        g.locations_with_any_forces_not_in_storm(p)
            .into_iter()
            .filter(move |x| !x.is_homeworld() || Shipment::may_ship_cross_planet(g, p) || g.has_shipment_permission(p, ShipmentPermission::CROSS))
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.ship_move.beginning_of_shipment_and_move_phase = false;
        game.ship_move.current_blocked_territories.clear();
        game.storm.storm_losses_to_take.clear();
        game.ship_move.chosen_destinations_with_allies.clear();
        game.ship_move.bene_gesserit_may_accompany = false;

        let mut received_payment_message = express_part!();
        let mut total_cost = 0;

        if !self.passed {
            let owner_of_karma = match self.karma_card {
                Some(c) => game.owner_of_treachery_card(c),
                None => None,
            };
            total_cost = self.pay_for_shipment(game)?;

            if self.is_no_field() && self.to.is_some() {
                let richese = self.get_player(game, Some(Faction::Richese)).required("get player")?;

                if self.cunning_no_field_value() >= 0 {
                    game.reveal_current_no_field(richese, None);
                    Player::ship_special_forces(game, richese, self.to.required("to")?, 1);
                    game.ship_move.current_no_field_value = self.cunning_no_field_value();
                    let me = self.player();
                    game.play_nexus_card(me, elems!["ship and reveal a second ", FactionSpecialForce::Richese]);
                }

                game.reveal_current_no_field(richese, None);
                game.ship_move.current_no_field_value = self.no_field_value();

                if self.initiator() != Some(Faction::Richese) {
                    //Immediately reveal No-Field
                    game.ship_move.latest_revealed_no_field_value = self.no_field_value();
                    game.ship_move.current_no_field_value = -1;
                }
            }

            if let Some(to) = self.to {
                if game.contains_conflicting_ally(self.player(), to) {
                    game.ship_move.chosen_destinations_with_allies.push(to.territory());
                }
            }

            game.ship_move.last_shipment_or_movement = Some(self.clone().into_event());
            let must_be_advisors = {
                let pl = game.player(self.player());
                let to = self.to;
                (pl.is(Faction::BeneGesserit) && pl.special_forces_in(to) > 0)
                    || (to.is_some() && pl.special_forces_in_territory(to.map(|t| t.territory())) > 0)
            };

            if self.is_site_to_site() {
                self.perform_site_to_site_shipment(game);
            } else if self.is_back_to_reserves() {
                if let Some(source) = self.source_location_for_shipment_to_reserves() {
                    let me = self.player();
                    let target = match self.target_world_for_shipment_to_reserves() {
                        Some(hw) => hw,
                        None => game.player(me).home_worlds[0],
                    };
                    // The amounts are negative, so the forces move from `source` to `target`.
                    Player::move_forces(game, me, source, target.location(), -self.force_amount);
                    Player::special_forces_back_to_reserves(game, me, source, target, -self.special_force_amount);
                }
            } else if self.is_no_field() && self.initiator() == Some(Faction::Richese) && self.to.is_some() {
                let to = self.to.required("to")?;
                self.perform_normal_shipment(game, self.player(), to, self.force_amount + self.smuggled_amount, 1);
            } else if let Some(to) = self.to {
                self.perform_normal_shipment(
                    game,
                    self.player(),
                    to,
                    self.force_amount + self.smuggled_amount,
                    self.special_force_amount + self.smuggled_special_amount,
                );
            }

            if self.use_richese_secret_ally {
                game.play_nexus_card(self.player(), elems!["ship from reserves as if shipping ", 1]);
            }

            if !self.by(Faction::Fremen) {
                game.stone(Milestone::Shipment);
            }

            if let Some(to) = self.to {
                Player::flip_forces(game, self.player(), to, must_be_advisors);
            }

            if !self.is_back_to_reserves() {
                game.check_intrusion(&self.clone().into_event())?;
            }

            game.determine_next_shipment_and_move_sub_phase()?;

            let received_payment = self.handle_received_shipment_payment(game, &mut received_payment_message);
            let m = self
                .get_verbose_message(game, total_cost, &received_payment_message, owner_of_karma)
                .with_log(self.log_record(game, total_cost, received_payment));
            self.log_objects(game, elems![m]);

            if (total_cost - received_payment) >= 4 {
                game.activate_banker(self.player());
            }

            game.flip_bene_gesserit_advisors_when_alone();
            let to = self.to;
            game.determine_occupation_location(to);
        } else {
            let m = self.get_verbose_message(game, total_cost, &received_payment_message, None).with_log(self.log_record(game, 0, 0));
            self.log_objects(game, elems![m]);
            game.determine_next_shipment_and_move_sub_phase()?;
        }
        Ok(())
    }

    pub fn pay_for_shipment(&self, game: &mut Game) -> GameResult<i32> {
        let cost_to_initiator = self.determine_cost_to_initiator(game);
        let me = self.player();
        let pl = game.player_mut(me);
        pl.resources -= cost_to_initiator;

        if self.is_using_karma() {
            let karma_card = self.karma_card;
            game.discard_card(karma_card, true)?;
            game.stone(Milestone::Karma);
        }

        if self.ally_contribution_amount > 0 {
            let ally = game.player(self.player()).ally;
            if let Some(a) = game.get_player(ally) {
                let ap = game.player_mut(a);
                ap.resources -= self.ally_contribution_amount;
            }
            game.decrease_permitted_use_of_ally_spice(self.initiator().required("faction")?, self.ally_contribution_amount);
        }

        Ok(cost_to_initiator + self.ally_contribution_amount)
    }

    pub fn handle_received_shipment_payment(self: &Rc<Self>, game: &mut Game, profit_message: &mut MessagePart) -> i32 {
        let total_cost = Shipment::determine_cost(game, self.player(), self);

        let mut receiver_profit = 0;
        let guild = self.get_player(game, Some(Faction::Guild));

        if let Some(guild) = guild {
            if !self.by(Faction::Guild) && !self.is_using_karma() {
                receiver_profit = self.determine_guild_profits(game);

                if receiver_profit > 0 {
                    if !game.prevented(FactionAdvantage::GuildReceiveShipment) {
                        game.modify_income_because_of_low_threshold_or_occupation(guild, &mut receiver_profit);
                        let op = game.player_mut(guild);
                        op.resources += receiver_profit;

                        if receiver_profit > 0 {
                            *profit_message = express_part!(" → ", Faction::Guild, " get ", Payment::of(receiver_profit));

                            if receiver_profit >= 5 {
                                game.apply_bureaucracy(self.initiator(), Some(Faction::Guild));
                            }
                        }
                    } else {
                        *profit_message =
                            express_part!(" → ", TreacheryCardType::Karma, " prevents ", Faction::Guild, " from receiving ", Payment::of(receiver_profit));
                        self.guild_income_prevented.set(true);
                        if !game.applicable(Rule::FullPhaseKarma) {
                            game.allow(FactionAdvantage::GuildReceiveShipment);
                        }
                    }
                }
            }
        }

        let ev = self.clone().into_event();
        game.set_recent_payment(receiver_profit, self.initiator(), Some(Faction::Guild), &ev);
        game.set_recent_payment_to_bank(total_cost - receiver_profit, self.initiator(), &ev);

        receiver_profit
    }

    pub fn perform_site_to_site_shipment(&self, game: &mut Game) {
        let (Some(from), Some(to)) = (self.from, self.to) else { return };

        Player::move_forces(game, self.player(), from, to, self.force_amount);
        Player::move_special_forces(game, self.player(), from, to, self.special_force_amount);
    }

    pub fn perform_normal_shipment(&self, game: &mut Game, initiator: PlayerId, to: Location, force_amount: i32, special_force_amount: i32) {
        let initiator_faction = game.player(initiator).faction;
        game.ship_move.bene_gesserit_may_accompany =
            (force_amount > 0 || special_force_amount > 0) && initiator_faction != Some(Faction::Fremen) && initiator_faction != Some(Faction::BeneGesserit);

        // ForceLocations is parsed afresh on every read; each read gives the same result here.
        if Shipment::homeworlds_to_ship_from(game, self.player(), false).len() > 1 && self.force_locations(game).len() > 0 {
            let fls: Vec<(Location, Battalion)> = self
                .force_locations(game)
                .iter()
                .filter(|(_, v)| v.amount_of_forces > 0)
                .map(|(k, v)| (*k, v.clone()))
                .collect();
            for fl in fls {
                if let Some(hw) = fl.0.as_homeworld() {
                    Player::ship_forces_from_homeworld(game, initiator, to, hw, fl.1.amount_of_forces);
                }
            }
        } else {
            Player::ship_forces(game, initiator, to, force_amount);
        }

        if Shipment::homeworlds_to_ship_from(game, self.player(), true).len() > 1 && self.force_locations(game).len() > 0 {
            let fls: Vec<(Location, Battalion)> = self
                .force_locations(game)
                .iter()
                .filter(|(_, v)| v.amount_of_special_forces > 0)
                .map(|(k, v)| (*k, v.clone()))
                .collect();
            for fl in fls {
                if let Some(hw) = fl.0.as_homeworld() {
                    Player::ship_special_forces_from_homeworld(game, initiator, to, hw, fl.1.amount_of_special_forces);
                }
            }
        } else {
            Player::ship_special_forces(game, initiator, to, special_force_amount);
        }

        if game.player(initiator).is(Faction::Fremen) && game.is_in_storm(to) {
            let kill_count: i32;
            if !game.prevented(FactionAdvantage::FremenProtectedFromStorm) && game.applicable(Rule::FremenStormLosses) {
                kill_count = 0;
                game.storm.storm_losses_to_take.push(LossToTake {
                    location: Some(to),
                    amount: TakeLosses::half_of(force_amount, special_force_amount),
                    faction: Some(Faction::Fremen),
                });
            } else {
                kill_count = Player::kill_forces(game, initiator, to, force_amount, special_force_amount, false);
            }

            if kill_count > 0 {
                let f = game.player(initiator).faction;
                let m = express!(kill_count, f, " forces are killed by the storm").with_log(ShipMoveLoss {
                    faction: f.expect("faction"),
                    storm: true,
                    travelling: false,
                    forces: kill_count,
                    territory: None,
                });
                self.log_objects(game, elems![m]);
            }
        }

        let f = game.player(initiator).faction;
        if f != Some(Faction::Fremen) && f != Some(Faction::Guild) && !self.is_site_to_site() {
            game.ship_move.ships_tech_token_income = true;
        }
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        if self.passed {
            return express!(self.initiator(), " pass shipment");
        }

        if self.is_back_to_reserves() {
            express!(self.initiator(), " ship from ", self.to, " to reserves").with_route(self.initiator().expect("faction"), self.sources(), None)
        } else {
            express!(self.initiator(), " ship to ", self.to).with_route(self.initiator().expect("faction"), self.sources(), self.to)
        }
    }

    pub fn sources(&self) -> Vec<Location> {
        if self.is_back_to_reserves() {
            return match self.to {
                Some(to) => vec![to],
                None => vec![],
            };
        }
        if self.is_site_to_site() {
            if let Some(from) = self.from {
                return vec![from];
            }
        }
        vec![]
    }

    pub fn get_verbose_message(&self, game: &Game, cost: i32, guild_income: &MessagePart, owner_of_karma: Option<PlayerId>) -> Message {
        if self.passed {
            return express!(self.initiator(), " pass shipment");
        }

        if self.is_back_to_reserves() {
            return express!(
                self.initiator(),
                " ship ",
                express_part_if!(self.force_amount > 0; self.force_amount, game.player(self.player()).force(), " "),
                "from ",
                self.to,
                " back to reserves",
                self.cost_message(game, cost),
                self.karama_message(game, owner_of_karma),
                guild_income.clone()
            )
            .with_route(self.initiator().expect("faction"), self.sources(), None);
        }

        if self.is_site_to_site() {
            return express!(
                self.initiator(),
                " site-to-site ship ",
                self.force_message(game),
                " from ",
                self.from,
                " to ",
                self.to,
                self.cost_message(game, cost),
                self.karama_message(game, owner_of_karma),
                guild_income.clone()
            )
            .with_route(self.initiator().expect("faction"), self.sources(), self.to);
        }

        if cost > 0 {
            return express!(
                self.initiator(),
                " ship ",
                self.force_message(game),
                " to ",
                self.to,
                self.no_field_message(),
                self.cost_message(game, cost),
                self.karama_message(game, owner_of_karma),
                guild_income.clone()
            )
            .with_route(self.initiator().expect("faction"), self.sources(), self.to);
        }

        express!(self.initiator(), " rally ", self.force_message(game), " in ", self.to).with_route(self.initiator().expect("faction"), self.sources(), self.to)
    }

    pub fn log_record(&self, game: &Game, cost: i32, guild_income: i32) -> ShipLogged {
        let kind = if self.passed {
            ShipKind::Passed
        } else if self.is_back_to_reserves() {
            ShipKind::BackToReserves
        } else if self.is_site_to_site() {
            ShipKind::SiteToSite
        } else {
            ShipKind::Normal
        };
        let blocked = self.passed && game.prevented_from_shipping(self.initiator());
        let blocked_by = if blocked {
            game.decks.treachery_discard_pile
                .as_ref()
                .and_then(|d| d.items.iter().copied().find(|c| c.type_() == TreacheryCardType::Karma))
        } else {
            None
        };
        let plain = !self.passed && (self.is_no_field() || self.guild_income_prevented.get());

        ShipLogged {
            faction: self.initiator().expect("faction"),
            kind,
            forces: if self.passed { 0 } else { self.force_amount },
            special_forces: if self.passed || self.is_back_to_reserves() { 0 } else { self.special_force_amount },
            from: if self.is_site_to_site() || self.is_back_to_reserves() { self.sources().into_iter().next() } else { None },
            to: if self.is_back_to_reserves() || self.passed { None } else { self.to },
            cost,
            ally_share: self.ally_contribution_amount,
            ally: game.player(self.player()).ally,
            guild_paid: guild_income > 0,
            karama: self.karma_card,
            blocked,
            blocked_by,
            plain,
        }
    }

    pub fn no_field_message(&self) -> MessagePart {
        express_part_if!(self.is_no_field() && self.initiator() != Some(Faction::Richese); " (using a ", FactionSpecialForce::Richese, ")")
    }

    pub fn force_message(&self, game: &Game) -> MessagePart {
        let pl = game.player(self.player());
        express_part!(
            express_part_if!(self.force_amount > 0; self.force_amount, pl.force()),
            express_part_if!(self.special_force_amount > 0; self.special_force_amount, pl.special_force())
        )
    }

    pub fn cost_message(&self, game: &Game, cost: i32) -> MessagePart {
        express_part!(
            " for ",
            Payment::of(cost),
            express_part_if!(self.ally_contribution_amount > 0; " (", Payment::of_by(self.ally_contribution_amount, game.player(self.player()).ally), ")")
        )
    }

    pub fn karama_message(&self, _game: &Game, owner_of_karma: Option<PlayerId>) -> MessagePart {
        let karma_card = self.karma_card;
        express_part_if!(
            karma_card.is_some();
            " using ",
            express_part_if!(owner_of_karma != self.player_opt(); "their allies "),
            TreacheryCardType::Karma,
            express_part_if!(karma_card.is_some_and(|c| c.type_() != TreacheryCardType::Karma); " (", karma_card, ")")
        )
    }

}

impl GameEventMethods for Shipment {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::Shipment
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::Shipment(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        Shipment::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        Shipment::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        Shipment::get_message(self, game)
    }

}

impl PassableGameEventMethods for Shipment {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}

impl crate::events::location_event::LocationEvent for Shipment {
    fn initiator(&self) -> Faction {
        GameEventMethods::initiator(self).expect("faction")
    }

    fn to(&self, _game: &Game) -> Option<Location> {
        self.to
    }

    fn total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        Shipment::total_amount_of_forces_added_to_location(self, game)
    }

    fn forces_added_to_location(&self, game: &Game) -> i32 {
        Shipment::forces_added_to_location(self, game)
    }

    fn special_forces_added_to_location(&self, game: &Game) -> i32 {
        Shipment::special_forces_added_to_location(self, game)
    }

}
