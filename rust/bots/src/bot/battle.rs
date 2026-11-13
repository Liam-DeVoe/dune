use crate::prelude::*;
use crate::support::unbox_enum;
use dotnet_compat::math::cmp_f32;

/// `a == b` on two `BattleInitiated?` references.
fn same_battle(a: &Option<Rc<BattleInitiated>>, b: &Option<Rc<BattleInitiated>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

/// `source.OrderByDescending(key)`: keys computed once each, stable (ties keep source order).
fn order_by_descending<T, K: Ord>(source: impl IntoIterator<Item = T>, key: impl FnMut(&T) -> K) -> Vec<T> {
    let mut key = key;
    let mut keyed: Vec<(K, T)> = source.into_iter().map(|v| (key(&v), v)).collect();
    keyed.sort_by(|a, b| b.0.cmp(&a.0));
    keyed.into_iter().map(|(_, v)| v).collect()
}

/// `source.OrderBy(key)` with an int key: stable.
fn order_by<T, K: Ord>(source: impl IntoIterator<Item = T>, key: impl FnMut(&T) -> K) -> Vec<T> {
    let mut key = key;
    let mut keyed: Vec<(K, T)> = source.into_iter().map(|v| (key(&v), v)).collect();
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    keyed.into_iter().map(|(_, v)| v).collect()
}

/// `source.OrderBy(key)` with a float key (`float.CompareTo`): stable.
fn order_by_f32<T>(source: impl IntoIterator<Item = T>, key: impl FnMut(&T) -> f32) -> Vec<T> {
    let mut key = key;
    let mut keyed: Vec<(f32, T)> = source.into_iter().map(|v| (key(&v), v)).collect();
    keyed.sort_by(|a, b| cmp_f32(a.0, b.0));
    keyed.into_iter().map(|(_, v)| v).collect()
}

/// `a.Union(b)` appended to `out`: distinct elements in order of first occurrence.
fn union_into(out: &mut Vec<Hero>, items: impl IntoIterator<Item = Hero>) {
    for h in items {
        if !out.contains(&h) {
            out.push(h);
        }
    }
}

impl ClassicBot {
    pub fn determine_battle_claimed(&self, game: &Game) -> Rc<BattleClaimed> {
        let about = game.battle_about_to_start.as_ref().nre();
        let territory = about.territory(game);
        let opponent = game.get_player(Some(about.target));
        let ecaz = game.get_player(Some(Faction::Ecaz));
        let ecaz_ally = game.player(ecaz.or_null()).allied_player(game);
        let ecaz_is_strongest = self.get_dial_needed(game, ecaz_ally.or_null(), territory.or_null(), opponent.or_null(), false)
            > self.get_dial_needed(game, ecaz.or_null(), territory.or_null(), opponent.or_null(), false);

        let mut result = BattleClaimed::new(game, self.faction(game));
        result.passed = if self.faction(game) == Faction::Ecaz { !ecaz_is_strongest } else { ecaz_is_strongest };
        Rc::new(result)
    }

    pub fn determine_switched_skilled_leader(&self, game: &Game) -> Option<Rc<SwitchedSkilledLeader>> {
        if !SwitchedSkilledLeader::can_be_played(game, self.player) {
            return None;
        }

        let switchable_leader = SwitchedSkilledLeader::switchable_leader(game, self.player).map(Hero::Leader);

        let switchable_leader_is_behind_shield = !game.is_in_front_of_shield(switchable_leader);
        let switchable_leader_must_be_used = Battle::valid_battle_heroes(game, self.player).len() <= 1
            || self.determine_battle_plan_if_not_waiting_for_prescience(game, true).and_then(|b| b.hero()) == switchable_leader;

        Self::log_info("Checking if {switchableLeader} should be switched...", vec![]);

        if switchable_leader_is_behind_shield ^ switchable_leader_must_be_used {
            return Some(Rc::new(SwitchedSkilledLeader::new(game, self.faction(game))));
        }

        Self::log_info("Switch not necessary.", vec![]);

        None
    }

    pub fn determine_battle_initiated(&self, game: &Game) -> Rc<BattleInitiated> {
        let battle = order_by_f32(Battle::battles_to_be_fought(game, self.player, false), |b| {
            Self::max_dial(game, game.get_player(Some(b.faction)).or_null(), b.territory, Some(self.player), false)
                - Self::max_dial(game, self.player, b.territory, game.get_player(Some(b.faction)), false)
        })
        .into_iter()
        .cs_first();

        let mut result = BattleInitiated::new(game, self.faction(game));
        result.target = battle.faction;
        result.set_territory(game, battle.territory.opt());
        Rc::new(result)
    }

    pub fn determine_treachery_called(&self, game: &Game) -> Option<Rc<TreacheryCalled>> {
        if !game.current_battle.as_ref().nre().is_aggressor_or_defender(game, self.player)
            && !TreacheryCalled::may_call_treachery(game, self.player)
        {
            return None;
        }

        let mut result = TreacheryCalled::new(game, self.faction(game));
        result.traitor_called = TreacheryCalled::may_call_treachery(game, self.player);
        Some(Rc::new(result))
    }

    pub fn determine_audit_cancelled(&self, game: &Game) -> Rc<AuditCancelled> {
        let mut result = AuditCancelled::new(game, self.faction(game));
        result.cancelled = false;
        Rc::new(result)
    }

    pub fn determine_audited(&self, game: &Game) -> Rc<Audited> {
        Rc::new(Audited::new(game, self.faction(game)))
    }

    pub fn determine_battle_concluded(&self, game: &Game) -> Rc<BattleConcluded> {
        let faction = self.faction(game);
        let current_battle = game.current_battle.as_ref().nre();
        let my_battleplan = current_battle.plan_of(game, Some(self.player));
        let opponent = current_battle.opponent_of(game, self.player);

        let mut discarded: Vec<TreacheryCard> = Vec::new();

        if BattleConcluded::may_choose_to_discard_cards(game) {
            let plan = my_battleplan.as_ref().nre();
            let me = game.player(self.player);
            if let Some(weapon) = plan.weapon() {
                if me.has(Some(weapon))
                    && weapon.type_() == TreacheryCardType::Useless
                    && faction != Faction::Choam
                    && !game.skilled_as_player_leader_skill(self.player, LeaderSkill::Warmaster)
                    && !game.owns_stronghold(faction, game.map.tueks_sietch())
                {
                    discarded.push(weapon);
                }
            }

            if let Some(defense) = plan.defense() {
                if me.has(Some(defense))
                    && defense.type_() == TreacheryCardType::Useless
                    && faction != Faction::Choam
                    && !game.skilled_as_player_leader_skill(self.player, LeaderSkill::Warmaster)
                    && !game.owns_stronghold(faction, game.map.tueks_sietch())
                {
                    discarded.push(defense);
                }
            }
        }

        let kill = match game.harkonnen_victim {
            Some(v) => !game.is_skilled(Some(Hero::Leader(v))) && Hero::Leader(v).value() < 4,
            None => false,
        };

        let replaced_special_forces = BattleConcluded::valid_replacement_force_amounts(game, self.player).into_iter().cs_max();

        let mut new_traitor: Option<Hero> = None;
        let mut to_replace: Option<Hero> = None;
        if !game.traitors_decipherer_can_look_at.is_empty() && game.decipherer_may_replace_traitor {
            new_traitor = order_by_descending(game.traitors_decipherer_can_look_at.iter().copied(), |t| t.value())
                .into_iter()
                .cs_first_or_default_where(|t| {
                    t.as_treachery_card().is_some() || (t.faction() != faction && game.is_alive(*t))
                });

            if let Some(new_traitor) = new_traitor {
                let replaceable: Vec<Hero> = BattleConcluded::valid_traitors_to_replace(game, self.player).collect();
                to_replace = replaceable.iter().copied().cs_first_or_default_where(|t| t.faction() == faction).or_else(|| {
                    order_by(replaceable.iter().copied(), |t| t.value())
                        .into_iter()
                        .cs_first_or_default_where(|t| t.as_treachery_card().is_none() && !game.is_alive(*t))
                });

                if to_replace.is_none() {
                    to_replace = order_by(replaceable.iter().copied(), |t| t.value())
                        .into_iter()
                        .cs_first_or_default_where(|t| t.as_treachery_card().is_none() && t.value() < new_traitor.value());
                }
            }
        }

        if to_replace.is_none() {
            new_traitor = None;
        }

        let mut result = BattleConcluded::new(game, faction);
        result.set_discarded_cards(&discarded);
        // opponent.TechTokens.FirstOrDefault(): default(TechToken) is TechToken.None.
        result.stolen_token = game.player(opponent.or_null()).tech_tokens.first().copied().unwrap_or(TechToken::None);
        result.kill = kill;
        result.special_force_losses_replaced = replaced_special_forces;
        result.set_new_traitor(new_traitor);
        result.set_traitor_to_replace(to_replace);
        result.add_extra_force = BattleConcluded::may_add_extra_force(game, self.player);
        Rc::new(result)
    }

    pub fn determine_battle(&self, game: &Game) -> Option<Rc<Battle>> {
        self.determine_battle_plan_if_not_waiting_for_prescience(game, false)
    }

    pub fn determine_battle_plan_if_not_waiting_for_prescience(&self, game: &Game, include_leader_in_front_of_shield: bool) -> Option<Rc<Battle>> {
        let opponent = game.current_battle.as_ref().nre().opponent_of(game, self.player);

        if Prescience::may_use_prescience(game, self.player)
            || (self.my_prescience(game).is_some() && game.current_battle.as_ref().nre().plan_of(game, opponent).is_none())
        {
            return None;
        }

        Some(self.determine_battle_plan(game, include_leader_in_front_of_shield))
    }

    pub fn determine_battle_plan(&self, game: &Game, include_leader_in_front_of_shield: bool) -> Rc<Battle> {
        Self::log_info("DetermineBattle()", vec![]);

        let current_battle = game.current_battle.as_ref().nre();
        let opponent = current_battle.opponent_of(game, self.player);

        if self.decided_shipment_action == ShipmentDecision::DummyShipment {
            Self::log_info("I'm spending as little as possible on this fight because this is a dummy shipment", vec![]);
            return self.construct_lost_battle_minimizing_losses(game, opponent.or_null(), current_battle.territory(game).or_null());
        }

        let forces_available = Battle::max_forces(game, self.player, false);
        let special_forces_available = Battle::max_forces(game, self.player, true);
        let voice = match &self.voice_plan {
            Some(v) if same_battle(&v.battle, &game.current_battle) => Some(v),
            _ => None,
        };

        let mut defense: Option<TreacheryCard> = None;
        let mut weapon: Option<TreacheryCard> = None;
        let mut hero: Option<Hero> = None;
        let mut messiah = false;
        let mut is_traitor = false;
        let mut lasgun_shield = false;
        let mut stone_burner = false;
        let mut banker_boost = 0i32;

        let mut dial_needed = self.get_dial_needed_for_battle(
            game,
            self.player,
            &self.param,
            self.i_will_be_aggressor_against(game, opponent),
            opponent.or_null(),
            current_battle.territory(game).or_null(),
            voice,
            self.my_prescience(game).map(|p| p.aspect).unwrap_or(PrescienceAspect::None),
            false,
            include_leader_in_front_of_shield,
            &mut defense,
            &mut weapon,
            &mut hero,
            &mut messiah,
            &mut is_traitor,
            &mut lasgun_shield,
            &mut stone_burner,
            &mut banker_boost,
        );

        if stone_burner {
            dial_needed = 0.0;
        }

        // LogInfo arguments are evaluated.
        let _ = game.player(opponent.or_null()).faction;
        let _ = game.current_battle.as_ref().nre().territory(game);

        let resources_from_ally = if self.ally(game) == Faction::Choam { game.resources_your_ally_can_pay(self.player) } else { 0 };
        let resources_for_battle = self.resources(game).wrapping_add(resources_from_ally);

        let mut forces_at_full_strength = 0;
        let mut forces_at_half_strength = 0;
        let mut special_forces_at_full_strength = 0;
        let mut special_forces_at_half_strength = 0;
        let mut dial_shortage = self.determine_dial_shortage_for_battle_single_faction_int32_int32_int32_territory_int32_int32_int32_int32(
            game,
            if lasgun_shield { 0.5 } else { dial_needed },
            game.player(opponent.or_null()).faction,
            forces_available,
            special_forces_available,
            resources_for_battle.wrapping_sub(banker_boost),
            game.current_battle.as_ref().nre().territory(game).or_null(),
            &mut forces_at_full_strength,
            &mut forces_at_half_strength,
            &mut special_forces_at_full_strength,
            &mut special_forces_at_half_strength,
        );

        if dial_shortage <= 3.0 && (weapon.is_none() || defense.is_none()) {
            let reinforcements = Battle::valid_weapons(game, self.player, defense, hero, current_battle.territory(game), false)
                .into_iter()
                .map(OrNull::or_null)
                .cs_first_or_default_where(|c| c.type_() == TreacheryCardType::Reinforcements);
            if reinforcements.is_some() {
                if weapon.is_none() {
                    weapon = reinforcements;
                } else {
                    defense = reinforcements;
                }

                dial_shortage -= 3.0;
            }
        }

        let predicted = self.win_was_predicted_by_me_this_turn(game, game.player(opponent.or_null()).faction);
        let total_forces = forces_at_full_strength
            .wrapping_add(forces_at_half_strength)
            .wrapping_add(special_forces_at_full_strength)
            .wrapping_add(special_forces_at_half_strength);
        let minimize_spending_in_this_lost_fight = predicted
            || (is_traitor && !messiah)
            || (self.resources(game).wrapping_add(self.resources_from_ally(game)) < 10
                && total_forces <= 8
                && dial_shortage > self.param.battle_dial_shortage_threshold_for_throwing);

        if !minimize_spending_in_this_lost_fight {
            if weapon.is_none() && !self.may_use_useless_as_karma(game) && self.faction(game) != Faction::Choam {
                weapon = Self::useless_as_weapon(game, self.player, defense);
            }
            if defense.is_none() && !self.may_use_useless_as_karma(game) && self.faction(game) != Faction::Choam {
                defense = Self::useless_as_defense(game, self.player, weapon);
            }

            self.remove_illegal_choices(game, &mut hero, &mut weapon, &mut defense, current_battle.territory(game));

            self.avoid_lasgun_shield_explosion(game, &mut weapon, &mut defense);

            Self::log_info("Leader: {0}, Weapon: {1}, Defense: {2}, ...", vec![]);

            let mut paid_by_arrakeen = 0;
            let cost = Battle::cost_game_player_int32_int32_int32(
                game,
                self.player,
                forces_at_full_strength,
                special_forces_at_full_strength,
                &mut paid_by_arrakeen,
            );
            let mut result = Battle::new(game, self.faction(game));
            result.set_hero(hero);
            result.messiah = messiah;
            result.forces = forces_at_full_strength;
            result.forces_at_half_strength = forces_at_half_strength;
            result.special_forces = special_forces_at_full_strength;
            result.special_forces_at_half_strength = special_forces_at_half_strength;
            result.ally_contribution_amount = cost.min(resources_from_ally.min(Battle::max_ally_resources(
                game,
                self.player,
                forces_at_full_strength,
                special_forces_at_full_strength,
            )));
            result.set_defense(defense);
            result.set_weapon(weapon);
            result.banker_bonus = banker_boost;
            return Rc::new(result);
        }

        Self::log_info("I'm spending as little as possible on this fight: ...", vec![]);

        self.construct_lost_battle_minimizing_losses(game, opponent.or_null(), current_battle.territory(game).or_null())
    }

    pub fn use_destructive_weapon_if_applicable(game: &Game, player: PlayerIndex, param: &BotParameters, enemy_can_defend_poison_tooth: bool, my_hero_surviving: &mut f32, enemy_hero_surviving: &mut f32, defense: &mut Option<TreacheryCard>, weapon: &mut Option<TreacheryCard>) {
        if weapon.is_some()
            || !(*my_hero_surviving < param.battle_mimimum_chance_to_assume_my_leader_survives)
            || !(*enemy_hero_surviving >= param.battle_mimimum_chance_to_assume_enemy_hero_survives)
        {
            return;
        }

        *weapon = Self::weapons(game, player, *defense, None, None)
            .into_iter()
            .cs_first_or_default_where(|c| c.type_() == TreacheryCardType::Rockmelter)
            .or_else(|| {
                Self::weapons(game, player, *defense, None, None)
                    .into_iter()
                    .cs_first_or_default_where(|c| c.type_() == TreacheryCardType::ArtilleryStrike)
            });

        if weapon.is_none() && !enemy_can_defend_poison_tooth {
            *weapon = Self::weapons(game, player, *defense, None, None)
                .into_iter()
                .cs_first_or_default_where(|c| c.type_() == TreacheryCardType::PoisonTooth);
        }

        if weapon.is_some() {
            *enemy_hero_surviving = 0.0;
            *my_hero_surviving = 0.0;
        }

        if let (Some(w), Some(d)) = (*weapon, *defense) {
            if Self::may_play_no_defense(game, player, Some(w))
                && ((w.type_() == TreacheryCardType::PoisonTooth && d.type_() != TreacheryCardType::Chemistry)
                    || (w.type_() == TreacheryCardType::ArtilleryStrike && !d.is_shield()))
            {
                *defense = None;
            }
        }
    }

    pub fn construct_lost_battle_minimizing_losses(&self, game: &Game, opponent: PlayerIndex, territory: Territory) -> Rc<Battle> {
        let mut lowest_available_hero = Battle::valid_battle_heroes(game, self.player)
            .into_iter()
            .cs_first_or_default_where(|h| h.as_treachery_card().is_some());
        if lowest_available_hero.is_none() {
            let mut is_traitor = false;
            self.select_hero_for_battle(game, self.player, opponent, false, true, false, None, None, &mut lowest_available_hero, &mut is_traitor, false);
        }

        let no_cards = lowest_available_hero.is_none() || self.may_use_useless_as_karma(game) || self.faction(game) == Faction::Choam;
        let mut weapon = if no_cards { None } else { Self::useless_as_weapon(game, self.player, None) };
        let no_cards = lowest_available_hero.is_none() || self.may_use_useless_as_karma(game) || self.faction(game) == Faction::Choam;
        let mut defense = if no_cards { None } else { Self::useless_as_defense(game, self.player, weapon) };

        self.remove_illegal_choices(game, &mut lowest_available_hero, &mut weapon, &mut defense, territory.opt());

        let mut harass = false;
        let me = game.player(self.player);
        if me.any_forces_in_territory(territory.opt()) >= 4 {
            if weapon.is_none()
                && Battle::valid_weapons(game, self.player, defense, lowest_available_hero, territory.opt(), false)
                    .into_iter()
                    .any(|c| c.or_null().type_() == TreacheryCardType::HarassAndWithdraw)
            {
                weapon = Some(me.treachery_cards.iter().copied().cs_first_where(|tc| tc.type_() == TreacheryCardType::HarassAndWithdraw));
                harass = true;
            } else if defense.is_none()
                && Battle::valid_defenses(game, self.player, weapon, territory.opt(), false)
                    .into_iter()
                    .any(|c| c.or_null().type_() == TreacheryCardType::HarassAndWithdraw)
            {
                defense = Some(me.treachery_cards.iter().copied().cs_first_where(|tc| tc.type_() == TreacheryCardType::HarassAndWithdraw));
                harass = true;
            }
        }

        let messiah = lowest_available_hero.is_some() && Battle::messiah_may_be_used_in_battle(game, self.player);

        let stronghold_free_forces = if game.has_stronghold_advantage(
            self.faction(game),
            StrongholdAdvantage::FreeResourcesForBattles,
            game.current_battle.as_ref().nre().territory(game).or_null(),
        ) {
            2
        } else {
            0
        };
        let special_at_full = stronghold_free_forces.min(Battle::max_forces(game, self.player, true));
        let normal_at_full = stronghold_free_forces.wrapping_sub(special_at_full).min(Battle::max_forces(game, self.player, false));

        if Battle::must_pay_for_any_forces_in_battle(game, self.player)
            && (special_at_full <= 0 || Battle::must_pay_for_special_forces_in_battle(game, self.player))
        {
            let mut result = Battle::new(game, self.faction(game));
            result.set_hero(lowest_available_hero);
            result.forces = if harass { 0 } else { normal_at_full };
            result.forces_at_half_strength = if harass { 0 } else { Battle::max_forces(game, self.player, false).wrapping_sub(normal_at_full) };
            result.special_forces = if harass { 0 } else { special_at_full };
            result.special_forces_at_half_strength =
                if harass { 0 } else { Battle::max_forces(game, self.player, true).wrapping_sub(special_at_full) };
            result.set_defense(defense);
            result.set_weapon(weapon);
            result.banker_bonus = 0;
            result.messiah = messiah;
            return Rc::new(result);
        }

        let mut result = Battle::new(game, self.faction(game));
        result.set_hero(lowest_available_hero);
        result.forces = if harass { 0 } else { Battle::max_forces(game, self.player, false) };
        result.forces_at_half_strength = 0;
        result.special_forces = if harass { 0 } else { Battle::max_forces(game, self.player, true) };
        result.special_forces_at_half_strength = 0;
        result.set_defense(defense);
        result.set_weapon(weapon);
        result.banker_bonus = 0;
        result.messiah = messiah;
        Rc::new(result)
    }

    pub fn remove_illegal_choices(&self, game: &Game, hero: &mut Option<Hero>, weapon: &mut Option<TreacheryCard>, defense: &mut Option<TreacheryCard>, territory: Option<Territory>) {
        let territory = territory.and_then(CsNull::opt);
        Self::log_info("Removing Illegal Choices...", vec![]);

        for _check in 0..3 {
            if hero.is_none() {
                *defense = None;
                *weapon = None;
                Self::log_info("Removed weapon and defense because no leader available", vec![]);
            }

            let weapon_clairvoyance = self.ruling_weapon_clairvoyance_for_player_battle(game);
            if weapon_clairvoyance.is_some() && !self.is_allowed_with_clairvoyance(game, weapon_clairvoyance.as_ref(), *weapon, true) {
                *weapon = Self::weapons(game, self.player, *defense, *hero, territory)
                    .into_iter()
                    .cs_first_or_default_where(|c| self.is_allowed_with_clairvoyance(game, weapon_clairvoyance.as_ref(), Some(*c), true));
                Self::log_info("Replaced weapon by: {weapon}", vec![]);
            }

            let def_clairvoyance = self.ruling_defense_clairvoyance_for_player_battle(game);
            if def_clairvoyance.is_some() && !self.is_allowed_with_clairvoyance(game, def_clairvoyance.as_ref(), *defense, false) {
                *defense = Self::defenses(game, self.player, *weapon, territory)
                    .into_iter()
                    .cs_first_or_default_where(|c| self.is_allowed_with_clairvoyance(game, def_clairvoyance.as_ref(), Some(*c), false));
                Self::log_info("Replaced defense by: {defense}", vec![]);
            }

            if let Some(w) = *weapon {
                if *defense == Some(w) && w.type_() == TreacheryCardType::Chemistry {
                    *weapon = None;
                }
            }

            if let Some(w) = *weapon {
                if *defense == Some(w) && w.type_() == TreacheryCardType::WeirdingWay {
                    *defense = None;
                }
            }

            if weapon.is_none() && defense.is_some_and(|d| d.type_() == TreacheryCardType::WeirdingWay) {
                *defense = None;
            }

            if defense.is_none() && weapon.is_some_and(|w| w.type_() == TreacheryCardType::Chemistry) {
                *weapon = None;
            }

            if defense.is_some() && *defense == *weapon {
                *defense = None;
            }

            if !Battle::valid_weapons(game, self.player, *defense, *hero, territory, true).contains(weapon) {
                *weapon = Self::weapons(game, self.player, *defense, *hero, territory)
                    .into_iter()
                    .cs_first_or_default_where(|w| w.type_() != TreacheryCardType::Chemistry);
                Self::log_info("Replaced weapon by: {weapon}", vec![]);
            }

            if !Battle::valid_defenses(game, self.player, *weapon, territory, true).contains(defense) {
                *defense = Self::defenses(game, self.player, *weapon, territory)
                    .into_iter()
                    .cs_first_or_default_where(|w| w.type_() != TreacheryCardType::WeirdingWay);
                Self::log_info("Replaced defense by: {defense}", vec![]);
            }
        }
    }

    pub fn avoid_lasgun_shield_explosion(&self, game: &Game, weapon: &mut Option<TreacheryCard>, defense: &mut Option<TreacheryCard>) {
        if !weapon.is_some_and(|w| w.type_() == TreacheryCardType::Laser) || !defense.is_some_and(|d| d.is_shield()) {
            return;
        }

        if Self::may_play_no_weapon(game, self.player, *defense) {
            *weapon = None;
        } else {
            *defense = None;
        }
    }

    pub fn determine_dial_shortage_for_battle(&self, game: &Game, dial_needed: f32, opponent: Faction, territory: Territory, forces_available: i32, special_forces_available: i32, resources_available: i32) -> f32 {
        let (mut a, mut b, mut c, mut d) = (0, 0, 0, 0);
        self.determine_dial_shortage_for_battle_single_faction_int32_int32_int32_territory_int32_int32_int32_int32(
            game,
            dial_needed,
            opponent,
            forces_available,
            special_forces_available,
            resources_available,
            territory,
            &mut a,
            &mut b,
            &mut c,
            &mut d,
        )
    }

    pub fn determine_dial_shortage_for_battle_single_faction_int32_int32_int32_territory_int32_int32_int32_int32(&self, game: &Game, dial_needed: f32, opponent: Faction, forces_available: i32, special_forces_available: i32, resources_available: i32, territory: Territory, forces_at_full_strength: &mut i32, forces_at_half_strength: &mut i32, special_forces_at_full_strength: &mut i32, special_forces_at_half_strength: &mut i32) -> f32 {
        let mut dial_needed = dial_needed;
        let mut forces_available = forces_available;
        let mut special_forces_available = special_forces_available;
        let faction = self.faction(game);
        let normal_strength = Battle::determine_normal_force_strength(game, faction);
        let special_strength = Battle::determine_special_force_strength(game, faction, opponent);
        let stronghold_bonus =
            if game.has_stronghold_advantage(faction, StrongholdAdvantage::FreeResourcesForBattles, territory) { 2 } else { 0 };
        let mut spice_left = resources_available.wrapping_add(stronghold_bonus);
        let cost_per_force = Battle::normal_force_cost(game, self.player);
        let cost_per_special_force = Battle::special_force_cost(game, self.player);
        let mut number_of_forces_with_cunning_bonus = match &game.current_emperor_cunning {
            Some(c) if c.initiator() == faction => 5,
            _ => 0,
        };

        Self::log_info("DetermineValidForcesInBattle: {0} {1} {2} {3}", vec![]);

        if Battle::must_pay_for_any_forces_in_battle(game, self.player) {
            *special_forces_at_full_strength = 0;
            while dial_needed > normal_strength && special_forces_available >= 1 && spice_left >= cost_per_special_force {
                dial_needed -= special_strength;
                *special_forces_at_full_strength += 1;
                special_forces_available -= 1;
                spice_left = spice_left.wrapping_sub(cost_per_special_force);
            }

            *forces_at_full_strength = 0;
            while dial_needed >= normal_strength && forces_available >= 1 && spice_left >= cost_per_force {
                if number_of_forces_with_cunning_bonus > 0 {
                    number_of_forces_with_cunning_bonus -= 1;
                    dial_needed -= special_strength;
                } else {
                    dial_needed -= normal_strength;
                }

                *forces_at_full_strength += 1;
                forces_available -= 1;
                spice_left = spice_left.wrapping_sub(cost_per_force);
            }

            *special_forces_at_half_strength = 0;
            while dial_needed > 0.0 && special_forces_available >= 1 {
                dial_needed -= 0.5f32 * special_strength;
                *special_forces_at_half_strength += 1;
                special_forces_available -= 1;
            }

            *forces_at_half_strength = 0;
            while dial_needed > 0.0 && forces_available >= 1 {
                if number_of_forces_with_cunning_bonus > 0 {
                    number_of_forces_with_cunning_bonus -= 1;
                    dial_needed -= 0.5f32 * special_strength;
                } else {
                    dial_needed -= 0.5f32 * normal_strength;
                }

                *forces_at_half_strength += 1;
                forces_available -= 1;
            }
        } else {
            *special_forces_at_full_strength = 0;
            while dial_needed >= special_strength && special_forces_available >= 1 && spice_left >= cost_per_special_force {
                dial_needed -= special_strength;
                *special_forces_at_full_strength += 1;
                special_forces_available -= 1;
                spice_left = spice_left.wrapping_sub(cost_per_special_force);
            }

            *forces_at_full_strength = 0;
            while dial_needed > 0.0 && forces_available >= 1 && spice_left >= cost_per_force {
                if number_of_forces_with_cunning_bonus > 0 {
                    number_of_forces_with_cunning_bonus -= 1;
                    dial_needed -= special_strength;
                } else {
                    dial_needed -= normal_strength;
                }

                *forces_at_full_strength += 1;
                forces_available -= 1;
                spice_left = spice_left.wrapping_sub(cost_per_force);
            }

            while dial_needed > 0.0 && special_forces_available >= 1 && spice_left >= cost_per_special_force {
                dial_needed -= special_strength;
                *special_forces_at_full_strength += 1;
                special_forces_available -= 1;
                spice_left = spice_left.wrapping_sub(cost_per_special_force);
            }

            *special_forces_at_half_strength = 0;
            *forces_at_half_strength = 0;
        }

        dial_needed
    }

    pub fn chance_of_my_leader_surviving(&self, game: &Game, player: PlayerIndex, opponent: PlayerIndex, voice_plan: Option<&VoicePlan>, prescience: Option<PrescienceAspect>, most_effective_defense: &mut Option<TreacheryCard>, chosen_weapon: Option<TreacheryCard>) -> f32 {
        let chosen_weapon = chosen_weapon.and_then(CsNull::opt);
        if Self::heroes_for_battle(game, opponent, true).is_empty() {
            Self::log_info("Opponent has no leaders", vec![]);
            *most_effective_defense = None;
            return 1.0;
        }

        if let Some(vp) = voice_plan {
            if vp.defense_to_use.is_some() {
                *most_effective_defense = vp.defense_to_use;
                return if vp.player_hero_will_certainly_survive { 1.0 } else { 0.5 };
            }
        }

        let available_defenses: Vec<TreacheryCard> =
            Self::defenses(game, player, chosen_weapon, game.current_battle.as_ref().and_then(|b| b.territory(game)))
                .into_iter()
                .filter(|def| {
                    Some(*def) != chosen_weapon
                        && (chosen_weapon.is_none() || !(chosen_weapon.unwrap().is_laser() && def.is_shield()))
                        && (def.type_() != TreacheryCardType::WeirdingWay || chosen_weapon.is_some())
                        && def.type_() != TreacheryCardType::Useless
                })
                .collect();

        if chosen_weapon.is_some_and(|w| w.is_artillery()) {
            *most_effective_defense = available_defenses.iter().copied().cs_first_or_default_where(|def| def.is_shield());
            return 0.0;
        }

        if chosen_weapon.is_some_and(|w| w.is_rock_melter()) {
            *most_effective_defense = None;
            return 0.0;
        }

        if chosen_weapon.is_some_and(|w| w.is_poison_tooth()) {
            *most_effective_defense =
                available_defenses.iter().copied().cs_first_or_default_where(|def| def.type_() == TreacheryCardType::Chemistry);
            return 0.0;
        }

        let opponent_plan = game.current_battle.as_ref().and_then(|b| b.plan_of(game, Some(opponent)));
        if prescience == Some(PrescienceAspect::Weapon) {
            if let Some(opponent_plan) = &opponent_plan {
                let opponent_weapon = opponent_plan.weapon();
                if opponent_weapon.is_none() || opponent_weapon.unwrap().type_() == TreacheryCardType::Useless {
                    *most_effective_defense = if Self::may_play_no_defense(game, player, chosen_weapon) {
                        None
                    } else {
                        cs::null_to_none(Self::defenses(game, player, chosen_weapon, None).into_iter().next())
                    };
                    return 1.0;
                }

                let opponent_weapon = opponent_weapon.unwrap();
                *most_effective_defense =
                    available_defenses.iter().copied().cs_first_or_default_where(|d| opponent_weapon.countered_by(Some(*d), chosen_weapon));
                return if most_effective_defense.is_some() { 1.0 } else { 0.0 };
            }
        }

        let my_clairvoyance = Self::player_clairvoyance_about_enemy_weapon_in_current_battle(game, player);
        if let Some(my_clairvoyance) = my_clairvoyance {
            Self::log_info("Clairvoyance detected!", vec![]);

            let question = my_clairvoyance.question.as_ref().nre();
            if question.is_about(game, TreacheryCardType::Projectile) {
                if my_clairvoyance.answer.is_yes() {
                    *most_effective_defense = available_defenses.iter().copied().cs_first_or_default_where(|d| d.is_projectile_defense());
                    return if most_effective_defense.is_some() { 1.0 } else { 0.0 };
                }

                if my_clairvoyance.answer.is_no() {
                    *most_effective_defense = available_defenses.iter().copied().cs_first_or_default_where(|d| d.is_poison_defense());
                    if most_effective_defense.is_some() {
                        return 0.5;
                    }
                }
            }

            let question = my_clairvoyance.question.as_ref().nre();
            if question.is_about(game, TreacheryCardType::Poison) {
                if my_clairvoyance.answer.is_yes() {
                    *most_effective_defense = available_defenses.iter().copied().cs_first_or_default_where(|d| d.is_poison_defense());
                    return if most_effective_defense.is_some() { 1.0 } else { 0.0 };
                }

                if my_clairvoyance.answer.is_no() {
                    *most_effective_defense = available_defenses.iter().copied().cs_first_or_default_where(|d| d.is_projectile_defense());
                    if most_effective_defense.is_some() {
                        return 0.5;
                    }
                }
            }
        }

        self.determine_best_defense(game, player, opponent, chosen_weapon, most_effective_defense)
    }

    pub fn nr_of_unknown_opponent_cards(game: &Game, player: PlayerIndex, opponent: PlayerIndex) -> i32 {
        let cards = &game.player(opponent).treachery_cards;
        if cards.is_empty() {
            return 0;
        }
        // KnownCards is pure; C# calls it once per card.
        let known = game.known_cards(player);
        cards.iter().filter(|c| !known.contains(c)).count() as i32
    }

    /// `mostEffectiveDefense` is declared non-nullable but gets `ObjectCounter.Highest`, which can be null.
    pub fn determine_best_defense(&self, game: &Game, player: PlayerIndex, opponent: PlayerIndex, chosen_weapon: Option<TreacheryCard>, most_effective_defense: &mut Option<TreacheryCard>) -> f32 {
        let chosen_weapon = chosen_weapon.and_then(CsNull::opt);
        let known_enemy_weapons = Self::known_opponent_weapons(game, player, opponent);
        let available_defenses: Vec<TreacheryCard> = Self::defenses(game, player, chosen_weapon, None)
            .into_iter()
            .filter(|def| {
                Some(*def) != chosen_weapon
                    && (chosen_weapon.is_none() || !(chosen_weapon.unwrap().is_laser() && def.is_shield()))
                    && (def.type_() != TreacheryCardType::WeirdingWay || chosen_weapon.is_some())
                    && def.type_() != TreacheryCardType::Useless
            })
            .collect();

        let mut defense_quality: ObjectCounter<TreacheryCard> = ObjectCounter::new();

        let unknown_cards = Self::cards_unknown_to_player(game, player);

        let best_defense_against_unknown_cards = available_defenses.iter().copied().lowest_or_default(&self.random, |def| {
            Self::number_of_unknown_weapons_that_could_kill_me_with_player_defense(&unknown_cards, Some(*def), chosen_weapon)
        });

        for def in available_defenses.iter().copied() {
            defense_quality.count(def);

            if Some(def) == best_defense_against_unknown_cards {
                defense_quality.count(def);
            }

            for known_weapon in known_enemy_weapons.iter().copied() {
                if known_weapon.countered_by(Some(def), chosen_weapon) {
                    Self::log_info("potentialWeapon is countered by", vec![]);
                    defense_quality.count2(def);
                }
            }
        }

        *most_effective_defense = defense_quality.highest().and_then(CsNull::opt);

        let defense_to_check = *most_effective_defense;

        if (most_effective_defense.is_none() && !known_enemy_weapons.is_empty())
            || known_enemy_weapons.iter().any(|w| !w.countered_by(defense_to_check, chosen_weapon))
        {
            return 0.0;
        }

        1.0 - self.chance_of_an_unknown_opponent_card_killing_my_leader(game, player, &unknown_cards, *most_effective_defense, opponent, chosen_weapon)
    }

    pub fn chance_of_an_unknown_opponent_card_killing_my_leader(&self, game: &Game, player: PlayerIndex, unknown_cards: &[TreacheryCard], used_defense: Option<TreacheryCard>, opponent: PlayerIndex, chosen_weapon: Option<TreacheryCard>) -> f32 {
        if unknown_cards.is_empty() {
            return 0.0;
        }
        let number = Self::number_of_unknown_weapons_that_could_kill_me_with_player_defense(unknown_cards, used_defense, chosen_weapon);
        let nr_of_unknown_opponent_cards = Self::nr_of_unknown_opponent_cards(game, player, opponent);

        // int - float is float; the float is then widened to the double parameter.
        let count = unknown_cards.len() as i32;
        1.0 - Self::cumulative_chance((count as f32 - number) as f64, count as f64, nr_of_unknown_opponent_cards) as f32
    }

    pub fn chance_of_an_unknown_opponent_card_saving_his_leader(&self, game: &Game, player: PlayerIndex, unknown_cards: &[TreacheryCard], used_weapon: Option<TreacheryCard>, opponent: PlayerIndex) -> f32 {
        if unknown_cards.is_empty() {
            return 0.0;
        }
        let number = Self::number_of_unknown_defenses_that_could_counter_player_weapon(unknown_cards, used_weapon);
        let nr_of_unknown_opponent_cards = Self::nr_of_unknown_opponent_cards(game, player, opponent);

        let count = unknown_cards.len() as i32;
        1.0 - Self::cumulative_chance((count as f32 - number) as f64, count as f64, nr_of_unknown_opponent_cards) as f32
    }

    pub fn cumulative_chance(a: f64, b: f64, length: i32) -> f64 {
        let mut result = 1.0f64;
        let mut i = 0i32;
        while i < length {
            result *= (a - i as f64) / (b - i as f64);
            i += 1;
        }
        result
    }

    pub fn number_of_unknown_weapons_that_could_kill_me_with_player_defense(unknown_cards: &[TreacheryCard], defense: Option<TreacheryCard>, chosen_weapon: Option<TreacheryCard>) -> f32 {
        let defense = defense.and_then(CsNull::opt);
        unknown_cards.iter().filter(|c| c.is_weapon() && (defense.is_none() || !c.countered_by(defense, chosen_weapon))).count() as i32
            as f32
    }

    pub fn number_of_unknown_defenses_that_could_counter_player_weapon(unknown_cards: &[TreacheryCard], weapon: Option<TreacheryCard>) -> f32 {
        let weapon = weapon.and_then(CsNull::opt);
        unknown_cards.iter().filter(|c| c.is_defense() && (weapon.is_none() || weapon.unwrap().countered_by(Some(**c), None))).count()
            as i32 as f32
    }

    /// The shared condition of the two properties below; they differ only in the question.
    fn ruling_clairvoyance_for_player_battle(&self, game: &Game, question: ClairvoyanceQuestion) -> Option<ClairVoyanceQandA> {
        let (Some(_), Some(qna)) = (&game.latest_clairvoyance, &game.latest_clairvoyance_qn_a) else {
            return None;
        };
        if qna.answer.initiator() == self.faction(game)
            && game.latest_clairvoyance_battle.is_some()
            && same_battle(&game.latest_clairvoyance_battle, &game.current_battle)
            && {
                let q = qna.question.as_ref().nre().question;
                q == question || q == ClairvoyanceQuestion::CardTypeInBattle
            }
        {
            Some(qna.clone())
        } else {
            None
        }
    }

    pub fn ruling_weapon_clairvoyance_for_player_battle(&self, game: &Game) -> Option<ClairVoyanceQandA> {
        self.ruling_clairvoyance_for_player_battle(game, ClairvoyanceQuestion::CardTypeAsWeaponInBattle)
    }

    pub fn ruling_defense_clairvoyance_for_player_battle(&self, game: &Game) -> Option<ClairVoyanceQandA> {
        self.ruling_clairvoyance_for_player_battle(game, ClairvoyanceQuestion::CardTypeAsDefenseInBattle)
    }

    pub fn chance_of_enemy_leader_dying(&self, game: &Game, player: PlayerIndex, opponent: PlayerIndex, voice: Option<&VoicePlan>, prescience: Option<PrescienceAspect>, most_effective_weapon: &mut Option<TreacheryCard>, enemy_can_defend_poison_tooth: &mut bool) -> f32 {
        *enemy_can_defend_poison_tooth = false;

        if Self::heroes_for_battle(game, opponent, true).is_empty() {
            Self::log_info("Opponent has no leaders", vec![]);
            *most_effective_weapon = None;
            return 0.0;
        }

        if let Some(voice) = voice {
            if voice.weapon_to_use.is_some() {
                *most_effective_weapon = voice.weapon_to_use;
                return if voice.opponent_hero_will_certainly_be_zero { 1.0 } else { 0.5 };
            }
        }

        let available_weapons: Vec<TreacheryCard> = order_by_f32(
            Self::weapons(game, player, None, None, None).into_iter().filter(|w| {
                w.type_() != TreacheryCardType::Useless
                    && w.type_() != TreacheryCardType::ArtilleryStrike
                    && w.type_() != TreacheryCardType::PoisonTooth
                    && w.type_() != TreacheryCardType::Rockmelter
                    && w.type_() != TreacheryCardType::HarassAndWithdraw
                    && w.type_() != TreacheryCardType::Recruits
            }),
            |w| Self::number_of_unknown_defenses_that_could_counter_player_weapon(&Self::cards_unknown_to_player(game, player), Some(*w)),
        );

        let opponent_plan = game.current_battle.as_ref().and_then(|b| b.plan_of(game, Some(opponent)));

        let mut opponent_may_be_using_a_weapon = true;

        //Prescience available?
        if prescience != Some(PrescienceAspect::None) {
            if let Some(opponent_plan) = &opponent_plan {
                if prescience == Some(PrescienceAspect::Defense) {
                    let opponent_defense = opponent_plan.defense();
                    *enemy_can_defend_poison_tooth = opponent_defense.is_some_and(|d| d.is_non_antidote_poison_defense());
                    *most_effective_weapon = available_weapons
                        .iter()
                        .copied()
                        .cs_first_or_default_where(|w| opponent_defense.is_none() || !w.countered_by(opponent_defense, None));

                    return if most_effective_weapon.is_some() { 1.0 } else { 0.0 };
                }

                if prescience == Some(PrescienceAspect::Weapon) && opponent_plan.weapon().is_none_or(|w| w.is_useless()) {
                    opponent_may_be_using_a_weapon = false;
                }
            }
        }

        let useful_weapons: Vec<TreacheryCard> = if opponent_may_be_using_a_weapon {
            available_weapons
        } else {
            available_weapons.into_iter().filter(|w| w.type_() != TreacheryCardType::MirrorWeapon).collect()
        };

        let known_enemy_defenses = Self::known_opponent_defenses(game, player, opponent);

        //Clairvoyance available?
        let my_clairvoyance = Self::my_clairvoyance_about_enemy_defense_in_current_battle(game, player);
        if let Some(my_clairvoyance) = my_clairvoyance {
            let question = my_clairvoyance.question.as_ref().nre();
            if question.is_about(game, TreacheryCardType::ProjectileDefense) {
                let answer = &game.latest_clairvoyance_qn_a.as_ref().nre().answer;
                if answer.is_no() {
                    *enemy_can_defend_poison_tooth = known_enemy_defenses.iter().any(|c| c.is_non_antidote_poison_defense());
                    *most_effective_weapon = useful_weapons.iter().copied().cs_first_or_default_where(|d| d.is_projectile_weapon());
                    if most_effective_weapon.is_some() {
                        return 1.0;
                    }
                } else if answer.is_yes() {
                    *most_effective_weapon = useful_weapons.iter().copied().cs_first_or_default_where(|d| d.is_poison_weapon());
                    if most_effective_weapon.is_some() {
                        return 1.0;
                    }
                }
            } else if question.is_about(game, TreacheryCardType::PoisonDefense) {
                let answer = &game.latest_clairvoyance_qn_a.as_ref().nre().answer;
                if answer.is_no() {
                    *most_effective_weapon = useful_weapons.iter().copied().cs_first_or_default_where(|d| d.is_poison_weapon());
                    if most_effective_weapon.is_some() {
                        return 1.0;
                    }
                } else if answer.is_yes() {
                    *enemy_can_defend_poison_tooth = known_enemy_defenses.iter().any(|c| c.is_non_antidote_poison_defense());
                    *most_effective_weapon = useful_weapons.iter().copied().cs_first_or_default_where(|d| d.is_projectile_weapon());
                    if most_effective_weapon.is_some() {
                        return 1.0;
                    }
                }
            }
        }

        let unknown_opponent_cards = Self::opponent_cards_unknown_to_player(game, player, opponent);

        *most_effective_weapon = useful_weapons
            .iter()
            .copied()
            .filter(|w| !known_enemy_defenses.iter().any(|defense| w.countered_by(Some(*defense), None)))
            .random_or_default(&self.random);

        if let Some(mew) = *most_effective_weapon {
            if mew.is_mirror_weapon() {
                if prescience == Some(PrescienceAspect::Weapon) {
                    if let Some(opponent_plan) = &opponent_plan {
                        return 1.0
                            - self.chance_of_an_unknown_opponent_card_saving_his_leader(
                                game,
                                player,
                                &unknown_opponent_cards,
                                opponent_plan.weapon(),
                                opponent,
                            );
                    }
                }
            } else if unknown_opponent_cards.is_empty() {
                return 1.0;
            } else {
                return 1.0
                    - self.chance_of_an_unknown_opponent_card_saving_his_leader(game, player, &unknown_opponent_cards, Some(mew), opponent);
            }
        }

        // C# parity: a MirrorWeapon chosen above is drawn again here (hazards 18.5).
        *most_effective_weapon =
            useful_weapons.iter().copied().filter(|w| !Self::is_known_to(game, opponent, *w)).random_or_default(&self.random);

        if most_effective_weapon.is_some() {
            return 0.5;
        }

        *enemy_can_defend_poison_tooth = known_enemy_defenses.iter().any(|c| c.is_non_antidote_poison_defense());

        0.0
    }

    pub fn is_allowed_with_clairvoyance(&self, game: &Game, clairvoyance: Option<&ClairVoyanceQandA>, to_use: Option<TreacheryCard>, as_weapon: bool) -> bool {
        let to_use = to_use.and_then(CsNull::opt);
        let in_scope = match (to_use, clairvoyance) {
            (Some(to_use), Some(cv)) => {
                let card_type = to_use.type_();
                ClairVoyancePlayed::is_in_scope_of(
                    as_weapon,
                    card_type,
                    unbox_enum::<TreacheryCardType>(&cv.question.as_ref().nre().parameter1(game)),
                )
            }
            _ => false,
        };

        match clairvoyance {
            None => true,
            Some(cv) => {
                cv.answer.answer == ClairVoyanceAnswer::Unknown
                    || (cv.answer.answer == ClairVoyanceAnswer::Yes && to_use.is_some() && in_scope)
                    || (cv.answer.answer == ClairVoyanceAnswer::No && (to_use.is_none() || !in_scope))
            }
        }
    }

    pub fn heroes_for_battle(game: &Game, player: PlayerIndex, include_in_front_of_shield: bool) -> Vec<Hero> {
        if include_in_front_of_shield {
            Battle::valid_battle_heroes(game, player)
        } else {
            Battle::valid_battle_heroes(game, player).into_iter().filter(|l| !game.is_in_front_of_shield(Some(*l))).collect()
        }
    }

    pub fn select_hero_for_battle(&self, game: &Game, player: PlayerIndex, opponent: PlayerIndex, highest: bool, forfeit: bool, messiah_used: bool, weapon: Option<TreacheryCard>, defense: Option<TreacheryCard>, hero: &mut Option<Hero>, is_traitor: &mut bool, include_in_front_of_shield: bool) -> i32 {
        *is_traitor = false;

        let p = game.player(player);
        let o = game.player(opponent);

        let ally = if p.ally != Faction::None { p.allied_player(game) } else { None };

        // The C# queries are lazy, but nothing in between changes the game, so building them now is equivalent.
        let mut known_non_traitors: Vec<Hero> = Vec::new();
        union_into(&mut known_non_traitors, p.traitors.iter().copied());
        union_into(&mut known_non_traitors, p.known_non_traitors.iter().copied());
        if let Some(a) = ally {
            let a = game.player(a);
            union_into(&mut known_non_traitors, a.traitors.iter().copied());
            union_into(&mut known_non_traitors, a.known_non_traitors.iter().copied());
        }
        for x in game.player_indices() {
            let xp = game.player(x);
            if x != opponent && (xp.ally != Faction::Harkonnen || xp.faction != o.ally) {
                union_into(&mut known_non_traitors, xp.revealed_traitors.iter().copied());
                union_into(&mut known_non_traitors, xp.told_traitors.iter().copied());
            }
        }
        if o.ally != Faction::Harkonnen {
            union_into(&mut known_non_traitors, o.told_non_traitors.iter().copied());
        }

        let mut known_traitors_for_opponents_in_battle: Vec<Hero> = Vec::new();
        for x in game.player_indices() {
            let xp = game.player(x);
            if x == opponent || (xp.faction == Faction::Harkonnen && xp.faction == o.ally) {
                // C# parity: the player's told traitors, not x's.
                union_into(&mut known_traitors_for_opponents_in_battle, xp.revealed_traitors.iter().copied());
                union_into(&mut known_traitors_for_opponents_in_battle, p.told_traitors.iter().copied());
            }
        }
        union_into(
            &mut known_traitors_for_opponents_in_battle,
            p.leaders
                .iter()
                .copied()
                .filter(|l| game.applicable(Rule::CapturedLeadersAreTraitorsToOwnFaction) && l.faction() == o.faction)
                .map(Hero::Leader),
        );

        let has_unknown_traitors_that_might_be_mine = game.player_indices().any(|x| {
            let xp = game.player(x);
            x != opponent
                && (xp.ally != Faction::Harkonnen || xp.faction != o.ally)
                && xp.traitors.iter().any(|l| !known_non_traitors.contains(l))
        });

        let highest_opponent_leader = cs::null_to_none(
            order_by_descending(Self::heroes_for_battle(game, opponent, true), |l| l.value()).into_iter().next(),
        );
        let heroes = || Self::heroes_for_battle(game, player, include_in_front_of_shield);
        let safe_leaders = || -> Vec<Hero> {
            heroes()
                .into_iter()
                .filter(|l| messiah_used || !has_unknown_traitors_that_might_be_mine || known_non_traitors.contains(l))
                .collect()
        };
        let value = |l: &Hero| l.value_in_combat_against(highest_opponent_leader);
        let value_auditor_10 = |l: &Hero| if l.hero_type() == HeroType::Auditor { 10 } else { l.value_in_combat_against(highest_opponent_leader) };

        let safe_hero: Option<Hero>;
        let unsafe_hero: Option<Hero>;

        if forfeit {
            safe_hero = None;
            let safe = safe_leaders();
            unsafe_hero = heroes()
                .into_iter()
                .filter(|l| !safe.contains(l))
                .lowest_or_default(&self.random, value_auditor_10)
                .or_else(|| heroes().into_iter().lowest_or_default(&self.random, value_auditor_10));
        } else if highest {
            safe_hero = safe_leaders().into_iter().highest_or_default(&self.random, value);
            unsafe_hero = heroes().into_iter().highest_or_default(&self.random, value_auditor_10);
        } else {
            safe_hero = safe_leaders().into_iter().lowest_or_default(&self.random, value_auditor_10);
            unsafe_hero = heroes().into_iter().one_of_lowest_n_or_default(&self.random, value_auditor_10, 2);
        }

        *hero = if safe_hero.is_none()
            || (o.faction != Faction::Harkonnen
                && !known_traitors_for_opponents_in_battle.contains(&unsafe_hero.or_null())
                && safe_hero.unwrap().value_in_combat_against(highest_opponent_leader)
                    < unsafe_hero.nre().value_in_combat_against(highest_opponent_leader).wrapping_sub(2))
        {
            unsafe_hero
        } else {
            safe_hero
        };

        *is_traitor = !messiah_used && known_traitors_for_opponents_in_battle.contains(&hero.or_null());

        let mut used_skill = LeaderSkill::None;
        match *hero {
            Some(h) => h.value_in_combat_against(highest_opponent_leader).wrapping_add(
                Battle::determine_skill_bonus_game_player_i_hero_treachery_card_treachery_card_int32_leader_skill(
                    game,
                    player,
                    Some(h),
                    weapon,
                    defense,
                    if p.resources > 3 { 3 } else { 0 },
                    &mut used_skill,
                ),
            ),
            None => 0,
        }
    }

    pub fn get_dial_needed(&self, game: &Game, p: PlayerIndex, territory: Territory, opponent: PlayerIndex, take_reinforcements_into_account: bool) -> f32 {
        //var opponent = GetOpponentThatOccupies(territory);
        let voice_plan = if Voice::may_use_voice(game, p) { Some(self.best_voice(game, None, p, opponent)) } else { None };
        let strength_of_opponent = Self::max_dial(game, opponent, territory, Some(p), false);
        let prescience = if Prescience::may_use_prescience(game, p) {
            self.best_prescience(game, p, opponent, strength_of_opponent, PrescienceAspect::None, territory)
        } else {
            PrescienceAspect::None
        };

        let (mut d, mut w, mut h, mut m, mut t, mut l, mut s, mut b) = (None, None, None, false, false, false, false, 0);
        //More could be done with the information obtained in the below call
        // C# parity: computes for Player, not p (hazards 18.5).
        self.get_dial_needed_for_battle(
            game,
            self.player,
            &self.param,
            self.i_will_be_aggressor_against(game, Some(opponent)),
            opponent,
            territory,
            voice_plan.as_ref(),
            prescience,
            take_reinforcements_into_account,
            true,
            &mut d,
            &mut w,
            &mut h,
            &mut m,
            &mut t,
            &mut l,
            &mut s,
            &mut b,
        )
    }

    pub fn get_dial_needed_territory_player_bool(&self, game: &Game, territory: Territory, opponent: Option<PlayerIndex>, take_reinforcements_into_account: bool) -> f32 {
        let Some(opponent) = opponent.and_then(CsNull::opt) else {
            return 0.0;
        };
        //More could be done with the information obtained in the below call
        self.get_dial_needed(game, self.player, territory, opponent, take_reinforcements_into_account)
    }

    pub fn get_dial_needed_for_battle(&self, game: &Game, player: PlayerIndex, param: &BotParameters, i_am_aggressor: bool, opponent: PlayerIndex, territory: Territory, voice: Option<&VoicePlan>, prescience: PrescienceAspect, take_reinforcements_into_account: bool, include_in_front_of_shield: bool, best_defense: &mut Option<TreacheryCard>, best_weapon: &mut Option<TreacheryCard>, hero: &mut Option<Hero>, messiah: &mut bool, is_traitor: &mut bool, lasgun_shield_detected: &mut bool, stone_burner_detected: &mut bool, banker_boost: &mut i32) -> f32 {
        let mut enemy_can_defend_poison_tooth = false;
        let mut chance_of_enemy_hero_surviving =
            1.0 - self.chance_of_enemy_leader_dying(game, player, opponent, voice, Some(prescience), best_weapon, &mut enemy_can_defend_poison_tooth);

        Self::log_info("Chance of enemy hero surviving: {0} with {1}", vec![]);

        let mut chance_of_my_hero_surviving =
            self.chance_of_my_leader_surviving(game, player, opponent, voice, Some(prescience), best_defense, *best_weapon);

        Self::use_destructive_weapon_if_applicable(
            game,
            player,
            param,
            enemy_can_defend_poison_tooth,
            &mut chance_of_my_hero_surviving,
            &mut chance_of_enemy_hero_surviving,
            best_defense,
            best_weapon,
        );

        let mut i_assume_my_leader_will_die = chance_of_my_hero_surviving < param.battle_mimimum_chance_to_assume_my_leader_survives;
        let mut i_assume_enemy_leader_will_die =
            chance_of_enemy_hero_surviving < param.battle_mimimum_chance_to_assume_enemy_hero_survives;

        let opponent_plan = game.current_battle.as_ref().and_then(|b| b.plan_of(game, Some(opponent)));
        *lasgun_shield_detected = Self::has_lasgun_shield(
            *best_weapon,
            *best_defense,
            if prescience == PrescienceAspect::Weapon { opponent_plan.as_ref().and_then(|p| p.weapon()) } else { None },
            if prescience == PrescienceAspect::Defense { opponent_plan.as_ref().and_then(|p| p.defense()) } else { None },
        );

        *stone_burner_detected = best_weapon.is_some_and(|w| w.is_rock_melter())
            || (prescience == PrescienceAspect::Weapon
                && opponent_plan.as_ref().is_some_and(|p| p.weapon().is_some_and(|w| w.is_rock_melter())));

        let mut my_messiah_bonus = 0i32;
        if Battle::messiah_available_for_battle(game, player) && !*lasgun_shield_detected {
            *messiah = true;
            my_messiah_bonus = 2;
        } else {
            *messiah = false;
        }

        *is_traitor = false;
        let my_hero_value = self.select_hero_for_battle(
            game,
            player,
            opponent,
            !*lasgun_shield_detected && !i_assume_my_leader_will_die,
            false,
            *messiah,
            *best_weapon,
            *best_defense,
            hero,
            is_traitor,
            include_in_front_of_shield,
        );

        let mut used_skill = LeaderSkill::None;
        let opponent_penalty = Battle::determine_skill_penalty_game_i_hero_player_leader_skill(game, *hero, opponent, &mut used_skill);

        *banker_boost = 0;
        if hero.is_none() {
            *messiah = false;
            my_messiah_bonus = 0;
        }

        if Self::can_only_use_traitors_or_facedancers(game, opponent) {
            Self::log_info("Opponent leader only has traitors or facedancers to use!", vec![]);
            *best_weapon = None;
            *best_defense = None;
            return 0.5;
        }

        if *is_traitor {
            Self::log_info("My leader is a traitor: chanceOfMyHeroSurviving = 0", vec![]);
            *best_weapon = None;
            *best_defense = None;
            return 99.0;
        }

        if *lasgun_shield_detected {
            Self::log_info("Lasgun/Shield detected!", vec![]);
            if best_weapon.is_some_and(|w| !w.is_laser()) && Self::may_play_no_weapon(game, player, *best_defense) {
                *best_weapon = None;
            }
            return 0.5;
        }

        if game.skilled_as(hero.or_null(), LeaderSkill::Banker) && !i_assume_my_leader_will_die {
            *banker_boost = game.player(player).resources.min(3);
        }

        if hero.is_some_and(|h| h.as_treachery_card().is_some())
            && best_defense.is_some_and(|d| !d.is_useless())
            && Self::may_play_no_defense(game, player, *best_weapon)
        {
            *best_defense = None;
        }

        Self::log_info("Chance of my hero surviving: {0} with {1} (my weapon: {2})", vec![]);

        let my_hero_to_fight_against = *hero;
        let opponent_leader = if prescience == PrescienceAspect::Leader && opponent_plan.is_some() {
            opponent_plan.as_ref().unwrap().hero()
        } else {
            let traitors = &game.player(player).traitors;
            order_by_descending(Self::heroes_for_battle(game, opponent, true), |l| l.value_in_combat_against(my_hero_to_fight_against))
                .into_iter()
                .cs_first_or_default_where(|l| !traitors.contains(l))
        };
        let opponent_leader_value = opponent_leader.map(|l| l.value_in_combat_against(*hero)).unwrap_or(0);
        let opponent_messiah_bonus = if Battle::messiah_available_for_battle(game, opponent) { 2 } else { 0 };
        let max_reinforcements = if take_reinforcements_into_account {
            (Self::max_reinforced_dial_to(game, player, opponent, territory) as f64).ceil() as i32
        } else {
            0
        };
        let opponent_dial = if prescience == PrescienceAspect::Dial && opponent_plan.is_some() {
            opponent_plan.as_ref().unwrap().dial(game, game.player(player).faction)
        } else {
            Self::max_dial(game, opponent, territory, Some(player), false)
        };
        let my_homeworld_bonus = game.player(player).get_homeworld_battle_contribution_and_lasgun_shield_limit(game, territory);
        let opponent_homeworld_bonus = game.player(opponent).get_homeworld_battle_contribution_and_lasgun_shield_limit(game, territory);

        let aggressor_term: f32 = if i_am_aggressor { 0.0 } else { 0.5 };
        let enemy_term =
            (if i_assume_enemy_leader_will_die { 0i32 } else { 1 }).wrapping_mul(opponent_leader_value.wrapping_add(opponent_messiah_bonus));
        let my_term = (if i_assume_my_leader_will_die { 0i32 } else { 1 })
            .wrapping_mul(my_hero_value.wrapping_add(opponent_penalty).wrapping_add(my_messiah_bonus));

        let mut result = opponent_dial + opponent_homeworld_bonus as f32 + max_reinforcements as f32 + enemy_term as f32 + aggressor_term
            - my_term as f32
            - my_homeworld_bonus as f32;

        if Self::max_dial(game, player, territory, Some(opponent), false) - result >= 5.0 {
            //I think I only need a small fraction of available forces.

            if !i_assume_my_leader_will_die && (chance_of_my_hero_surviving as f64) < 0.8 {
                i_assume_my_leader_will_die = true;
            }

            if i_assume_enemy_leader_will_die && (chance_of_enemy_hero_surviving as f64) > 0.1 {
                i_assume_enemy_leader_will_die = false;
            }

            let enemy_term = (if i_assume_enemy_leader_will_die { 0i32 } else { 1 })
                .wrapping_mul(opponent_leader_value.wrapping_add(opponent_messiah_bonus));
            let my_term = (if i_assume_my_leader_will_die { 0i32 } else { 1 })
                .wrapping_mul(my_hero_value.wrapping_add(opponent_penalty).wrapping_add(my_messiah_bonus));

            result = opponent_dial + max_reinforcements as f32 + enemy_term as f32 + aggressor_term - my_term as f32;
        }

        // LogInfo arguments are evaluated: opponent.Faction.
        let _ = game.player(opponent).faction;
        Self::log_info("{13}/{14}: opponentDial ({0}) + ...", vec![]);

        result
    }

    pub fn can_only_use_traitors_or_facedancers(game: &Game, p: PlayerIndex) -> bool {
        let pl = game.player(p);
        pl.leaders.iter().all(|l| {
            let l = Hero::Leader(*l);
            (!pl.messiah_available(game) && pl.traitors.contains(&l))
                || pl.face_dancers.contains(&l)
                || (!pl.messiah_available(game)
                    && pl.ally == Faction::Harkonnen
                    && game.player(pl.allied_player(game).or_null()).traitors.contains(&l))
                || (pl.ally == Faction::Tleilaxu && game.player(pl.allied_player(game).or_null()).face_dancers.contains(&l))
        })
    }

    pub fn has_lasgun_shield(my_weapon: Option<TreacheryCard>, my_defense: Option<TreacheryCard>, enemy_weapon: Option<TreacheryCard>, enemy_defense: Option<TreacheryCard>) -> bool {
        (my_weapon.is_some_and(|w| w.is_laser()) && enemy_defense.is_some_and(|d| d.is_shield()))
            || (enemy_weapon.is_some_and(|w| w.is_laser()) && my_defense.is_some_and(|d| d.is_shield()))
    }

    /// Not pure: `DetermineBattleOutcome` runs on the live plans and changes the game
    /// (bots-plan.md, "Bots are not pure").
    pub fn determine_rock_was_melted(&self, game: &mut Game) -> Rc<RockWasMelted> {
        let agg = game.aggressor_plan.clone();
        let def = game.defender_plan.clone();
        let territory = game.current_battle.as_ref().nre().territory(game).or_null();
        let outcome = Battle::determine_battle_outcome(agg.as_ref().nre(), def.as_ref().nre(), territory, game);
        let _ = outcome.get_message(game);
        Self::log_info("outcome", vec![]);
        let mut result = RockWasMelted::new(game, self.faction(game));
        result.kill = outcome.winner == Some(self.player);
        Rc::new(result)
    }

    pub fn determine_residual_played(&self, game: &Game) -> Rc<ResidualPlayed> {
        Rc::new(ResidualPlayed::new(game, self.faction(game)))
    }

    pub fn determine_portable_antidote_used(&self, game: &Game) -> Option<Rc<PortableAntidoteUsed>> {
        let current_battle = game.current_battle.as_ref().nre();
        let opponent = current_battle.opponent_of(game, self.player);
        let opponent_plan = current_battle.plan_of(game, opponent);
        let my_plan = current_battle.plan_of(game, Some(self.player));
        let portable_antidote = game.player(self.player).treachery_cards.iter().copied().cs_first_or_default_where(|c| c.is_portable_antidote());

        let my_plan = my_plan.as_ref().nre();
        if my_plan.defense().is_none() {
            let opponent_plan = opponent_plan.as_ref().nre();
            if let Some(ow) = opponent_plan.weapon() {
                if ow.countered_by(portable_antidote, my_plan.weapon()) {
                    return Some(Rc::new(PortableAntidoteUsed::new(game, self.faction(game))));
                }
            }
        }

        None
    }

    pub fn determine_thought(&self, game: &Game) -> Option<Rc<Thought>> {
        let opponent = game.current_battle.as_ref().nre().opponent_of(game, self.player);
        let valid_cards = order_by_descending(Thought::valid_cards(game), |c| self.card_quality(game, *c, opponent));

        if valid_cards.is_empty() {
            return None;
        }

        let card = valid_cards
            .iter()
            .copied()
            .cs_first_or_default_where(|x| !Self::is_known_to(game, self.player, *x))
            .or_else(|| cs::null_to_none(valid_cards.first().copied()));

        let mut result = Thought::new(game, self.faction(game));
        result.set_card(card);
        Some(Rc::new(result))
    }

    pub fn determine_thought_answered(&self, game: &Game) -> Rc<ThoughtAnswered> {
        let mut result = ThoughtAnswered::new(game, self.faction(game));
        let card = ThoughtAnswered::valid_cards(game, self.player)
            .into_iter()
            .lowest_or_default(&self.random, |c| self.card_quality(game, *c, Some(self.player)));
        result.set_card(card);
        Rc::new(result)
    }

    pub fn determine_hms_advantage_chosen(&self, game: &Game) -> Rc<HMSAdvantageChosen> {
        let mut adv = StrongholdAdvantage::None;
        // FirstOrDefault on a list of enums: default(StrongholdAdvantage) is None.
        let first = |pred: &dyn Fn(StrongholdAdvantage) -> bool| {
            HMSAdvantageChosen::valid_advantages(game, self.player).into_iter().find(|a| pred(*a)).unwrap_or(StrongholdAdvantage::None)
        };

        let plan = self.determine_battle_plan(game, false);
        if !plan.has_poison() && !plan.has_antidote() {
            adv = first(&|a| a == StrongholdAdvantage::CountDefensesAsAntidote);
        }
        if adv == StrongholdAdvantage::None && self.resources(game) < 5 {
            adv = first(&|a| a == StrongholdAdvantage::FreeResourcesForBattles);
        }
        if adv == StrongholdAdvantage::None {
            adv = first(&|a| a == StrongholdAdvantage::CollectResourcesForDial);
        }
        if adv == StrongholdAdvantage::None && !plan.has_useless() {
            adv = first(&|a| a == StrongholdAdvantage::CollectResourcesForUseless);
        }
        if adv == StrongholdAdvantage::None {
            adv = first(&|a| a == StrongholdAdvantage::FreeResourcesForBattles);
        }
        if adv == StrongholdAdvantage::None {
            adv = first(&|a| a == StrongholdAdvantage::WinTies);
        }
        if adv == StrongholdAdvantage::None {
            adv = first(&|_| true);
        }

        let mut result = HMSAdvantageChosen::new(game, self.faction(game));
        result.advantage = adv;
        Rc::new(result)
    }

    pub fn determine_retreat(&self, game: &Game) -> Option<Rc<Retreat>> {
        let mut forces_to_retreat = Retreat::max_forces(game, self.player);
        let mut special_forces_to_retreat = Retreat::max_special_forces(game, self.player);
        let max_forces = Retreat::max_total_forces(game, self.player);
        while forces_to_retreat.wrapping_add(special_forces_to_retreat) > max_forces {
            if forces_to_retreat > 0 {
                forces_to_retreat -= 1;
            } else {
                special_forces_to_retreat = special_forces_to_retreat.wrapping_sub(1);
            }
        }

        let to = Retreat::valid_targets(game, self.player)
            .filter(|l| self.resources_in(game, *l) > 0)
            .highest_or_default(&self.random, |l| self.resources_in(game, *l))
            .or_else(|| Retreat::valid_targets(game, self.player).cs_first_or_default_where(|l| l.is_protected_from_storm()))
            .or_else(|| cs::null_to_none(Retreat::valid_targets(game, self.player).next()));

        if forces_to_retreat > 0 || special_forces_to_retreat > 0 {
            let mut result = Retreat::new(game, self.faction(game));
            result.set_location(game, to);
            result.forces = forces_to_retreat;
            result.special_forces = special_forces_to_retreat;
            return Some(Rc::new(result));
        }

        None
    }

    pub fn determine_loser_concluded(&self, game: &Game) -> Rc<LoserConcluded> {
        let to_keep = cs::null_to_none(
            order_by_descending(
                LoserConcluded::cards_loser_may_keep(game).into_iter().filter(|c| self.card_quality(game, *c, Some(self.player)) > 2),
                |c| self.card_quality(game, *c, Some(self.player)),
            )
            .into_iter()
            .next(),
        );
        let mut result = LoserConcluded::new(game, self.faction(game));
        result.set_kept_card(to_keep);
        result.assassinate = LoserConcluded::can_assassinate(game, self.player);
        Rc::new(result)
    }
}
