use crate::prelude::*;
use dotnet_compat::linq::sum_i32;
use std::cell::OnceCell;
use treachery_engine::compat::lazy::LazyQuery;

/// `Player.AnyForcesIn(Territory)`.
fn any_forces_in_t(game: &Game, p: PlayerIndex, t: Territory) -> i32 {
    game.player(p).any_forces_in_territory(t.opt())
}

/// `Player.AnyForcesIn(Location)`.
fn any_forces_in_l(game: &Game, p: PlayerIndex, l: Location) -> i32 {
    game.player(p).any_forces_in(l.opt())
}

impl ClassicBot {
    // ---- Guild ----

    pub fn determine_set_shipment_permission(&self, game: &Game) -> Option<Rc<SetShipmentPermission>> {
        if game.current_phase != Phase::BeginningOfShipAndMove {
            return None;
        }

        let to_deny: Vec<Faction> =
            game.shipment_permissions.keys().copied().filter(|f| self.is_winning_opponent_faction(game, *f)).collect();
        if !to_deny.is_empty() {
            let mut e = SetShipmentPermission::new(game, self.faction(game));
            e.factions = Some(to_deny);
            e.permission = ShipmentPermission::None;
            return Some(Rc::new(e));
        }

        let discount_permission = if self.resources(game) > 10 || !self.winning_opponents_i_wish_to_attack(game, 99, true).is_empty() {
            ShipmentPermission::GuildRate
        } else {
            ShipmentPermission::None
        };
        let permission = ShipmentPermission(ShipmentPermission::Cross.0 | ShipmentPermission::ToHomeworld.0 | discount_permission.0);
        let to_allow: Vec<Faction> = SetShipmentPermission::valid_targets(game, self.player)
            .filter(|f| {
                !self.is_winning_opponent_faction(game, *f)
                    && match game.shipment_permissions.get(f) {
                        None => true,
                        Some(current_permission) => *current_permission != permission,
                    }
            })
            .collect();
        if !to_allow.is_empty() {
            let mut e = SetShipmentPermission::new(game, self.faction(game));
            e.factions = Some(to_allow);
            e.permission = permission;
            Some(Rc::new(e))
        } else {
            None
        }
    }

    // ---- Emperor ----

    pub fn determine_emperor_discarded(&self, game: &Game) -> Option<Rc<EmperorDiscarded>> {
        let me = game.player(self.player);
        if !(me.resources >= 10 && !me.has_room_for_cards(game)) {
            return None;
        }

        // OrderBy computes each key once, in element order, then sorts stably.
        let mut keyed: Vec<(i32, TreacheryCard)> =
            me.treachery_cards.iter().map(|c| (self.card_quality(game, *c, None), *c)).collect();
        keyed.sort_by(|a, b| a.0.cmp(&b.0));
        let worst_card = keyed.into_iter().map(|(_, c)| c).cs_first();

        if self.card_quality(game, worst_card, None) <= 1 {
            let mut e = EmperorDiscarded::new(game, self.faction(game));
            e.set_card(worst_card.opt());
            Some(Rc::new(e))
        } else {
            None
        }
    }

    // ---- Ixian ----

    pub fn determine_replaced_card_won(&self, game: &Game) -> Rc<ReplacedCardWon> {
        let replace = match game.card_just_won {
            Some(c) => self.card_quality(game, c, Some(self.player)) <= 2,
            None => false,
        };
        let mut e = ReplacedCardWon::new(game, self.faction(game));
        e.passed = !replace;
        Rc::new(e)
    }

    pub fn determine_ixian_removed_card_from_auction(&self, game: &Game) -> Rc<IxianRemovedCardFromAuction> {
        let me = game.player(self.player);
        let p = Some(self.player);
        let items = || game.cards_on_auction.as_ref().nre().items.iter().copied();

        let mut to_be_removed: Option<TreacheryCard> = None;
        if me.treachery_cards.iter().any(|c| c.type_() == TreacheryCardType::ProjectileAndPoison) {
            to_be_removed = items().cs_first_or_default_where(|c| c.type_() == TreacheryCardType::ShieldAndAntidote);
        }
        if to_be_removed.is_none() && me.treachery_cards.iter().all(|c| c.type_() != TreacheryCardType::ShieldAndAntidote) {
            to_be_removed = items().cs_first_or_default_where(|c| c.type_() == TreacheryCardType::ProjectileAndPoison);
        }
        if to_be_removed.is_none() && me.treachery_cards.iter().any(|c| c.is_projectile_weapon()) {
            to_be_removed = items().cs_first_or_default_where(|c| c.is_projectile_defense());
        }
        if to_be_removed.is_none() && me.treachery_cards.iter().any(|c| c.is_poison_weapon()) {
            to_be_removed = items().cs_first_or_default_where(|c| c.is_poison_defense());
        }
        if to_be_removed.is_none() && !me.treachery_cards.iter().any(|c| c.is_projectile_defense()) {
            to_be_removed = items().cs_first_or_default_where(|c| c.is_projectile_weapon());
        }
        if to_be_removed.is_none() && !me.treachery_cards.iter().any(|c| c.is_poison_defense()) {
            to_be_removed = items().cs_first_or_default_where(|c| c.is_poison_weapon());
        }

        if to_be_removed.is_none() {
            if me.treachery_cards.len() >= 3 || self.resources_including_ally_and_emperor_contribution(game) < 5 {
                //Remove the best card from auction
                to_be_removed = items().highest_or_default(&self.random, |c| self.card_quality(game, *c, p));
            } else {
                //Remove the worst card from auction
                to_be_removed = items().lowest_or_default(&self.random, |c| self.card_quality(game, *c, p));
            }
        }

        let to_be_removed = match to_be_removed {
            Some(c) => c,
            None => items().cs_first(),
        };

        let put_on_top = (self.card_quality(game, to_be_removed, p) <= 2 && self.ally(game) == Faction::None)
            || (self.card_quality(game, to_be_removed, p) >= 4 && self.ally(game) != Faction::None);

        let mut e = IxianRemovedCardFromAuction::new(game, self.faction(game));
        e.set_card(to_be_removed.opt());
        e.put_on_top = put_on_top;
        Rc::new(e)
    }

    pub fn determine_ixian_swapped_card_on_bid(&self, game: &Game) -> Rc<IxianSwappedCardOnBid> {
        let p = Some(self.player);
        let card = game
            .player(self.player)
            .treachery_cards
            .iter()
            .copied()
            .lowest_or_default(&self.random, |c| self.card_quality(game, *c, p));

        if let Some(card) = card {
            if self.card_quality(game, card, p) <= 2 {
                let mut e = IxianSwappedCardOnBid::new(game, self.faction(game));
                e.passed = false;
                e.set_card(Some(card));
                return Rc::new(e);
            }
        }
        let mut e = IxianSwappedCardOnBid::new(game, self.faction(game));
        e.passed = true;
        Rc::new(e)
    }

    pub fn determine_ixian_selected_starting_card(&self, game: &Game) -> Rc<IxianSelectedStartingCard> {
        let cards: &Vec<TreacheryCard> = &game.starting_treachery_cards.as_ref().nre().items;
        let it = || cards.iter().copied();

        let mut card = it().cs_first_or_default_where(|c| c.type_() == TreacheryCardType::ProjectileAndPoison);
        if card.is_none() {
            card = it().cs_first_or_default_where(|c| c.type_() == TreacheryCardType::ShieldAndAntidote);
        }
        if card.is_none() {
            card = it().cs_first_or_default_where(|c| c.type_() == TreacheryCardType::Laser);
        }

        if card.is_none() && it().any(|c| c.is_poison_weapon()) && !it().any(|c| c.is_poison_defense()) {
            card = it().cs_first_or_default_where(|c| c.is_poison_weapon());
        }

        if card.is_none() && it().any(|c| c.is_projectile_weapon()) && !it().any(|c| c.is_projectile_defense()) {
            card = it().cs_first_or_default_where(|c| c.is_projectile_weapon());
        }

        if card.is_none() {
            card = it().cs_first_or_default_where(|c| c.type_() == TreacheryCardType::WeirdingWay);
        }
        if card.is_none() {
            card = it().cs_first_or_default_where(|c| c.type_() == TreacheryCardType::Chemistry);
        }
        if card.is_none() {
            card = it().cs_first_or_default_where(|c| c.is_projectile_weapon());
        }
        if card.is_none() {
            card = it().cs_first_or_default_where(|c| c.is_poison_weapon());
        }
        if card.is_none() {
            card = it().cs_first_or_default_where(|c| c.is_projectile_defense());
        }
        if card.is_none() {
            card = it().cs_first_or_default_where(|c| c.is_poison_defense());
        }
        if card.is_none() {
            card = it().cs_first_or_default_where(|c| c.type_() != TreacheryCardType::Useless);
        }
        if card.is_none() {
            card = it().next().and_then(|c| c.opt());
        }

        let mut e = IxianSelectedStartingCard::new(game, self.faction(game));
        e.set_card(card);
        Rc::new(e)
    }

    pub fn determine_perform_hms_movement(&self, game: &Game) -> Rc<PerformHmsMovement> {
        let current_location = game.map.hidden_mobile_stronghold().attached_to_location(&game.map);
        let faction = self.faction(game);
        let target_event = |target: Option<Location>| {
            let mut e = PerformHmsMovement::new(game, faction);
            e.passed = false;
            e.set_target(game, target);
            Rc::new(e)
        };

        let richest_adjacent_spice_location = PerformHmsMovement::valid_locations(game)
            .filter(|l| current_location != Some(*l) && self.resources_in(game, *l) > 0)
            .highest_or_default(&self.random, |l| self.resources_in(game, *l));
        if let Some(l) = richest_adjacent_spice_location {
            return target_event(Some(l));
        }

        let reachable_from_current_location =
            game.map.find_neighbours(current_location.or_null(), game.hms_moves_left, false, faction, game, false);

        let richest_reachable_spice_location = reachable_from_current_location
            .iter()
            .copied()
            .filter(|l| current_location != Some(*l) && self.resources_in(game, *l) > 0)
            .highest_or_default(&self.random, |l| self.resources_in(game, *l));
        if let Some(richest) = richest_reachable_spice_location {
            let next_step_towards_spice =
                PerformHmsMovement::valid_locations(game).cs_first_or_default_where(|l| self.within_distance(game, *l, richest, 1));

            if next_step_towards_spice.is_some() {
                return target_event(next_step_towards_spice);
            }

            let next_step_towards_spice =
                PerformHmsMovement::valid_locations(game).cs_first_or_default_where(|l| self.within_distance(game, *l, richest, 2));
            return target_event(next_step_towards_spice);
        }

        //If there is nowhere to go, move toward Imperial Basin or Polar Sink
        let alternative_location = if sum_i32(game.battalions_in(game.map.arrakeen()).iter().map(|b| b.total_amount_of_forces())) == 0
            || sum_i32(game.battalions_in(game.map.carthag()).iter().map(|b| b.total_amount_of_forces())) == 0
        {
            game.map.imperial_basin().middle_location()
        } else {
            game.map.polar_sink()
        };

        if current_location != Some(alternative_location) {
            let next = PerformHmsMovement::valid_locations(game).cs_first_or_default_where(|l| *l == alternative_location);
            if next.is_some() {
                return target_event(next);
            }

            let next = PerformHmsMovement::valid_locations(game)
                .cs_first_or_default_where(|l| self.within_distance(game, *l, alternative_location, 1));
            if next.is_some() {
                return target_event(next);
            }

            let next = PerformHmsMovement::valid_locations(game)
                .cs_first_or_default_where(|l| self.within_distance(game, *l, alternative_location, 2));
            if next.is_some() {
                return target_event(next);
            }
        }

        let mut e = PerformHmsMovement::new(game, faction);
        e.passed = true;
        Rc::new(e)
    }

    pub fn determine_perform_hms_placement(&self, game: &Game) -> Rc<PerformHmsPlacement> {
        let faction = self.faction(game);
        let mut e = PerformHmsPlacement::new(game, faction);
        if faction != Faction::Ixian {
            let target = game.map.shield_wall().locations().iter().copied().cs_first();
            e.set_target(game, target.opt());
            return Rc::new(e);
        }

        if sum_i32(game.battalions_in(game.map.arrakeen()).iter().map(|b| b.total_amount_of_forces())) == 0
            || sum_i32(game.battalions_in(game.map.carthag()).iter().map(|b| b.total_amount_of_forces())) == 0
        {
            let imperial_basin = game.map.imperial_basin();
            let target = PerformHmsPlacement::valid_locations(game, self.player).cs_first_where(|l| l.territory() == imperial_basin);
            e.set_target(game, target.opt());
            return Rc::new(e);
        }

        e.set_target(game, Some(game.map.polar_sink()));
        Rc::new(e)
    }

    // ---- Fremen ----

    pub fn determine_take_losses(&self, game: &Game) -> Rc<TakeLosses> {
        let normal_forces =
            std::cmp::min(TakeLosses::losses_to_take(game).amount, TakeLosses::valid_max_force_amount(game, self.player));
        let special_forces = TakeLosses::losses_to_take(game).amount.wrapping_sub(normal_forces);
        let use_useless = TakeLosses::can_prevent_losses(game, self.player);
        let mut e = TakeLosses::new(game, self.faction(game));
        e.force_amount = normal_forces;
        e.special_force_amount = special_forces;
        e.use_useless_card = use_useless;
        Rc::new(e)
    }

    pub fn determine_perform_fremen_setup(&self, game: &Game) -> Rc<PerformFremenSetup> {
        let faction = self.faction(game);
        let mut force_locations: NetDictionary<Location, Battalion> = NetDictionary::new();
        let south = game.map.false_wall_south().middle_location();
        let forces = 3i32.wrapping_add(self.d(game, 1, 4));
        let special = if game.player(self.player).special_forces_in_reserve() > 0 { 1 } else { 0 };
        force_locations.add(south, Battalion::new(faction, forces, special, south));

        let forces_left = 10i32.wrapping_sub(sum_i32(force_locations.values().map(|b| b.total_amount_of_forces())));

        if forces_left > 0 {
            let amount_of_special_forces = if game.player(self.player).special_forces_in_reserve() > 1 { 1 } else { 0 };
            let west = game.map.false_wall_west().middle_location();
            force_locations.add(
                west,
                Battalion::new(faction, forces_left.wrapping_sub(amount_of_special_forces), amount_of_special_forces, west),
            );
        }

        let mut e = PerformFremenSetup::new(game, faction);
        e.set_force_locations(game, &force_locations);
        Rc::new(e)
    }

    pub fn determine_fremen_sent_monster(&self, game: &Game) -> Rc<FremenSentMonster> {
        let target = FremenSentMonster::valid_targets(game).highest_or_default(&self.random, |t| {
            self.total_max_dial_of_opponents(game, *t) + 2i32.wrapping_mul(any_forces_in_t(game, self.player, *t)) as f32
        });
        let mut e = FremenSentMonster::new(game, self.faction(game));
        e.set_territory(game, target);
        Rc::new(e)
    }

    pub fn determine_fremen_rides_monster(&self, game: &Game) -> Rc<FremenRidesMonster> {
        let me = game.player(self.player);
        let mut target: Option<Location> = None;
        let valid_locations: Vec<Location> = FremenRidesMonster::valid_targets(game, self.player).collect();
        // ToDictionary: a duplicate key throws ArgumentException.
        let mut battalions_to_move: NetDictionary<Location, Battalion> = NetDictionary::new();
        for l in FremenRidesMonster::valid_sources(game) {
            let b = me.battalion_in_location(l).nre();
            battalions_to_move.add(l, b);
        }
        let forces_from_reserves = std::cmp::min(FremenRidesMonster::max_forces_from_reserves(game, self.player, false), 4);
        let special_forces_from_reserves = std::cmp::min(FremenRidesMonster::max_forces_from_reserves(game, self.player, true), 2);

        let map = &game.map;
        if valid_locations.contains(&map.tueks_sietch()) && self.vacant_and_safe_from_storm(game, map.tueks_sietch()) {
            target = Some(map.tueks_sietch());
        }
        if target.is_none() && valid_locations.contains(&map.carthag()) && self.vacant_and_safe_from_storm(game, map.carthag()) {
            target = Some(map.carthag());
        }
        if target.is_none() && valid_locations.contains(&map.arrakeen()) && self.vacant_and_safe_from_storm(game, map.arrakeen()) {
            target = Some(map.arrakeen());
        }
        if target.is_none()
            && valid_locations.contains(&map.habbanya_sietch())
            && self.vacant_and_safe_from_storm(game, map.habbanya_sietch())
        {
            target = Some(map.habbanya_sietch());
        }

        if target.is_none() {
            target = game
                .resources_on_planet
                .iter()
                .map(|(k, v)| (*k, *v))
                .filter(|(k, _)| valid_locations.contains(k) && self.vacant_and_safe_from_storm(game, *k))
                .highest_or_default(&self.random, |r| r.1)
                .map(|r| r.0);
        }

        if target.is_none() {
            let strength = sum_i32(
                battalions_to_move
                    .values()
                    .map(|b| b.amount_of_forces.wrapping_add(b.amount_of_special_forces.wrapping_mul(2))),
            )
            .wrapping_add(forces_from_reserves)
            .wrapping_add(special_forces_from_reserves.wrapping_mul(2));

            if target.is_none() {
                target = valid_locations
                    .iter()
                    .copied()
                    .filter(|l| {
                        game.resources_on_planet.contains_key(l)
                            && self.total_max_dial_of_opponents(game, l.territory()) < strength as f32
                    })
                    .highest_or_default(&self.random, |l| *game.resources_on_planet.index(l));
            }
            if target.is_none() {
                target = valid_locations
                    .iter()
                    .copied()
                    .filter(|l| {
                        *l != map.sietch_tabr()
                            && l.is_stronghold()
                            && self.total_max_dial_of_opponents(game, l.territory()) < strength as f32
                    })
                    .lowest_or_default(&self.random, |l| self.total_max_dial_of_opponents(game, l.territory()));
            }
        }

        if target.is_none() {
            target = Some(map.polar_sink());
        }

        if target.is_some()
            && sum_i32(battalions_to_move.values().map(|b| b.total_amount_of_forces()))
                .wrapping_add(forces_from_reserves)
                .wrapping_add(special_forces_from_reserves)
                > 0
        {
            let mut e = FremenRidesMonster::new(game, self.faction(game));
            e.passed = false;
            e.set_force_locations(game, &NetDictionary::from_dictionary(&battalions_to_move));
            e.set_to(game, target);
            e.forces_from_reserves = forces_from_reserves;
            e.special_forces_from_reserves = special_forces_from_reserves;
            return Rc::new(e);
        }
        let mut e = FremenRidesMonster::new(game, self.faction(game));
        e.passed = true;
        Rc::new(e)
    }

    // ---- BeneGesserit ----

    pub fn determine_bene_gesserit_placement(&self, game: &Game) -> Rc<PerformBeneGesseritPlacement> {
        let map = &game.map;
        let target = if any_forces_in_l(game, self.player, map.arrakeen()) == 0 {
            map.carthag()
        } else if any_forces_in_l(game, self.player, map.carthag()) == 0 {
            map.arrakeen()
        } else if any_forces_in_l(game, self.player, map.habbanya_sietch()) == 0 {
            map.habbanya_sietch()
        } else {
            map.carthag()
        };

        let mut e = PerformBeneGesseritPlacement::new(game, self.faction(game));
        e.set_target(game, Some(target));
        Rc::new(e)
    }

    pub fn determine_bene_gesserit_flip(&self, game: &Game) -> Rc<BeneGesseritFlip> {
        let territory = BeneGesseritFlip::get_territory(game);

        let mut e = BeneGesseritFlip::new(game, self.faction(game));
        e.as_advisors = !self.i_want_to_be_fighters_in(game, territory);
        Rc::new(e)
    }

    /// `Game.Players.Where(p => p != Player && p != AlliedPlayer && Game.MeetsNormalVictoryCondition(p, true) && Game.CountChallengedVictoryPoints(p) < 2)`,
    /// enumerated by `Contains(x)` (it stops at the first match).
    fn potential_winning_opponents_contains(&self, game: &Game, x: Option<PlayerIndex>) -> bool {
        let allied = self.allied_player(game);
        game.player_indices()
            .filter(|p| {
                *p != self.player
                    && Some(*p) != allied
                    && game.meets_normal_victory_condition(*p, true)
                    && game.count_challenged_victory_points(*p) < 2
            })
            .any(|p| Some(p) == x)
    }

    pub fn i_want_to_be_fighters_in(&self, game: &Game, territory: Territory) -> bool {
        if !territory.is_stronghold() {
            return false;
        }

        if self.i_am_winning(game) {
            return true;
        }

        let Some(opponent) = self.get_opponent_that_occupies_territory(game, territory) else {
            return false;
        };

        let amount_i_can_reinforce = ClassicBot::max_reinforced_dial_to(game, self.player, opponent, territory);
        let max_dial = ClassicBot::max_dial(game, self.player, territory, Some(opponent), false);
        let dial_needed = self.get_dial_needed_territory_player_bool(game, territory, Some(opponent), false);

        (!game.player(opponent).is(Faction::Harkonnen) && self.win_was_predicted_by_me_this_turn(game, game.player(opponent).faction))
            || dial_needed <= max_dial
            || (self.last_turn(game) && dial_needed - 1.0 <= amount_i_can_reinforce + max_dial)
            || (self.potential_winning_opponents_contains(game, Some(opponent)) && dial_needed <= amount_i_can_reinforce + max_dial)
    }

    pub fn determine_bene_gesserit_battle_announcement(&self, game: &Game) -> Option<Rc<BeneGesseritBattleAnnouncement>> {
        if game.current_main_phase > MainPhase::Bidding
            && self.nr_of_battles_to_fight(game) <= Battle::valid_battle_heroes(game, self.player).len() as i32
        {
            // LowestOrDefault runs ToArray (the Where with its draws) first, then the selector twice per element (hazards 16.3).
            let territory = BeneGesseritBattleAnnouncement::valid_territories(game, self.player)
                .filter(|t| self.i_want_to_announce_battle_in(game, *t))
                .lowest_or_default(&self.random, |t| {
                    self.get_dial_needed_territory_player_bool(game, *t, self.get_opponent_that_occupies_territory(game, *t), false)
                });

            if let Some(territory) = territory {
                let mut e = BeneGesseritBattleAnnouncement::new(game, self.faction(game));
                e.set_territory(game, Some(territory));
                return Some(Rc::new(e));
            }
        }

        None
    }

    pub fn i_want_to_announce_battle_in(&self, game: &Game, territory: Territory) -> bool {
        let Some(opponent) = self.get_opponent_that_occupies_territory(game, territory) else {
            return false;
        };

        let dial_needed = self.get_dial_needed_territory_player_bool(game, territory, Some(opponent), false);

        let last_turn_confidence_bonus = if self.last_turn(game) { 3 } else { 0 };

        (territory.is_stronghold() && self.win_was_predicted_by_me_this_turn(game, game.player(opponent).faction))
            || dial_needed + last_turn_confidence_bonus as f32
                <= ClassicBot::max_dial(game, self.player, territory, Some(opponent), false)
                    + ClassicBot::max_reinforced_dial_to(game, self.player, opponent, territory)
    }

    pub fn determine_bene_gesserit_accompanies(&self, game: &Game) -> Rc<BeneGesseritAccompanies> {
        let me = game.player(self.player);
        let faction = self.faction(game);
        let no_accompany = || {
            let mut e = BeneGesseritAccompanies::new(game, faction);
            e.set_location(game, None);
            e.accompanies = false;
            Rc::new(e)
        };

        if me.forces_in_reserve() <= 0 {
            return no_accompany();
        }

        let target = BeneGesseritAccompanies::valid_targets(game, self.player)
            .into_iter()
            .cs_first_or_default_where(|l| l.is_stronghold());

        let mut shipping_opponent_can_win = false;
        if let Some(target) = target {
            let opponent = self.get_opponent_that_occupies(game, target);
            shipping_opponent_can_win = self.potential_winning_opponents_contains(game, opponent);
        }

        if let Some(target) = target {
            if !shipping_opponent_can_win
                && (self.resources_including_ally_contribution(game) < 5 || me.forces_in_reserve() > 3)
                && any_forces_in_l(game, self.player, target) < 8
                && ((!self.last_turn(game) && any_forces_in_l(game, self.player, target) > 0)
                    || (!me.has_ally() && me.forces_in_locations.keys().filter(|l| self.is_stronghold(game, **l)).count() <= 3))
            {
                let mut e = BeneGesseritAccompanies::new(game, faction);
                e.set_location(game, Some(target));
                e.accompanies = true;
                e.extra_advisor = false;
                return Rc::new(e);
            }
        }

        let polar_sink = game.map.polar_sink();
        if BeneGesseritAccompanies::valid_targets(game, self.player).contains(&polar_sink)
            && (self.resources_including_ally_contribution(game) < 5 || me.forces_in_reserve() > 3)
            && !(self.last_turn(game) && game.has_acted_or_passed.contains(&faction))
            && any_forces_in_l(game, self.player, polar_sink) < 8
        {
            let mut e = BeneGesseritAccompanies::new(game, faction);
            e.set_location(game, Some(polar_sink));
            e.accompanies = true;
            e.extra_advisor = BeneGesseritAccompanies::may_send_extra_advisor(game, self.player, polar_sink);
            return Rc::new(e);
        }

        no_accompany()
    }

    pub fn determine_bene_gesserit_prediction(&self, game: &Game) -> Rc<BeneGesseritPrediction> {
        let humans = || self.others(game).into_iter().filter(|p| !game.is_bot(*p));
        let mut predicted = if self.d(game, 1, 2) == 1 { humans().random_or_default(&self.random) } else { None };
        if predicted.is_none() {
            predicted = humans().random_or_default(&self.random);
        }
        if predicted.is_none() {
            predicted = self.others(game).into_iter().random_or_default(&self.random);
        }

        let predicted = game.player(predicted.or_null());
        let maximum_turns = game.maximum_turns();
        // Nested D: the inner draw comes first (hazards 16.5). A negative `sides` throws after the draw.
        let turn = if predicted.is(Faction::Harkonnen) {
            let sides = if self.d(game, 1, 2) == 1 { std::cmp::min(3, maximum_turns) } else { maximum_turns };
            self.d(game, 1, sides)
        } else if predicted.is(Faction::Atreides) || predicted.is(Faction::Ixian) {
            if self.d(game, 1, 2) == 1 {
                self.d(game, 1, maximum_turns)
            } else {
                2i32.wrapping_add(self.d(game, 1, maximum_turns.wrapping_sub(2)))
            }
        } else if predicted.is(Faction::Tleilaxu) {
            3i32.wrapping_add(self.d(game, 1, maximum_turns.wrapping_sub(3)))
        } else if self.d(game, 1, 2) == 1 {
            1i32.wrapping_add(self.d(game, 1, maximum_turns.wrapping_sub(1)))
        } else {
            2i32.wrapping_add(self.d(game, 1, maximum_turns.wrapping_sub(2)))
        };

        let mut e = BeneGesseritPrediction::new(game, self.faction(game));
        e.to_win = predicted.faction;
        e.turn = turn;
        Rc::new(e)
    }

    pub fn determine_voice(&mut self, game: &Game) -> Option<Rc<Voice>> {
        Self::log_info("DetermineVoice()", vec![]);

        self.voice_plan = None;

        let battle = game.current_battle.clone().nre();
        if battle.is_aggressor_or_defender(game, self.player) {
            let opponent = battle.opponent_of_faction(game, self.faction(game));
            let plan = self.best_voice(game, game.current_battle.clone(), self.player, opponent.or_null());
            let voice = plan.voice.clone();
            self.voice_plan = Some(plan);
            Self::log_info_message(Some(&voice.get_message(game)));
            return Some(voice);
        }

        None
    }

    /// `battle` is kept by reference: `Bot_Battle.cs:165` compares it with `Game.CurrentBattle` by identity.
    pub fn best_voice(&self, game: &Game, battle: Option<Rc<BattleInitiated>>, p: PlayerIndex, opponent: PlayerIndex) -> VoicePlan {
        let mut result = VoicePlan {
            battle,
            voice: Rc::new(Voice::default()),
            weapon_to_use: None,
            defense_to_use: None,
            player_hero_will_certainly_survive: false,
            opponent_hero_will_certainly_be_zero: false,
        };

        if self.win_was_predicted_by_me_this_turn(game, game.player(opponent).faction) {
            result.weapon_to_use = None;
            result.defense_to_use = None;
            let mut v = Voice::new(game, game.player(p).faction);
            v.must = true;
            v.type_ = TreacheryCardType::Laser;
            result.voice = Rc::new(v);
            result.opponent_hero_will_certainly_be_zero = true;
            result.player_hero_will_certainly_survive = true;
        }

        let known_opponent_defenses = ClassicBot::known_opponent_defenses(game, self.player, opponent);
        let known_opponent_weapons = ClassicBot::known_opponent_weapons(game, self.player, opponent);
        let nr_of_unknown_opponent_cards = ClassicBot::nr_of_unknown_opponent_cards(game, self.player, opponent);

        let weapons: Vec<TreacheryCard> = ClassicBot::weapons(game, self.player, None, None, None)
            .into_iter()
            .filter(|w| {
                w.type_() != TreacheryCardType::Useless
                    && w.type_() != TreacheryCardType::ArtilleryStrike
                    && w.type_() != TreacheryCardType::PoisonTooth
            })
            .collect();
        let ws = || weapons.iter().copied();
        result.weapon_to_use = ws().cs_first_or_default_where(|w| w.type_() == TreacheryCardType::ProjectileAndPoison); //use poison blade if available
        if result.weapon_to_use.is_none() {
            result.weapon_to_use = ws().cs_first_or_default_where(|w| w.type_() == TreacheryCardType::Laser); //use lasgun if available
        }
        if result.weapon_to_use.is_none() {
            result.weapon_to_use = ws().cs_first_or_default_where(|w| game.known_cards(self.player).contains(w)); //use a known weapon if available
        }
        if result.weapon_to_use.is_none() {
            result.weapon_to_use = ws().next().and_then(|w| w.opt()); //use any weapon
        }

        let mut type_ = TreacheryCardType::None;
        let mut must = false;

        let opponent_might_have_defenses = !(!known_opponent_defenses.is_empty() && nr_of_unknown_opponent_cards == 0);

        let cards_player_has_or_might_have = ClassicBot::cards_player_has_or_might_have(game, opponent);
        let might_have = |f: &dyn Fn(TreacheryCard) -> bool| cards_player_has_or_might_have.iter().any(|c| f(*c));
        let kod = || known_opponent_defenses.iter().copied();
        let kow = || known_opponent_weapons.iter().copied();

        if opponent_might_have_defenses {
            if let Some(weapon_to_use) = result.weapon_to_use {
                result.opponent_hero_will_certainly_be_zero = true;

                //opponent might have defenses and player has a weapon. Use voice to disable the corresponding defense, if possible by forcing use of the wrong defense and otherwise by forcing not to use the correct defense.
                if weapon_to_use.type_() == TreacheryCardType::ProjectileAndPoison {
                    let useless_defense = kod().cs_first_or_default_where(|d| d.type_() != TreacheryCardType::ShieldAndAntidote);
                    if let Some(d) = useless_defense {
                        must = true;
                        type_ = d.type_();
                    } else if might_have(&|c| c.type_() == TreacheryCardType::ShieldAndAntidote) {
                        must = false;
                        type_ = TreacheryCardType::ShieldAndAntidote;
                    }
                } else if weapon_to_use.is_laser() {
                    let useless_defense = kod().cs_first_or_default_where(|d| d.is_poison_defense() && !d.is_shield());
                    if let Some(d) = useless_defense {
                        must = true;
                        type_ = d.type_();
                    } else if might_have(&|c| c.is_shield()) {
                        must = false;
                        type_ = TreacheryCardType::Shield;
                    }
                } else if weapon_to_use.is_projectile_weapon() {
                    let useless_defense = kod().cs_first_or_default_where(|d| d.is_poison_defense() && !d.is_projectile_defense());
                    if let Some(d) = useless_defense {
                        must = true;
                        type_ = d.type_();
                    } else {
                        must = false;

                        if !kod().any(|d| d.is_projectile_defense() && d.type_() != TreacheryCardType::WeirdingWay)
                            && kod().any(|d| d.type_() == TreacheryCardType::WeirdingWay)
                            && kow().any(|d| d.is_weapon())
                            && might_have(&|c| c.type_() == TreacheryCardType::WeirdingWay)
                        {
                            type_ = TreacheryCardType::WeirdingWay;
                        } else if !game.applicable(Rule::BeneGesseritVoiceMustNameSpecialCards) && might_have(&|c| c.is_projectile_defense()) {
                            type_ = TreacheryCardType::ProjectileDefense;
                        } else if might_have(&|c| c.type_() == TreacheryCardType::ShieldAndAntidote) {
                            type_ = TreacheryCardType::ShieldAndAntidote;
                        } else if might_have(&|c| c.type_() == TreacheryCardType::Shield) {
                            type_ = TreacheryCardType::Shield;
                        }
                    }
                } else if weapon_to_use.is_poison_weapon() {
                    // C# parity: `d.IsProjectileDefense && !d.IsProjectileDefense` is never true (hazards 18).
                    let useless_defense = kod().cs_first_or_default_where(|d| d.is_projectile_defense() && !d.is_projectile_defense());
                    if let Some(d) = useless_defense {
                        must = true;
                        type_ = d.type_();
                    } else if !game.applicable(Rule::BeneGesseritVoiceMustNameSpecialCards) && might_have(&|c| c.is_poison_defense()) {
                        must = false;
                        type_ = TreacheryCardType::PoisonDefense;
                    } else if might_have(&|c| c.type_() == TreacheryCardType::ShieldAndAntidote) {
                        must = false;
                        type_ = TreacheryCardType::ShieldAndAntidote;
                    } else if might_have(&|c| c.type_() == TreacheryCardType::Chemistry) {
                        must = false;
                        type_ = TreacheryCardType::Chemistry;
                    } else if might_have(&|c| c.type_() == TreacheryCardType::Antidote) {
                        must = false;
                        type_ = TreacheryCardType::Antidote;
                    }
                }

                Self::log_info(
                    "Using {0}, disable enemy defense: {1} {2}",
                    vec![
                        ExpressionElement::from(result.weapon_to_use),
                        ExpressionElement::from(if must { "must use" } else { "may not use" }),
                        ExpressionElement::from(type_),
                    ],
                );
            }
        }

        if type_ == TreacheryCardType::None {
            let opponent_might_have_weapons = !(!known_opponent_weapons.is_empty() && nr_of_unknown_opponent_cards == 0);

            //I have no weapon or the opponent has no defense. Focus on disabling their weapon.
            let mut defense: Option<TreacheryCard> = None;
            self.determine_best_defense(game, self.player, opponent, None, &mut defense);
            result.defense_to_use = defense.and_then(|d| d.opt());

            if opponent_might_have_weapons {
                if let Some(defense_to_use) = result.defense_to_use {
                    //opponent might have weapons and player has a defense. Use voice to disable the corresponding weapon, if possible by forcing use of the wrong weapon and otherwise by forcing not to use the correct weapon.
                    if defense_to_use.type_() == TreacheryCardType::ShieldAndAntidote {
                        let useless_weapon = kow().cs_first_or_default_where(|w| {
                            w.type_() != TreacheryCardType::Laser
                                && w.type_() != TreacheryCardType::PoisonTooth
                                && w.type_() != TreacheryCardType::Rockmelter
                        });
                        if let Some(w) = useless_weapon {
                            must = true;
                            type_ = w.type_();
                            result.player_hero_will_certainly_survive = true;
                        } else if kow().any(|w| w.type_() == TreacheryCardType::PoisonTooth) {
                            must = false;
                            type_ = TreacheryCardType::PoisonTooth;
                        } else if kow().any(|w| w.type_() == TreacheryCardType::Laser) {
                            must = false;
                            type_ = TreacheryCardType::Laser;
                        } else if kow().any(|w| w.type_() == TreacheryCardType::Rockmelter) {
                            must = false;
                            type_ = TreacheryCardType::Rockmelter;
                        }

                        Self::log_info(
                            "Using {0}, disable enemy weapon: {1} {2}",
                            vec![
                                ExpressionElement::from(result.defense_to_use),
                                ExpressionElement::from(if must { "must use" } else { "may not use" }),
                                ExpressionElement::from(type_),
                            ],
                        );
                    } else if defense_to_use.is_projectile_defense() {
                        let useless_weapon = kow().cs_first_or_default_where(|w| w.is_projectile_weapon() && !w.is_poison_weapon());
                        if let Some(w) = useless_weapon {
                            must = true;
                            type_ = w.type_();
                            result.player_hero_will_certainly_survive = true;
                        } else if kow().any(|w| w.type_() == TreacheryCardType::Laser) {
                            must = false;
                            type_ = TreacheryCardType::Laser;
                            result.player_hero_will_certainly_survive = nr_of_unknown_opponent_cards == 0;
                        } else if kow().any(|w| w.type_() == TreacheryCardType::PoisonTooth) {
                            must = false;
                            type_ = TreacheryCardType::PoisonTooth;
                            result.player_hero_will_certainly_survive = nr_of_unknown_opponent_cards == 0;
                        } else if !kow().any(|d| d.is_poison_weapon() && d.type_() != TreacheryCardType::Chemistry)
                            && kow().any(|d| d.type_() == TreacheryCardType::Chemistry)
                            && kod().any(|d| d.is_defense())
                            && might_have(&|c| c.type_() == TreacheryCardType::Chemistry)
                        {
                            must = false;
                            type_ = TreacheryCardType::Chemistry;
                        } else if might_have(&|c| c.is_poison_weapon()) {
                            must = false;
                            type_ = TreacheryCardType::Poison;
                        }

                        Self::log_info(
                            "Using {0}, disable enemy weapon: {1} {2}",
                            vec![
                                ExpressionElement::from(result.defense_to_use),
                                ExpressionElement::from(if must { "must use" } else { "may not use" }),
                                ExpressionElement::from(type_),
                            ],
                        );
                    } else if defense_to_use.is_poison_defense() {
                        let useless_weapon = kow().cs_first_or_default_where(|w| !w.is_projectile_weapon() && w.is_poison_weapon());
                        if let Some(w) = useless_weapon {
                            must = true;
                            type_ = w.type_();
                            result.player_hero_will_certainly_survive = true;
                        } else if kow().any(|w| w.type_() == TreacheryCardType::Laser) {
                            must = false;
                            type_ = TreacheryCardType::Laser;
                            result.player_hero_will_certainly_survive = nr_of_unknown_opponent_cards == 0;
                        } else if kow().any(|w| w.type_() == TreacheryCardType::PoisonTooth) {
                            must = false;
                            type_ = TreacheryCardType::PoisonTooth;
                            result.player_hero_will_certainly_survive = nr_of_unknown_opponent_cards == 0;
                        } else if might_have(&|c| c.is_projectile_weapon()) {
                            must = false;
                            type_ = TreacheryCardType::Projectile;
                        }

                        Self::log_info(
                            "Using {0}, disable enemy weapon: {1} {2}",
                            vec![
                                ExpressionElement::from(result.defense_to_use),
                                ExpressionElement::from(if must { "must use" } else { "may not use" }),
                                ExpressionElement::from(type_),
                            ],
                        );
                    }
                }
            }
        }

        if type_ == TreacheryCardType::None
            && game
                .player(opponent)
                .treachery_cards
                .iter()
                .any(|c| game.known_cards(self.player).contains(c) && c.type_() == TreacheryCardType::Mercenary)
        {
            type_ = TreacheryCardType::Mercenary;
            must = true;
        }

        if type_ == TreacheryCardType::None {
            must = false;
            if self.d(game, 1, 2) > 1 {
                type_ = if might_have(&|c| c.is_projectile_weapon()) { TreacheryCardType::Projectile } else { TreacheryCardType::Poison };
            } else {
                type_ = if might_have(&|c| c.is_poison_weapon()) { TreacheryCardType::Poison } else { TreacheryCardType::Projectile };
            }

            Self::log_info(
                "Remaining category: {0} {1}",
                vec![ExpressionElement::from(if must { "must use" } else { "may not use" }), ExpressionElement::from(type_)],
            );
        }

        let mut voice = Voice::new(game, game.player(p).faction);
        voice.must = must;
        voice.type_ = type_;
        result.voice = Rc::new(voice);

        result
    }

    // ---- Atreides ----

    pub fn determine_prescience(&self, game: &Game) -> Option<Rc<Prescience>> {
        let battle = game.current_battle.clone().nre();
        let opponent = battle.opponent_of_faction(game, self.faction(game));

        if Voice::may_use_voice(game, opponent.or_null())
            && game.current_voice.is_none()
            && battle.plan_of(game, opponent).is_none()
        {
            //Wait for voice or finalized battle plan
            return None;
        }

        if !battle.is_aggressor_or_defender(game, self.player) {
            return None;
        }

        let existing_aspect = match &game.current_prescience {
            Some(p) => p.aspect,
            None => PrescienceAspect::None,
        };
        let mut e = Prescience::new(game, self.faction(game));
        let territory = battle.territory(game).or_null();
        let max_dial = ClassicBot::max_dial(game, self.player, territory, opponent, false);
        e.aspect = self.best_prescience(game, self.player, opponent.or_null(), max_dial, existing_aspect, territory);
        Some(Rc::new(e))
    }

    pub fn best_prescience(&self, game: &Game, p: PlayerIndex, opponent: PlayerIndex, max_force_strength_in_battle: f32, earlier_prescience: PrescienceAspect, territory: Territory) -> PrescienceAspect {
        let my_defenses: Vec<TreacheryCard> = Battle::valid_defenses(game, p, None, territory.opt(), false)
            .into_iter()
            .map(|c| c.or_null())
            .filter(|c| game.known_cards(p).contains(c))
            .collect();
        let my_weapons: Vec<TreacheryCard> = Battle::valid_weapons(game, p, None, None, territory.opt(), false)
            .into_iter()
            .map(|c| c.or_null())
            .filter(|c| game.known_cards(p).contains(c))
            .collect();

        let cards_opponent_has_or_might_have = ClassicBot::cards_player_has_or_might_have(game, opponent);
        let weapon_is_certain = ClassicBot::count_different_weapon_types(&cards_opponent_has_or_might_have) <= 1;
        let defense_is_certain = ClassicBot::count_different_defense_types(&cards_opponent_has_or_might_have) <= 1;

        let i_have_shield_snooper = my_defenses.iter().any(|d| d.type_() == TreacheryCardType::ShieldAndAntidote);
        let i_have_poison_blade = my_weapons.iter().any(|d| d.type_() == TreacheryCardType::ProjectileAndPoison);

        if earlier_prescience != PrescienceAspect::Weapon
            && !weapon_is_certain
            && my_defenses.iter().any(|d| d.is_projectile_defense())
            && my_defenses.iter().any(|d| d.is_poison_defense())
            && !i_have_shield_snooper
        {
            //I don't have shield snooper and I have choice between shield and snooper, therefore ask for the weapon used
            PrescienceAspect::Weapon
        } else if earlier_prescience != PrescienceAspect::Defense
            && !defense_is_certain
            && my_weapons.iter().any(|d| d.is_projectile_weapon())
            && my_weapons.iter().any(|d| d.is_poison_weapon())
            && !i_have_poison_blade
        {
            //I don't have poison blade and I have choice between poison weapon and projectile weapon, therefore ask for the defense used
            PrescienceAspect::Defense
        } else if earlier_prescience != PrescienceAspect::Weapon && !weapon_is_certain && !my_defenses.is_empty() && !i_have_shield_snooper {
            PrescienceAspect::Weapon
        } else if earlier_prescience != PrescienceAspect::Defense && !defense_is_certain && !my_weapons.is_empty() && !i_have_poison_blade {
            PrescienceAspect::Defense
        } else if earlier_prescience != PrescienceAspect::Dial
            && max_force_strength_in_battle > 2.0
            && Prescience::valid_aspects(game, p).contains(&PrescienceAspect::Dial)
        {
            PrescienceAspect::Dial
        } else if earlier_prescience != PrescienceAspect::Leader {
            PrescienceAspect::Leader
        } else {
            PrescienceAspect::None
        }
    }

    // ---- Tleilaxu ----

    pub fn determine_set_increased_revival_limits(&self, game: &Game) -> Option<Rc<SetIncreasedRevivalLimits>> {
        let targets: Vec<Faction> = SetIncreasedRevivalLimits::valid_targets(game, self.player).collect();
        if game.factions_with_increased_revival_limits.as_ref().nre().len() != targets.len() {
            let mut e = SetIncreasedRevivalLimits::new(game, self.faction(game));
            e.factions = Some(targets);
            return Some(Rc::new(e));
        }

        None
    }

    pub fn determine_face_dancer_revealed(&self, game: &Game) -> Rc<FaceDancerRevealed> {
        if FaceDanced::may_call_face_dancer(game, self.player) {
            // Game.WinnerHero is read (and may throw) per face dancer.
            let face_dancer = game
                .player(self.player)
                .face_dancers
                .iter()
                .copied()
                .cs_first_or_default_where(|f| game.winner_hero().nre().is_face_dancer(*f));
            let face_danced_hero_is_living_leader = match face_dancer {
                Some(f) if f.as_leader().is_some() => game.is_alive(f),
                _ => false,
            };

            if (FaceDanced::maximum_number_of_forces(game, self.player) > 0 || face_danced_hero_is_living_leader)
                && game.battle_winner != Some(self.ally(game))
            {
                let mut result = FaceDancerRevealed::new(game, self.faction(game));
                result.passed = false;
                Self::log_info_message(Some(&result.get_message(game)));
                return Rc::new(result);
            }
        }

        let mut e = FaceDancerRevealed::new(game, self.faction(game));
        e.passed = true;
        Rc::new(e)
    }

    pub fn determine_face_danced(&self, game: &Game) -> Rc<FaceDanced> {
        let faction = self.faction(game);
        let passed = || {
            let mut e = FaceDanced::new(game, faction);
            e.passed = true;
            Rc::new(e)
        };

        let mut forces_from_planet: NetDictionary<Location, Battalion> = NetDictionary::new();

        let mut to_place = FaceDanced::maximum_number_of_forces(game, self.player);

        let Some(biggest) = self.biggest_battalion_threatened_by_storm_without_spice(game) else {
            return passed();
        };

        let to_take = biggest.battalion.take(to_place, false);
        let to_take_total = to_take.total_amount_of_forces();
        forces_from_planet.add(biggest.location, to_take);
        to_place = to_place.wrapping_sub(to_take_total);

        let from_reserves = std::cmp::min(game.player(self.player).forces_in_reserve(), to_place);

        let mut target_location =
            FaceDanced::valid_target_locations(game).cs_first_or_default_where(|l| game.resources_on_planet.contains_key(l));
        if target_location.is_none() {
            target_location = FaceDanced::valid_target_locations(game).next().and_then(|l| l.opt());
        }

        let Some(target_location) = target_location else {
            return passed();
        };

        let mut target_locations: NetDictionary<Location, Battalion> = NetDictionary::new();
        target_locations.add(
            target_location,
            Battalion::new(
                faction,
                sum_i32(forces_from_planet.values().map(|b| b.amount_of_forces)).wrapping_add(from_reserves),
                sum_i32(forces_from_planet.values().map(|b| b.amount_of_special_forces)),
                target_location,
            ),
        );

        let mut result = FaceDanced::new(game, faction);
        result.set_force_locations(game, &forces_from_planet);
        result.forces_from_reserve = from_reserves;
        result.set_target_force_locations(game, &target_locations);
        Self::log_info_message(Some(&result.get_message(game)));
        Rc::new(result)
    }

    pub fn determine_face_dancer_replaced(&self, game: &Game) -> Rc<FaceDancerReplaced> {
        // OrderBy: one key per element, stable sort.
        let mut keyed: Vec<(i32, Hero)> =
            FaceDancerReplaced::valid_face_dancers(game, self.player).map(|f| (f.value(), f)).collect();
        keyed.sort_by(|a, b| a.0.cmp(&b.0));
        let replaceable: Vec<Hero> = keyed.into_iter().map(|(_, f)| f).collect();

        let me = game.player(self.player);
        let allied_leaders: Option<Vec<Leader>> = self.allied_player(game).map(|a| game.player(a).leaders.clone());
        let leaders_contain = |leaders: &[Leader], f: Hero| f.as_leader().is_some_and(|l| leaders.contains(&l));

        let mut to_replace = replaceable.iter().copied().cs_first_or_default_where(|f| {
            leaders_contain(&me.leaders, *f) || allied_leaders.as_ref().is_some_and(|al| leaders_contain(al, *f))
        });
        if to_replace.is_none() {
            to_replace = replaceable.iter().copied().cs_first_or_default_where(|f| f.as_leader().is_some() && !game.is_alive(*f));
        }

        let mut e = FaceDancerReplaced::new(game, self.faction(game));
        if let Some(to_replace) = to_replace {
            e.passed = false;
            e.set_selected_dancer(Some(to_replace));
        } else {
            e.passed = true;
        }
        Rc::new(e)
    }

    pub fn determine_request_tleilaxu_revival(&mut self, game: &Game) -> Option<Rc<RequestTleilaxuRevival>> {
        let faction = self.faction(game);
        if game.current_revival_requests.iter().any(|r| r.initiator() == faction)
            || game.early_revivals_offers.keys().any(|h| h.faction() == faction)
        {
            return None;
        }

        if self.turn_when_revival_was_requested == game.current_turn {
            return None;
        }

        // SafeOrKnownTraitorLeaders is a property, read per element. It is pure, so it is computed
        // once, on the first element, where C# first reads it (it can throw).
        let safe: OnceCell<Vec<Hero>> = OnceCell::new();
        let mut to_revive = RequestTleilaxuRevival::valid_targets(game, self.player)
            .filter(|l| safe.get_or_init(|| self.safe_or_known_traitor_leaders(game)).contains(l))
            .highest_or_default(&self.random, |l| l.value());

        if to_revive.is_none() {
            let opponents = self.opponents(game);
            let known_opponent_traitors = || opponents.iter().flat_map(|p| game.player(*p).revealed_traitors.iter().copied());
            to_revive = RequestTleilaxuRevival::valid_targets(game, self.player)
                .filter(|l| !known_opponent_traitors().cs_contains(l))
                .highest_or_default(&self.random, |l| l.value());
        }

        if to_revive.is_some() && Battle::valid_battle_heroes(game, self.player).len() <= 3 {
            to_revive = RequestTleilaxuRevival::valid_targets(game, self.player).highest_or_default(&self.random, |l| l.value());
        }

        if let Some(to_revive) = to_revive {
            self.turn_when_revival_was_requested = game.current_turn;
            let mut e = RequestTleilaxuRevival::new(game, faction);
            e.set_hero(Some(to_revive));
            return Some(Rc::new(e));
        }

        None
    }

    pub fn determine_accept_or_cancel_tleilaxu_revival(&mut self, game: &Game) -> Option<Rc<AcceptOrCancelTleilaxuRevival>> {
        let request_to_handle = game.current_revival_requests.first().cloned();
        if let Some(request_to_handle) = request_to_handle {
            let hero = request_to_handle.hero().or_null();

            let price;
            if request_to_handle.initiator() == self.ally(game) {
                price = 0;
            } else if let Some(value) = self.price_set_earlier.get(&hero) {
                price = *value;
            } else {
                let me = game.player(self.player);
                let condition = (me.face_dancers.contains(&hero) && !me.revealed_face_dancers.contains(&hero))
                    || (me.has_ally() && {
                        let a = game.player(self.allied_player(game).nre());
                        a.traitors.contains(&hero) && !a.revealed_traitors.contains(&hero)
                    });
                if condition {
                    price = 1i32.wrapping_add(self.d(game, 1, hero.value()));
                } else {
                    price = 2i32.wrapping_add(self.d(game, 2, hero.value()));
                }

                self.price_set_earlier.add(hero, price);
            }

            let mut e = AcceptOrCancelTleilaxuRevival::new(game, self.faction(game));
            e.set_hero(hero.opt());
            e.price = price;
            e.cancel = false;
            return Some(Rc::new(e));
        }

        None
    }

    // ---- Choam ----

    pub fn determine_resources_audited(&self, game: &Game) -> Option<Rc<ResourcesAudited>> {
        let battle = match &game.current_battle {
            Some(b) if b.is_aggressor_or_defender(game, self.player) => b.clone(),
            _ => return None,
        };

        let opponent = game.player(battle.opponent_of(game, self.player).or_null());
        let me = game.player(self.player);

        if opponent.treachery_cards.iter().any(|tc| !me.known_cards.contains(tc))
            && ResourcesAudited::valid_factions(game, self.player).cs_contains(&opponent.faction)
        {
            let mut e = ResourcesAudited::new(game, self.faction(game));
            e.target = opponent.faction;
            return Some(Rc::new(e));
        }

        None
    }

    /// `Game.Players.Where(p => p.Faction != Ally).Count(p => p.Resources < 2 || (Game.Applicable(Rule.BeneGesseritAutoCharity) && p.Faction == Faction.BeneGesserit))`
    fn nr_of_poor_players(&self, game: &Game) -> i32 {
        let ally = self.ally(game);
        game.player_indices()
            .map(|p| game.player(p))
            .filter(|p| p.faction != ally)
            .filter(|p| p.resources < 2 || (game.applicable(Rule::BeneGesseritAutoCharity) && p.faction == Faction::BeneGesserit))
            .count() as i32
    }

    pub fn determine_choam_economics(&self, game: &Game) -> Option<Rc<ChoamEconomics>> {
        if self.resources_including_ally_contribution(game) >= 14 && self.nr_of_poor_players(game) >= 2 {
            let mut e = ChoamEconomics::new(game, self.faction(game));
            e.status = ChoamEconomicsStatus::Cancel;
            return Some(Rc::new(e));
        }

        if self.resources_including_ally_contribution(game) <= 8 && self.nr_of_poor_players(game) <= 2 {
            let mut e = ChoamEconomics::new(game, self.faction(game));
            e.status = ChoamEconomicsStatus::Double;
            return Some(Rc::new(e));
        }

        None
    }

    pub fn determine_choam_remove_force(&self, game: &Game) -> Option<Rc<ChoamRemoveForce>> {
        let opponents = self.winning_opponents_i_wish_to_attack(game, 20, true);

        let map = &game.map;
        let strongholds: [Location; 6] = [
            map.carthag(),
            map.arrakeen(),
            map.tueks_sietch(),
            map.sietch_tabr(),
            map.habbanya_sietch(),
            map.hidden_mobile_stronghold().into(),
        ];

        for stronghold in strongholds {
            let opponent_with_one_battalion_in_stronghold = opponents.iter().copied().cs_first_or_default_where(|opp| {
                game.player(*opp).battalion_in_location(stronghold).nre().total_amount_of_forces() == 1
            });

            if let Some(opp) = opponent_with_one_battalion_in_stronghold {
                let mut e = ChoamRemoveForce::new(game, self.faction(game));
                e.set_location(game, Some(stronghold));
                e.target = game.player(opp).faction;
                e.special_force = game.player(opp).special_forces_in(Some(stronghold)) > 0;
                return Some(Rc::new(e));
            }
        }

        None
    }

    // ---- Moritani ----

    pub fn determine_extortion_prevented(&self, game: &Game) -> Option<Rc<ExtortionPrevented>> {
        if game.player(self.player).ally != Faction::Moritani && self.resources(game) > 12 && self.d(game, 1, 6) >= 3 {
            return Some(Rc::new(ExtortionPrevented::new(game, self.faction(game))));
        }

        None
    }

    pub fn determine_perform_moritani_setup(&self, game: &Game) -> Rc<PerformMoritaniSetup> {
        let map = &game.map;
        let valid = || PerformMoritaniSetup::valid_locations(game);

        let target = if valid().cs_contains(&map.carthag()) {
            map.carthag()
        } else if valid().cs_contains(&map.arrakeen()) {
            map.arrakeen()
        } else if valid().cs_contains(&map.tueks_sietch()) {
            map.tueks_sietch()
        } else if valid().cs_contains(&map.habbanya_sietch()) {
            map.habbanya_sietch()
        } else if valid().cs_contains(&map.imperial_basin().middle_location()) {
            map.imperial_basin().middle_location()
        } else if valid().cs_contains(&map.polar_sink()) {
            map.polar_sink()
        } else {
            valid().cs_first_where(|l| l.is_protected_from_storm())
        };

        let mut e = PerformMoritaniSetup::new(game, self.faction(game));
        e.set_target(game, target.opt());
        Rc::new(e)
    }

    pub fn determine_terror_planted(&self, game: &Game) -> Option<Rc<TerrorPlanted>> {
        if game.current_main_phase != MainPhase::Contemplate {
            return None;
        }

        let available_to_place: Vec<TerrorType> =
            TerrorPlanted::valid_terror_types(game, false).filter(|t| !game.terror_on_planet.contains_key(t)).collect();
        let valid_strongholds: Vec<Territory> = TerrorPlanted::valid_strongholds(game, self.player).collect();

        if available_to_place.is_empty() || valid_strongholds.is_empty() {
            let mut e = TerrorPlanted::new(game, self.faction(game));
            e.passed = true;
            return Some(Rc::new(e));
        }

        let mut type_ = TerrorType::None;

        if available_to_place.contains(&TerrorType::Robbery)
            && sum_i32(self.opponents(game).iter().map(|p| game.player(*p).resources))
                > (self.opponents(game).len() as i32).wrapping_mul(10)
        {
            type_ = TerrorType::Robbery;
        }

        if type_ == TerrorType::None
            && available_to_place.contains(&TerrorType::Atomics)
            && valid_strongholds.iter().any(|t| {
                sum_i32(self.opponents(game).iter().map(|o| any_forces_in_t(game, *o, *t))) > 12
                    && sum_i32(self.me_and_my_ally(game).iter().map(|o| any_forces_in_t(game, *o, *t))) < 2
            })
        {
            type_ = TerrorType::Atomics;
        }

        if type_ == TerrorType::None && available_to_place.contains(&TerrorType::Extortion) && self.resources(game) < 5 {
            type_ = TerrorType::Extortion;
        }

        if type_ == TerrorType::None {
            type_ = available_to_place.iter().copied().random_or_default(&self.random).unwrap_or_default();
        }

        let mut stronghold = if type_ == TerrorType::Atomics {
            valid_strongholds
                .iter()
                .copied()
                .highest_or_default(&self.random, |t| sum_i32(self.opponents(game).iter().map(|o| any_forces_in_t(game, *o, *t))))
        } else {
            valid_strongholds.iter().copied().highest_or_default(&self.random, |t| any_forces_in_t(game, self.player, *t))
        };

        if stronghold.is_none() && game.player(self.player).has_ally() {
            stronghold = valid_strongholds
                .iter()
                .copied()
                .highest_or_default(&self.random, |t| any_forces_in_t(game, self.allied_player(game).nre(), *t) > 0);
        }

        if stronghold.is_none() {
            stronghold = valid_strongholds.iter().copied().random_or_default(&self.random);
        }

        let mut e = TerrorPlanted::new(game, self.faction(game));
        if let Some(stronghold) = stronghold {
            e.type_ = type_;
            e.set_stronghold(game, Some(stronghold));
        } else {
            e.passed = true;
        }
        Some(Rc::new(e))
    }

    pub fn determine_terror_revealed(&self, game: &Game) -> Rc<TerrorRevealed> {
        //This is for now just quite random
        let territory = TerrorRevealed::get_territory(game);
        // C# parity: `AlliedPlayer?.AnyForcesIn(territory) == 0` is false without an ally.
        let may_use_atomics = sum_i32(
            self.opponents(game)
                .iter()
                .map(|o| game.player(*o).any_forces_in_territory(TerrorRevealed::get_territory(game))),
        ) > 5
            && self.allied_player(game).map(|a| game.player(a).any_forces_in_territory(territory)) == Some(0);

        let token_is_valid = |t: &TerrorType| {
            (*t != TerrorType::SneakAttack || TerrorRevealed::valid_sneak_attack_targets(game, self.player).next().is_some())
                && (*t != TerrorType::Atomics || may_use_atomics)
        };

        if TerrorRevealed::get_types(game).filter(token_is_valid).next().is_none() {
            let mut e = TerrorRevealed::new(game, self.faction(game));
            e.passed = true;
            return Rc::new(e);
        }

        let type_ = TerrorRevealed::get_types(game).filter(token_is_valid).random_or_default(&self.random).unwrap_or_default();
        let card_in_sabotage = game.player(self.player).treachery_cards.iter().copied().cs_first_or_default_where(|c| c.is_useless());
        let victim = game.get_player(Some(TerrorRevealed::get_victim(game)));

        let offer_alliance = TerrorRevealed::may_offer_alliance(game)
            && self.player_standing(game, victim.or_null()) as f32 > 1.5f32 * self.player_standing(game, self.player) as f32;

        let mut e = TerrorRevealed::new(game, self.faction(game));
        if offer_alliance {
            e.alliance_offered = true;
            return Rc::new(e);
        }
        e.type_ = type_;
        e.robbery_takes_card = false;
        e.set_card_to_give_in_sabotage(card_in_sabotage);
        e.forces_in_sneak_attack = TerrorRevealed::max_amount_of_forces_in_sneak_attack(game, self.player);
        e.set_sneak_attack_to(game, TerrorRevealed::valid_sneak_attack_targets(game, self.player).next().and_then(|l| l.opt()));
        Rc::new(e)
    }

    // ---- Ecaz ----

    pub fn determine_ambassador_placed(&mut self, game: &Game) -> Option<Rc<AmbassadorPlaced>> {
        let resources = self.resources(game);
        if (resources <= 1 || game.ambassadors_placed_this_turn != 0) && resources <= 3i32.wrapping_add(game.ambassadors_placed_this_turn) {
            return None;
        }

        let valid = || AmbassadorPlaced::valid_strongholds(game, self.player);
        let mut stronghold = valid().filter(|s| any_forces_in_t(game, self.player, *s) > 0).random_or_default(&self.random);
        if stronghold.is_none() && game.player(self.player).has_ally() {
            stronghold = valid()
                .filter(|s| any_forces_in_t(game, self.allied_player(game).nre(), *s) > 0)
                .random_or_default(&self.random);
        }

        let avoid_entering = stronghold.is_some();

        if stronghold.is_none() {
            stronghold = valid().filter(|s| self.vacant_territory(game, *s)).random_or_default(&self.random);
        }
        if stronghold.is_none() {
            stronghold = valid().random_or_default(&self.random);
        }

        let mut ambassador = Ambassador::None;
        let available_ambassadors: Vec<Ambassador> = AmbassadorPlaced::valid_ambassadors(game, self.player);
        if avoid_entering {
            if available_ambassadors.contains(&Ambassador::Harkonnen) {
                ambassador = Ambassador::Harkonnen;
            }
            if ambassador == Ambassador::None && available_ambassadors.contains(&Ambassador::Atreides) {
                ambassador = Ambassador::Atreides;
            }
        }

        let p = Some(self.player);
        if ambassador == Ambassador::None
            && available_ambassadors.contains(&Ambassador::Tleilaxu)
            && game.player(self.player).leaders.iter().filter(|l| !game.is_alive(Hero::Leader(**l))).count() >= 2
            && game.player(self.player).forces_killed >= 4
        {
            ambassador = Ambassador::Tleilaxu;
        }
        if ambassador == Ambassador::None
            && available_ambassadors.contains(&Ambassador::Guild)
            && game.player(self.player).treachery_cards.iter().any(|c| c.is_weapon())
            && game.player(self.player).treachery_cards.iter().any(|c| c.is_defense())
        {
            ambassador = Ambassador::Guild;
        }
        if ambassador == Ambassador::None
            && available_ambassadors.contains(&Ambassador::Choam)
            && game.player(self.player).treachery_cards.iter().filter(|c| self.card_quality(game, **c, p) <= 1).count() >= 2
        {
            ambassador = Ambassador::Choam;
        }
        if ambassador == Ambassador::None
            && available_ambassadors.contains(&Ambassador::Ixian)
            && game.player(self.player).treachery_cards.iter().any(|c| self.card_quality(game, *c, p) <= 1)
        {
            ambassador = Ambassador::Ixian;
        }
        if ambassador == Ambassador::None
            && available_ambassadors.contains(&Ambassador::Richese)
            && game.player(self.player).has_room_for_cards(game)
            && game.player(self.player).resources > 6
        {
            ambassador = Ambassador::Richese;
        }
        if ambassador == Ambassador::None && available_ambassadors.contains(&Ambassador::Emperor) && self.resources(game) <= 4 {
            ambassador = Ambassador::Emperor;
        }
        if ambassador == Ambassador::None
            && available_ambassadors.contains(&Ambassador::Fremen)
            && self.determine_moved_battalion(game, false).is_some()
        {
            ambassador = Ambassador::Fremen;
        }
        if ambassador == Ambassador::None && available_ambassadors.contains(&Ambassador::Ecaz) && !game.player(self.player).has_ally() {
            ambassador = Ambassador::Ecaz;
        }
        if game.ambassadors_placed_this_turn == 0 && ambassador == Ambassador::None {
            ambassador = AmbassadorPlaced::valid_ambassadors(game, self.player).into_iter().random_or_default(&self.random).unwrap_or_default();
        }

        if ambassador != Ambassador::None {
            let mut e = AmbassadorPlaced::new(game, self.faction(game));
            e.ambassador = ambassador;
            e.set_stronghold(game, stronghold);
            Some(Rc::new(e))
        } else {
            None
        }
    }

    pub fn determine_ambassador_activated(&mut self, game: &Game) -> Option<Rc<AmbassadorActivated>> {
        let victim_player = AmbassadorActivated::get_victim_player(game);
        let mut ambassador = AmbassadorActivated::get_ambassador(game);
        let mut bene_gesserit_selected_ambassador = Ambassador::None;

        if ambassador == Ambassador::BeneGesserit {
            //This is for now just random
            bene_gesserit_selected_ambassador = AmbassadorActivated::get_valid_bene_gesserit_ambassadors(game)
                .into_iter()
                .random_or_default(&self.random)
                .unwrap_or_default();
            ambassador = bene_gesserit_selected_ambassador;
        }

        let faction = self.faction(game);
        let p = Some(self.player);
        let new_event = || {
            let mut e = AmbassadorActivated::new(game, faction);
            e.bene_gesserit_selected_ambassador = bene_gesserit_selected_ambassador;
            e
        };

        match ambassador {
            Ambassador::Choam => {
                let to_discard: Vec<TreacheryCard> = AmbassadorActivated::get_valid_choam_cards(game, self.player)
                    .into_iter()
                    .filter(|c| self.card_quality(game, *c, p) < 2)
                    .collect();
                if !to_discard.is_empty() {
                    let mut e = new_event();
                    e.set_choam_cards(&to_discard);
                    return Some(Rc::new(e));
                }
                self.pass_ambassador_activated(game)
            }

            Ambassador::Ecaz => {
                let offer_alliance = AmbassadorActivated::alliance_can_be_offered(game, self.player)
                    && self.player_standing(game, victim_player.or_null()) as f64
                        > 0.33 * self.player_standing(game, self.player) as f64;
                let take_vidal = !offer_alliance && AmbassadorActivated::vidal_can_be_taken(game, self.player);
                let offer_vidal = offer_alliance
                    && AmbassadorActivated::vidal_can_be_offered_to_new_ally(game, self.player)
                    && ClassicBot::heroes_for_battle(game, self.player, true).len() >= 3
                    && ClassicBot::heroes_for_battle(game, victim_player.or_null(), true).len() < 3;

                if offer_alliance || take_vidal {
                    let mut e = new_event();
                    e.ecaz_offer_alliance = offer_alliance;
                    e.ecaz_take_vidal = take_vidal;
                    e.ecaz_give_vidal_to_ally = offer_vidal;
                    return Some(Rc::new(e));
                }

                self.pass_ambassador_activated(game)
            }

            Ambassador::Fremen => {
                let map = &game.map;
                let mut target: Option<Location> = None;
                let valid_locations: Vec<Location> = AmbassadorActivated::valid_fremen_targets(game, self.player).collect();
                if valid_locations.contains(&map.tueks_sietch()) && self.vacant_and_safe_from_storm(game, map.tueks_sietch()) {
                    target = Some(map.tueks_sietch());
                }
                if target.is_none() && valid_locations.contains(&map.carthag()) && self.vacant_and_safe_from_storm(game, map.carthag()) {
                    target = Some(map.carthag());
                }
                if target.is_none() && valid_locations.contains(&map.arrakeen()) && self.vacant_and_safe_from_storm(game, map.arrakeen()) {
                    target = Some(map.arrakeen());
                }
                if target.is_none()
                    && valid_locations.contains(&map.habbanya_sietch())
                    && self.vacant_and_safe_from_storm(game, map.habbanya_sietch())
                {
                    target = Some(map.habbanya_sietch());
                }
                if target.is_none() {
                    target = game
                        .resources_on_planet
                        .iter()
                        .map(|(k, v)| (*k, *v))
                        .filter(|(k, _)| valid_locations.contains(k) && self.vacant_and_safe_from_storm(game, *k))
                        .highest_or_default(&self.random, |r| r.1)
                        .map(|r| r.0);
                }

                if target.is_some() {
                    let to_move = self.determine_moved_battalion(game, true);
                    if let Some(to_move) = to_move {
                        if AmbassadorActivated::valid_fremen_sources(game, self.player).contains(&to_move.from.territory()) {
                            let mut forces_to_move: NetDictionary<Location, Battalion> = NetDictionary::new();
                            forces_to_move.add(to_move.from, to_move.battalion.clone());

                            let mut e = new_event();
                            e.set_fremen_or_guild_to(game, target);
                            e.set_fremen_force_locations(game, &forces_to_move);
                            return Some(Rc::new(e));
                        }
                    }
                }

                self.pass_ambassador_activated(game)
            }

            Ambassador::Ixian => {
                let to_replace = AmbassadorActivated::get_valid_ixian_cards(game, self.player)
                    .into_iter()
                    .cs_first_or_default_where(|c| self.card_quality(game, *c, p) < 2);
                if to_replace.is_some() {
                    let mut e = new_event();
                    e.set_ixian_card(to_replace);
                    return Some(Rc::new(e));
                }
                self.pass_ambassador_activated(game)
            }

            Ambassador::Richese => {
                if self.resources(game) > 3 && game.player(self.player).has_room_for_cards(game) {
                    return Some(Rc::new(new_event()));
                }
                self.pass_ambassador_activated(game)
            }

            Ambassador::Guild => {
                let dangerous_opponents: Vec<PlayerIndex> =
                    self.opponents(game).into_iter().filter(|o| self.is_almost_winning_opponent(game, *o)).collect();

                // One lazy chain: each target goes through every stage (ConstructAttack draws) before the next (hazards 16.4).
                // LowestOrDefault materializes it first, then runs its selector twice per element.
                let possible_attacks = AmbassadorActivated::valid_guild_targets(game, self.player)
                    .filter(|l| l.is_stronghold())
                    .filter(|l| dangerous_opponents.is_empty() || dangerous_opponents.iter().any(|o| game.player(*o).occupies(game, Some(*l))))
                    .filter(|l| {
                        l.territory().is_stronghold()
                            && any_forces_in_l(game, self.player, *l) == 0
                            && self.ally_doesnt_block(game, l.territory())
                            && !self.storm_will_probably_hit(game, *l)
                            && !self.in_storm(game, *l)
                            && self.i_dont_have_advisors_in(game, *l)
                    })
                    .map(|l| self.construct_attack(game, l, 0, 0, 4))
                    .filter(|s| match s.opponent {
                        Some(o) if s.forces_to_ship <= 4 => !self.win_was_predicted_by_me_this_turn(game, game.player(o).faction),
                        _ => false,
                    });

                let attack = possible_attacks
                    .filter(|s| s.has_forces() && s.shortage_for_shipment == 0.0)
                    .lowest_or_default(&self.random, |s| s.dial_needed + ClassicBot::determine_penalty(game, s.opponent));

                if let Some(attack) = attack {
                    let mut e = new_event();
                    e.set_fremen_or_guild_to(game, Some(attack.location));
                    e.guild_force_amount = AmbassadorActivated::valid_guild_max_forces(game, self.player);
                    return Some(Rc::new(e));
                }
                self.pass_ambassador_activated(game)
            }

            Ambassador::Tleilaxu => {
                let safe: OnceCell<Vec<Hero>> = OnceCell::new();
                let mut hero_to_revive = AmbassadorActivated::valid_tleilaxu_heroes(game, self.player)
                    .into_iter()
                    .filter(|l| safe.get_or_init(|| self.safe_or_known_traitor_leaders(game)).contains(l))
                    .highest_or_default(&self.random, |l| l.value());
                if hero_to_revive.is_none() {
                    hero_to_revive = AmbassadorActivated::valid_tleilaxu_heroes(game, self.player)
                        .into_iter()
                        .highest_or_default(&self.random, |l| l.value());
                }

                if hero_to_revive.is_some()
                    && (game.player(self.player).forces_in_reserve() > 3 || Battle::valid_battle_heroes(game, self.player).len() <= 1)
                {
                    let mut e = new_event();
                    e.set_tleilaxu_hero(hero_to_revive);
                    e.tleilaxu_assign_skill = Revival::may_assign_skill(game, self.player, hero_to_revive);
                    return Some(Rc::new(e));
                }
                if AmbassadorActivated::valid_tleilaxu_max_amount(game, self.player) >= 3 {
                    let mut e = new_event();
                    e.tleilaxu_amount_of_forces = AmbassadorActivated::valid_tleilaxu_max_amount(game, self.player);
                    return Some(Rc::new(e));
                }
                self.pass_ambassador_activated(game)
            }

            _ => Some(Rc::new(new_event())),
        }
    }

    pub fn pass_ambassador_activated(&self, game: &Game) -> Option<Rc<AmbassadorActivated>> {
        if AmbassadorActivated::may_pass(game, self.player) {
            let mut e = AmbassadorActivated::new(game, self.faction(game));
            e.passed = true;
            Some(Rc::new(e))
        } else {
            None
        }
    }

    // ---- Richese ----

    pub fn determine_richese_announces_black_market(&self, game: &Game) -> Rc<RicheseAnnouncesBlackMarket> {
        let p = Some(self.player);
        let card = game
            .player(self.player)
            .treachery_cards
            .iter()
            .copied()
            .cs_first_or_default_where(|c| self.card_quality(game, *c, p) < 3);
        let mut e = RicheseAnnouncesBlackMarket::new(game, self.faction(game));
        if card.is_some() {
            e.passed = false;
            e.set_card(card);
            // Initializers run in source order: AuctionType draws before Direction.
            e.auction_type =
                if self.d(game, 1, 2) > 1 { AuctionType::BlackMarketSilent } else { AuctionType::BlackMarketOnceAround };
            e.direction = if self.d(game, 1, 2) > 1 { 1 } else { -1 };
            return Rc::new(e);
        }
        e.passed = true;
        Rc::new(e)
    }

    pub fn determine_richese_revealed_no_field(&self, game: &Game) -> Option<Rc<RicheseRevealedNoField>> {
        if game.current_phase == Phase::ShipmentAndMoveConcluded {
            let location_with_no_field = self
                .forces_on_planet(game)
                .iter()
                .find(|(_, b)| b.amount_of_special_forces > 0)
                .map(|(l, _)| *l);
            if let Some(l) = location_with_no_field {
                if game.resources_on_planet.contains_key(&l) && !self.occupied_by_opponent(game, l.territory()) {
                    return Some(Rc::new(RicheseRevealedNoField::new(game, self.faction(game))));
                }
            }
        }

        None
    }

    pub fn determine_richese_announces_auction(&self, game: &Game) -> Rc<RicheseAnnouncesAuction> {
        let mut e = RicheseAnnouncesAuction::new(game, self.faction(game));
        e.first = self.d(game, 1, 2) > 1;
        Rc::new(e)
    }

    pub fn determine_richese_specifies_auction(&self, game: &Game) -> Rc<RicheseSpecifiesAuction> {
        // A copy of the cache, shuffled with the bot's random.
        let mut to_auction = Deck::new(game.richese_cache.iter().copied(), Some(self.random.clone()));
        to_auction.shuffle();
        let mut e = RicheseSpecifiesAuction::new(game, self.faction(game));
        let card = if !to_auction.is_empty() { Some(to_auction.draw()) } else { None };
        e.set_card(card.and_then(|c| c.opt()));
        e.auction_type = if self.d(game, 1, 2) > 1 { AuctionType::RicheseSilent } else { AuctionType::RicheseOnceAround };
        e.direction = if self.d(game, 1, 2) > 1 { 1 } else { -1 };
        Rc::new(e)
    }

    pub fn determine_richese_keeps_unsold_card(&self, game: &Game) -> Rc<RicheseKeepsUnsoldCard> {
        let mut e = RicheseKeepsUnsoldCard::new(game, self.faction(game));
        e.passed = false;
        Rc::new(e)
    }

    // ---- Harkonnen ----

    pub fn determine_captured(&self, game: &Game) -> Rc<Captured> {
        let mut e = Captured::new(game, self.faction(game));
        e.passed = false;
        Rc::new(e)
    }
}

#[derive(Clone)]
pub struct VoicePlan {
    /// C#: `public BattleInitiated? Battle` (Bot_FactionSpecific.cs:1228). Compared by reference.
    pub battle: Option<Rc<BattleInitiated>>,
    pub voice: Rc<Voice>,
    pub weapon_to_use: Option<TreacheryCard>,
    pub defense_to_use: Option<TreacheryCard>,
    pub player_hero_will_certainly_survive: bool,
    pub opponent_hero_will_certainly_be_zero: bool,
}
