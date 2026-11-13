//! Shipment planning.
//!
//! `determine_shipment` tries a list of planners in a fixed order and keeps the first shipment
//! that validates. Each planner returns a [`PlannedShipment`] instead of storing it, and
//! `determine_shipment` stores the winner in the bot so the move and battle decisions can read it.
//!
//! Hazards:
//! - Several planners filter or map candidates through helpers that draw random numbers
//!   (`occupying_opponent_in`, `max_potential_force_shortage`, `construct_attack`). These run as
//!   iterator chains, so each candidate passes every stage before the next one starts.
//! - `valid_shipment_locations` is lazy, and `any`/`find` on it stop early. That decides which
//!   neighbour searches run, and the map caches those.
//! - Dials are `f32`. Rounding up widens to `f64` first, and `math_min_f32`/`math_max_f32` follow
//!   the .NET NaN rules.

use crate::prelude::*;
use dotnet_compat::math::{f64_to_i32, math_max_f32, math_min_f32};

impl ClassicBot {
    /// Picks this turn's shipment (a pass if nothing suits) and remembers it, with its reason and
    /// any follow-up move, for the decisions that come later in the turn.
    pub fn determine_shipment(&mut self, game: &Game) -> Rc<Shipment> {
        self.decided_shipment_action = ShipmentDecision::None;
        self.decided_shipment = None;
        self.final_destination = None;

        let plan = self.plan_shipment(game).unwrap_or_else(|| {
            let mut pass = Shipment::new(game, self.faction(game));
            pass.passed = true;
            PlannedShipment { action: ShipmentDecision::None, shipment: pass, then_move_to: None }
        });
        self.adopt_shipment(plan)
    }

    /// Stores `plan` as the decided shipment.
    pub fn adopt_shipment(&mut self, plan: PlannedShipment) -> Rc<Shipment> {
        let shipment = Rc::new(plan.shipment);
        self.decided_shipment_action = plan.action;
        self.decided_shipment = Some(shipment.clone());
        self.final_destination = plan.then_move_to;
        shipment
    }

    /// The first planner, in order of priority, that comes up with a valid shipment.
    fn plan_shipment(&self, game: &Game) -> Option<PlannedShipment> {
        let me = self.player;
        let faction = self.faction(game);
        let p = game.player(me);

        if game.prevented_from_shipping(faction)
            || p.forces_in_reserve() + p.special_forces_in_reserve() <= 0
            || self.resources_including_ally_contribution(game) <= 0
        {
            return None;
        }

        let winning = self.i_am_winning(game);
        let ally = self.ally(game);
        let will_do_everything_to_prevent_normal_win = !game.applicable(Rule::DisableGuildSpecialVictory)
            && (faction == Faction::Guild
                || ally == Faction::Guild
                || (faction == Faction::Fremen && !game.is_playing(Faction::Guild))
                || (ally == Faction::Fremen && !game.is_playing(Faction::Guild)));

        let extra_forces = if self.last_turn(game) || (faction == Faction::BeneGesserit && game.applicable(Rule::BeneGesseritAdvisors)) {
            1
        } else {
            self.d(1, self.param.shipment_dial_for_extra_forces_to_ship)
        };
        let min_resources_to_keep =
            if game.applicable(Rule::AdvancedCombat) { self.param.shipment_minimum_resources_to_keep_for_battle } else { 0 };
        let in_great_need_of_spice = (faction == Faction::Harkonnen || faction == Faction::Atreides || faction == Faction::Richese)
            && self.resources_including_ally_contribution(game) <= 2;
        // Only a non-Emperor draws here.
        let still_needs_resources =
            faction != Faction::Emperor && self.resources_including_ally_contribution(game) < 15 + self.d(1, 20);
        let has_weapons = p.treachery_cards.iter().any(|c| c.is_weapon());
        let has_cards = !p.treachery_cards.is_empty();
        let feeling_confident = self.resources(game) >= 20 && p.forces_killed + p.special_forces_killed < 10;

        let maximum_challenged_strongholds = if self.last_turn(game) && will_do_everything_to_prevent_normal_win {
            20
        } else {
            self.param.shipment_minimum_other_players_i_trust_to_prevent_a_win - self.nr_of_non_winning_players_to_ship_and_move_including_me(game)
        };
        let shortage_to_accept = self.param.shipment_dial_shortage_to_accept;
        let max_unsupported_forces = self.param.battle_maximum_unsupported_forces;
        let attack_max_unsupported_forces = || if feeling_confident || self.last_turn(game) { 20 } else { 0 };

        if let Some(plan) = self.plan_prevent_normal_win(
            game,
            maximum_challenged_strongholds,
            extra_forces,
            shortage_to_accept,
            min_resources_to_keep,
            max_unsupported_forces,
        ) {
            return Some(plan);
        }
        if faction == Faction::Fremen
            && game.current_turn < 3
            && !winning
            && let Some(plan) = self.plan_fremen_to_spice_or_discovery(game)
        {
            return Some(plan);
        }
        if faction != Faction::Fremen
            && ally != Faction::Fremen
            && let Some(plan) = self.plan_prevent_fremen_win(game)
        {
            return Some(plan);
        }
        if winning
            && has_cards
            && let Some(plan) =
                self.plan_strengthen_weakest_stronghold(game, false, extra_forces, shortage_to_accept, !self.may_flip_to_advisors(game))
        {
            return Some(plan);
        }
        if !winning
            && !self.almost_last_turn(game)
            && in_great_need_of_spice
            && !p.is(Faction::Emperor)
            && let Some(plan) = self.plan_stronghold_near_spice(game)
        {
            return Some(plan);
        }
        if !winning
            && has_cards
            && let Some(plan) = self.plan_take_vacant_stronghold(
                game,
                self.opponents_to_ship_and_move(game).len() as i32 + extra_forces,
                min_resources_to_keep,
                max_unsupported_forces,
            )
        {
            return Some(plan);
        }
        if !winning
            && has_cards
            && let Some(plan) = self.plan_attack_empty_homeworld(game, min_resources_to_keep, max_unsupported_forces)
        {
            return Some(plan);
        }
        if faction == Faction::Ixian
            && p.any_forces_in(Some(game.map.hidden_mobile_stronghold().location())) == 0
            && let Some(plan) =
                self.plan_attack_weak_hms(game, 1, 0, if self.last_turn(game) { 99 } else { max_unsupported_forces })
        {
            return Some(plan);
        }
        if !winning
            && ((feeling_confident && has_cards) || has_weapons)
            && let Some(plan) =
                self.plan_attack_weak_stronghold(game, extra_forces, min_resources_to_keep, attack_max_unsupported_forces())
        {
            return Some(plan);
        }
        if faction != Faction::Fremen
            && !winning
            && !self.almost_last_turn(game)
            && still_needs_resources
            && let Some(plan) = self.plan_stronghold_near_spice(game)
        {
            return Some(plan);
        }
        if faction == Faction::Fremen
            && !winning
            && !self.last_turn(game)
            && still_needs_resources
            && let Some(plan) = self.plan_fremen_to_spice_or_discovery(game)
        {
            return Some(plan);
        }
        if Shipment::may_ship_with_discount(game, me)
            && !winning
            && !self.almost_last_turn(game)
            && still_needs_resources
            && let Some(plan) = self.plan_discounted_to_spice_or_discovery(game)
        {
            return Some(plan);
        }
        if let Some(plan) = self.plan_unlock_move_bonus(game, min_resources_to_keep) {
            return Some(plan);
        }
        if faction == Faction::Guild
            && !self.last_turn(game)
            && Shipment::may_ship_to_reserves(game, me)
            && let Some(plan) = self.plan_back_to_reserves(game)
        {
            return Some(plan);
        }
        if let Some(plan) = self.plan_dummy_attack(game, min_resources_to_keep) {
            return Some(plan);
        }
        if let Some(plan) =
            self.plan_strengthen_weakest_stronghold(game, true, extra_forces, shortage_to_accept, !self.may_flip_to_advisors(game))
        {
            return Some(plan);
        }
        if faction == Faction::Fremen
            && p.any_forces_in(Some(game.map.polar_sink())) <= 2
            && (self.almost_last_turn(game) || self.last_turn(game) || p.forces_in_reserve() + p.special_forces_in_reserve() * 2 >= 8)
            && let Some(plan) = self.plan_fremen_to_polar_sink(game)
        {
            return Some(plan);
        }
        if !winning && has_cards {
            return self.plan_attack_weak_stronghold(game, extra_forces, min_resources_to_keep, attack_max_unsupported_forces());
        }
        None
    }

    /// With Homeworlds, whether shipping these forces leaves too few at home.
    pub fn will_leave_my_home_defenseless(&self, game: &Game, shipped_amount_of_forces: i32, shipped_amount_of_special_forces: i32) -> bool {
        let p = game.player(self.player);
        game.applicable(Rule::Homeworlds)
            && ((self.faction(game) != Faction::Emperor
                && p.forces_in_reserve() + p.special_forces_in_reserve() - shipped_amount_of_forces - shipped_amount_of_special_forces <= 3)
                || (self.faction(game) == Faction::Emperor
                    && (p.forces_in_reserve() - shipped_amount_of_forces <= 3
                        || p.special_forces_in_reserve() - shipped_amount_of_special_forces <= 2)))
    }

    /// Whether `location` is among the places the bot may ship to.
    fn may_ship_to(&self, game: &Game, location: Location) -> bool {
        self.valid_shipment_locations(game, false, false).any(|l| l == location)
    }

    /// The filter of both queries in `plan_discounted_to_spice_or_discovery`.
    /// The order of the `&&` conditions is kept deliberately.
    fn suitable_for_guild_shipment(&self, game: &Game, l: Location) -> bool {
        self.may_ship_to(game, l)
            && self.i_dont_have_advisors_in(game, l)
            && self.total_max_dial_of_opponents(game, l.territory()) <= self.param.shipment_max_enemy_force_strength_fighting_for_spice
            && self.ally_doesnt_block(game, l.territory())
            && !self.storm_will_probably_hit(game, l)
            && self.probably_safe_from_monster(game, l.territory())
    }

    /// For the Guild and its ally: ship at the discount straight onto the richest safe spice,
    /// or else onto a discovery.
    pub fn plan_discounted_to_spice_or_discovery(&self, game: &Game) -> Option<PlannedShipment> {
        let me = self.player;
        let best_location = game
            .resources_on_planet
            .iter()
            .map(|(k, v)| (*k, *v))
            .filter(|(k, _)| self.suitable_for_guild_shipment(game, *k) && self.nearby_battalions_outside_strongholds(game, *k).is_empty())
            .highest_or_default(&self.random, |kvp| kvp.1)
            .map(|kvp| kvp.0);

        if let Some(best_location) = best_location {
            let amount = *game.resources_on_planet.index(&best_location);
            let forces_needed_for_collection = self.make_even_if_efficient_for_shipping(
                game,
                math_min_f32(amount as f32 / game.resource_collection_rate(me) as f32, 4.0)
                    + self.total_max_dial_of_opponents(game, best_location.territory()),
            );

            let size = self.shipment_size(
                game,
                forces_needed_for_collection as f32,
                false,
                best_location,
                None,
                game.player(me).forces_in_reserve(),
                0,
                0,
                99,
                false,
            );
            if size.shortage <= 0.0 {
                return self.planned(game, ShipmentDecision::AtResources, best_location, &size, None, Funding::WithAllyAndKarma);
            }
        }

        let best_location = game
            .discoveries_on_planet
            .iter()
            .map(|(k, _)| *k)
            .find(|k| self.suitable_for_guild_shipment(game, *k) && self.nearby_battalions_outside_strongholds(game, *k).is_empty())?;

        let forces_needed =
            self.make_even_if_efficient_for_shipping(game, 1.0 + self.total_max_dial_of_opponents(game, best_location.territory()));
        let size = self.shipment_size(game, forces_needed as f32, false, best_location, None, game.player(me).forces_in_reserve(), 0, 0, 99, false);
        if size.shortage <= 0.0 {
            self.planned(game, ShipmentDecision::AtResources, best_location, &size, None, Funding::WithAllyAndKarma)
        } else {
            None
        }
    }

    /// For the Guild: take a stranded battalion back to reserves.
    pub fn plan_back_to_reserves(&self, game: &Game) -> Option<PlannedShipment> {
        let b = self.biggest_battalion_in_spiceless_non_stronghold_location_not_near_stronghold_and_spice(game)?;
        let size = ShipmentSize::of(-b.battalion.amount_of_forces, -b.battalion.amount_of_special_forces);
        self.planned(game, ShipmentDecision::BackToReserves, b.location, &size, None, Funding::WithAlly)
    }

    pub fn plan_attack_weak_hms(&self, game: &Game, extra_forces: i32, min_resources_to_keep: i32, max_unsupported_forces: i32) -> Option<PlannedShipment> {
        let hms = game.map.hidden_mobile_stronghold().location();
        let opponent = self.get_opponent_that_occupies(game, hms.territory());

        if opponent.is_some() && self.may_ship_to(game, hms) && self.i_dont_have_advisors_in(game, hms) {
            let attack = self.construct_attack(game, hms, extra_forces, min_resources_to_keep, max_unsupported_forces);
            if attack.has_forces() {
                return self.plan_attack(game, ShipmentDecision::AttackWeakStronghold, &attack);
            }
        }
        None
    }

    pub fn plan_fremen_to_polar_sink(&self, game: &Game) -> Option<PlannedShipment> {
        let p = game.player(self.player);
        let size = if self.last_turn(game) {
            ShipmentSize::of(p.forces_in_reserve(), p.special_forces_in_reserve())
        } else {
            let special_forces = std::cmp::min(1, p.special_forces_in_reserve());
            let forces = std::cmp::min(p.forces_in_reserve(), 8 - std::cmp::min(8, 2 * special_forces));
            ShipmentSize::of(forces, special_forces)
        };

        self.planned(game, ShipmentDecision::PolarSink, game.map.polar_sink(), &size, None, Funding::Own)
    }

    /// For Fremen: ship onto the richest uncontested spice, or else onto a discovery.
    pub fn plan_fremen_to_spice_or_discovery(&self, game: &Game) -> Option<PlannedShipment> {
        let me = self.player;
        let suitable = |l: Location| {
            self.may_ship_to(game, l)
                && game.player(me).any_forces_in(Some(l)) == 0
                && self.total_max_dial_of_opponents(game, l.territory()) <= self.param.shipment_max_enemy_force_strength_fighting_for_spice
                && self.ally_doesnt_block(game, l.territory())
                && self.nearby_battalions_outside_strongholds(game, l).is_empty()
        };

        let best_location = game
            .resources_on_planet
            .iter()
            .map(|(k, v)| (*k, *v))
            .filter(|(k, _)| suitable(*k))
            .highest_or_default(&self.random, |kvp| kvp.1)
            .map(|kvp| kvp.0);

        if let Some(best_location) = best_location {
            let amount = *game.resources_on_planet.index(&best_location);
            let forces_to_rally = math_max_f32(
                7.0,
                2.0 + self.total_max_dial_of_opponents(game, best_location.territory()) + amount as f32 / game.resource_collection_rate(me) as f32,
            );

            let size = self.shipment_size_from_reserves(game, forces_to_rally, false, best_location, None, 0, 99, false);
            if size.shortage <= 3.0 {
                return self.planned(game, ShipmentDecision::AtResources, best_location, &size, None, Funding::Own);
            }
        }

        let best_location = game.discoveries_on_planet.iter().map(|(k, _)| *k).find(|k| suitable(*k))?;
        let forces_to_rally = math_max_f32(5.0, 2.0 + self.total_max_dial_of_opponents(game, best_location.territory()));
        let size = self.shipment_size_from_reserves(game, forces_to_rally, false, best_location, None, 0, 99, false);
        if size.shortage <= 3.0 {
            self.planned(game, ShipmentDecision::AtResources, best_location, &size, None, Funding::Own)
        } else {
            None
        }
    }

    /// Ship to a stronghold within reach of rich spice, planning to move on to the spice afterwards.
    pub fn plan_stronghold_near_spice(&self, game: &Game) -> Option<PlannedShipment> {
        if self.battalion_that_should_be_moved_due_to_ally_presence(game).is_some() {
            return None;
        }

        let richest_location = self.richest_location_near_shippable_stronghold_far_from_existing_forces(game)?;

        let me = self.player;
        let faction = self.faction(game);
        let location_to_ship_to = self
            .valid_shipment_locations(game, false, false)
            .find(|&sh| {
                (!game.player(me).has_ally()
                    || self.ally(game) == Faction::Ecaz
                    || game.player(self.allied_player(game).expect("allied player")).any_forces_in(Some(sh)) == 0
                    || faction != Faction::Richese
                    || !game.has_low_threshold(Faction::Richese))
                    && !self.in_storm(game, sh)
                    && game.player(me).any_forces_in(Some(sh)) <= 8
                    && sh.is_stronghold()
                    && self.within_range(game, sh, richest_location, &self.sample_battalion(game, sh))
            })
            .filter(|&sh| game.player(me).any_forces_in(Some(sh)) < 10)?;

        let forces_needed_for_collection =
            self.make_even_if_efficient_for_shipping(game, std::cmp::min(6, self.determine_forces_needed_for_collection(game, richest_location)) as f32);
        let opponent = Self::occupying_opponents_in(game, me, richest_location.territory()).into_iter().next();
        let size = self.shipment_size_from_reserves(
            game,
            forces_needed_for_collection as f32 + self.total_max_dial_of_opponents(game, richest_location.territory()),
            false,
            location_to_ship_to,
            opponent,
            0,
            99,
            faction == Faction::Ixian,
        );

        self.planned(
            game,
            ShipmentDecision::StrongholdNearResources,
            location_to_ship_to,
            &size,
            Some(richest_location),
            Funding::WithAllyAndKarma,
        )
    }

    /// Reinforce the stronghold I hold that is most short of strength against possible attackers.
    pub fn plan_strengthen_weakest_stronghold(&self, game: &Game, only_if_threatened: bool, extra_forces: i32, shortage_to_accept: f32, take_reinforcements_into_account: bool) -> Option<PlannedShipment> {
        let me = self.player;
        // `occupying_opponent_in` in the filter and `max_potential_force_shortage` in the map both
        // draw, element by element.
        let my_weak_strongholds: Vec<(Location, f32)> = self
            .valid_shipment_locations(game, false, false)
            .filter(|&s| {
                s.is_stronghold()
                    && self.i_dont_have_advisors_in(game, s)
                    && game.player(me).occupying_forces_in_location(s) > 0
                    && self.ally_doesnt_block(game, s.territory())
                    && (!only_if_threatened || self.occupying_opponent_in(game, me, s.territory()).is_some())
                    && !self.in_storm(game, s)
            })
            .map(|s| (s, self.max_potential_force_shortage(game, take_reinforcements_into_account, s)))
            .collect();

        let (location, difference) = my_weak_strongholds.into_iter().filter(|s| s.1 > 0.0).highest_or_default(&self.random, |s| s.1)?;

        let opponent = self.opponents(game).into_iter().find(|o| game.player(*o).any_forces_in(Some(location)) > 0);
        let dial_needed = self.make_even_if_efficient_for_shipping(game, difference + extra_forces as f32);
        let size = self.shipment_size_from_reserves(game, dial_needed as f32, true, location, opponent, 0, 0, true);
        if size.shortage <= shortage_to_accept {
            self.planned(game, ShipmentDecision::StrengthenWeakStronghold, location, &size, None, Funding::WithAllyAndKarma)
        } else {
            None
        }
    }

    /// Ship a token force into a stronghold whose occupant I can lure with a traitor, or (when I
    /// have a Useless card to get rid of) whose occupant is a safe opponent for that.
    pub fn plan_dummy_attack(&self, game: &Game, min_resources_to_keep: i32) -> Option<PlannedShipment> {
        let me = self.player;
        let target_where = |suitable_opponent: &dyn Fn(Option<PlayerIndex>) -> bool| {
            self.valid_shipment_locations(game, false, false).find(|&l| {
                game.player(me).any_forces_in_territory(Some(l.territory())) == 0
                    && self.ally_doesnt_block(game, l.territory())
                    && !self.in_storm(game, l)
                    && l.territory().is_stronghold()
                    && suitable_opponent(self.occupying_opponent_in(game, me, l.territory()))
            })
        };

        let mut target = target_where(&|o| self.opponent_is_suitable_for_traitor_lure(game, o));
        if target.is_none()
            && !self.may_use_useless_as_karma(game)
            && game.player(me).treachery_cards.iter().any(|c| c.type_() == TreacheryCardType::Useless)
            && game.player(me).tech_tokens.is_empty()
        {
            target = target_where(&|o| self.opponent_is_suitable_for_useless_card_dump_attack(game, o));
        }
        let target = target?;

        if self.faction(game) == Faction::Richese && Shipment::valid_no_field_values(game, me).contains(&0) {
            return self.plan_zero_no_field_shipment(game, ShipmentDecision::DummyShipment, target);
        }

        let opponent = self.occupying_opponent_in(game, me, target.territory());
        let size = self.shipment_size_from_reserves(game, 0.5, true, target, opponent, min_resources_to_keep, 1, false);
        if size.shortage <= 0.0 {
            self.planned(game, ShipmentDecision::DummyShipment, target, &size, None, Funding::WithAlly)
        } else {
            None
        }
    }

    /// With forces in the Polar Sink and no ornithopters, ship into Arrakeen or Carthag so those
    /// forces can move further.
    pub fn plan_unlock_move_bonus(&self, game: &Game, min_resources_to_keep: i32) -> Option<PlannedShipment> {
        let me = self.player;
        if game.has_ornithopters(me) || game.player(me).any_forces_in(Some(game.map.polar_sink())) <= 0 {
            return None;
        }

        let target = self
            .valid_shipment_locations(game, false, false)
            .filter(|l| self.i_dont_have_advisors_in(game, *l))
            .filter(|l| {
                (*l == game.map.arrakeen() && self.ally_doesnt_block(game, game.map.arrakeen()))
                    || (*l == game.map.carthag() && self.ally_doesnt_block(game, game.map.carthag()))
            })
            .lowest_or_default(&self.random, |l| self.total_max_dial_of_opponents(game, l.territory()))?;

        let opponent = self.occupying_opponent_in(game, me, target.territory());
        let needed = match opponent {
            Some(opponent) => {
                let take = self.opponents_to_ship_and_move(game).contains(&opponent);
                math_max_f32(
                    0.5,
                    self.max_potential_force_shortage(game, take, target) - game.player(me).any_forces_in(Some(game.map.polar_sink())) as f32,
                )
            }
            None => 0.5,
        };

        let size = self.shipment_size_from_reserves(game, needed, true, target, opponent, min_resources_to_keep, 1, false);
        if size.shortage <= 0.0 {
            self.planned(game, ShipmentDecision::DummyShipment, target, &size, None, Funding::WithAlly)
        } else {
            None
        }
    }

    pub fn plan_take_vacant_stronghold(&self, game: &Game, force_strength: i32, min_resources_to_keep: i32, max_unsupported_forces: i32) -> Option<PlannedShipment> {
        let me = self.player;
        let map = &game.map;

        let preferred = if self.faction(game) == Faction::Ixian && self.vacant_and_valid(game, map.hidden_mobile_stronghold().location()) {
            Some(map.hidden_mobile_stronghold().location())
        } else if game.player(me).any_forces_in(Some(map.carthag())) < 8 && self.vacant_and_valid(game, map.arrakeen()) {
            Some(map.arrakeen())
        } else if game.player(me).any_forces_in(Some(map.arrakeen())) < 8 && self.vacant_and_valid(game, map.carthag()) {
            Some(map.carthag())
        } else if self.vacant_and_valid(game, map.tueks_sietch()) {
            Some(map.tueks_sietch())
        } else if self.vacant_and_valid(game, map.habbanya_sietch()) {
            Some(map.habbanya_sietch())
        } else if self.vacant_and_valid(game, map.carthag()) {
            Some(map.carthag())
        } else if self.vacant_and_valid(game, map.arrakeen()) {
            Some(map.arrakeen())
        } else if self.vacant_and_valid(game, map.sietch_tabr()) && (self.last_turn(game) || !game.is_in_storm(map.sietch_tabr())) {
            Some(map.sietch_tabr())
        } else if game.is_special_stronghold(map.shield_wall()) {
            let shield_wall = map.shield_wall().locations();
            [shield_wall.first(), shield_wall.last()].into_iter().flatten().copied().find(|l| self.vacant_and_valid(game, *l))
        } else {
            None
        };

        let target = preferred.or_else(|| {
            self.valid_shipment_locations(game, false, false)
                .filter(|l| l.is_stronghold())
                .find(|l| self.vacant_and_valid(game, *l))
        })?;

        let dial_needed = self.make_even_if_efficient_for_shipping(game, force_strength as f32);
        let size = self.shipment_size_from_reserves(game, dial_needed as f32, false, target, None, min_resources_to_keep, max_unsupported_forces, true);
        if size.shortage <= 2.0 {
            self.planned(game, ShipmentDecision::VacantStronghold, target, &size, None, Funding::WithAllyAndKarma)
        } else {
            None
        }
    }

    pub fn plan_attack_empty_homeworld(&self, game: &Game, min_resources_to_keep: i32, max_unsupported_forces: i32) -> Option<PlannedShipment> {
        let target = self
            .valid_shipment_locations(game, false, false)
            .filter(|l| l.is_homeworld() && !game.any_forces_in(l.territory()))
            .random_or_default(&self.random)?;

        let size = self.shipment_size_from_reserves(game, 1.0, false, target, None, min_resources_to_keep, max_unsupported_forces, true);
        if size.shortage <= 0.0 {
            self.planned(game, ShipmentDecision::Homeworld, target, &size, None, Funding::WithAllyAndKarma)
        } else {
            None
        }
    }

    /// A stronghold that is vacant, safe from the storm, and a place I may ship to.
    pub fn vacant_and_valid(&self, game: &Game, location: Location) -> bool {
        self.vacant_and_safe_from_storm(game, location)
            && self.valid_shipment_locations(game, false, false).filter(|l| l.is_stronghold()).any(|l| l == location)
    }

    /// When the Fremen are about to win, ship everything into one of their sietches.
    pub fn plan_prevent_fremen_win(&self, game: &Game) -> Option<PlannedShipment> {
        if !(game.is_playing(Faction::Fremen) && game.fremen_victory_condition_met()) {
            return None;
        }

        let valid_unblocked = |target: Location| {
            self.valid_shipment_locations(game, false, false)
                .filter(|l| self.ally_doesnt_block(game, l.territory()) && self.i_dont_have_advisors_in(game, *l))
                .any(|l| l == target)
        };
        let target = [game.map.habbanya_sietch(), game.map.sietch_tabr()].into_iter().find(|t| valid_unblocked(*t))?;

        let size = self.shipment_size_from_reserves(game, 99.0, true, target, None, 0, 99, true);
        self.planned(game, ShipmentDecision::PreventFremenWin, target, &size, None, Funding::WithAllyAndKarma)
    }

    /// Attack the cheapest stronghold held by an opponent who is about to win.
    pub fn plan_prevent_normal_win(&self, game: &Game, maximum_challenged_strongholds: i32, extra_forces: i32, risk_appetite: f32, min_resources_to_keep: i32, max_unsupported_forces: i32) -> Option<PlannedShipment> {
        let mut potential_winning_opponents = self.winning_opponents_i_wish_to_attack(game, maximum_challenged_strongholds, false);
        if potential_winning_opponents.is_empty() {
            potential_winning_opponents = self.winning_opponents_i_wish_to_attack(game, maximum_challenged_strongholds, true);
        }

        // `construct_attack` draws, element by element.
        let attacks: Vec<Attack> = self
            .valid_shipment_locations(game, false, false)
            .filter(|&l| {
                (l.territory().is_stronghold() || game.is_special_stronghold(l.territory()))
                    && !self.in_storm(game, l)
                    && self.ally_doesnt_block(game, l.territory())
                    && potential_winning_opponents.iter().any(|p| game.player(*p).occupies(game, Some(l)))
                    && self.i_dont_have_advisors_in(game, l)
            })
            .map(|s| self.construct_attack(game, s, extra_forces, min_resources_to_keep, max_unsupported_forces))
            .collect();

        let attack = attacks
            .into_iter()
            .filter(|a| a.has_forces() && a.shortage_for_shipment <= risk_appetite)
            .lowest_or_default(&self.random, |a| a.dial_needed)?;

        self.plan_attack(game, ShipmentDecision::PreventNormalWin, &attack)
    }

    /// Attack the stronghold I can take most cheaply, preferring those of almost-winning opponents.
    pub fn plan_attack_weak_stronghold(&self, game: &Game, extra_forces: i32, min_resources_to_keep: i32, max_unsupported_forces: i32) -> Option<PlannedShipment> {
        let dangerous_opponents: Vec<PlayerIndex> =
            self.opponents(game).into_iter().filter(|o| self.is_almost_winning_opponent(game, *o)).collect();

        let me = self.player;
        // One chain, consumed once by `lowest_or_default`.
        let attack = self
            .valid_shipment_locations(game, false, false)
            .filter(|l| dangerous_opponents.is_empty() || dangerous_opponents.iter().any(|p| game.player(*p).occupies(game, Some(*l))))
            .filter(|&l| {
                l.territory().is_stronghold()
                    && game.player(me).any_forces_in(Some(l)) == 0
                    && self.ally_doesnt_block(game, l.territory())
                    && !self.storm_will_probably_hit(game, l)
                    && !self.in_storm(game, l)
                    && self.i_dont_have_advisors_in(game, l)
            })
            .map(|l| self.construct_attack(game, l, extra_forces, min_resources_to_keep, max_unsupported_forces))
            .filter(|s| match s.opponent {
                Some(o) => !self.win_was_predicted_by_me_this_turn(game, game.player(o).faction),
                None => false,
            })
            .filter(|s| s.has_forces() && s.shortage_for_shipment - self.unlocked_forces_in_polar_sink(game, s.location) as f32 <= extra_forces as f32)
            .lowest_or_default(&self.random, |s| s.dial_needed + Self::determine_penalty(game, s.opponent))?;

        self.plan_attack(game, ShipmentDecision::AttackWeakStronghold, &attack)
    }

    /// Rounds a dial up to whole forces, at least one, and to an even number when shipping at the
    /// discount (where two forces cost the same as one).
    pub fn make_even_if_efficient_for_shipping(&self, game: &Game, dial_needed: f32) -> i32 {
        let mut rounded_dial_needed = f64_to_i32((dial_needed as f64).ceil());
        if rounded_dial_needed <= 0 {
            rounded_dial_needed = 1;
        }

        if Shipment::may_ship_with_discount(game, self.player) && rounded_dial_needed % 2 != 0 {
            return rounded_dial_needed + 1;
        }

        rounded_dial_needed
    }

    /// `shipment_size` with everything in my reserves available.
    pub fn shipment_size_from_reserves(&self, game: &Game, dial_needed: f32, dialling_for_battle: bool, location: Location, opponent_to_check: Option<PlayerIndex>, min_resources_to_keep: i32, max_unsupported_forces: i32, prefer_special_forces: bool) -> ShipmentSize {
        let p = game.player(self.player);
        self.shipment_size(
            game,
            dial_needed,
            dialling_for_battle,
            location,
            opponent_to_check,
            p.forces_in_reserve(),
            p.special_forces_in_reserve(),
            min_resources_to_keep,
            max_unsupported_forces,
            prefer_special_forces,
        )
    }

    /// How many forces to ship to `location`, and which No-Field to use, to bring `dial_needed`
    /// worth of strength there with the spice I can spend. The result's `shortage` is the dial
    /// still missing.
    pub fn shipment_size(&self, game: &Game, dial_needed: f32, dialling_for_battle: bool, location: Location, opponent_to_check: Option<PlayerIndex>, forces_available: i32, special_forces_available: i32, min_resources_to_keep: i32, max_unsupported_forces: i32, prefer_special_forces: bool) -> ShipmentSize {
        let me = self.player;
        let opponent = opponent_to_check
            .or_else(|| self.opponents(game).into_iter().find(|o| game.player(*o).any_forces_in_territory(Some(location.territory())) > 0));

        let faction = self.faction(game);
        let must_pay_for_forces = Battle::must_pay_for_any_forces_in_battle(game, me);
        let sizing = ForceSizing {
            game,
            me,
            location,
            dial_needed,
            max_unsupported_forces,
            prefer_special_forces,
            normal_strength: Battle::determine_normal_force_strength(game, faction),
            special_strength: Battle::determine_special_force_strength(
                game,
                faction,
                opponent.map(|o| game.player(o).faction).unwrap_or(Faction::Harkonnen),
            ),
            spice_available: self.resources_including_ally_contribution(game) - min_resources_to_keep,
            no_spice_for_force_modifier: if must_pay_for_forces { 0.5 } else { 1.0 },
            cost_per_force_in_battle: if dialling_for_battle && must_pay_for_forces { 1 } else { 0 },
        };
        let no_field_values = Shipment::valid_no_field_values(game, me);
        let max_forces_with_no_field = no_field_values.iter().copied().max().unwrap_or(-1);

        let size = sizing.fill(forces_available, special_forces_available);

        if max_forces_with_no_field < 0
            || Shipment::determine_cost_game_player_int32_int32_location_boolean_boolean_boolean_boolean_boolean(game, me, 2, 0, location, false, false, false, false, false) == 0
            || sizing.spice_available <= 0
        {
            return size;
        }

        // Try again with at most as many forces as the biggest No-Field covers.
        let (forces_no_field, special_forces_no_field) = if prefer_special_forces {
            let special_forces = std::cmp::min(special_forces_available, max_forces_with_no_field);
            (std::cmp::min(forces_available, max_forces_with_no_field - special_forces), special_forces)
        } else {
            let forces = std::cmp::min(forces_available, max_forces_with_no_field);
            (forces, std::cmp::min(special_forces_available, max_forces_with_no_field - forces))
        };
        let with_no_field = sizing.fill(forces_no_field, special_forces_no_field);

        if with_no_field.shortage <= size.shortage {
            let needed = with_no_field.forces + with_no_field.special_forces;
            let mut big_enough_no_fields: Vec<i32> = no_field_values.iter().copied().filter(|v| *v >= needed).collect();
            let mut cunning_no_field_value = -1;
            if Shipment::may_use_cunning_no_field(game, me) && !no_field_values.is_empty() && big_enough_no_fields.is_empty()
                && let Some(biggest_cunning_no_field) = Shipment::valid_cunning_no_field_values(game, me, max_forces_with_no_field).into_iter().max()
            {
                big_enough_no_fields = no_field_values.iter().copied().filter(|v| v + biggest_cunning_no_field >= needed).collect();
                cunning_no_field_value = if !big_enough_no_fields.is_empty() { biggest_cunning_no_field } else { -1 };
            }

            // The smallest No-Field that covers the forces, else the biggest one there is.
            let no_field_value = big_enough_no_fields.iter().copied().min().unwrap_or(max_forces_with_no_field);
            let special_forces_shipped = std::cmp::min(special_forces_no_field, no_field_value);
            let forces_shipped = std::cmp::min(forces_no_field, no_field_value - special_forces_shipped);
            return ShipmentSize {
                forces: if faction == Faction::Richese { 0 } else { forces_shipped },
                special_forces: if faction == Faction::Richese { 1 } else { special_forces_shipped },
                no_field_value,
                cunning_no_field_value,
                shortage: with_no_field.shortage,
            };
        }

        size
    }

    /// One force of mine that can move from `from` to `to`, if `from` is mine alone and out of the storm.
    pub fn find_one_troop_that_can_safely_move(&self, game: &Game, from: Location, to: Location) -> Option<Battalion> {
        let me = self.player;
        if game.player(me).forces_in_locations.contains_key(&from)
            && from.sector(&game.map) != game.sector_in_storm
            && self.not_occupied_by_others(game, from.territory())
        {
            let bat = game.player(me).forces_in_locations.index(&from).clone();
            if self.not_occupied_by_others(game, from.territory()) && bat.total_amount_of_forces() > 1 {
                let one_of_them = bat.take(1, true);
                if PlacementEvent::valid_targets(game, me, from, &one_of_them).contains(&to) {
                    return Some(one_of_them);
                }
            }
        }

        None
    }

    /// Ships an attack, using a Karama card and my ally's spice where that helps.
    fn plan_attack(&self, game: &Game, action: ShipmentDecision, attack: &Attack) -> Option<PlannedShipment> {
        self.planned(game, action, attack.location, &attack.size(), None, Funding::WithAllyAndKarma)
    }

    /// Builds the shipment and returns it as a plan if it leaves my homeworld defended and is valid.
    fn planned(&self, game: &Game, action: ShipmentDecision, to: Location, size: &ShipmentSize, then_move_to: Option<Location>, funding: Funding) -> Option<PlannedShipment> {
        if self.will_leave_my_home_defenseless(game, size.forces, size.special_forces) {
            return None;
        }

        let shipment = self.construct_shipment(game, size, to, funding);
        shipment.is_valid(game).then_some(PlannedShipment { action, shipment, then_move_to })
    }

    /// Returns the event unshared, so the funding can still be set on it.
    fn construct_shipment(&self, game: &Game, size: &ShipmentSize, location: Location, funding: Funding) -> Shipment {
        let me = self.player;
        let mut nr_of_smuggled_forces = 0;
        let mut nr_of_smuggled_special_forces = 0;
        if game.skilled_as_player_leader_skill(me, LeaderSkill::Smuggler) {
            if size.special_forces > 0 && self.faction(game) != Faction::Richese {
                nr_of_smuggled_special_forces = 1;
            } else if size.forces > 0 {
                nr_of_smuggled_forces = 1;
            }
        }

        let mut result = Shipment::new(game, self.faction(game));
        result.force_amount = size.forces - nr_of_smuggled_forces;
        result.special_force_amount = size.special_forces - nr_of_smuggled_special_forces;
        result.smuggled_amount = nr_of_smuggled_forces;
        result.smuggled_special_amount = nr_of_smuggled_special_forces;
        result.set_no_field_value(size.no_field_value);
        result.set_cunning_no_field_value(size.cunning_no_field_value);
        result.passed = false;
        // A negative size takes forces off the planet (`plan_back_to_reserves`). It has to say so, as
        // the shipment form does: a normal shipment of negative amounts is refused. C# sent it as
        // normal, so its bots never shipped back.
        result.shipment_type = if size.forces + size.special_forces < 0 { ShipmentType::ShipmentBack } else { ShipmentType::ShipmentNormal };
        result.set_from(game, None);
        result.set_karma_card(None);
        result.set_to(game, Some(location));

        if funding == Funding::WithAllyAndKarma && size.no_field_value < 0 {
            self.use_karma_if_applicable(game, &mut result);
        }
        if funding != Funding::Own {
            self.use_ally_resources(game, &mut result);
        }
        result
    }

    /// A one-force battalion of mine as it would arrive in `l`, for range checks.
    pub fn sample_battalion(&self, game: &Game, l: Location) -> Battalion {
        let faction = self.faction(game);
        if faction == Faction::BeneGesserit && game.player(self.player).special_forces_in_territory(Some(l.territory())) > 0 {
            //Forces shipped by BG into a territory with advisors must become advisors
            return Battalion::new(faction, 0, 1, l);
        }
        Battalion::new(faction, 1, 1, l)
    }

    /// Where I may ship, minus territories a deal forbids me to enter. Lazy, and the deal list is
    /// searched again for every location, stopping at the first match.
    pub fn valid_shipment_locations<'a>(&self, game: &'a Game, from_planet: bool, secret_ally: bool) -> impl Iterator<Item = Location> + 'a {
        let faction = self.faction(game);
        Shipment::valid_shipment_locations(game, self.player, from_planet, secret_ally).filter(move |l| {
            let t = Some(l.territory());
            !game
                .deals
                .iter()
                .filter(|deal| deal.bound_faction == faction && deal.type_ == DealType::DontShipOrMoveTo)
                .map(|deal| deal.get_parameter1(game))
                .any(|f| f == t)
        })
    }

    pub fn force_locations_outside_strongholds(&self, game: &Game) -> Vec<(Location, Battalion)> {
        self.forces_on_planet(game).iter().filter(|f| !f.0.is_stronghold()).map(|(k, v)| (*k, v.clone())).collect()
    }

    /// The richest spice that none of my forces on the planet can reach, but that forces shipped
    /// into a stronghold could.
    pub fn richest_location_near_shippable_stronghold_far_from_existing_forces(&self, game: &Game) -> Option<Location> {
        let me = self.player;
        game.resources_on_planet
            .iter()
            .map(|(k, v)| (*k, *v))
            .filter(|&(k, _)| {
                !self.force_locations_outside_strongholds(game).iter().any(|(lk, lv)| self.within_range(game, *lk, k, lv))
                    && self
                        .valid_shipment_locations(game, false, false)
                        .any(|sh| sh.is_stronghold() && self.within_range(game, sh, k, &self.sample_battalion(game, sh)))
                    && game.player(me).any_forces_in(Some(k)) == 0
                    && self.ally_doesnt_block(game, k.territory())
                    && self.total_max_dial_of_opponents(game, k.territory()) <= self.param.shipment_max_enemy_force_strength_fighting_for_spice
                    && !self.storm_will_probably_hit(game, k)
                    && self.probably_safe_from_monster(game, k.territory())
            })
            .highest_or_default(&self.random, |kvp| kvp.1)
            .map(|kvp| kvp.0)
    }

    /// How many forces it takes to collect all the spice in `location` in one turn.
    pub fn determine_forces_needed_for_collection(&self, game: &Game, location: Location) -> i32 {
        if let Some(value) = game.resources_on_planet.get(&location) {
            // Divided in f32, then widened to f64 to round up.
            return f64_to_i32(((*value as f32 / game.resource_collection_rate(self.player) as f32) as f64).ceil());
        }

        0
    }

    pub fn use_karma_if_applicable(&self, game: &Game, shipment: &mut Shipment) {
        let me = self.player;
        if game.player(me).has_karma(game)
            && !game.karma_prevented(self.faction(game))
            && (!self.param.karma_save_card_to_use_special_karma_ability || game.player(me).special_karma_power_used)
            && !Shipment::may_ship_with_discount(game, me)
            && Shipment::determine_cost(game, me, shipment) > 7
        {
            let card = Karma::valid_karma_cards(game, me).next();
            shipment.set_karma_card(card);
        }
    }

    pub fn use_ally_resources(&self, game: &Game, shipment: &mut Shipment) {
        shipment.ally_contribution_amount =
            std::cmp::min(shipment.determine_cost_to_initiator(game), game.resources_your_ally_can_pay(self.player));
    }

    /// Sizes an attack on whoever occupies `location`. Without an occupant the attack is empty.
    pub fn construct_attack(&self, game: &Game, location: Location, extra_forces: i32, min_resources_to_keep: i32, max_unsupported_forces: i32) -> Attack {
        let me = self.player;
        let opponent = self.occupying_opponent_in(game, me, location.territory());

        let (dial_needed, size) = match opponent {
            Some(opponent) => {
                let dial_needed = self.dial_needed_against(game, location.territory(), Some(opponent), true);
                let size = self.shipment_size_from_reserves(
                    game,
                    math_max_f32(dial_needed, 0.5) + extra_forces as f32,
                    true,
                    location,
                    Some(opponent),
                    min_resources_to_keep,
                    max_unsupported_forces,
                    !self.emperor_versus_fremen(game, opponent),
                );
                (dial_needed, size)
            }
            None => (0.0, ShipmentSize::of(0, 0)),
        };

        Attack {
            location,
            opponent,
            dial_needed,
            forces_to_ship: size.forces,
            special_forces_to_ship: size.special_forces,
            no_field_value: size.no_field_value,
            cunning_no_field_value: size.cunning_no_field_value,
            shortage_for_shipment: size.shortage,
        }
    }

    /// How much more strength the strongest opponent could bring to `s` (counting reinforcements
    /// if asked) than I have there.
    pub fn max_potential_force_shortage(&self, game: &Game, take_reinforcements_into_account: bool, s: Location) -> f32 {
        let me = self.player;
        let mut opponents = Self::occupying_opponents_in(game, me, s.territory());

        let mut max_dial_of_opponents = 0.0f32;
        if !opponents.is_empty() {
            max_dial_of_opponents = self.total_max_dial_of_opponents(game, s.territory());
        }

        let mut max_reinforced_dial = 0.0f32;
        if take_reinforcements_into_account {
            if opponents.is_empty() {
                opponents = self.opponents_to_ship_and_move(game);
            }

            let most_dangerous_opponent = opponents
                .iter()
                .copied()
                .highest_or_default(&self.random, |opp| Self::max_reinforced_dial_to(game, me, *opp, s.territory()));

            if let Some(most_dangerous_opponent) = most_dangerous_opponent {
                max_reinforced_dial = Self::max_reinforced_dial_to(game, me, most_dangerous_opponent, s.territory());
            }
        }

        max_dial_of_opponents + max_reinforced_dial - Self::max_dial(game, me, s.territory(), opponents.first().copied(), false)
    }

    pub fn emperor_versus_fremen(&self, game: &Game, opponent: PlayerIndex) -> bool {
        self.faction(game) == Faction::Emperor && game.player(opponent).faction == Faction::Fremen
    }

    /// My forces in the Polar Sink that a shipment into Arrakeen or Carthag would let move further.
    pub fn unlocked_forces_in_polar_sink(&self, game: &Game, location: Location) -> i32 {
        if location == game.map.arrakeen() || location == game.map.carthag() {
            game.player(self.player).any_forces_in(Some(game.map.polar_sink()))
        } else {
            0
        }
    }

    /// The extra cost the bot assigns to attacking a bot that has no human ally.
    pub fn determine_penalty(game: &Game, opponent: Option<PlayerIndex>) -> f32 {
        match opponent {
            Some(o)
                if game.player(o).is_bot(game)
                    // Throws when the ally is not in the game.
                    && (!game.player(o).has_ally() || game.player(game.player(o).allied_player(game).expect("allied player")).is_bot(game)) =>
            {
                BotParameters::PENALTY_FOR_ATTACKING_BOTS
            }
            _ => 0.0,
        }
    }

    /// A Richese shipment of one force under a zero No-Field.
    fn plan_zero_no_field_shipment(&self, game: &Game, action: ShipmentDecision, location: Location) -> Option<PlannedShipment> {
        let mut shipment = Shipment::new(game, self.faction(game));
        shipment.force_amount = 0;
        shipment.special_force_amount = 1;
        shipment.passed = false;
        shipment.set_from(game, None);
        shipment.set_karma_card(None);
        shipment.shipment_type = ShipmentType::ShipmentNormal;
        shipment.set_no_field_value(0);
        shipment.set_to(game, Some(location));

        shipment.is_valid(game).then_some(PlannedShipment { action, shipment, then_move_to: None })
    }

    /// An opponent with a living leader who is my traitor or face dancer, and no weapon I know of
    /// (unless I hold a Mercenary).
    pub fn opponent_is_suitable_for_traitor_lure(&self, game: &Game, opponent: Option<PlayerIndex>) -> bool {
        let Some(opponent) = opponent else { return false };

        let me = game.player(self.player);
        game.player(opponent).leaders.iter().any(|l| {
            let h = Hero::Leader(*l);
            game.is_alive(h) && (me.traitors.contains(&h) || me.face_dancers.contains(&h))
        }) && (Self::known_opponent_weapons(game, self.player, opponent).is_empty() || me.has_treachery_card_type(TreacheryCardType::Mercenary))
    }

    /// An opponent I can lose a battle to without losing a leader, so I can discard a Useless card.
    pub fn opponent_is_suitable_for_useless_card_dump_attack(&self, game: &Game, opponent: Option<PlayerIndex>) -> bool {
        let Some(opponent) = opponent else { return false };

        !(game.applicable(Rule::HarkonnenCapturesOrKillsLeaders) && game.player(opponent).faction == Faction::Harkonnen)
            && (Self::known_opponent_weapons(game, self.player, opponent).is_empty()
                || game.player(self.player).has_treachery_card_type(TreacheryCardType::Mercenary))
    }
}

/// Why the bot chose its shipment. The move and battle decisions later in the turn read it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShipmentDecision {
    None,
    PreventNormalWin,
    PreventFremenWin,
    VacantStronghold,
    StrengthenWeakStronghold,
    AttackWeakStronghold,
    StrongholdNearResources,
    AtResources,
    BackToReserves,
    PolarSink,
    DummyShipment,
    Homeworld,
}

/// A valid shipment a planner came up with.
pub struct PlannedShipment {
    pub action: ShipmentDecision,
    pub shipment: Shipment,
    /// Where to move the shipped forces afterwards, if the plan includes a move.
    pub then_move_to: Option<Location>,
}

/// How many forces a shipment carries, the No-Field values it uses (-1 for none), and how much
/// dial is still missing after shipping them.
#[derive(Clone, Copy, Debug)]
pub struct ShipmentSize {
    pub forces: i32,
    pub special_forces: i32,
    pub no_field_value: i32,
    pub cunning_no_field_value: i32,
    pub shortage: f32,
}

impl ShipmentSize {
    /// Forces shipped openly, with nothing missing.
    fn of(forces: i32, special_forces: i32) -> Self {
        ShipmentSize { forces, special_forces, no_field_value: -1, cunning_no_field_value: -1, shortage: 0.0 }
    }
}

/// Who pays for a planned shipment besides me.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Funding {
    Own,
    WithAlly,
    /// My ally's spice, and a Karama card if the shipment is expensive.
    WithAllyAndKarma,
}

/// The inputs that stay fixed while `shipment_size` adds forces one by one.
struct ForceSizing<'a> {
    game: &'a Game,
    me: PlayerIndex,
    location: Location,
    dial_needed: f32,
    max_unsupported_forces: i32,
    prefer_special_forces: bool,
    normal_strength: f32,
    special_strength: f32,
    spice_available: i32,
    no_spice_for_force_modifier: f32,
    cost_per_force_in_battle: i32,
}

impl ForceSizing<'_> {
    /// Adds forces from the given supply while the dial is short and the shipment (plus paying
    /// for the forces in battle) stays affordable.
    fn fill(&self, mut forces_available: i32, mut special_forces_available: i32) -> ShipmentSize {
        let mut dial_needed = self.dial_needed;
        let mut forces = 0;
        let mut special_forces = 0;
        let mut cost_of_battle = 0;

        let affordable = |forces: i32, special_forces: i32, cost_of_battle: i32| {
            let ship_cost = Shipment::determine_cost_game_player_int32_int32_location_boolean_boolean_boolean_boolean_boolean(
                self.game, self.me, forces, special_forces, self.location, false, false, false, false, false,
            );
            ship_cost <= self.spice_available && ship_cost + cost_of_battle - self.spice_available < self.max_unsupported_forces
        };
        // Forces I can't pay for in battle count at reduced strength.
        let modifier = |cost_of_battle: i32| if cost_of_battle <= self.spice_available { 1.0f32 } else { self.no_spice_for_force_modifier };

        if self.prefer_special_forces && self.special_strength > self.normal_strength {
            while dial_needed > 0.0
                && (dial_needed > self.normal_strength || forces_available == 0)
                && special_forces_available >= 1
                && affordable(forces, special_forces + 1, cost_of_battle)
            {
                special_forces += 1;
                special_forces_available -= 1;
                cost_of_battle += self.cost_per_force_in_battle;
                dial_needed -= self.special_strength * modifier(cost_of_battle);
            }
        }

        while dial_needed > 0.0 && forces_available >= 1 && affordable(forces + 1, special_forces, cost_of_battle) {
            forces += 1;
            forces_available -= 1;
            cost_of_battle += self.cost_per_force_in_battle;
            dial_needed -= self.normal_strength * modifier(cost_of_battle);
        }

        while dial_needed > 0.0 && special_forces_available >= 1 && affordable(forces, special_forces + 1, cost_of_battle) {
            special_forces += 1;
            special_forces_available -= 1;
            cost_of_battle += self.cost_per_force_in_battle;
            dial_needed -= self.special_strength * modifier(cost_of_battle);
        }

        ShipmentSize { forces, special_forces, no_field_value: -1, cunning_no_field_value: -1, shortage: dial_needed }
    }
}

/// A planned shipment into a location an opponent occupies, sized to beat them.
pub struct Attack {
    pub location: Location,
    pub opponent: Option<PlayerIndex>,
    pub dial_needed: f32,
    pub forces_to_ship: i32,
    pub special_forces_to_ship: i32,
    pub no_field_value: i32,
    pub cunning_no_field_value: i32,
    pub shortage_for_shipment: f32,
}

impl Attack {
    pub fn has_forces(&self) -> bool {
        self.forces_to_ship > 0 || self.special_forces_to_ship > 0
    }

    fn size(&self) -> ShipmentSize {
        ShipmentSize {
            forces: self.forces_to_ship,
            special_forces: self.special_forces_to_ship,
            no_field_value: self.no_field_value,
            cunning_no_field_value: self.cunning_no_field_value,
            shortage: self.shortage_for_shipment,
        }
    }
}
