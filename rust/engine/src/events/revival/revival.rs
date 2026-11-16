use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct Revival {
    pub amount_of_forces: u16,
    pub amount_of_special_forces: u16,
    pub extra_forces_paid_by_emperor: u16,
    pub extra_special_forces_paid_by_emperor: u16,
    pub number_of_forces_in_location: u16,
    pub number_of_special_forces_in_location: u16,
    pub location: Option<Location>,
    pub hero: Option<Hero>,
    pub assign_skill: bool,
    pub uses_emperor_secret_ally: bool,
    /// The fields of `Event` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for Revival {
    fn default() -> Self {
        Revival {
            amount_of_forces: Default::default(),
            amount_of_special_forces: Default::default(),
            extra_forces_paid_by_emperor: Default::default(),
            extra_special_forces_paid_by_emperor: Default::default(),
            number_of_forces_in_location: Default::default(),
            number_of_special_forces_in_location: Default::default(),
            location: None,
            hero: TreacheryCardManager::get(0).map(Hero::TreacheryCard),
            assign_skill: Default::default(),
            uses_emperor_secret_ally: Default::default(),
            base: Default::default(),
        }
    }
}

impl Revival {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn to(&self, _game: &Game) -> Option<Location> {
        self.location
    }

    pub fn total_amount_of_forces_added_to_location(&self, _game: &Game) -> i32 {
        i32::from(self.number_of_forces_in_location) + i32::from(self.number_of_special_forces_in_location)
    }

    pub fn forces_added_to_location(&self, _game: &Game) -> i32 {
        i32::from(self.number_of_forces_in_location)
    }

    pub fn special_forces_added_to_location(&self, _game: &Game) -> i32 {
        i32::from(self.number_of_special_forces_in_location)
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        // `var p = Player;` does not throw; the first dereference (line 84) does.
        let p_opt = self.player_opt();
        let initiator = self.initiator();
        let forces = i32::from(self.amount_of_forces) + i32::from(self.extra_forces_paid_by_emperor);
        let special_forces = i32::from(self.amount_of_special_forces) + i32::from(self.extra_special_forces_paid_by_emperor);

        if forces <= 0 && special_forces <= 0 && self.hero.is_none() {
            return Some(express!("Select forces or a leader to revive"));
        }

        let p = p_opt.expect("player");
        if let Some(hero) = self.hero {
            if Self::is_living_leader(game, hero) && !Self::is_ghola_returning_home(game, initiator.expect("faction"), hero) {
                return Some(express!("You can't revive a leader who isn't dead"));
            }
        }

        let pl = game.player(p);
        if forces > i32::from(pl.forces_killed) {
            return Some(express!("You can't revive that many ", pl.force()));
        }
        if special_forces > i32::from(pl.special_forces_killed) {
            return Some(express!("You can't revive that many ", pl.special_force()));
        }

        if initiator != Some(Faction::Ixian) && special_forces > 1 {
            return Some(express!("You can't revive more than one ", pl.special_force(), " per turn"));
        }
        if special_forces > 0
            && initiator != Some(Faction::Ixian)
            && game.revival.factions_that_revived_special_forces_this_turn.contains_opt(initiator)
        {
            return Some(express!("You already revived a ", pl.special_force(), " this turn"));
        }

        let limit = game.get_max_revivals(game, p, self.uses_emperor_secret_ally);
        if (i32::from(self.amount_of_forces) + i32::from(self.amount_of_special_forces)) > limit {
            return Some(express!("You can't revive more than ", limit));
        }

        let ally_limit = Self::valid_max_revivals_by_emperor(game, p);
        if (i32::from(self.extra_forces_paid_by_emperor) + i32::from(self.extra_special_forces_paid_by_emperor))
            > Self::valid_max_revivals_by_emperor(game, p)
        {
            return Some(express!("Your ally won't revive more than ", ally_limit));
        }

        if initiator != Some(Faction::Tleilaxu)
            && game.revival.leader_revivals_this_turn.get_opt(initiator).copied().unwrap_or(0) > 0
        {
            return Some(express!("You cannot revive more leaders this turn"));
        }

        let cost_of_revival = Self::determine_cost(
            game,
            p,
            self.hero,
            i32::from(self.amount_of_forces),
            i32::from(self.amount_of_special_forces),
            i32::from(self.extra_forces_paid_by_emperor),
            i32::from(self.extra_special_forces_paid_by_emperor),
            self.uses_emperor_secret_ally,
        );
        if cost_of_revival.total_cost_for_player() > i32::from(game.player(p).resources) {
            return Some(express!("You can't pay that many"));
        }

        if self.assign_skill && self.hero.is_none() {
            return Some(express!("You must revive a leader to assign a skill to"));
        }
        if self.assign_skill && !Self::may_assign_skill(game, p, self.hero) {
            return Some(express!("You can't assign a skill to this leader"));
        }

        if self.uses_emperor_secret_ally && !Self::may_use_emperor_secret_ally(game, p) {
            return Some(express!("you can't use ", Faction::Emperor, " cunning"));
        }

        if self.location.is_some() {
            let location = self.location;
            // Contains on the lazy Where: stops at the first match (see valid_revived_force_locations).
            if !Self::valid_revived_force_locations(game, p).any(|l| Some(l) == location) {
                return Some(express!("You can't place revived forces there"));
            }

            if i32::from(self.number_of_forces_in_location)
                > Self::number_of_forces_that_may_be_placed_on_planet(
                    game,
                    p,
                    self.uses_emperor_secret_ally,
                    i32::from(self.amount_of_forces) + i32::from(self.extra_forces_paid_by_emperor),
                )
                || i32::from(self.number_of_special_forces_in_location)
                    > Self::number_of_special_forces_that_may_be_placed_on_planet(
                        game,
                        p,
                        i32::from(self.amount_of_special_forces) + i32::from(self.extra_special_forces_paid_by_emperor),
                    )
            {
                return Some(express!("You can't place that many forces directly on the planet"));
            }
        }

        None
    }

    pub fn determine_cost_of_forces_for_emperor(g: &Game, emperor: PlayerId, ally: Faction, forces: i32, special_forces: i32) -> i32 {
        let halves = forces * Self::get_price_per_force_in_halves(g, emperor)
            + special_forces * Self::get_price_per_special_force_in_halves(g, emperor, ally);
        half_rounded_up(halves)
    }

    pub fn determine_cost(g: &Game, initiator: PlayerId, hero: Option<Hero>, amount_of_forces: i32, amount_of_special_forces: i32, extra_forces_paid_by_emperor: i32, extra_special_forces_paid_by_emperor: i32, uses_emperor_secret_ally: bool) -> RevivalCost {
        let mut result = RevivalCost::new();
        let ip = g.player(initiator);

        result.cost_for_emperor = if ip.ally == Some(Faction::Emperor) {
            Self::determine_cost_of_forces_for_emperor(
                g,
                ip.allied_player(g).expect("allied player"),
                ip.faction.expect("faction"),
                extra_forces_paid_by_emperor,
                extra_special_forces_paid_by_emperor,
            )
        } else {
            0
        };
        let mut nr_of_paid_special_forces = 0;
        let mut number_of_forces_revived_for_free = 0;
        result.cost_for_force_revival_for_player = Self::get_price_of_force_revival(
            g,
            initiator,
            amount_of_forces,
            amount_of_special_forces,
            uses_emperor_secret_ally,
            &mut nr_of_paid_special_forces,
            &mut number_of_forces_revived_for_free,
        );
        result.includes_costs_for_special_forces = nr_of_paid_special_forces > 0;
        result.number_of_forces_revived_for_free = number_of_forces_revived_for_free;

        result.cost_to_revive_hero = Self::get_price_of_hero_revival(g, initiator, hero);
        result.can_be_paid = i32::from(ip.resources) >= result.total_cost_for_player();

        result
    }

    pub fn get_price_of_force_revival(g: &Game, initiator: PlayerId, amount_of_forces: i32, amount_of_special_forces: i32, uses_emperor_secret_ally: bool, nr_of_paid_special_forces: &mut i32, number_of_forces_revived_for_free: &mut i32) -> i32 {
        let ip = g.player(initiator);
        let nr_of_free_revivals = g.free_revivals(initiator, uses_emperor_secret_ally);
        *nr_of_paid_special_forces = if ip.is(Faction::Emperor) && ip.has_low_threshold(g, World::SalusaSecundus) {
            amount_of_special_forces
        } else {
            std::cmp::max(0, amount_of_special_forces - nr_of_free_revivals)
        };

        let nr_of_free_revivals_left =
            nr_of_free_revivals - (amount_of_special_forces - (*nr_of_paid_special_forces));
        let nr_of_paid_normal_forces = std::cmp::max(0, amount_of_forces - nr_of_free_revivals_left);
        *number_of_forces_revived_for_free = amount_of_forces + amount_of_special_forces - (*nr_of_paid_special_forces) - nr_of_paid_normal_forces;

        let price_of_special_forces: i32 = if ip.is(Faction::Ixian) { 3 } else { 2 };
        let price_of_normal_forces: i32 =
            if ip.is(Faction::Choam) && !g.prevented(FactionAdvantage::ChoamRevival) { 1 } else { 2 };
        let mut cost = (*nr_of_paid_special_forces * price_of_special_forces) + (nr_of_paid_normal_forces * price_of_normal_forces);

        if Self::may_revive_with_discount(g, initiator) {
            cost = half_rounded_up(cost);
        }

        cost
    }

    /// Whether `hero` is a leader or the Messiah and alive. Reviving one would put them in the
    /// tanks instead.
    pub fn is_living_leader(game: &Game, hero: Hero) -> bool {
        game.leaders.leader_state.get(&hero).is_some_and(|s| s.alive())
    }

    /// Whether reviving `hero` takes back a living leader of `initiator` that the Tleilaxu hold
    /// as a ghola. That leader isn't dead, and the revival moves them back to their faction.
    fn is_ghola_returning_home(game: &Game, initiator: Faction, hero: Hero) -> bool {
        let Hero::Leader(leader) = hero else { return false };
        leader.faction() == initiator
            && initiator != Faction::Tleilaxu
            && game
                .get_player(Some(Faction::Tleilaxu))
                .is_some_and(|tleilaxu| game.player(tleilaxu).leaders.contains(&leader))
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    /// Only the early-revival branch is lazy; the others are computed in full.
    pub fn valid_revival_heroes(g: &Game, p: PlayerId) -> impl Iterator<Item = Hero> {
        use either::Either;
        use std::iter::empty;
        let mut result: Vec<Hero> = Vec::new();
        let pl = g.player(p);

        if pl.faction != Some(Faction::Tleilaxu) {
            if g.revival.leader_revivals_this_turn.contains_key_opt(pl.faction) {
                return Either::Left(Either::Left(empty()));
            }

            result.extend(Self::normally_revivable_heroes(g, p));
        } else if pl.leaders.iter().filter(|l| g.is_alive(Hero::Leader(**l))).count() < 5 {
            result.extend(Self::unrestricted_revivable_heroes(g, p));
        }

        if result.is_empty() {
            let tleilaxu = g.get_player(Some(Faction::Tleilaxu));
            // The gholas Where reads p.Faction per element; KilledHeroes builds its list now;
            // the Union and the outer Where run as the caller pulls.
            let gholas = match tleilaxu {
                Some(tleilaxu) => Either::Left(g.player(tleilaxu).leaders.iter().filter(move |l| Some(l.faction()) == g.player(p).faction).map(|l| Hero::Leader(*l))),
                None => Either::Right(empty()),
            };
            let killed = g.killed_heroes(p);
            return Either::Left(Either::Right(
                crate::iter::unique(killed.into_iter().chain(gholas)).filter(move |h| Self::is_allowed_early_revival(g, *h)),
            ));
        }

        let mut living_leaders = pl.leaders.iter().filter(|l| g.is_alive(Hero::Leader(**l))).count() as i32;
        living_leaders += g
            .players
            .iter()
            .enumerate()
            .filter(|(i, _)| g.player_at(*i) != p)
            .flat_map(|(_, player)| player.leaders.iter().filter(|l| Some(l.faction()) == pl.faction))
            .count() as i32;

        if pl.is(Faction::Tleilaxu)
            && g.applicable(Rule::TleilaxuGholas)
            && living_leaders < 5
            && !g.prevented(FactionAdvantage::TleilaxuReviveGhola)
        {
            result.extend(
                g.leaders.leader_state
                    .iter()
                    .filter(|(key, value)| {
                        key.faction() != Some(Faction::Tleilaxu)
                            && **key != LeaderManager::messiah()
                            && key.hero_type() != HeroType::Auditor
                            && !value.alive()
                    })
                    .map(|(key, _)| *key),
            );
        }

        Either::Right(result.into_iter())
    }

    pub fn is_allowed_early_revival(g: &Game, h: Hero) -> bool {
        g.revival.early_revivals_offers.get(&h).is_some_and(|value| *value < i32::MAX)
    }

    pub fn normally_revivable_heroes(g: &Game, p: PlayerId) -> Vec<Hero> {
        let mut result: Vec<Hero> = Vec::new();
        let pl = g.player(p);

        if Self::all_available_leaders_have_died_once(g, p) || Self::at_least_five_leaders_have_died_once(g, p) {
            if !pl.leaders.is_empty() {
                let lowest_death_count =
                    pl.leaders.iter().map(|l| g.death_count(Hero::Leader(*l))).min().expect("a leader");

                result.extend(
                    pl.leaders
                        .iter()
                        .filter(|l| {
                            g.death_count(Hero::Leader(**l)) == lowest_death_count && !g.is_alive(Hero::Leader(**l))
                        })
                        .map(|l| Hero::Leader(*l)),
                );

                if pl.is(Faction::Atreides)
                    && !g.is_alive(LeaderManager::messiah())
                    && g.death_count(LeaderManager::messiah()) == lowest_death_count
                {
                    result.push(LeaderManager::messiah());
                }
            }
        }

        if pl.faction == Some(Faction::Choam) {
            let auditor = pl.leaders.iter().copied().find(|l| l.hero_type() == HeroType::Auditor);
            if let Some(auditor) = auditor {
                if !g.is_alive(Hero::Leader(auditor)) && !result.contains(&Hero::Leader(auditor)) {
                    result.push(Hero::Leader(auditor));
                }
            }
        }

        if pl.faction == Some(Faction::Ecaz) {
            if let Some(vidal) = g.vidal() {
                if !g.is_alive(Hero::Leader(vidal)) && !result.contains(&Hero::Leader(vidal)) {
                    result.push(Hero::Leader(vidal));
                }
            }
        }

        result
    }

    pub fn at_least_five_leaders_have_died_once(g: &Game, p: PlayerId) -> bool {
        g.player(p).leaders.iter().filter(|l| g.death_count(Hero::Leader(**l)) > 0).count() >= 5
    }

    pub fn all_available_leaders_have_died_once(g: &Game, p: PlayerId) -> bool {
        g.player(p).leaders.iter().all(|l| g.death_count(Hero::Leader(*l)) > 0)
    }

    pub fn unrestricted_revivable_heroes(g: &Game, p: PlayerId) -> Vec<Hero> {
        let mut result: Vec<Hero> = Vec::new();
        let pl = g.player(p);

        result.extend(
            pl.leaders
                .iter()
                .filter(|l| !g.is_alive(Hero::Leader(**l)) && l.hero_type() != HeroType::Auditor)
                .map(|l| Hero::Leader(*l)),
        );

        if pl.is(Faction::Atreides) && !g.is_alive(LeaderManager::messiah()) {
            result.push(LeaderManager::messiah());
        }

        result
    }

    pub fn valid_max_revivals(g: &Game, p: PlayerId, special_forces: bool, using_emperor_secret_ally: bool) -> i32 {
        let normal_force_revival_limit = g.get_max_revivals(g, p, using_emperor_secret_ally);
        let pl = g.player(p);

        if !special_forces {
            return std::cmp::min(normal_force_revival_limit, i32::from(pl.forces_killed));
        }

        std::cmp::min(
            if pl.is(Faction::Ixian) {
                normal_force_revival_limit
            } else if g.revival.factions_that_revived_special_forces_this_turn.contains_opt(pl.faction) {
                0
            } else {
                1
            },
            i32::from(pl.special_forces_killed),
        )
    }

    pub fn may_assign_skill(g: &Game, p: PlayerId, h: Option<Hero>) -> bool {
        let Some(h) = h else { return false };

        let harkonnen = g.get_player(Some(Faction::Harkonnen));
        let pl = g.player(p);

        let captured_leaders_to_consider: Vec<Leader> = match harkonnen {
            Some(harkonnen) => g.player(harkonnen).leaders.iter().copied().filter(|l| Some(l.faction()) == pl.faction).collect(),
            None => Vec::new(),
        };

        h.as_leader().is_some()
            && Some(h.faction()) == Some(pl.faction)
            && h.hero_type() != HeroType::Auditor
            && g.applicable(Rule::LeaderSkills)
            && !g.leaders.captured_leaders.iter().any(|(k, v)| Some(*v) == pl.faction && g.is_skilled(Some(Hero::Leader(*k))))
            && !(pl.is(Faction::Ecaz) && g.is_skilled(g.vidal().map(Hero::Leader)))
            && !pl.leaders.iter().any(|l| g.is_skilled(Some(Hero::Leader(*l))))
            && !captured_leaders_to_consider.iter().any(|l| g.is_skilled(Some(Hero::Leader(*l))))
    }

    pub fn may_revive_with_discount(g: &Game, p: PlayerId) -> bool {
        let pl = g.player(p);
        (pl.is(Faction::Tleilaxu) && !g.prevented(FactionAdvantage::TleilaxuRevivalDiscount))
            || (pl.ally == Some(Faction::Tleilaxu)
                && g.alliances.tleilaxu_allows_revival_discount
                && !g.prevented(FactionAdvantage::TleilaxuRevivalDiscountAlly))
    }

    pub fn get_price_of_hero_revival(g: &Game, initiator: PlayerId, hero: Option<Hero>) -> i32 {
        let tleilaxu_discount_prevented = g.prevented(FactionAdvantage::TleilaxuRevivalDiscount);

        let Some(hero) = hero else { return 0 };

        let ip = g.player(initiator);
        let mut price = hero.cost_to_revive();

        if ip.is(Faction::Tleilaxu) && !tleilaxu_discount_prevented {
            price = half_rounded_up(price);
        } else if let Some(offer) = (!Self::normally_revivable_heroes(g, initiator).contains(&hero))
            .then(|| g.revival.early_revivals_offers.get(&hero).copied())
            .flatten()
        {
            price = offer;
        } else if ip.ally == Some(Faction::Tleilaxu)
            && g.alliances.tleilaxu_allows_revival_discount
            && !tleilaxu_discount_prevented
        {
            price = half_rounded_up(price);
        }

        price
    }

    /// The spice one force costs to revive, in halves of a spice: a discount halves it.
    pub fn get_price_per_force_in_halves(g: &Game, reviving_player: PlayerId) -> i32 {
        let rp = g.player(reviving_player);
        let price = if rp.is(Faction::Choam) && !g.prevented(FactionAdvantage::ChoamRevival) { 1 } else { 2 };
        if Self::may_revive_with_discount(g, reviving_player) { price } else { 2 * price }
    }

    /// The spice one special force costs to revive, in halves of a spice: a discount halves it.
    pub fn get_price_per_special_force_in_halves(g: &Game, reviving_player: PlayerId, of_revived_forces: Faction) -> i32 {
        let price = if of_revived_forces == Faction::Ixian { 3 } else { 2 };
        if Self::may_revive_with_discount(g, reviving_player) { price } else { 2 * price }
    }

    pub fn valid_max_revivals_by_emperor(g: &Game, p: PlayerId) -> i32 {
        let pl = g.player(p);
        if pl.ally != Some(Faction::Emperor) {
            return 0;
        }

        // `GetPlayer(Faction.Emperor)!`: null is only dereferenced in the loop below.
        let emperor = g.get_player(Some(Faction::Emperor));

        let potential_maximum = if pl.ally == Some(Faction::Emperor) && !g.prevented(FactionAdvantage::EmperorLetAllyReviveExtraForces) {
            g.alliances.emperor_will_pay_for_extra_revival
        } else {
            0
        };

        let price_of_special_forces: i32 = if pl.faction == Some(Faction::Ixian) { 3 } else { 2 };

        // Counting loops kept as loops, with wrapping arithmetic.
        let mut special_forces_paid_by_emperor: i32 = 0;
        while ((special_forces_paid_by_emperor + 1) * price_of_special_forces)
            <= i32::from(g.player(emperor.expect("emperor")).resources)
            && (special_forces_paid_by_emperor + 1) <= potential_maximum
        {
            special_forces_paid_by_emperor += 1;
        }

        let mut forces_paid_by_emperor: i32 = 0;
        while ((special_forces_paid_by_emperor * price_of_special_forces) + ((forces_paid_by_emperor + 1) * 2))
            <= i32::from(g.player(emperor.expect("emperor")).resources)
            && (special_forces_paid_by_emperor + forces_paid_by_emperor + 1) <= potential_maximum
        {
            forces_paid_by_emperor += 1;
        }

        special_forces_paid_by_emperor + forces_paid_by_emperor
    }

    pub fn may_use_emperor_secret_ally(game: &Game, player: PlayerId) -> bool {
        game.player(player).nexus == Some(Faction::Emperor) && NexusPlayed::can_use_secret_ally(game, player)
    }

    pub fn number_of_forces_that_may_be_placed_on_planet(game: &Game, player: PlayerId, uses_emperor_secret_ally: bool, nr_of_forces_to_revive: i32) -> i32 {
        let pl = game.player(player);
        if pl.is(Faction::Tleilaxu) && pl.has_any_high_threshold(game) {
            std::cmp::min(nr_of_forces_to_revive, game.free_revivals(player, uses_emperor_secret_ally))
        } else {
            0
        }
    }

    pub fn number_of_special_forces_that_may_be_placed_on_planet(game: &Game, player: PlayerId, nr_of_special_forces_to_revive: i32) -> i32 {
        let pl = game.player(player);
        if pl.is(Faction::Fremen) && pl.has_any_high_threshold(game) {
            std::cmp::min(nr_of_special_forces_to_revive, 1)
        } else {
            0
        }
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    /// The predicate can throw (`AlliedPlayer` null), and callers stop enumerating at the first match.
    pub fn valid_revived_force_locations(g: &Game, p: PlayerId) -> impl Iterator<Item = Location> {
        g.board.map.locations(false).filter(move |l| {
            let pl = g.player(p);
            (pl.faction == Some(Faction::Fremen) || !Sector::in_storm(l.sector(&g.board.map), g.board.sector_in_storm))
                && (!l.territory().is_stronghold() || g.nr_of_occupants_excluding_faction(*l, pl.faction) < 2)
                && l.visible(&g.board.map)
                && (!pl.has_ally()
                    || *l == g.board.map.polar_sink()
                    || !g.player(pl.allied_player(g).expect("allied player")).occupies(g, Some(*l)))
                && (pl.faction == Some(Faction::Tleilaxu)
                    || (pl.faction == Some(Faction::Fremen) && pl.any_forces_in_territory(Some(l.territory())) >= 1))
        })
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let initiator = self.initiator();
        let me = self.player();

        if self.uses_emperor_secret_ally {
            game.play_nexus_card(
                me,
                vec!["revive ".into(), 3.into(), " additional forces beyond revival limits for free".into()],
            );
        }

        //Payment
        let cost = Self::determine_cost(
            game,
            me,
            self.hero,
            i32::from(self.amount_of_forces),
            i32::from(self.amount_of_special_forces),
            i32::from(self.extra_forces_paid_by_emperor),
            i32::from(self.extra_special_forces_paid_by_emperor),
            self.uses_emperor_secret_ally,
        );
        if cost.cost_for_emperor > 0 {
            let emperor = game.get_player(Some(Faction::Emperor)).required("get player")?;
            crate::game::amounts::spend(&mut game.player_mut(emperor).resources, cost.cost_for_emperor, "spice")?;
        }
        {
            crate::game::amounts::spend(&mut game.player_mut(me).resources, cost.total_cost_for_player(), "spice")?;
        }

        let high_threshold_extra_suboids = if initiator == Some(Faction::Ixian)
            && game.player(me).has_any_high_threshold(game)
            && cost.includes_costs_for_special_forces
        {
            std::cmp::max(
                0,
                std::cmp::min(
                    2,
                    (i32::from(game.player(me).forces_killed) - i32::from(self.amount_of_forces)) - i32::from(self.extra_forces_paid_by_emperor),
                ),
            )
        } else {
            0
        };

        //Force revival
        Player::revive_forces(
            game,
            me,
            (i32::from(self.amount_of_forces) + i32::from(self.extra_forces_paid_by_emperor)) + high_threshold_extra_suboids,
        )?;
        Player::revive_special_forces(
            game,
            me,
            i32::from(self.amount_of_special_forces) + i32::from(self.extra_special_forces_paid_by_emperor),
        )?;

        if i32::from(self.amount_of_special_forces) + i32::from(self.extra_special_forces_paid_by_emperor) > 0 {
            game.revival.factions_that_revived_special_forces_this_turn.push(initiator.required("faction")?);
        }

        //Register free revival
        let mut first_free_revival_this_turn = false;
        let nr_of_free_revivals = std::cmp::min(
            game.free_revivals(me, self.uses_emperor_secret_ally),
            i32::from(self.amount_of_forces) + i32::from(self.amount_of_special_forces),
        );

        if nr_of_free_revivals > 0 {
            let free_revivals_this_turn = game.revival.free_revivals_this_turn.get_opt(initiator).copied().unwrap_or(0);
            first_free_revival_this_turn = free_revivals_this_turn == 0;
            game.revival.free_revivals_this_turn.insert(initiator.required("faction")?, free_revivals_this_turn + nr_of_free_revivals);
        }

        //Tech token activated?
        if first_free_revival_this_turn && initiator != Some(Faction::Tleilaxu) {
            game.revival.revival_tech_token_income = true;
        }

        //Tleilaxu income
        let tleilaxu = game.get_player(Some(Faction::Tleilaxu));
        let mut total_profits_for_tleilaxu: i32 = 0;
        if let Some(tleilaxu) = tleilaxu {
            if first_free_revival_this_turn && !game.revival.tleilaxu_started_revival_with_low_threshold {
                total_profits_for_tleilaxu += 1;
            }

            if initiator != Some(Faction::Tleilaxu) {
                total_profits_for_tleilaxu += cost.total_cost_for_force_revival();
                total_profits_for_tleilaxu += cost.cost_to_revive_hero;
            }

            if total_profits_for_tleilaxu > 0 && game.prevented(FactionAdvantage::TleilaxuReceiveRevive) {
                total_profits_for_tleilaxu = 0;
                game.log_prevention_by_karma(FactionAdvantage::TleilaxuReceiveRevive);
                if !game.applicable(Rule::FullPhaseKarma) {
                    game.allow(FactionAdvantage::TleilaxuReceiveRevive);
                }
            }

            {
                crate::game::amounts::gain(&mut game.player_mut(tleilaxu).resources, total_profits_for_tleilaxu, "spice")?;
            }

            if total_profits_for_tleilaxu >= 5 {
                game.apply_bureaucracy(initiator, Some(Faction::Tleilaxu));
            }
        }

        if (cost.total_cost() - total_profits_for_tleilaxu) >= 4 {
            game.activate_banker(me);
        }

        //Hero revival
        let mut as_ghola = false;
        if let Some(hero) = self.hero {
            let v = game.revival.leader_revivals_this_turn.get_opt(initiator).copied().unwrap_or(0) + 1;
            game.revival.leader_revivals_this_turn.insert(initiator.required("faction")?, v);
            let leader = hero.as_leader();

            if initiator != hero.faction() && leader.is_some() {
                as_ghola = true;
                game.revive(me, Hero::Leader(leader.required("leader")?));
            } else if tleilaxu.is_some_and(|tleilaxu| game.player(tleilaxu).leaders.iter().any(|l| Hero::Leader(*l) == hero))
                && game.is_alive(hero)
                && leader.is_some()
            {
                //Transfer of ghola
                let leader = leader.required("leader")?;
                let tleilaxu = tleilaxu.required("tleilaxu")?;
                let leaders = &mut game.player_mut(tleilaxu).leaders;
                if let Some(i) = leaders.iter().position(|l| *l == leader) {
                    leaders.remove(i);
                }
                game.player_mut(me).leaders.push(leader);
            } else {
                game.revive(me, hero);
            }

            if self.assign_skill {
                game.prepare_skill_assignment_to_revived_leader(me, leader);
            }

            game.revival.early_revivals_offers.shift_remove(&hero);
        }

        //Logging
        game.stone(Milestone::Revival);
        self.register_and_log_revival(game, &cost, total_profits_for_tleilaxu, as_ghola, high_threshold_extra_suboids);

        if let Some(location) = self.location {
            if i32::from(self.number_of_special_forces_in_location) > 0 || i32::from(self.number_of_forces_in_location) > 0 {
                let mut normal_force_message = express!();
                if i32::from(self.number_of_forces_in_location) > 0 {
                    Player::ship_forces(game, me, location, i32::from(self.number_of_forces_in_location))?;
                    normal_force_message = express!(i32::from(self.number_of_forces_in_location), game.player(me).force());
                }

                let mut special_force_message = express!();
                if i32::from(self.number_of_special_forces_in_location) > 0 {
                    Player::ship_special_forces(game, me, location, i32::from(self.number_of_special_forces_in_location))?;
                    special_force_message =
                        express!(i32::from(self.number_of_special_forces_in_location), game.player(me).special_force());
                }

                let to = self.location;
                game.log(vec![
                    initiator.into(),
                    " place ".into(),
                    normal_force_message.into(),
                    special_force_message.into(),
                    " in ".into(),
                    to.into(),
                ]);
                game.ship_move.last_shipment_or_movement = Some(self.clone().into_event());
            }

            if game.check_intrusion(&self.clone().into_event(), self.initiator().expect("faction"))? {
                game.ship_move.phase_before_revival_caused_intrusion = Some(game.turn.current_phase);
                let bene_gesserit = game.last_bene_gesserit_intrusion().is_some();
                let terror = game.last_terror_trigger().is_some();
                game.enter_first(&[(bene_gesserit, Step::Enter(Phase::BeneGesseritIntrudedByRevival)), (terror, Step::Enter(Phase::TerrorTriggeredByRevival))], Step::Enter(Phase::AmbassadorTriggeredByRevival))?;
            }
        }
        Ok(())
    }

    pub fn register_and_log_revival(&self, game: &mut Game, cost: &RevivalCost, tleilaxu_received_resources: i32, as_ghola: bool, high_threshold_extra_suboids: i32) {
        let initiator = self.initiator();
        let me = self.player();
        let total_amount_of_forces = i32::from(self.amount_of_forces) + i32::from(self.extra_forces_paid_by_emperor) + i32::from(self.amount_of_special_forces) + i32::from(self.extra_special_forces_paid_by_emperor) + high_threshold_extra_suboids;

        let v = game.revival.total_revivals_this_turn.get_opt(initiator).copied().unwrap_or(0) + total_amount_of_forces;
        game.revival.total_revivals_this_turn.insert(initiator.expect("faction"), v);

        // Each kind of force counted once; Ixian's free extra suboids have their own part.
        let forces = i32::from(self.amount_of_forces) + i32::from(self.extra_forces_paid_by_emperor);
        let special_forces = i32::from(self.amount_of_special_forces) + i32::from(self.extra_special_forces_paid_by_emperor);

        let total_cost = cost.total_cost_for_player() + cost.cost_for_emperor;
        let paid_by_emperor = i32::from(self.extra_forces_paid_by_emperor) > 0 || i32::from(self.extra_special_forces_paid_by_emperor) > 0;

        let pl = game.player(me);
        let message = express!(
            initiator,
            " revive ",
            express_part_if!(self.hero.is_some(); self.hero),
            express_part_if!(as_ghola; " as Ghola"),
            express_part_if!(self.hero.is_some() && (forces + special_forces) > 0; " and "),
            express_part_if!(forces > 0; forces, pl.force()),
            express_part_if!(special_forces > 0; special_forces, pl.special_force()),
            " for ",
            Payment::of(total_cost),
            express_part_if!(paid_by_emperor; " (", Payment::of_by(cost.cost_for_emperor, Some(Faction::Emperor)), ")"),
            express_part_if!(tleilaxu_received_resources > 0; " → ", Faction::Tleilaxu, " get ", Payment::of(tleilaxu_received_resources)),
            express_part_if!(high_threshold_extra_suboids > 0; " + ", high_threshold_extra_suboids, FactionForce::Ixian, " for free")
        );
        let mut row = RevivalRow::new(
            initiator.expect("faction"),
            forces + high_threshold_extra_suboids,
            special_forces,
            total_cost,
        );
        row.hero = self.hero;
        row.as_ghola = as_ghola;
        row.paid_by_other = if paid_by_emperor { cost.cost_for_emperor } else { 0 };
        row.payer = if paid_by_emperor { Some(Faction::Emperor) } else { None };
        row.tleilaxu_income = tleilaxu_received_resources;
        game.log(vec![message.with_log(row).into()]);
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " perform revival")
    }

}

impl GameEventMethods for Revival {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> EventType {
        EventType::Revival
    }

    fn into_event(self: Rc<Self>) -> Event {
        Event::Revival(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        Revival::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        Revival::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        Revival::get_message(self, game)
    }

}

impl crate::events::location_event::LocationEvent for Revival {
    fn to(&self, game: &Game) -> Option<Location> {
        Revival::to(self, game)
    }

    fn total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        Revival::total_amount_of_forces_added_to_location(self, game)
    }

    fn forces_added_to_location(&self, game: &Game) -> i32 {
        Revival::forces_added_to_location(self, game)
    }

    fn special_forces_added_to_location(&self, game: &Game) -> i32 {
        Revival::special_forces_added_to_location(self, game)
    }

}
