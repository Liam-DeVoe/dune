use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct Revival {
    pub amount_of_forces: i32,
    pub amount_of_special_forces: i32,
    pub extra_forces_paid_by_emperor: i32,
    pub extra_special_forces_paid_by_emperor: i32,
    pub number_of_forces_in_location: i32,
    pub number_of_special_forces_in_location: i32,
    #[canon(name = "_locationId")]
    pub location_id: i32,
    #[canon(name = "_heroId")]
    pub hero_id: i32,
    pub assign_skill: bool,
    pub uses_emperor_secret_ally: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
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
            location_id: -1,
            hero_id: Default::default(),
            assign_skill: Default::default(),
            uses_emperor_secret_ally: Default::default(),
            base: Default::default(),
        }
    }
}

impl Revival {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn location(&self, game: &Game) -> Option<Location> {
        game.map.location_lookup().find(self.location_id)
    }

    pub fn set_location(&mut self, game: &Game, value: Option<Location>) {
        self.location_id = game.map.location_lookup().get_id(value);
    }

    pub fn to(&self, game: &Game) -> Option<Location> {
        self.location(game)
    }

    pub fn hero(&self) -> Option<Hero> {
        LeaderManager::hero_lookup().find(self.hero_id)
    }

    pub fn set_hero(&mut self, value: Option<Hero>) {
        self.hero_id = LeaderManager::hero_lookup().get_id(value);
    }

    pub fn total_amount_of_forces_added_to_location(&self, _game: &Game) -> i32 {
        self.number_of_forces_in_location + self.number_of_special_forces_in_location
    }

    pub fn forces_added_to_location(&self, _game: &Game) -> i32 {
        self.number_of_forces_in_location
    }

    pub fn special_forces_added_to_location(&self, _game: &Game) -> i32 {
        self.number_of_special_forces_in_location
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        // `var p = Player;` does not throw; the first dereference (line 84) does.
        let p_opt = self.player_opt();
        let initiator = self.initiator();
        let forces = self.amount_of_forces + self.extra_forces_paid_by_emperor;
        let special_forces = self.amount_of_special_forces + self.extra_special_forces_paid_by_emperor;

        if forces <= 0 && special_forces <= 0 && self.hero().is_none() {
            return Some(express!("Select forces or a leader to revive"));
        }

        let p = p_opt.expect("player");
        let pl = game.player(p);
        if forces > pl.forces_killed {
            return Some(express!("You can't revive that many ", pl.force()));
        }
        if special_forces > pl.special_forces_killed {
            return Some(express!("You can't revive that many ", pl.special_force()));
        }

        if initiator != Faction::Ixian && special_forces > 1 {
            return Some(express!("You can't revive more than one ", pl.special_force(), " per turn"));
        }
        if special_forces > 0
            && initiator != Faction::Ixian
            && game.factions_that_revived_special_forces_this_turn.contains(&initiator)
        {
            return Some(express!("You already revived a ", pl.special_force(), " this turn"));
        }

        let limit = game.get_max_revivals(game, p, self.uses_emperor_secret_ally);
        if (self.amount_of_forces + self.amount_of_special_forces) > limit {
            return Some(express!("You can't revive more than ", limit));
        }

        let ally_limit = Self::valid_max_revivals_by_emperor(game, p);
        if (self.extra_forces_paid_by_emperor + self.extra_special_forces_paid_by_emperor)
            > Self::valid_max_revivals_by_emperor(game, p)
        {
            return Some(express!("Your ally won't revive more than ", ally_limit));
        }

        if initiator != Faction::Tleilaxu
            && game.leader_revivals_this_turn.get(&initiator).copied().unwrap_or(0) > 0
        {
            return Some(express!("You cannot revive more leaders this turn"));
        }

        let cost_of_revival = Self::determine_cost(
            game,
            p,
            self.hero(),
            self.amount_of_forces,
            self.amount_of_special_forces,
            self.extra_forces_paid_by_emperor,
            self.extra_special_forces_paid_by_emperor,
            self.uses_emperor_secret_ally,
        );
        if cost_of_revival.total_cost_for_player() > game.player(p).resources {
            return Some(express!("You can't pay that many"));
        }

        if self.assign_skill && self.hero().is_none() {
            return Some(express!("You must revive a leader to assign a skill to"));
        }
        if self.assign_skill && !Self::may_assign_skill(game, p, self.hero()) {
            return Some(express!("You can't assign a skill to this leader"));
        }

        if self.uses_emperor_secret_ally && !Self::may_use_emperor_secret_ally(game, p) {
            return Some(express!("you can't use ", Faction::Emperor, " cunning"));
        }

        if self.location(game).is_some() {
            let location = self.location(game);
            // Contains on the lazy Where: stops at the first match (see valid_revived_force_locations).
            if !Self::valid_revived_force_locations(game, p).any(|l| Some(l) == location) {
                return Some(express!("You can't place revived forces there"));
            }

            if self.number_of_forces_in_location
                > Self::number_of_forces_that_may_be_placed_on_planet(
                    game,
                    p,
                    self.uses_emperor_secret_ally,
                    self.amount_of_forces + self.extra_forces_paid_by_emperor,
                )
                || self.number_of_special_forces_in_location
                    > Self::number_of_special_forces_that_may_be_placed_on_planet(
                        game,
                        p,
                        self.amount_of_special_forces + self.extra_special_forces_paid_by_emperor,
                    )
            {
                return Some(express!("You can't place that many forces directly on the planet"));
            }
        }

        None
    }

    pub fn determine_cost_of_forces_for_emperor(g: &Game, emperor: PlayerIndex, ally: Faction, forces: i32, special_forces: i32) -> i32 {
        // float arithmetic, then Math.Ceiling widens to double (hazards 9.1/9.2).
        let sum: f32 = forces as f32 * Self::get_price_per_force(g, emperor)
            + special_forces as f32 * Self::get_price_per_special_force(g, emperor, ally);
        (sum as f64).ceil() as i32
    }

    pub fn determine_cost(g: &Game, initiator: PlayerIndex, hero: Option<Hero>, amount_of_forces: i32, amount_of_special_forces: i32, extra_forces_paid_by_emperor: i32, extra_special_forces_paid_by_emperor: i32, uses_emperor_secret_ally: bool) -> RevivalCost {
        let mut result = RevivalCost::new();
        let ip = g.player(initiator);

        result.cost_for_emperor = if ip.ally == Faction::Emperor {
            Self::determine_cost_of_forces_for_emperor(
                g,
                ip.allied_player(g).expect("allied player"),
                ip.faction,
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
        result.can_be_paid = ip.resources >= result.total_cost_for_player();

        result
    }

    pub fn get_price_of_force_revival(g: &Game, initiator: PlayerIndex, amount_of_forces: i32, amount_of_special_forces: i32, uses_emperor_secret_ally: bool, nr_of_paid_special_forces: &mut i32, number_of_forces_revived_for_free: &mut i32) -> i32 {
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
            cost = (0.5 * cost as f64).ceil() as i32;
        }

        cost
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    /// Only the early-revival branch is lazy; the others are computed in full.
    pub fn valid_revival_heroes(g: &Game, p: PlayerIndex) -> impl Iterator<Item = Hero> {
        use either::Either;
        use std::iter::empty;
        let mut result: Vec<Hero> = Vec::new();
        let pl = g.player(p);

        if pl.faction != Faction::Tleilaxu {
            if g.leader_revivals_this_turn.contains_key(&pl.faction) {
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
                Some(tleilaxu) => Either::Left(g.player(tleilaxu).leaders.iter().filter(move |l| l.faction() == g.player(p).faction).map(|l| Hero::Leader(*l))),
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
            .flat_map(|(_, player)| player.leaders.iter().filter(|l| l.faction() == pl.faction))
            .count() as i32;

        if pl.is(Faction::Tleilaxu)
            && g.applicable(Rule::TleilaxuGholas)
            && living_leaders < 5
            && !g.prevented(FactionAdvantage::TleilaxuReviveGhola)
        {
            result.extend(
                g.leader_state
                    .iter()
                    .filter(|(key, value)| {
                        key.faction() != Faction::Tleilaxu
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
        g.early_revivals_offers.get(&h).is_some_and(|value| *value < i32::MAX)
    }

    pub fn normally_revivable_heroes(g: &Game, p: PlayerIndex) -> Vec<Hero> {
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

        if pl.faction == Faction::Choam {
            let auditor = pl.leaders.iter().copied().find(|l| l.hero_type() == HeroType::Auditor);
            if let Some(auditor) = auditor {
                if !g.is_alive(Hero::Leader(auditor)) && !result.contains(&Hero::Leader(auditor)) {
                    result.push(Hero::Leader(auditor));
                }
            }
        }

        if pl.faction == Faction::Ecaz {
            if let Some(vidal) = g.vidal() {
                if !g.is_alive(Hero::Leader(vidal)) && !result.contains(&Hero::Leader(vidal)) {
                    result.push(Hero::Leader(vidal));
                }
            }
        }

        result
    }

    pub fn at_least_five_leaders_have_died_once(g: &Game, p: PlayerIndex) -> bool {
        g.player(p).leaders.iter().filter(|l| g.death_count(Hero::Leader(**l)) > 0).count() >= 5
    }

    pub fn all_available_leaders_have_died_once(g: &Game, p: PlayerIndex) -> bool {
        g.player(p).leaders.iter().all(|l| g.death_count(Hero::Leader(*l)) > 0)
    }

    pub fn unrestricted_revivable_heroes(g: &Game, p: PlayerIndex) -> Vec<Hero> {
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

    pub fn valid_max_revivals(g: &Game, p: PlayerIndex, special_forces: bool, using_emperor_secret_ally: bool) -> i32 {
        let normal_force_revival_limit = g.get_max_revivals(g, p, using_emperor_secret_ally);
        let pl = g.player(p);

        if !special_forces {
            return std::cmp::min(normal_force_revival_limit, pl.forces_killed);
        }

        std::cmp::min(
            if pl.is(Faction::Ixian) {
                normal_force_revival_limit
            } else if g.factions_that_revived_special_forces_this_turn.contains(&pl.faction) {
                0
            } else {
                1
            },
            pl.special_forces_killed,
        )
    }

    pub fn may_assign_skill(g: &Game, p: PlayerIndex, h: Option<Hero>) -> bool {
        let Some(h) = h else { return false };

        let harkonnen = g.get_player(Some(Faction::Harkonnen));
        let pl = g.player(p);

        let captured_leaders_to_consider: Vec<Leader> = match harkonnen {
            Some(harkonnen) => g.player(harkonnen).leaders.iter().copied().filter(|l| l.faction() == pl.faction).collect(),
            None => Vec::new(),
        };

        h.as_leader().is_some()
            && h.faction() == pl.faction
            && h.hero_type() != HeroType::Auditor
            && g.applicable(Rule::LeaderSkills)
            && !g.captured_leaders.iter().any(|(k, v)| *v == pl.faction && g.is_skilled(Some(Hero::Leader(*k))))
            && !(pl.is(Faction::Ecaz) && g.is_skilled(g.vidal().map(Hero::Leader)))
            && !pl.leaders.iter().any(|l| g.is_skilled(Some(Hero::Leader(*l))))
            && !captured_leaders_to_consider.iter().any(|l| g.is_skilled(Some(Hero::Leader(*l))))
    }

    pub fn may_revive_with_discount(g: &Game, p: PlayerIndex) -> bool {
        let pl = g.player(p);
        (pl.is(Faction::Tleilaxu) && !g.prevented(FactionAdvantage::TleilaxuRevivalDiscount))
            || (pl.ally == Faction::Tleilaxu
                && g.tleilaxu_allows_revival_discount
                && !g.prevented(FactionAdvantage::TleilaxuRevivalDiscountAlly))
    }

    pub fn get_price_of_hero_revival(g: &Game, initiator: PlayerIndex, hero: Option<Hero>) -> i32 {
        let tleilaxu_discount_prevented = g.prevented(FactionAdvantage::TleilaxuRevivalDiscount);

        let Some(hero) = hero else { return 0 };

        let ip = g.player(initiator);
        let mut price = hero.cost_to_revive();

        if ip.is(Faction::Tleilaxu) && !tleilaxu_discount_prevented {
            price = (0.5 * price as f64).ceil() as i32;
        } else if let Some(offer) = (!Self::normally_revivable_heroes(g, initiator).contains(&hero))
            .then(|| g.early_revivals_offers.get(&hero).copied())
            .flatten()
        {
            price = offer;
        } else if ip.ally == Faction::Tleilaxu
            && g.tleilaxu_allows_revival_discount
            && !tleilaxu_discount_prevented
        {
            price = (0.5 * price as f64).ceil() as i32;
        }

        price
    }

    pub fn get_price_per_force(g: &Game, reviving_player: PlayerIndex) -> f32 {
        let rp = g.player(reviving_player);
        (if Self::may_revive_with_discount(g, reviving_player) { 0.5f32 } else { 1.0 })
            * (if rp.is(Faction::Choam) && !g.prevented(FactionAdvantage::ChoamRevival) { 1 } else { 2 }) as f32
    }

    pub fn get_price_per_special_force(g: &Game, reviving_player: PlayerIndex, of_revived_forces: Faction) -> f32 {
        (if Self::may_revive_with_discount(g, reviving_player) { 0.5f32 } else { 1.0 })
            * (if of_revived_forces == Faction::Ixian { 3 } else { 2 }) as f32
    }

    pub fn valid_max_revivals_by_emperor(g: &Game, p: PlayerIndex) -> i32 {
        let pl = g.player(p);
        if pl.ally != Faction::Emperor {
            return 0;
        }

        // `GetPlayer(Faction.Emperor)!`: null is only dereferenced in the loop below.
        let emperor = g.get_player(Some(Faction::Emperor));

        let potential_maximum = if pl.ally == Faction::Emperor && !g.prevented(FactionAdvantage::EmperorLetAllyReviveExtraForces) {
            g.emperor_will_pay_for_extra_revival
        } else {
            0
        };

        let price_of_special_forces: i32 = if pl.faction == Faction::Ixian { 3 } else { 2 };

        // Counting loops kept as loops, with wrapping arithmetic (hazards 1.9).
        let mut special_forces_paid_by_emperor: i32 = 0;
        while ((special_forces_paid_by_emperor + 1) * price_of_special_forces)
            <= g.player(emperor.expect("emperor")).resources
            && (special_forces_paid_by_emperor + 1) <= potential_maximum
        {
            special_forces_paid_by_emperor += 1;
        }

        let mut forces_paid_by_emperor: i32 = 0;
        while ((special_forces_paid_by_emperor * price_of_special_forces) + ((forces_paid_by_emperor + 1) * 2))
            <= g.player(emperor.expect("emperor")).resources
            && (special_forces_paid_by_emperor + forces_paid_by_emperor + 1) <= potential_maximum
        {
            forces_paid_by_emperor += 1;
        }

        special_forces_paid_by_emperor + forces_paid_by_emperor
    }

    pub fn may_use_emperor_secret_ally(game: &Game, player: PlayerIndex) -> bool {
        game.player(player).nexus == Faction::Emperor && NexusPlayed::can_use_secret_ally(game, player)
    }

    pub fn number_of_forces_that_may_be_placed_on_planet(game: &Game, player: PlayerIndex, uses_emperor_secret_ally: bool, nr_of_forces_to_revive: i32) -> i32 {
        let pl = game.player(player);
        if pl.is(Faction::Tleilaxu) && pl.has_high_threshold_0(game) {
            std::cmp::min(nr_of_forces_to_revive, game.free_revivals(player, uses_emperor_secret_ally))
        } else {
            0
        }
    }

    pub fn number_of_special_forces_that_may_be_placed_on_planet(game: &Game, player: PlayerIndex, nr_of_special_forces_to_revive: i32) -> i32 {
        let pl = game.player(player);
        if pl.is(Faction::Fremen) && pl.has_high_threshold_0(game) {
            std::cmp::min(nr_of_special_forces_to_revive, 1)
        } else {
            0
        }
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    /// The predicate can throw (`AlliedPlayer` null), and callers stop enumerating at the first match.
    pub fn valid_revived_force_locations(g: &Game, p: PlayerIndex) -> impl Iterator<Item = Location> {
        g.map.locations(false).filter(move |l| {
            let pl = g.player(p);
            (pl.faction == Faction::Fremen || l.sector(&g.map) != g.sector_in_storm)
                && (!l.territory().is_stronghold() || g.nr_of_occupants_excluding_faction(*l, pl.faction) < 2)
                && l.visible(&g.map)
                && (!pl.has_ally()
                    || *l == g.map.polar_sink()
                    || !g.player(pl.allied_player(g).expect("allied player")).occupies(g, Some(*l)))
                && (pl.faction == Faction::Tleilaxu
                    || (pl.faction == Faction::Fremen && pl.any_forces_in_territory(Some(l.territory())) >= 1))
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
            self.hero(),
            self.amount_of_forces,
            self.amount_of_special_forces,
            self.extra_forces_paid_by_emperor,
            self.extra_special_forces_paid_by_emperor,
            self.uses_emperor_secret_ally,
        );
        if cost.cost_for_emperor > 0 {
            let emperor = game.get_player(Some(Faction::Emperor)).required("get player")?;
            let r = &mut game.player_mut(emperor).resources;
            *r -= cost.cost_for_emperor;
        }
        {
            let r = &mut game.player_mut(me).resources;
            *r -= cost.total_cost_for_player();
        }

        let high_threshold_extra_suboids = if initiator == Faction::Ixian
            && game.player(me).has_high_threshold_0(game)
            && cost.includes_costs_for_special_forces
        {
            std::cmp::max(
                0,
                std::cmp::min(
                    2,
                    (game.player(me)
                        .forces_killed - self.amount_of_forces) - self.extra_forces_paid_by_emperor,
                ),
            )
        } else {
            0
        };

        //Force revival
        Player::revive_forces(
            game,
            me,
            (self.amount_of_forces + self.extra_forces_paid_by_emperor) + high_threshold_extra_suboids,
        );
        Player::revive_special_forces(
            game,
            me,
            self.amount_of_special_forces + self.extra_special_forces_paid_by_emperor,
        );

        if self.amount_of_special_forces + self.extra_special_forces_paid_by_emperor > 0 {
            game.factions_that_revived_special_forces_this_turn.push(initiator);
        }

        //Register free revival
        let mut first_free_revival_this_turn = false;
        let nr_of_free_revivals = std::cmp::min(
            game.free_revivals(me, self.uses_emperor_secret_ally),
            self.amount_of_forces + self.amount_of_special_forces,
        );

        if nr_of_free_revivals > 0 {
            let free_revivals_this_turn = game.free_revivals_this_turn.get(&initiator).copied().unwrap_or(0);
            first_free_revival_this_turn = free_revivals_this_turn == 0;
            game.free_revivals_this_turn.insert(initiator, free_revivals_this_turn + nr_of_free_revivals);
        }

        //Tech token activated?
        if first_free_revival_this_turn && initiator != Faction::Tleilaxu {
            game.revival_tech_token_income = true;
        }

        //Tleilaxu income
        let tleilaxu = game.get_player(Some(Faction::Tleilaxu));
        let mut total_profits_for_tleilaxu: i32 = 0;
        if let Some(tleilaxu) = tleilaxu {
            if first_free_revival_this_turn && !game.tleilaxu_started_revival_with_low_threshold {
                total_profits_for_tleilaxu += 1;
            }

            if initiator != Faction::Tleilaxu {
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
                let r = &mut game.player_mut(tleilaxu).resources;
                *r += total_profits_for_tleilaxu;
            }

            if total_profits_for_tleilaxu >= 5 {
                game.apply_bureaucracy(initiator, Faction::Tleilaxu);
            }
        }

        if (cost.total_cost() - total_profits_for_tleilaxu) >= 4 {
            game.activate_banker(me);
        }

        //Hero revival
        let mut as_ghola = false;
        if let Some(hero) = self.hero() {
            let v = game.leader_revivals_this_turn.get(&initiator).copied().unwrap_or(0) + 1;
            game.leader_revivals_this_turn.insert(initiator, v);
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

            game.early_revivals_offers.remove(&hero);
        }

        //Logging
        game.stone(Milestone::Revival);
        self.register_and_log_revival(game, &cost, total_profits_for_tleilaxu, as_ghola, high_threshold_extra_suboids);

        if let Some(location) = self.location(game) {
            if self.number_of_special_forces_in_location > 0 || self.number_of_forces_in_location > 0 {
                let mut normal_force_message = express!();
                if self.number_of_forces_in_location > 0 {
                    Player::ship_forces(game, me, location, self.number_of_forces_in_location);
                    normal_force_message = express!(self.number_of_forces_in_location, game.player(me).force());
                }

                let mut special_force_message = express!();
                if self.number_of_special_forces_in_location > 0 {
                    Player::ship_special_forces(game, me, location, self.number_of_special_forces_in_location);
                    special_force_message =
                        express!(self.number_of_special_forces_in_location, game.player(me).special_force());
                }

                let to = self.location(game);
                game.log(vec![
                    initiator.into(),
                    " place ".into(),
                    normal_force_message.into(),
                    special_force_message.into(),
                    " in ".into(),
                    to.into(),
                ]);
                game.last_shipment_or_movement = Some(self.clone().into_event());
            }

            if game.check_intrusion(&self.clone().into_event())? {
                game.phase_before_revival_caused_intrusion = game.current_phase;
                let bene_gesserit = game.last_bene_gesserit_intrusion().is_some();
                let terror = game.last_terror_trigger().is_some();
                game.enter_boolean_phase_boolean_phase_phase(
                    bene_gesserit,
                    Phase::BeneGesseritIntrudedByRevival,
                    terror,
                    Phase::TerrorTriggeredByRevival,
                    Phase::AmbassadorTriggeredByRevival,
                );
            }
        }
        Ok(())
    }

    pub fn register_and_log_revival(&self, game: &mut Game, cost: &RevivalCost, tleilaxu_received_resources: i32, as_ghola: bool, high_threshold_extra_suboids: i32) {
        let initiator = self.initiator();
        let me = self.player();
        let total_amount_of_forces = self
            .amount_of_forces + self.extra_forces_paid_by_emperor + self.amount_of_special_forces + self.extra_special_forces_paid_by_emperor + high_threshold_extra_suboids;

        let v = game.total_revivals_this_turn.get(&initiator).copied().unwrap_or(0) + total_amount_of_forces;
        game.total_revivals_this_turn.insert(initiator, v);

        // Each kind of force counted once; Ixian's free extra suboids have their own part.
        let forces = self.amount_of_forces + self.extra_forces_paid_by_emperor;
        let special_forces = self.amount_of_special_forces + self.extra_special_forces_paid_by_emperor;
        let total_cost = cost.total_cost_for_player() + cost.cost_for_emperor;
        let paid_by_emperor = self.extra_forces_paid_by_emperor > 0 || self.extra_special_forces_paid_by_emperor > 0;

        let pl = game.player(me);
        let message = express!(
            initiator,
            " revive ",
            express_part_if!(self.hero().is_some(); self.hero()),
            express_part_if!(as_ghola; " as Ghola"),
            express_part_if!(self.hero().is_some() && (forces + special_forces) > 0; " and "),
            express_part_if!(forces > 0; forces, pl.force()),
            express_part_if!(special_forces > 0; special_forces, pl.special_force()),
            " for ",
            Payment::of(total_cost),
            express_part_if!(paid_by_emperor; " (", Payment::of_by(cost.cost_for_emperor, Faction::Emperor), ")"),
            express_part_if!(tleilaxu_received_resources > 0; " → ", Faction::Tleilaxu, " get ", Payment::of(tleilaxu_received_resources)),
            express_part_if!(high_threshold_extra_suboids > 0; " + ", high_threshold_extra_suboids, FactionForce::Ixian, " for free")
        );
        let mut row = RevivalRow::new(
            initiator,
            forces + high_threshold_extra_suboids,
            special_forces,
            total_cost,
        );
        row.hero = self.hero();
        row.as_ghola = as_ghola;
        row.paid_by_other = if paid_by_emperor { cost.cost_for_emperor } else { 0 };
        row.payer = if paid_by_emperor { Faction::Emperor } else { Faction::None };
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

    fn event_type(&self) -> GameEventType {
        GameEventType::Revival
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::Revival(self)
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

impl crate::events::location_event::ILocationEvent for Revival {
    fn initiator(&self) -> Faction {
        GameEventMethods::initiator(self)
    }

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
