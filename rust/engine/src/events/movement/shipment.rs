use crate::prelude::*;
use crate::compat::lazy::LazyQuery;

#[derive(Clone, Debug, Canon)]
pub struct Shipment {
    pub shipment_type: ShipmentType,
    #[canon(name = "_toId")]
    pub to_id: i32,
    #[canon(name = "_fromId")]
    pub from_id: i32,
    pub force_amount: i32,
    pub special_force_amount: i32,
    pub smuggled_amount: i32,
    pub smuggled_special_amount: i32,
    #[canon(name = "_noFieldValue")]
    pub no_field_value: i32,
    #[canon(name = "_cunningNoFieldValue")]
    pub cunning_no_field_value: i32,
    pub ally_contribution_amount: i32,
    #[canon(name = "_karmaCardId")]
    pub karma_card_id: i32,
    pub use_richese_secret_ally: bool,
    #[canon(name = "_forceLocations")]
    pub force_locations: Option<String>,
    /// Interior mutability: written after the event is stored in game state.
    #[canon(name = "_guildIncomePrevented")]
    pub guild_income_prevented: Cell<bool>,
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for Shipment {
    fn default() -> Self {
        Shipment {
            shipment_type: Default::default(),
            to_id: Default::default(),
            from_id: Default::default(),
            force_amount: Default::default(),
            special_force_amount: Default::default(),
            smuggled_amount: Default::default(),
            smuggled_special_amount: Default::default(),
            no_field_value: Default::default(),
            cunning_no_field_value: Default::default(),
            ally_contribution_amount: Default::default(),
            karma_card_id: Default::default(),
            use_richese_secret_ally: Default::default(),
            force_locations: Some("".to_string()),
            guild_income_prevented: Cell::new(Default::default()),
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

/// The elements of a C# `params object?[]` argument list.
macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(ExpressionElement::from($e)),*] };
}

impl Shipment {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn to(&self, game: &Game) -> Option<Location> {
        game.map.location_lookup().find(self.to_id)
    }

    pub fn set_to(&mut self, game: &Game, value: Option<Location>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.to_id = game.map.location_lookup().get_id(value);
    }

    pub fn source_location_for_shipment_to_reserves(&self, game: &Game) -> Option<Location> {
        self.to(game)
    }

    pub fn from(&self, game: &Game) -> Option<Location> {
        game.map.location_lookup().find(self.from_id)
    }

    pub fn set_from(&mut self, game: &Game, value: Option<Location>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.from_id = game.map.location_lookup().get_id(value);
    }

    pub fn target_world_for_shipment_to_reserves(&self, game: &Game) -> Option<Homeworld> {
        self.from(game).and_then(|l| l.as_homeworld())
    }

    pub fn no_field_value(&self) -> i32 {
        self.no_field_value.wrapping_sub(1)
    }

    pub fn set_no_field_value(&mut self, value: i32) {
        self.no_field_value = value.wrapping_add(1);
    }

    pub fn cunning_no_field_value(&self) -> i32 {
        self.cunning_no_field_value.wrapping_sub(1)
    }

    pub fn set_cunning_no_field_value(&mut self, value: i32) {
        self.cunning_no_field_value = value.wrapping_add(1);
    }

    pub fn karma_card(&self) -> Option<TreacheryCard> {
        TreacheryCardManager::lookup().find(self.karma_card_id)
    }

    pub fn set_karma_card(&mut self, value: Option<TreacheryCard>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.karma_card_id = TreacheryCardManager::get_id(value);
    }

    pub fn is_using_karma(&self) -> bool {
        self.karma_card().is_some()
    }

    pub fn is_back_to_reserves(&self) -> bool {
        // ForceAmount + SpecialForceAmount is an unchecked int addition (hazards 1.3).
        self.shipment_type == ShipmentType::ShipmentBack
            || (self.shipment_type == ShipmentType::Unknown && self.force_amount.wrapping_add(self.special_force_amount) < 0)
    }

    pub fn is_site_to_site(&self, game: &Game) -> bool {
        self.shipment_type == ShipmentType::ShipmentSiteToSite
            || (self.shipment_type == ShipmentType::Unknown && self.from(game).is_some())
    }

    pub fn is_no_field(&self) -> bool {
        self.no_field_value() >= 0
    }

    pub fn total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        self.forces_added_to_location(game).wrapping_add(self.special_forces_added_to_location(game))
    }

    pub fn forces_added_to_location(&self, _game: &Game) -> i32 {
        if self.use_richese_secret_ally { 1 } else { self.force_amount.wrapping_add(self.smuggled_amount) }
    }

    pub fn special_forces_added_to_location(&self, _game: &Game) -> i32 {
        if self.use_richese_secret_ally { 0 } else { self.special_force_amount.wrapping_add(self.smuggled_special_amount) }
    }

    pub fn force_locations(&self, game: &Game) -> NetDictionary<Location, Battalion> {
        // Player.Faction: NullReferenceException when Player is null, before the string is parsed.
        let f = game.player(self.player()).faction;
        Shipment::parse_force_locations(game, f, self.force_locations.as_deref())
    }

    pub fn set_force_locations(&mut self, game: &Game, value: &NetDictionary<Location, Battalion>) {
        self.force_locations = Some(Shipment::force_locations_string(game, value));
    }

    pub fn determine_guild_profits(&self, game: &Game) -> i32 {
        let initiator = self.player();
        let cost = self.determine_cost_to_initiator(game);
        let extra = if game.player(initiator).ally != Faction::Guild || game.applicable(Rule::GuildShipmentContributionsFlowBack) {
            self.ally_contribution_amount
        } else {
            0
        };
        cost.wrapping_add(extra)
    }

    pub fn determine_cost_to_initiator(&self, g: &Game) -> i32 {
        let Some(_to) = self.to(g) else { return 0 };

        Shipment::determine_cost(g, self.player(), self).wrapping_sub(self.ally_contribution_amount)
    }

    pub fn force_locations_string(g: &Game, force_locations: &NetDictionary<Location, Battalion>) -> String {
        force_locations
            .iter()
            .map(|(k, v)| format!("{}:{}|{}", g.map.location_lookup().get_id(Some(*k)), v.amount_of_forces, v.amount_of_special_forces))
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn parse_force_locations(g: &Game, f: Faction, force_locations: Option<&str>) -> NetDictionary<Location, Battalion> {
        let mut result = NetDictionary::new();
        if let Some(s) = force_locations {
            if !s.is_empty() {
                for location_amount_pair in s.split(',') {
                    let location_and_amounts: Vec<&str> = location_amount_pair.split(':').collect();
                    let location = g
                        .map
                        .location_lookup()
                        .find(dotnet_compat::parse::convert_to_int32(Some(*cs::index_array(&location_and_amounts, 0))));

                    let Some(location) = location else { continue };

                    let amounts: Vec<&str> = cs::index_array(&location_and_amounts, 1).split('|').collect();
                    let amount_of_normal_forces = dotnet_compat::parse::convert_to_int32(Some(*cs::index_array(&amounts, 0)));
                    let amount_of_normal_special_forces = dotnet_compat::parse::convert_to_int32(Some(*cs::index_array(&amounts, 1)));
                    result.add(location, Battalion::new(f, amount_of_normal_forces, amount_of_normal_special_forces, location));
                }
            }
        }

        result
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if self.passed {
            return None;
        }

        // `var p = Player;` may be null (hazards 1.8): it throws only where C# dereferences it.
        let p_opt = self.player_opt();
        let p = || p_opt.nre();

        if !self.is_site_to_site(game) && game.prevented_from_shipping(self.initiator()) {
            return Some(express!(TreacheryCardType::Karma, " prevents you from shipping"));
        }

        if self.is_back_to_reserves() && !Shipment::may_ship_to_reserves(game, p()) {
            return Some(express!("You can't ship back to reserves"));
        }
        if self.is_back_to_reserves() {
            let source = self.source_location_for_shipment_to_reserves(game);
            let valid = Shipment::valid_source_locations(game, p());
            if !valid.cs_contains(&source.or_null()) {
                return Some(express!("You can't ship back to reserves from there"));
            }
        }
        if self.is_back_to_reserves() {
            if let Some(target) = self.target_world_for_shipment_to_reserves(game) {
                // List<Homeworld>.Contains: Location.Equals, by Id.
                if !game.player(p()).home_worlds.iter().any(|hw| hw.location() == target.location()) {
                    return Some(express!("You can't ship back to that home world"));
                }
            }
        }

        if self.is_site_to_site(game) && !Shipment::may_ship_cross_planet(game, p()) {
            return Some(express!("You can't ship site-to-site"));
        }
        if self.is_site_to_site(game) {
            let from = self.from(game);
            let valid = Shipment::valid_source_locations(game, p());
            if !valid.cs_contains(&from.or_null()) {
                return Some(express!("You can't ship site-to-site from there"));
            }
        }

        if !self.is_back_to_reserves() && (self.force_amount < 0 || self.special_force_amount < 0) {
            return Some(express!("Can't ship less than zero forces"));
        }
        if self.force_amount == 0 && self.special_force_amount == 0 {
            return Some(express!("Select forces to ship"));
        }
        let Some(to) = self.to(game) else {
            return Some(express!("Target location not selected"));
        };

        let cost = Shipment::determine_cost(game, p(), self);
        if cost > game.player(p()).resources.wrapping_add(self.ally_contribution_amount) {
            return Some(express!("You can't pay that much"));
        }
        if cost < self.ally_contribution_amount {
            return Some(express!("Your ally is paying more than needed"));
        }
        let cross_planet = self.is_back_to_reserves() || (self.is_site_to_site(game) && !self.from(game).is_some_and(|f| f.as_homeworld().is_some()));
        if !Shipment::valid_shipment_locations(game, p(), cross_planet, self.use_richese_secret_ally).cs_contains(&to) {
            return Some(express!("Cannot ship there"));
        }

        //no field checks, requires refactoring
        if self.no_field_value() >= 0 && !Shipment::may_use_no_field(game, p()) {
            return Some(express!("You can't use a No-Field"));
        }

        let is_richese_no_field_shipment = game.player(p()).faction == Faction::Richese && self.no_field_value() >= 0;
        if is_richese_no_field_shipment && self.force_amount > 0 && !game.player(p()).has_high_threshold_0(game) {
            return Some(express!("You can't do both normal and No-Field shipment"));
        }
        if is_richese_no_field_shipment && self.special_force_amount != 1 {
            return Some(express!("Invalid special force value for No-Field shipment"));
        }

        if game.player(p()).faction == Faction::Richese
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
        if self.cunning_no_field_value() >= 0 && self.force_amount > 0 && !game.player(p()).has_high_threshold_0(game) {
            return Some(express!("You can't do both normal and No-Field shipment"));
        }
        if self.cunning_no_field_value() >= 0
            && !Shipment::valid_cunning_no_field_values(game, p(), self.no_field_value()).contains(&self.cunning_no_field_value())
        {
            return Some(express!("Invalid Cunning No-Field value"));
        }

        // C# parity: the sums wrap (hazards 1.3), so int.MaxValue + 1 passes the check.
        if !self.is_site_to_site(game) && self.force_amount.wrapping_add(self.smuggled_amount) > game.player(p()).forces_in_reserve() {
            return Some(express!("Not enough ", game.player(p()).force(), " in reserve"));
        }
        if !self.is_site_to_site(game)
            && !is_richese_no_field_shipment
            && self.special_force_amount.wrapping_add(self.smuggled_special_amount) > game.player(p()).special_forces_in_reserve()
        {
            return Some(express!("Not enough ", game.player(p()).special_force(), " in reserve"));
        }

        let is_smuggling = self.smuggled_amount > 0 || self.smuggled_special_amount > 0;
        let is_shipping_from_off_planet = !(self.is_back_to_reserves() || self.is_site_to_site(game))
            && self.initiator() != Faction::Fremen
            && (self.force_amount > 0 || self.special_force_amount > 0);
        if is_smuggling && (!is_shipping_from_off_planet || !Shipment::may_smuggle(game, p(), Some(to))) {
            return Some(express!("You can't smuggle forces here"));
        }
        if self.smuggled_amount.wrapping_add(self.smuggled_special_amount) > 1 {
            return Some(express!("You can't smuggle more than 1 force"));
        }

        if self.is_site_to_site(game) && self.force_amount > game.player(p()).forces_in(self.from(game)) {
            return Some(express!("Not enough ", game.player(p()).force(), " for site-to-site shipment"));
        }
        if self.is_site_to_site(game) && self.special_force_amount > game.player(p()).special_forces_in(self.from(game)) {
            return Some(express!("Not enough ", game.player(p()).special_force(), " for site-to-site shipment"));
        }

        if self.is_no_field() && game.player(p()).faction != Faction::Richese {
            let pl = game.player(p());
            let forces_to_ship = std::cmp::min(self.no_field_value(), pl.forces_in_reserve().wrapping_add(pl.special_forces_in_reserve()));
            if self.force_amount.wrapping_add(self.special_force_amount) != forces_to_ship {
                return Some(express!("Using a No-Field of ", self.no_field_value(), ", you must select ", forces_to_ship, " forces to ship"));
            }
        }

        if let Some(karma_card) = self.karma_card() {
            if !Shipment::valid_karma_cards(game, p()).contains(&karma_card) {
                return Some(express!("Invalid ", TreacheryCardType::Karma, " card: ", self.karma_card()));
            }
        }

        if self.use_richese_secret_ally && !Shipment::may_use_richese_secret_ally(game, p()) {
            return Some(express!(Faction::Richese, " are not your secret ally"));
        }
        if self.use_richese_secret_ally && self.force_amount.wrapping_add(self.special_force_amount) > 5 {
            return Some(express!("You can't ship that much using ", Faction::Richese, " as your secret ally"));
        }

        // Sum is checked: OverflowException (hazards 1.3).
        let number_of_forces_selected = dotnet_compat::linq::sum_i32(self.force_locations(game).values().map(|b| b.amount_of_forces));
        if self.forces_added_to_location(game) > 0
            && Shipment::homeworlds_to_ship_from(game, p(), false).len() > 1
            && number_of_forces_selected <= 0
        {
            return Some(express!("Please specify where to ship your ", game.player(p()).force(), " from"));
        }
        if number_of_forces_selected > 0 && number_of_forces_selected != self.forces_added_to_location(game) {
            return Some(express!("Select the source of in total ", self.forces_added_to_location(game), game.player(p()).force()));
        }

        let number_of_special_forces_selected =
            dotnet_compat::linq::sum_i32(self.force_locations(game).values().map(|b| b.amount_of_special_forces));
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

    pub fn homeworlds_to_ship_from(game: &Game, p: PlayerIndex, special: bool) -> Vec<Homeworld> {
        let pl = game.player(p);
        if special {
            pl.home_worlds.iter().copied().filter(|hw| pl.special_forces_in(Some(hw.location())) > 0).collect()
        } else {
            pl.home_worlds.iter().copied().filter(|hw| pl.forces_in(Some(hw.location())) > 0).collect()
        }
    }

    pub fn fremen_spawn_locations(g: &Game, p: PlayerIndex) -> Vec<Location> {
        let m = &g.map;
        g.map
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

    pub fn determine_cost(g: &Game, p: PlayerIndex, s: &Shipment) -> i32 {
        match s.to(g) {
            None => 0,
            Some(to) => {
                let forces = dotnet_compat::math::abs_i32(s.force_amount);
                let special_forces = dotnet_compat::math::abs_i32(s.special_force_amount);
                Shipment::determine_cost_game_player_int32_int32_location_boolean_boolean_boolean_boolean_boolean(
                    g,
                    p,
                    forces,
                    special_forces,
                    to,
                    s.is_using_karma(),
                    s.is_back_to_reserves(),
                    s.is_site_to_site(g),
                    s.is_richese_no_field(),
                    s.is_non_richese_no_field(),
                )
            }
        }
    }

    pub fn is_richese_no_field(&self) -> bool {
        self.initiator() == Faction::Richese && self.is_no_field()
    }

    pub fn is_non_richese_no_field(&self) -> bool {
        (self.initiator() != Faction::Richese && self.is_no_field()) || self.use_richese_secret_ally
    }

    pub fn determine_cost_game_player_int32_int32_location_boolean_boolean_boolean_boolean_boolean(g: &Game, p: PlayerIndex, forces: i32, special_forces: i32, to: Location, karama_shipment: bool, back_to_reserves: bool, site_to_site: bool, no_field_by_richese: bool, no_field_by_non_richese: bool) -> i32 {
        let mut total_forces = forces.wrapping_add(special_forces);
        let no_field_or_secret_ally = no_field_by_richese || no_field_by_non_richese;

        if no_field_by_non_richese {
            total_forces = std::cmp::min(forces.wrapping_add(special_forces), 1);
        } else if no_field_by_richese {
            total_forces = std::cmp::max(forces, 1);
        }

        // (int)Math.Ceiling(0.5f * totalForces): float product, widened to double for Math.Ceiling.
        if back_to_reserves {
            return dotnet_compat::math::f64_to_i32(((0.5f32 * total_forces as f32) as f64).ceil());
        }

        if !site_to_site
            && g.player(p).is(Faction::Fremen)
            && Shipment::fremen_spawn_locations(g, p).contains(&to)
            && !no_field_or_secret_ally
        {
            return 0;
        }

        // The int product can wrap before it is widened to double (hazards 15).
        let mut cost_of_shipment: f64 =
            dotnet_compat::math::abs_i32(total_forces).wrapping_mul(if to.territory().has_reduced_shipping_cost() { 1 } else { 2 }) as f64;

        if Shipment::may_ship_with_discount(g, p) || karama_shipment || (site_to_site && g.has_shipment_permission(p, ShipmentPermission::GuildRate)) {
            cost_of_shipment /= 2.0;
        }

        dotnet_compat::math::f64_to_i32(cost_of_shipment.ceil())
    }

    pub fn valid_max_normal_shipment_forces(game: &Game, p: PlayerIndex, special_forces: bool, used_no_field: i32) -> i32 {
        let pl = game.player(p);
        let no_field_max = if used_no_field < 0 || pl.faction == Faction::Richese { i32::MAX } else { used_no_field };
        if special_forces {
            std::cmp::min(pl.special_forces_in_reserve(), no_field_max)
        } else {
            std::cmp::min(pl.forces_in_reserve(), no_field_max)
        }
    }

    pub fn default_no_field_special_force_amount(faction: Faction, no_field_value: i32) -> i32 {
        if faction == Faction::Richese && no_field_value >= 0 { 1 } else { 0 }
    }

    pub fn valid_max_secret_ally_shipment_forces(game: &Game, p: PlayerIndex, special_forces: bool) -> i32 {
        let pl = game.player(p);
        if special_forces {
            std::cmp::min(pl.special_forces_in_reserve(), 5)
        } else {
            std::cmp::min(pl.forces_in_reserve(), 5)
        }
    }

    pub fn valid_max_shipment_back_forces(game: &Game, p: PlayerIndex, special_forces: bool, source: Option<Location>) -> i32 {
        let source = source.and_then(crate::compat::cs::CsNull::opt);
        if source.is_none() {
            return 0;
        }
        let pl = game.player(p);
        if special_forces { pl.special_forces_in(source) } else { pl.forces_in(source) }
    }

    pub fn valid_max_shipment_site_to_site_forces(game: &Game, p: PlayerIndex, special_forces: bool, source: Option<Location>) -> i32 {
        let source = source.and_then(crate::compat::cs::CsNull::opt);
        if source.is_none() {
            return 0;
        }

        let pl = game.player(p);
        if special_forces { pl.special_forces_in(source) } else { pl.forces_in(source) }
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_shipment_locations(g: &Game, p: PlayerIndex, cross_planet: bool, richese_secret_ally: bool) -> impl Iterator<Item = Location> {
        use crate::compat::lazy::Either;
        if !g.player(p).is(Faction::Fremen) || richese_secret_ally {
            return Either::A(Shipment::regular_shipment_locations(g, p, cross_planet));
        }

        if Shipment::may_ship_cross_planet(g, p) {
            // FremenSpawnLocations(g, p).Union(RegularShipmentLocations(...)).Distinct(): both calls run now,
            // the Where inside RegularShipmentLocations and the Union run as the caller pulls. The
            // Distinct after the Union passes everything through.
            let spawn = Shipment::fremen_spawn_locations(g, p);
            let regular = Shipment::regular_shipment_locations(g, p, cross_planet);
            Either::B(Either::A(crate::compat::lazy::distinct(crate::compat::lazy::union(spawn.into_iter(), regular))))
        } else {
            Either::B(Either::B(Shipment::fremen_spawn_locations(g, p).into_iter()))
        }
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn regular_shipment_locations(g: &Game, p: PlayerIndex, cross_planet: bool) -> impl Iterator<Item = Location> {
        g.map
            .locations(g.applicable(Rule::Homeworlds))
            .filter(move |l| {
                let l = *l;
                Some(l.territory()) != g.atomics_aftermath
                    && l.sector(&g.map) != g.sector_in_storm
                    && (l.id() != g.map.hidden_mobile_stronghold().location().id() || g.player(p).is(Faction::Ixian))
                    && Shipment::is_either_valid_homeworld_or_no_homeworld(g, p, l, cross_planet)
                    && Shipment::is_either_valid_discovery_or_no_discovery(&g.map, l)
                    && g.is_not_full(p, l)
            })
    }

    pub fn is_either_valid_homeworld_or_no_homeworld(g: &Game, p: PlayerIndex, l: Location, from_planet: bool) -> bool {
        let Some(hw) = l.as_homeworld() else { return true };
        let pl = g.player(p);

        g.applicable(Rule::Homeworlds)
            && (!from_planet
                || pl.is(Faction::Guild)
                || !pl.is_native_homeworld(hw)
                || (pl.is_native_homeworld(hw) && Shipment::may_ship_to_reserves(g, p))
                || (pl.is_native_homeworld(hw) && g.has_shipment_permission(p, ShipmentPermission::ToHomeworld))) // from planet to OWN homeworld
            && (!from_planet
                || pl.is(Faction::Guild)
                || pl.is_native_homeworld(hw)
                || (!pl.is_native_homeworld(hw) && g.has_shipment_permission(p, ShipmentPermission::Cross))) // from planet to OTHER homeworld
            && g.players.iter().any(|native| native.is_native_homeworld(hw))
            && (!pl.has_ally() || {
                // p.AlliedPlayer may be null when the ally is not in the game: NullReferenceException.
                let ally = g.player(pl.allied_player(g).nre());
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

    pub fn valid_no_field_values(g: &Game, p: PlayerIndex) -> Vec<i32> {
        let pl = g.player(p);
        if (pl.faction != Faction::Richese || g.prevented(FactionAdvantage::RicheseNofield)) && (pl.ally != Faction::Richese || !g.richese_allows_use_of_no_field) {
            return vec![];
        }

        let mut result = Vec::new();
        if pl.faction == Faction::Richese && g.latest_revealed_no_field_value != 0 && g.current_no_field_value != 0 {
            result.push(0);
        }
        if g.latest_revealed_no_field_value != 3 && g.current_no_field_value != 3 {
            result.push(3);
        }
        if g.latest_revealed_no_field_value != 5 && g.current_no_field_value != 5 {
            result.push(5);
        }
        result
    }

    pub fn valid_cunning_no_field_values(g: &Game, p: PlayerIndex, value_of_normal_no_field: i32) -> Vec<i32> {
        let pl = g.player(p);
        if (pl.faction != Faction::Richese || g.prevented(FactionAdvantage::RicheseNofield)) && (pl.ally != Faction::Richese || !g.richese_allows_use_of_no_field) {
            return vec![];
        }

        let mut result = Vec::new();
        if pl.faction == Faction::Richese && g.latest_revealed_no_field_value != 0 && g.current_no_field_value != 0 && value_of_normal_no_field != 0 {
            result.push(0);
        }
        if g.latest_revealed_no_field_value != 3 && g.current_no_field_value != 3 && value_of_normal_no_field != 3 {
            result.push(3);
        }
        if g.latest_revealed_no_field_value != 5 && g.current_no_field_value != 5 && value_of_normal_no_field != 5 {
            result.push(5);
        }
        result
    }

    pub fn valid_karma_cards(g: &Game, p: PlayerIndex) -> Vec<TreacheryCard> {
        let mut result = Shipment::valid_own_karma_cards(g, p);

        let ally_card = Shipment::valid_ally_karma_card(g, p);
        if let Some(c) = ally_card {
            result.push(c);
        }

        result
    }

    pub fn valid_own_karma_cards(g: &Game, p: PlayerIndex) -> Vec<TreacheryCard> {
        Karma::valid_karma_cards(g, p).collect()
    }

    pub fn valid_ally_karma_card(g: &Game, p: PlayerIndex) -> Option<TreacheryCard> {
        g.get_permitted_use_of_ally_karma(g.player(p).faction)
    }

    pub fn can_karma(g: &Game, p: PlayerIndex) -> bool {
        !Shipment::valid_karma_cards(g, p).is_empty()
    }

    pub fn may_smuggle(g: &Game, p: PlayerIndex, l: Option<Location>) -> bool {
        let l = l.and_then(crate::compat::cs::CsNull::opt);
        match l {
            None => false,
            Some(l) => {
                !g.any_forces_in(l.territory())
                    && g.player(p).faction != Faction::Fremen
                    && g.skilled_as_player_leader_skill(p, LeaderSkill::Smuggler)
            }
        }
    }

    pub fn may_use_no_field(g: &Game, p: PlayerIndex) -> bool {
        let pl = g.player(p);
        (pl.faction == Faction::Richese && !g.prevented(FactionAdvantage::RicheseNofield)) || (pl.ally == Faction::Richese && g.richese_allows_use_of_no_field)
    }

    pub fn may_use_cunning_no_field(game: &Game, p: PlayerIndex) -> bool {
        game.player(p).faction == Faction::Richese && NexusPlayed::can_use_cunning(game, p)
    }

    pub fn may_use_richese_secret_ally(g: &Game, p: PlayerIndex) -> bool {
        g.player(p).nexus == Faction::Richese && NexusPlayed::can_use_secret_ally(g, p)
    }

    /// `p.Initiated(g.CurrentGuildSecretAlly)`
    fn initiated_current_guild_secret_ally(g: &Game, p: PlayerIndex) -> bool {
        let e = g.current_guild_secret_ally.clone().map(|e| e.into_event());
        g.player(p).initiated(e.as_ref())
    }

    pub fn may_ship_cross_planet(g: &Game, p: PlayerIndex) -> bool {
        let pl = g.player(p);
        (pl.is(Faction::Guild) && !g.prevented(FactionAdvantage::GuildSpecialShipments))
            || (pl.ally == Faction::Guild && g.guild_allows_shipping_discount)
            || Shipment::initiated_current_guild_secret_ally(g, p)
            || g.has_shipment_permission(p, ShipmentPermission::Cross)
    }

    pub fn may_ship_to_reserves(g: &Game, p: PlayerIndex) -> bool {
        let pl = g.player(p);
        (pl.is(Faction::Guild) && !g.prevented(FactionAdvantage::GuildSpecialShipments))
            || Shipment::initiated_current_guild_secret_ally(g, p)
            || g.has_shipment_permission(p, ShipmentPermission::ToHomeworld)
    }

    pub fn may_ship_with_discount(g: &Game, p: PlayerIndex) -> bool {
        let pl = g.player(p);
        (pl.is(Faction::Guild) && !g.prevented(FactionAdvantage::GuildShipmentsDiscount))
            || (pl.ally == Faction::Guild && g.guild_allows_shipping_discount && !g.prevented(FactionAdvantage::GuildShipmentsDiscountAlly))
            || Shipment::initiated_current_guild_secret_ally(g, p)
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_source_locations(g: &Game, p: PlayerIndex) -> impl Iterator<Item = Location> {
        g.locations_with_any_forces_not_in_storm(p)
            .into_iter()
            .filter(move |x| !x.is_homeworld() || Shipment::may_ship_cross_planet(g, p) || g.has_shipment_permission(p, ShipmentPermission::Cross))
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        game.beginning_of_shipment_and_move_phase = false;
        game.current_blocked_territories.clear();
        game.storm_losses_to_take.clear();
        game.chosen_destinations_with_allies.clear();
        game.bene_gesserit_may_accompany = false;

        let mut received_payment_message = express_part!();
        let mut total_cost = 0;

        if !self.passed {
            let owner_of_karma = match self.karma_card() {
                Some(c) => game.owner_of_treachery_card(c),
                None => None,
            };
            total_cost = self.pay_for_shipment(game);

            if self.is_no_field() && self.to(game).is_some() {
                let richese = self.get_player(game, Some(Faction::Richese)).nre();

                if self.cunning_no_field_value() >= 0 {
                    game.reveal_current_no_field(richese, None);
                    Player::ship_special_forces(game, richese, self.to(game).nre(), 1);
                    game.current_no_field_value = self.cunning_no_field_value();
                    let me = self.player();
                    game.play_nexus_card(me, elems!["Cunning", "ship and reveal a second ", FactionSpecialForce::Richese]);
                }

                game.reveal_current_no_field(richese, None);
                game.current_no_field_value = self.no_field_value();

                if self.initiator() != Faction::Richese {
                    //Immediately reveal No-Field
                    game.latest_revealed_no_field_value = self.no_field_value();
                    game.current_no_field_value = -1;
                }
            }

            if let Some(to) = self.to(game) {
                if game.contains_conflicting_ally(self.player(), to) {
                    game.chosen_destinations_with_allies.push(to.territory());
                }
            }

            game.last_shipment_or_movement = Some(self.clone().into_event());
            let must_be_advisors = {
                let pl = game.player(self.player());
                let to = self.to(game);
                (pl.is(Faction::BeneGesserit) && pl.special_forces_in(to) > 0)
                    || (to.is_some() && pl.special_forces_in_territory(to.map(|t| t.territory())) > 0)
            };

            if self.is_site_to_site(game) {
                self.perform_site_to_site_shipment(game);
            } else if self.is_back_to_reserves() {
                if let Some(source) = self.source_location_for_shipment_to_reserves(game) {
                    let me = self.player();
                    let target = match self.target_world_for_shipment_to_reserves(game) {
                        Some(hw) => hw,
                        None => *cs::index(&game.player(me).home_worlds, 0),
                    };
                    // C# parity: only ForceAmount is moved, although special forces are charged for (hazards 18).
                    Player::ship_forces_location_homeworld_int32(game, me, source, target, self.force_amount);
                }
            } else if self.is_no_field() && self.initiator() == Faction::Richese && self.to(game).is_some() {
                let to = self.to(game).nre();
                self.perform_normal_shipment(game, self.player(), to, self.force_amount.wrapping_add(self.smuggled_amount), 1);
            } else if let Some(to) = self.to(game) {
                self.perform_normal_shipment(
                    game,
                    self.player(),
                    to,
                    self.force_amount.wrapping_add(self.smuggled_amount),
                    self.special_force_amount.wrapping_add(self.smuggled_special_amount),
                );
            }

            if self.use_richese_secret_ally {
                game.play_nexus_card(self.player(), elems!["ship from reserves as if shipping ", 1]);
            }

            if !self.by(Faction::Fremen) {
                game.stone(Milestone::Shipment);
            }

            if let Some(to) = self.to(game) {
                Player::flip_forces(game, self.player(), to, must_be_advisors);
            }

            if !self.is_back_to_reserves() {
                game.check_intrusion(&self.clone().into_event());
            }

            game.determine_next_shipment_and_move_sub_phase();

            let received_payment = self.handle_received_shipment_payment(game, &mut received_payment_message);
            let m = self
                .get_verbose_message(game, total_cost, &received_payment_message, owner_of_karma)
                .with_log(self.log_record(game, total_cost, received_payment));
            self.log_objects(game, elems![m]);

            if total_cost.wrapping_sub(received_payment) >= 4 {
                game.activate_banker(self.player());
            }

            game.flip_bene_gesserit_advisors_when_alone();
            let to = self.to(game);
            game.determine_occupation_location(to);
        } else {
            let m = self.get_verbose_message(game, total_cost, &received_payment_message, None).with_log(self.log_record(game, 0, 0));
            self.log_objects(game, elems![m]);
            game.determine_next_shipment_and_move_sub_phase();
        }
    }

    pub fn pay_for_shipment(&self, game: &mut Game) -> i32 {
        let cost_to_initiator = self.determine_cost_to_initiator(game);
        let me = self.player();
        let pl = game.player_mut(me);
        pl.resources = pl.resources.wrapping_sub(cost_to_initiator);

        if self.is_using_karma() {
            let karma_card = self.karma_card();
            game.discard_treachery_card_boolean(karma_card, true);
            game.stone(Milestone::Karma);
        }

        if self.ally_contribution_amount > 0 {
            let ally = game.player(self.player()).ally;
            if let Some(a) = game.get_player(Some(ally)) {
                let ap = game.player_mut(a);
                ap.resources = ap.resources.wrapping_sub(self.ally_contribution_amount);
            }
            game.decrease_permitted_use_of_ally_spice(self.initiator(), self.ally_contribution_amount);
        }

        cost_to_initiator.wrapping_add(self.ally_contribution_amount)
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
                        op.resources = op.resources.wrapping_add(receiver_profit);

                        if receiver_profit > 0 {
                            *profit_message = express_part!(" → ", Faction::Guild, " get ", Payment::of(receiver_profit));

                            if receiver_profit >= 5 {
                                game.apply_bureaucracy(self.initiator(), Faction::Guild);
                            }
                        }
                    } else {
                        // C# parity: no space before the payment (hazards 11).
                        *profit_message =
                            express_part!(" → ", TreacheryCardType::Karma, " prevents ", Faction::Guild, " from receiving", Payment::of(receiver_profit));
                        self.guild_income_prevented.set(true);
                        if !game.applicable(Rule::FullPhaseKarma) {
                            game.allow(FactionAdvantage::GuildReceiveShipment);
                        }
                    }
                }
            }
        }

        let ev = self.clone().into_event();
        game.set_recent_payment(receiver_profit, self.initiator(), Faction::Guild, &ev);
        game.set_recent_payment_int32_faction_game_event(total_cost.wrapping_sub(receiver_profit), self.initiator(), &ev);

        receiver_profit
    }

    pub fn perform_site_to_site_shipment(&self, game: &mut Game) {
        let (Some(from), Some(to)) = (self.from(game), self.to(game)) else { return };

        Player::move_forces(game, self.player(), from, to, self.force_amount);
        Player::move_special_forces(game, self.player(), from, to, self.special_force_amount);
    }

    pub fn perform_normal_shipment(&self, game: &mut Game, initiator: PlayerIndex, to: Location, force_amount: i32, special_force_amount: i32) {
        let initiator_faction = game.player(initiator).faction;
        game.bene_gesserit_may_accompany =
            (force_amount > 0 || special_force_amount > 0) && initiator_faction != Faction::Fremen && initiator_faction != Faction::BeneGesserit;

        // ForceLocations is parsed afresh on every read (hazards 12); each read gives the same result here.
        if Shipment::homeworlds_to_ship_from(game, self.player(), false).len() > 1 && self.force_locations(game).len() > 0 {
            let fls: Vec<(Location, Battalion)> = self
                .force_locations(game)
                .iter()
                .filter(|(_, v)| v.amount_of_forces > 0)
                .map(|(k, v)| (*k, v.clone()))
                .collect();
            for fl in fls {
                if let Some(hw) = fl.0.as_homeworld() {
                    Player::ship_forces_location_homeworld_int32(game, initiator, to, hw, fl.1.amount_of_forces);
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
                    Player::ship_special_forces_location_homeworld_int32(game, initiator, to, hw, fl.1.amount_of_special_forces);
                }
            }
        } else {
            Player::ship_special_forces(game, initiator, to, special_force_amount);
        }

        if game.player(initiator).is(Faction::Fremen) && game.is_in_storm(to) {
            let kill_count: i32;
            if !game.prevented(FactionAdvantage::FremenProtectedFromStorm) && game.applicable(Rule::FremenStormLosses) {
                kill_count = 0;
                game.storm_losses_to_take.push(LossToTake {
                    location: Some(to),
                    amount: TakeLosses::half_of(force_amount, special_force_amount),
                    faction: Faction::Fremen,
                });
            } else {
                kill_count = Player::kill_forces(game, initiator, to, force_amount, special_force_amount, false);
            }

            if kill_count > 0 {
                let f = game.player(initiator).faction;
                let m = express!(kill_count, f, " forces are killed by the storm").with_log(ShipMoveLoss {
                    faction: f,
                    storm: true,
                    travelling: false,
                    forces: kill_count,
                    territory: None,
                });
                self.log_objects(game, elems![m]);
            }
        }

        let f = game.player(initiator).faction;
        if f != Faction::Fremen && f != Faction::Guild && !self.is_site_to_site(game) {
            game.ships_tech_token_income = true;
        }
    }

    pub fn get_message(&self, game: &Game) -> Message {
        if self.passed {
            return express!(self.initiator(), " pass shipment");
        }

        if self.is_back_to_reserves() {
            express!(self.initiator(), " ship from ", self.to(game), " to reserves").with_route(self.initiator(), self.sources(game), None)
        } else {
            express!(self.initiator(), " ship to ", self.to(game)).with_route(self.initiator(), self.sources(game), self.to(game))
        }
    }

    pub fn sources(&self, game: &Game) -> Vec<Location> {
        if self.is_back_to_reserves() {
            return match self.to(game) {
                Some(to) => vec![to],
                None => vec![],
            };
        }
        if self.is_site_to_site(game) {
            if let Some(from) = self.from(game) {
                return vec![from];
            }
        }
        vec![]
    }

    pub fn get_verbose_message(&self, game: &Game, cost: i32, guild_income: &MessagePart, owner_of_karma: Option<PlayerIndex>) -> Message {
        let owner_of_karma = owner_of_karma.and_then(crate::compat::cs::CsNull::opt);
        if self.passed {
            return express!(self.initiator(), " pass shipment");
        }

        if self.is_back_to_reserves() {
            return express!(
                self.initiator(),
                " ship ",
                express_part_if!(self.force_amount > 0; self.force_amount, game.player(self.player()).force(), " "),
                "from ",
                self.to(game),
                " back to reserves",
                self.cost_message(game, cost),
                self.karama_message(game, owner_of_karma),
                guild_income.clone()
            )
            .with_route(self.initiator(), self.sources(game), None);
        }

        if self.is_site_to_site(game) {
            return express!(
                self.initiator(),
                " site-to-site ship ",
                self.force_message(game),
                " from ",
                self.from(game),
                " to ",
                self.to(game),
                self.cost_message(game, cost),
                self.karama_message(game, owner_of_karma),
                guild_income.clone()
            )
            .with_route(self.initiator(), self.sources(game), self.to(game));
        }

        if cost > 0 {
            return express!(
                self.initiator(),
                " ship ",
                self.force_message(game),
                " to ",
                self.to(game),
                self.no_field_message(),
                self.cost_message(game, cost),
                self.karama_message(game, owner_of_karma),
                guild_income.clone()
            )
            .with_route(self.initiator(), self.sources(game), self.to(game));
        }

        express!(self.initiator(), " rally ", self.force_message(game), " in ", self.to(game)).with_route(self.initiator(), self.sources(game), self.to(game))
    }

    pub fn log_record(&self, game: &Game, cost: i32, guild_income: i32) -> ShipLogged {
        let kind = if self.passed {
            ShipKind::Passed
        } else if self.is_back_to_reserves() {
            ShipKind::BackToReserves
        } else if self.is_site_to_site(game) {
            ShipKind::SiteToSite
        } else {
            ShipKind::Normal
        };
        let blocked = self.passed && game.prevented_from_shipping(self.initiator());
        let blocked_by = if blocked {
            game.treachery_discard_pile
                .as_ref()
                .and_then(|d| d.items.iter().copied().find(|c| c.type_() == TreacheryCardType::Karma))
        } else {
            None
        };
        let plain = !self.passed && (self.is_no_field() || self.guild_income_prevented.get());

        ShipLogged {
            faction: self.initiator(),
            kind,
            forces: if self.passed { 0 } else { self.force_amount },
            special_forces: if self.passed || self.is_back_to_reserves() { 0 } else { self.special_force_amount },
            from: if self.is_site_to_site(game) || self.is_back_to_reserves() { self.sources(game).into_iter().next() } else { None },
            to: if self.is_back_to_reserves() || self.passed { None } else { self.to(game) },
            cost,
            ally_share: self.ally_contribution_amount,
            ally: game.player(self.player()).ally,
            guild_paid: guild_income > 0,
            karama: self.karma_card(),
            blocked,
            blocked_by,
            plain,
        }
    }

    pub fn no_field_message(&self) -> MessagePart {
        express_part_if!(self.is_no_field() && self.initiator() != Faction::Richese; " (using a ", FactionSpecialForce::Richese, ")")
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

    pub fn karama_message(&self, _game: &Game, owner_of_karma: Option<PlayerIndex>) -> MessagePart {
        let owner_of_karma = owner_of_karma.and_then(crate::compat::cs::CsNull::opt);
        let karma_card = self.karma_card();
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

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
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

impl crate::events::location_event::ILocationEvent for Shipment {
    fn initiator(&self) -> Faction {
        GameEventMethods::initiator(self)
    }

    fn to(&self, game: &Game) -> Option<Location> {
        Shipment::to(self, game)
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
