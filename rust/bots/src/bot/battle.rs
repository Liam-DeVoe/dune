use std::cmp::Reverse;

use crate::prelude::*;
use crate::support::unbox_enum;
use dotnet_compat::math::cmp_f32;

/// Whether `battle` is the battle being fought now (the same object). No battle matches no
/// current battle.
pub(crate) fn is_current_battle(game: &Game, battle: Option<&Rc<BattleInitiated>>) -> bool {
    match (battle, game.current_battle.as_ref()) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

/// The dial I need to win a battle, and the leader and cards I'd fight with.
struct DialEstimate {
    dial_needed: f32,
    weapon: Option<TreacheryCard>,
    defense: Option<TreacheryCard>,
    hero: Option<Hero>,
    messiah: bool,
    is_traitor: bool,
    lasgun_shield: bool,
    stone_burner: bool,
    banker_boost: i32,
}

/// The forces to dial at full and half strength, and the dial still missing with them.
struct ForceDial {
    forces: i32,
    forces_at_half_strength: i32,
    special_forces: i32,
    special_forces_at_half_strength: i32,
    shortage: f32,
}

/// The weapon to use against the enemy leader and the chance that it kills them.
struct WeaponChoice {
    weapon: Option<TreacheryCard>,
    kill_chance: f32,
    enemy_can_defend_poison_tooth: bool,
}

/// The leader to fight with and the strength it adds to my dial.
struct HeroChoice {
    hero: Option<Hero>,
    value: i32,
    is_traitor: bool,
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

        if switchable_leader_is_behind_shield ^ switchable_leader_must_be_used {
            return Some(Rc::new(SwitchedSkilledLeader::new(game, self.faction(game))));
        }

        None
    }

    pub fn determine_battle_initiated(&self, game: &Game) -> Rc<BattleInitiated> {
        // The battle where my dial is strongest compared with my opponent's.
        let (_, battle) = Battle::battles_to_be_fought(game, self.player, false)
            .into_iter()
            .map(|b| {
                let opponent = game.get_player(Some(b.faction));
                let advantage = Self::max_dial(game, opponent.or_null(), b.territory, Some(self.player), false)
                    - Self::max_dial(game, self.player, b.territory, opponent, false);
                (advantage, b)
            })
            .min_by(|a, b| cmp_f32(a.0, b.0))
            .nre();

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
            let discard_if_useless = |card: &TreacheryCard| {
                me.has(Some(*card))
                    && card.type_() == TreacheryCardType::Useless
                    && faction != Faction::Choam
                    && !game.skilled_as_player_leader_skill(self.player, LeaderSkill::Warmaster)
                    && !game.owns_stronghold(faction, game.map.tueks_sietch())
            };
            discarded.extend(plan.weapon().filter(discard_if_useless));
            discarded.extend(plan.defense().filter(discard_if_useless));
        }

        let kill = match game.harkonnen_victim {
            Some(v) => !game.is_skilled(Some(Hero::Leader(v))) && Hero::Leader(v).value() < 4,
            None => false,
        };

        let replaced_special_forces = BattleConcluded::valid_replacement_force_amounts(game, self.player).into_iter().cs_max();

        let mut new_traitor: Option<Hero> = None;
        let mut to_replace: Option<Hero> = None;
        if !game.traitors_decipherer_can_look_at.is_empty() && game.decipherer_may_replace_traitor {
            // The most valuable hero card, or living leader of another faction, I may take as a traitor.
            new_traitor = game
                .traitors_decipherer_can_look_at
                .iter()
                .copied()
                .filter(|t| t.as_treachery_card().is_some() || (t.faction() != faction && game.is_alive(*t)))
                .min_by_key(|t| Reverse(t.value()));

            if let Some(new_traitor) = new_traitor {
                // A traitor of my own faction, else my cheapest dead one, else my cheapest one worth
                // less than the new traitor.
                let mut replaceable: Vec<Hero> = BattleConcluded::valid_traitors_to_replace(game, self.player).collect();
                to_replace = replaceable.iter().copied().find(|t| t.faction() == faction);
                replaceable.sort_by_key(|t| t.value());
                to_replace = to_replace
                    .or_else(|| replaceable.iter().copied().find(|t| t.as_treachery_card().is_none() && !game.is_alive(*t)))
                    .or_else(|| {
                        replaceable.iter().copied().find(|t| t.as_treachery_card().is_none() && t.value() < new_traitor.value())
                    });
            }
        }

        if to_replace.is_none() {
            new_traitor = None;
        }

        let mut result = BattleConcluded::new(game, faction);
        result.set_discarded_cards(&discarded);
        result.stolen_token = game.player(opponent.nre()).tech_tokens.first().copied().unwrap_or(TechToken::None);
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
        let current_battle = game.current_battle.as_ref().nre();
        let opponent = current_battle.opponent_of(game, self.player);
        let territory = current_battle.territory(game);

        if self.decided_shipment_action == ShipmentDecision::DummyShipment {
            return self.construct_lost_battle_minimizing_losses(game, opponent.or_null(), territory.or_null());
        }

        let forces_available = Battle::max_forces(game, self.player, false);
        let special_forces_available = Battle::max_forces(game, self.player, true);
        let voice = self.voice_plan.as_ref().filter(|v| is_current_battle(game, v.battle.as_ref()));

        let DialEstimate { dial_needed, mut weapon, mut defense, hero, messiah, is_traitor, lasgun_shield, stone_burner, banker_boost } =
            self.get_dial_needed_for_battle(
                game,
                self.i_will_be_aggressor_against(game, opponent),
                opponent.or_null(),
                territory.or_null(),
                voice,
                self.my_prescience(game).map(|p| p.aspect).unwrap_or(PrescienceAspect::None),
                false,
                include_leader_in_front_of_shield,
            );
        let dial_to_reach = if lasgun_shield {
            0.5
        } else if stone_burner {
            0.0
        } else {
            dial_needed
        };

        let resources_from_ally = if self.ally(game) == Faction::Choam { game.resources_your_ally_can_pay(self.player) } else { 0 };
        let opponent_faction = game.player(opponent.or_null()).faction;
        let mut forces = self.forces_for_dial(
            game,
            dial_to_reach,
            opponent_faction,
            forces_available,
            special_forces_available,
            self.resources(game) + resources_from_ally - banker_boost,
            territory.or_null(),
        );

        if forces.shortage <= 3.0 && (weapon.is_none() || defense.is_none()) {
            let reinforcements = Self::weapons(game, self.player, defense, hero, territory)
                .into_iter()
                .find(|c| c.type_() == TreacheryCardType::Reinforcements);
            if reinforcements.is_some() {
                if weapon.is_none() {
                    weapon = reinforcements;
                } else {
                    defense = reinforcements;
                }

                forces.shortage -= 3.0;
            }
        }

        let total_forces =
            forces.forces + forces.forces_at_half_strength + forces.special_forces + forces.special_forces_at_half_strength;
        let minimize_spending_in_this_lost_fight = self.win_was_predicted_by_me_this_turn(game, opponent_faction)
            || (is_traitor && !messiah)
            || (self.resources(game) + self.resources_from_ally(game) < 10
                && total_forces <= 8
                && forces.shortage > self.param.battle_dial_shortage_threshold_for_throwing);

        if minimize_spending_in_this_lost_fight {
            return self.construct_lost_battle_minimizing_losses(game, opponent.or_null(), territory.or_null());
        }

        let may_play_useless = !self.may_use_useless_as_karma(game) && self.faction(game) != Faction::Choam;
        if weapon.is_none() && may_play_useless {
            weapon = Self::useless_as_weapon(game, self.player, defense);
        }
        if defense.is_none() && may_play_useless {
            defense = Self::useless_as_defense(game, self.player, weapon);
        }

        self.remove_illegal_choices(game, hero, &mut weapon, &mut defense, territory);
        self.avoid_lasgun_shield_explosion(game, &mut weapon, &mut defense);

        let mut paid_by_arrakeen = 0;
        let cost = Battle::cost_game_player_int32_int32_int32(game, self.player, forces.forces, forces.special_forces, &mut paid_by_arrakeen);
        let mut result = Battle::new(game, self.faction(game));
        result.set_hero(hero);
        result.messiah = messiah;
        result.forces = forces.forces;
        result.forces_at_half_strength = forces.forces_at_half_strength;
        result.special_forces = forces.special_forces;
        result.special_forces_at_half_strength = forces.special_forces_at_half_strength;
        result.ally_contribution_amount = cost
            .min(resources_from_ally.min(Battle::max_ally_resources(game, self.player, forces.forces, forces.special_forces)));
        result.set_defense(defense);
        result.set_weapon(weapon);
        result.banker_bonus = banker_boost;
        Rc::new(result)
    }

    /// A weapon worth playing when my leader will probably die anyway: a Rock Melter, else an
    /// Artillery Strike, else a Poison Tooth if the enemy can't defend against it.
    fn destructive_weapon(game: &Game, player: PlayerIndex, defense: Option<TreacheryCard>, enemy_can_defend_poison_tooth: bool) -> Option<TreacheryCard> {
        let weapons = Self::weapons(game, player, defense, None, None);
        let of_type = |t: TreacheryCardType| weapons.iter().copied().find(|c| c.type_() == t);
        of_type(TreacheryCardType::Rockmelter)
            .or_else(|| of_type(TreacheryCardType::ArtilleryStrike))
            .or_else(|| if enemy_can_defend_poison_tooth { None } else { of_type(TreacheryCardType::PoisonTooth) })
    }

    pub fn construct_lost_battle_minimizing_losses(&self, game: &Game, opponent: PlayerIndex, territory: Territory) -> Rc<Battle> {
        // A hero card if I have one, else my least valuable leader.
        let hero = Battle::valid_battle_heroes(game, self.player)
            .into_iter()
            .find(|h| h.as_treachery_card().is_some())
            .or_else(|| self.select_hero_for_battle(game, self.player, opponent, false, true, false, None, None, false).hero);

        let play_cards = hero.is_some() && !self.may_use_useless_as_karma(game) && self.faction(game) != Faction::Choam;
        let mut weapon = if play_cards { Self::useless_as_weapon(game, self.player, None) } else { None };
        let mut defense = if play_cards { Self::useless_as_defense(game, self.player, weapon) } else { None };

        self.remove_illegal_choices(game, hero, &mut weapon, &mut defense, territory.opt());

        let mut harass = false;
        let me = game.player(self.player);
        let is_harass = |c: &TreacheryCard| c.type_() == TreacheryCardType::HarassAndWithdraw;
        if me.any_forces_in_territory(territory.opt()) >= 4 {
            if weapon.is_none() && Self::weapons(game, self.player, defense, hero, territory.opt()).iter().any(is_harass) {
                weapon = Some(me.treachery_cards.iter().copied().cs_first_where(is_harass));
                harass = true;
            } else if defense.is_none() && Self::defenses(game, self.player, weapon, territory.opt()).iter().any(is_harass) {
                defense = Some(me.treachery_cards.iter().copied().cs_first_where(is_harass));
                harass = true;
            }
        }

        let messiah = hero.is_some() && Battle::messiah_may_be_used_in_battle(game, self.player);

        let stronghold_free_forces = if game.has_stronghold_advantage(
            self.faction(game),
            StrongholdAdvantage::FreeResourcesForBattles,
            game.current_battle.as_ref().nre().territory(game).or_null(),
        ) {
            2
        } else {
            0
        };
        let max_forces = Battle::max_forces(game, self.player, false);
        let max_special_forces = Battle::max_forces(game, self.player, true);
        let special_at_full = stronghold_free_forces.min(max_special_forces);
        let normal_at_full = (stronghold_free_forces - special_at_full).min(max_forces);

        // When forces cost spice, only the ones the stronghold pays for fight at full strength.
        let must_pay = Battle::must_pay_for_any_forces_in_battle(game, self.player)
            && (special_at_full <= 0 || Battle::must_pay_for_special_forces_in_battle(game, self.player));
        let (forces, forces_at_half_strength, special_forces, special_forces_at_half_strength) = if harass {
            (0, 0, 0, 0)
        } else if must_pay {
            (normal_at_full, max_forces - normal_at_full, special_at_full, max_special_forces - special_at_full)
        } else {
            (max_forces, 0, max_special_forces, 0)
        };

        let mut result = Battle::new(game, self.faction(game));
        result.set_hero(hero);
        result.forces = forces;
        result.forces_at_half_strength = forces_at_half_strength;
        result.special_forces = special_forces;
        result.special_forces_at_half_strength = special_forces_at_half_strength;
        result.set_defense(defense);
        result.set_weapon(weapon);
        result.banker_bonus = 0;
        result.messiah = messiah;
        Rc::new(result)
    }

    /// Replaces cards that the clairvoyance answer forbids or that can't be played together. Three
    /// rounds, because each replacement can make another card illegal.
    fn remove_illegal_choices(&self, game: &Game, hero: Option<Hero>, weapon: &mut Option<TreacheryCard>, defense: &mut Option<TreacheryCard>, territory: Option<Territory>) {
        let territory = territory.and_then(CsNull::opt);

        for _ in 0..3 {
            if hero.is_none() {
                *defense = None;
                *weapon = None;
            }

            let weapon_clairvoyance = self.ruling_weapon_clairvoyance(game);
            if weapon_clairvoyance.is_some() && !self.is_allowed_with_clairvoyance(game, weapon_clairvoyance.as_ref(), *weapon, true) {
                *weapon = Self::weapons(game, self.player, *defense, hero, territory)
                    .into_iter()
                    .find(|c| self.is_allowed_with_clairvoyance(game, weapon_clairvoyance.as_ref(), Some(*c), true));
            }

            let def_clairvoyance = self.ruling_defense_clairvoyance(game);
            if def_clairvoyance.is_some() && !self.is_allowed_with_clairvoyance(game, def_clairvoyance.as_ref(), *defense, false) {
                *defense = Self::defenses(game, self.player, *weapon, territory)
                    .into_iter()
                    .find(|c| self.is_allowed_with_clairvoyance(game, def_clairvoyance.as_ref(), Some(*c), false));
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

            if !Battle::valid_weapons(game, self.player, *defense, hero, territory, true).contains(weapon) {
                *weapon = Self::weapons(game, self.player, *defense, hero, territory)
                    .into_iter()
                    .find(|w| w.type_() != TreacheryCardType::Chemistry);
            }

            if !Battle::valid_defenses(game, self.player, *weapon, territory, true).contains(defense) {
                *defense = Self::defenses(game, self.player, *weapon, territory)
                    .into_iter()
                    .find(|w| w.type_() != TreacheryCardType::WeirdingWay);
            }
        }
    }

    fn avoid_lasgun_shield_explosion(&self, game: &Game, weapon: &mut Option<TreacheryCard>, defense: &mut Option<TreacheryCard>) {
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
        self.forces_for_dial(game, dial_needed, opponent, forces_available, special_forces_available, resources_available, territory)
            .shortage
    }

    /// How many forces to dial at full and at half strength to reach `dial_needed` with
    /// `resources_available` spice, and how much dial is still missing with them.
    fn forces_for_dial(&self, game: &Game, mut dial_needed: f32, opponent: Faction, mut forces_available: i32, mut special_forces_available: i32, resources_available: i32, territory: Territory) -> ForceDial {
        let faction = self.faction(game);
        let normal_strength = Battle::determine_normal_force_strength(game, faction);
        let special_strength = Battle::determine_special_force_strength(game, faction, opponent);
        let stronghold_bonus =
            if game.has_stronghold_advantage(faction, StrongholdAdvantage::FreeResourcesForBattles, territory) { 2 } else { 0 };
        let mut spice_left = resources_available + stronghold_bonus;
        let cost_per_force = Battle::normal_force_cost(game, self.player);
        let cost_per_special_force = Battle::special_force_cost(game, self.player);

        // With Emperor's cunning, my first five normal forces are as strong as special forces.
        let mut forces_with_cunning_bonus = match &game.current_emperor_cunning {
            Some(c) if c.initiator() == faction => 5,
            _ => 0,
        };
        let mut next_force_strength = || {
            if forces_with_cunning_bonus > 0 {
                forces_with_cunning_bonus -= 1;
                special_strength
            } else {
                normal_strength
            }
        };

        let mut result =
            ForceDial { forces: 0, forces_at_half_strength: 0, special_forces: 0, special_forces_at_half_strength: 0, shortage: 0.0 };

        if Battle::must_pay_for_any_forces_in_battle(game, self.player) {
            while dial_needed > normal_strength && special_forces_available >= 1 && spice_left >= cost_per_special_force {
                dial_needed -= special_strength;
                result.special_forces += 1;
                special_forces_available -= 1;
                spice_left -= cost_per_special_force;
            }

            while dial_needed >= normal_strength && forces_available >= 1 && spice_left >= cost_per_force {
                dial_needed -= next_force_strength();
                result.forces += 1;
                forces_available -= 1;
                spice_left -= cost_per_force;
            }

            while dial_needed > 0.0 && special_forces_available >= 1 {
                dial_needed -= 0.5f32 * special_strength;
                result.special_forces_at_half_strength += 1;
                special_forces_available -= 1;
            }

            while dial_needed > 0.0 && forces_available >= 1 {
                dial_needed -= 0.5f32 * next_force_strength();
                result.forces_at_half_strength += 1;
                forces_available -= 1;
            }
        } else {
            while dial_needed >= special_strength && special_forces_available >= 1 && spice_left >= cost_per_special_force {
                dial_needed -= special_strength;
                result.special_forces += 1;
                special_forces_available -= 1;
                spice_left -= cost_per_special_force;
            }

            while dial_needed > 0.0 && forces_available >= 1 && spice_left >= cost_per_force {
                dial_needed -= next_force_strength();
                result.forces += 1;
                forces_available -= 1;
                spice_left -= cost_per_force;
            }

            while dial_needed > 0.0 && special_forces_available >= 1 && spice_left >= cost_per_special_force {
                dial_needed -= special_strength;
                result.special_forces += 1;
                special_forces_available -= 1;
                spice_left -= cost_per_special_force;
            }
        }

        result.shortage = dial_needed;
        result
    }

    /// The defenses `player` would consider next to `chosen_weapon`: not the weapon itself, no
    /// shield next to a lasgun, no Weirding Way without a weapon, and nothing useless.
    fn usable_defenses(game: &Game, player: PlayerIndex, chosen_weapon: Option<TreacheryCard>, territory: Option<Territory>) -> Vec<TreacheryCard> {
        Self::defenses(game, player, chosen_weapon, territory)
            .into_iter()
            .filter(|def| {
                Some(*def) != chosen_weapon
                    && !(chosen_weapon.is_some_and(|w| w.is_laser()) && def.is_shield())
                    && (def.type_() != TreacheryCardType::WeirdingWay || chosen_weapon.is_some())
                    && def.type_() != TreacheryCardType::Useless
            })
            .collect()
    }

    /// The chance that my leader survives, and the defense that gives that chance.
    fn chance_of_my_leader_surviving(&self, game: &Game, player: PlayerIndex, opponent: PlayerIndex, voice_plan: Option<&VoicePlan>, prescience: Option<PrescienceAspect>, chosen_weapon: Option<TreacheryCard>) -> (f32, Option<TreacheryCard>) {
        let chosen_weapon = chosen_weapon.and_then(CsNull::opt);
        if Self::heroes_for_battle(game, opponent, true).is_empty() {
            return (1.0, None);
        }

        if let Some(vp) = voice_plan.filter(|vp| vp.defense_to_use.is_some()) {
            return (if vp.player_hero_will_certainly_survive { 1.0 } else { 0.5 }, vp.defense_to_use);
        }

        let available_defenses =
            Self::usable_defenses(game, player, chosen_weapon, game.current_battle.as_ref().and_then(|b| b.territory(game)));
        let first_defense = |pred: &dyn Fn(TreacheryCard) -> bool| available_defenses.iter().copied().find(|d| pred(*d));

        if chosen_weapon.is_some_and(|w| w.is_artillery()) {
            return (0.0, first_defense(&|d| d.is_shield()));
        }

        if chosen_weapon.is_some_and(|w| w.is_rock_melter()) {
            return (0.0, None);
        }

        if chosen_weapon.is_some_and(|w| w.is_poison_tooth()) {
            return (0.0, first_defense(&|d| d.type_() == TreacheryCardType::Chemistry));
        }

        let opponent_plan = game.current_battle.as_ref().and_then(|b| b.plan_of(game, Some(opponent)));
        if let Some(opponent_plan) = opponent_plan.filter(|_| prescience == Some(PrescienceAspect::Weapon)) {
            return match opponent_plan.weapon().filter(|w| w.type_() != TreacheryCardType::Useless) {
                None => {
                    let defense = if Self::may_play_no_defense(game, player, chosen_weapon) {
                        None
                    } else {
                        Self::defenses(game, player, chosen_weapon, None).first().copied()
                    };
                    (1.0, defense)
                }
                Some(opponent_weapon) => {
                    let defense = first_defense(&|d| opponent_weapon.countered_by(Some(d), chosen_weapon));
                    (if defense.is_some() { 1.0 } else { 0.0 }, defense)
                }
            };
        }

        if let Some(my_clairvoyance) = Self::player_clairvoyance_about_enemy_weapon_in_current_battle(game, player) {
            let question = my_clairvoyance.question.as_ref().nre();
            let answer = &my_clairvoyance.answer;
            // For each weapon type asked about: the defense against it, and the one against the other type.
            let defenses_by_weapon: [(TreacheryCardType, fn(TreacheryCard) -> bool, fn(TreacheryCard) -> bool); 2] = [
                (TreacheryCardType::Projectile, TreacheryCard::is_projectile_defense, TreacheryCard::is_poison_defense),
                (TreacheryCardType::Poison, TreacheryCard::is_poison_defense, TreacheryCard::is_projectile_defense),
            ];
            for (weapon_type, counter, counter_other) in defenses_by_weapon {
                if !question.is_about(game, weapon_type) {
                    continue;
                }

                if answer.is_yes() {
                    let defense = first_defense(&counter);
                    return (if defense.is_some() { 1.0 } else { 0.0 }, defense);
                }

                if answer.is_no() {
                    if let Some(defense) = first_defense(&counter_other) {
                        return (0.5, Some(defense));
                    }
                }
            }
        }

        let mut defense = None;
        let chance = self.determine_best_defense(game, player, opponent, chosen_weapon, &mut defense);
        (chance, defense)
    }

    pub fn nr_of_unknown_opponent_cards(game: &Game, player: PlayerIndex, opponent: PlayerIndex) -> i32 {
        let cards = &game.player(opponent).treachery_cards;
        if cards.is_empty() {
            return 0;
        }
        let known = game.known_cards(player);
        cards.iter().filter(|c| !known.contains(c)).count() as i32
    }

    /// Picks into `most_effective_defense` the defense that counters the most known and unknown
    /// enemy weapons, and returns the chance that my leader survives with it.
    pub fn determine_best_defense(&self, game: &Game, player: PlayerIndex, opponent: PlayerIndex, chosen_weapon: Option<TreacheryCard>, most_effective_defense: &mut Option<TreacheryCard>) -> f32 {
        let chosen_weapon = chosen_weapon.and_then(CsNull::opt);
        let known_enemy_weapons = Self::known_opponent_weapons(game, player, opponent);
        let available_defenses = Self::usable_defenses(game, player, chosen_weapon, None);

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

    fn chance_of_an_unknown_opponent_card_killing_my_leader(&self, game: &Game, player: PlayerIndex, unknown_cards: &[TreacheryCard], used_defense: Option<TreacheryCard>, opponent: PlayerIndex, chosen_weapon: Option<TreacheryCard>) -> f32 {
        if unknown_cards.is_empty() {
            return 0.0;
        }
        let killers = Self::number_of_unknown_weapons_that_could_kill_me_with_player_defense(unknown_cards, used_defense, chosen_weapon);
        Self::chance_opponent_holds_one_of(game, player, opponent, unknown_cards, killers)
    }

    fn chance_of_an_unknown_opponent_card_saving_his_leader(&self, game: &Game, player: PlayerIndex, unknown_cards: &[TreacheryCard], used_weapon: Option<TreacheryCard>, opponent: PlayerIndex) -> f32 {
        if unknown_cards.is_empty() {
            return 0.0;
        }
        let savers = Self::number_of_unknown_defenses_that_could_counter_player_weapon(unknown_cards, used_weapon);
        Self::chance_opponent_holds_one_of(game, player, opponent, unknown_cards, savers)
    }

    /// The chance that the opponent's cards unknown to `player`, taken from `unknown_cards`,
    /// include at least one of `matching` of those cards.
    fn chance_opponent_holds_one_of(game: &Game, player: PlayerIndex, opponent: PlayerIndex, unknown_cards: &[TreacheryCard], matching: i32) -> f32 {
        let nr_of_unknown_opponent_cards = Self::nr_of_unknown_opponent_cards(game, player, opponent);
        let count = unknown_cards.len() as i32;
        1.0 - Self::cumulative_chance((count - matching) as f64, count as f64, nr_of_unknown_opponent_cards) as f32
    }

    /// The chance of drawing `length` cards from `b` without hitting any outside the `a` safe ones.
    fn cumulative_chance(a: f64, b: f64, length: i32) -> f64 {
        let mut result = 1.0f64;
        for i in 0..length {
            result *= (a - i as f64) / (b - i as f64);
        }
        result
    }

    fn number_of_unknown_weapons_that_could_kill_me_with_player_defense(unknown_cards: &[TreacheryCard], defense: Option<TreacheryCard>, chosen_weapon: Option<TreacheryCard>) -> i32 {
        let defense = defense.and_then(CsNull::opt);
        unknown_cards.iter().filter(|c| c.is_weapon() && (defense.is_none() || !c.countered_by(defense, chosen_weapon))).count() as i32
    }

    fn number_of_unknown_defenses_that_could_counter_player_weapon(unknown_cards: &[TreacheryCard], weapon: Option<TreacheryCard>) -> i32 {
        let weapon = weapon.and_then(CsNull::opt);
        unknown_cards.iter().filter(|c| c.is_defense() && weapon.is_none_or(|w| w.countered_by(Some(**c), None))).count() as i32
    }

    /// My clairvoyance answer about this battle that covers `question` or any card in battle.
    fn ruling_clairvoyance(&self, game: &Game, question: ClairvoyanceQuestion) -> Option<ClairVoyanceQandA> {
        let (Some(_), Some(qna)) = (&game.latest_clairvoyance, &game.latest_clairvoyance_qn_a) else {
            return None;
        };
        if qna.answer.initiator() == self.faction(game)
            && game.latest_clairvoyance_battle.is_some()
            && is_current_battle(game, game.latest_clairvoyance_battle.as_ref())
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

    pub fn ruling_weapon_clairvoyance(&self, game: &Game) -> Option<ClairVoyanceQandA> {
        self.ruling_clairvoyance(game, ClairvoyanceQuestion::CardTypeAsWeaponInBattle)
    }

    pub fn ruling_defense_clairvoyance(&self, game: &Game) -> Option<ClairVoyanceQandA> {
        self.ruling_clairvoyance(game, ClairvoyanceQuestion::CardTypeAsDefenseInBattle)
    }

    /// The weapon to use against the enemy leader and the chance that it kills them.
    fn chance_of_enemy_leader_dying(&self, game: &Game, player: PlayerIndex, opponent: PlayerIndex, voice: Option<&VoicePlan>, prescience: Option<PrescienceAspect>) -> WeaponChoice {
        let choice = |kill_chance: f32, weapon: Option<TreacheryCard>, enemy_can_defend_poison_tooth: bool| WeaponChoice {
            weapon,
            kill_chance,
            enemy_can_defend_poison_tooth,
        };

        if Self::heroes_for_battle(game, opponent, true).is_empty() {
            return choice(0.0, None, false);
        }

        if let Some(voice) = voice.filter(|v| v.weapon_to_use.is_some()) {
            return choice(if voice.opponent_hero_will_certainly_be_zero { 1.0 } else { 0.5 }, voice.weapon_to_use, false);
        }

        // The weapons the fewest unknown cards can counter come first.
        let mut available_weapons: Vec<TreacheryCard> = Self::weapons(game, player, None, None, None)
            .into_iter()
            .filter(|w| {
                !matches!(
                    w.type_(),
                    TreacheryCardType::Useless
                        | TreacheryCardType::ArtilleryStrike
                        | TreacheryCardType::PoisonTooth
                        | TreacheryCardType::Rockmelter
                        | TreacheryCardType::HarassAndWithdraw
                        | TreacheryCardType::Recruits
                )
            })
            .collect();
        let unknown_cards = Self::cards_unknown_to_player(game, player);
        available_weapons.sort_by_cached_key(|w| Self::number_of_unknown_defenses_that_could_counter_player_weapon(&unknown_cards, Some(*w)));

        let opponent_plan = game.current_battle.as_ref().and_then(|b| b.plan_of(game, Some(opponent)));

        let mut opponent_may_be_using_a_weapon = true;
        if let Some(opponent_plan) = &opponent_plan {
            if prescience == Some(PrescienceAspect::Defense) {
                let opponent_defense = opponent_plan.defense();
                let weapon =
                    available_weapons.iter().copied().find(|w| opponent_defense.is_none() || !w.countered_by(opponent_defense, None));
                return choice(
                    if weapon.is_some() { 1.0 } else { 0.0 },
                    weapon,
                    opponent_defense.is_some_and(|d| d.is_non_antidote_poison_defense()),
                );
            }

            if prescience == Some(PrescienceAspect::Weapon) && opponent_plan.weapon().is_none_or(|w| w.is_useless()) {
                opponent_may_be_using_a_weapon = false;
            }
        }

        let useful_weapons: Vec<TreacheryCard> = available_weapons
            .into_iter()
            .filter(|w| opponent_may_be_using_a_weapon || w.type_() != TreacheryCardType::MirrorWeapon)
            .collect();

        let known_enemy_defenses = Self::known_opponent_defenses(game, player, opponent);
        let enemy_knowingly_defends_poison_tooth = || known_enemy_defenses.iter().any(|c| c.is_non_antidote_poison_defense());
        let mut enemy_can_defend_poison_tooth = false;

        if let Some(my_clairvoyance) = Self::my_clairvoyance_about_enemy_defense_in_current_battle(game, player) {
            let question = my_clairvoyance.question.as_ref().nre();
            let answer = || &game.latest_clairvoyance_qn_a.as_ref().nre().answer;
            // Whether a projectile (true) or a poison (false) weapon gets past the defense the answer reveals.
            let projectile_gets_through = if question.is_about(game, TreacheryCardType::ProjectileDefense) {
                if answer().is_no() {
                    Some(true)
                } else if answer().is_yes() {
                    Some(false)
                } else {
                    None
                }
            } else if question.is_about(game, TreacheryCardType::PoisonDefense) {
                if answer().is_no() {
                    Some(false)
                } else if answer().is_yes() {
                    Some(true)
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(projectile) = projectile_gets_through {
                if projectile {
                    enemy_can_defend_poison_tooth = enemy_knowingly_defends_poison_tooth();
                }
                let weapon = useful_weapons
                    .iter()
                    .copied()
                    .find(|w| if projectile { w.is_projectile_weapon() } else { w.is_poison_weapon() });
                if weapon.is_some() {
                    return choice(1.0, weapon, enemy_can_defend_poison_tooth);
                }
            }
        }

        let unknown_opponent_cards = Self::opponent_cards_unknown_to_player(game, player, opponent);

        let weapon = useful_weapons
            .iter()
            .copied()
            .filter(|w| !known_enemy_defenses.iter().any(|defense| w.countered_by(Some(*defense), None)))
            .random_or_default(&self.random);

        if let Some(weapon) = weapon {
            if !weapon.is_mirror_weapon() {
                let saved = self.chance_of_an_unknown_opponent_card_saving_his_leader(game, player, &unknown_opponent_cards, Some(weapon), opponent);
                return choice(1.0 - saved, Some(weapon), enemy_can_defend_poison_tooth);
            }

            if let Some(opponent_plan) = opponent_plan.as_ref().filter(|_| prescience == Some(PrescienceAspect::Weapon)) {
                let saved =
                    self.chance_of_an_unknown_opponent_card_saving_his_leader(game, player, &unknown_opponent_cards, opponent_plan.weapon(), opponent);
                return choice(1.0 - saved, Some(weapon), enemy_can_defend_poison_tooth);
            }
        }

        // C# parity: a MirrorWeapon chosen above is drawn again here (hazards 18.5).
        let weapon = useful_weapons.iter().copied().filter(|w| !Self::is_known_to(game, opponent, *w)).random_or_default(&self.random);

        if weapon.is_some() {
            return choice(0.5, weapon, enemy_can_defend_poison_tooth);
        }

        choice(0.0, None, enemy_knowingly_defends_poison_tooth())
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

    /// Picks a leader for battle: a safe one (not a possible traitor) unless an unsafe one is much
    /// stronger, the strongest if `highest` and a weak one otherwise.
    fn select_hero_for_battle(&self, game: &Game, player: PlayerIndex, opponent: PlayerIndex, highest: bool, forfeit: bool, messiah_used: bool, weapon: Option<TreacheryCard>, defense: Option<TreacheryCard>, include_in_front_of_shield: bool) -> HeroChoice {
        let p = game.player(player);
        let o = game.player(opponent);

        let ally = if p.ally != Faction::None { p.allied_player(game) } else { None };

        // Heroes known not to be traitors for anyone who could betray me.
        let mut known_non_traitors: Vec<Hero> = p.traitors.iter().chain(p.known_non_traitors.iter()).copied().collect();
        if let Some(a) = ally {
            let a = game.player(a);
            known_non_traitors.extend(a.traitors.iter().chain(a.known_non_traitors.iter()).copied());
        }
        for x in game.player_indices() {
            let xp = game.player(x);
            if x != opponent && (xp.ally != Faction::Harkonnen || xp.faction != o.ally) {
                known_non_traitors.extend(xp.revealed_traitors.iter().chain(xp.told_traitors.iter()).copied());
            }
        }
        if o.ally != Faction::Harkonnen {
            known_non_traitors.extend(o.told_non_traitors.iter().copied());
        }

        // Heroes known to be traitors for my opponent's side.
        let mut known_traitors_for_opponents_in_battle: Vec<Hero> = Vec::new();
        for x in game.player_indices() {
            let xp = game.player(x);
            if x == opponent || (xp.faction == Faction::Harkonnen && xp.faction == o.ally) {
                // C# parity: the player's told traitors, not x's.
                known_traitors_for_opponents_in_battle.extend(xp.revealed_traitors.iter().chain(p.told_traitors.iter()).copied());
            }
        }
        known_traitors_for_opponents_in_battle.extend(
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

        let highest_opponent_leader = Self::heroes_for_battle(game, opponent, true).into_iter().min_by_key(|l| Reverse(l.value()));
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

        let hero = if safe_hero.is_none()
            || (o.faction != Faction::Harkonnen
                && !known_traitors_for_opponents_in_battle.contains(&unsafe_hero.or_null())
                && safe_hero.unwrap().value_in_combat_against(highest_opponent_leader)
                    < unsafe_hero.nre().value_in_combat_against(highest_opponent_leader) - 2)
        {
            unsafe_hero
        } else {
            safe_hero
        };

        let is_traitor = !messiah_used && known_traitors_for_opponents_in_battle.contains(&hero.or_null());

        let mut used_skill = LeaderSkill::None;
        let value = match hero {
            Some(h) => {
                h.value_in_combat_against(highest_opponent_leader)
                    + Battle::determine_skill_bonus_game_player_i_hero_treachery_card_treachery_card_int32_leader_skill(
                        game,
                        player,
                        Some(h),
                        weapon,
                        defense,
                        if p.resources > 3 { 3 } else { 0 },
                        &mut used_skill,
                    )
            }
            None => 0,
        };
        HeroChoice { hero, value, is_traitor }
    }

    pub fn get_dial_needed(&self, game: &Game, p: PlayerIndex, territory: Territory, opponent: PlayerIndex, take_reinforcements_into_account: bool) -> f32 {
        let voice_plan = if Voice::may_use_voice(game, p) { Some(self.best_voice(game, None, p, opponent)) } else { None };
        let strength_of_opponent = Self::max_dial(game, opponent, territory, Some(p), false);
        let prescience = if Prescience::may_use_prescience(game, p) {
            self.best_prescience(game, p, opponent, strength_of_opponent, PrescienceAspect::None, territory)
        } else {
            PrescienceAspect::None
        };

        // C# parity: computes for Player, not p (hazards 18.5).
        self.get_dial_needed_for_battle(
            game,
            self.i_will_be_aggressor_against(game, Some(opponent)),
            opponent,
            territory,
            voice_plan.as_ref(),
            prescience,
            take_reinforcements_into_account,
            true,
        )
        .dial_needed
    }

    pub fn dial_needed_against(&self, game: &Game, territory: Territory, opponent: Option<PlayerIndex>, take_reinforcements_into_account: bool) -> f32 {
        let Some(opponent) = opponent.and_then(CsNull::opt) else {
            return 0.0;
        };
        self.get_dial_needed(game, self.player, territory, opponent, take_reinforcements_into_account)
    }

    /// The dial I need to beat `opponent` in `territory`, with the leader and cards I'd use.
    fn get_dial_needed_for_battle(&self, game: &Game, i_am_aggressor: bool, opponent: PlayerIndex, territory: Territory, voice: Option<&VoicePlan>, prescience: PrescienceAspect, take_reinforcements_into_account: bool, include_in_front_of_shield: bool) -> DialEstimate {
        let player = self.player;
        let param = &self.param;
        let enemy = self.chance_of_enemy_leader_dying(game, player, opponent, voice, Some(prescience));
        let mut weapon = enemy.weapon;
        let mut chance_of_enemy_hero_surviving = 1.0 - enemy.kill_chance;
        let (mut chance_of_my_hero_surviving, mut defense) =
            self.chance_of_my_leader_surviving(game, player, opponent, voice, Some(prescience), weapon);

        if weapon.is_none()
            && chance_of_my_hero_surviving < param.battle_minimum_chance_to_assume_my_leader_survives
            && chance_of_enemy_hero_surviving >= param.battle_minimum_chance_to_assume_enemy_hero_survives
        {
            if let Some(destructive) = Self::destructive_weapon(game, player, defense, enemy.enemy_can_defend_poison_tooth) {
                weapon = Some(destructive);
                chance_of_enemy_hero_surviving = 0.0;
                chance_of_my_hero_surviving = 0.0;
                if defense.is_some_and(|d| {
                    Self::may_play_no_defense(game, player, weapon)
                        && ((destructive.type_() == TreacheryCardType::PoisonTooth && d.type_() != TreacheryCardType::Chemistry)
                            || (destructive.type_() == TreacheryCardType::ArtilleryStrike && !d.is_shield()))
                }) {
                    defense = None;
                }
            }
        }

        let i_assume_my_leader_will_die = chance_of_my_hero_surviving < param.battle_minimum_chance_to_assume_my_leader_survives;
        let i_assume_enemy_leader_will_die = chance_of_enemy_hero_surviving < param.battle_minimum_chance_to_assume_enemy_hero_survives;

        let opponent_plan = game.current_battle.as_ref().and_then(|b| b.plan_of(game, Some(opponent)));
        let opponent_weapon = if prescience == PrescienceAspect::Weapon { opponent_plan.as_ref().and_then(|p| p.weapon()) } else { None };
        let opponent_defense = if prescience == PrescienceAspect::Defense { opponent_plan.as_ref().and_then(|p| p.defense()) } else { None };
        let lasgun_shield = Self::has_lasgun_shield(weapon, defense, opponent_weapon, opponent_defense);
        let stone_burner = weapon.is_some_and(|w| w.is_rock_melter()) || opponent_weapon.is_some_and(|w| w.is_rock_melter());

        let mut messiah = Battle::messiah_available_for_battle(game, player) && !lasgun_shield;

        let HeroChoice { hero, value: my_hero_value, is_traitor } = self.select_hero_for_battle(
            game,
            player,
            opponent,
            !lasgun_shield && !i_assume_my_leader_will_die,
            false,
            messiah,
            weapon,
            defense,
            include_in_front_of_shield,
        );

        let mut used_skill = LeaderSkill::None;
        let opponent_penalty = Battle::determine_skill_penalty_game_i_hero_player_leader_skill(game, hero, opponent, &mut used_skill);

        if hero.is_none() {
            messiah = false;
        }
        let my_messiah_bonus = if messiah { 2 } else { 0 };

        let estimate = move |dial_needed: f32, weapon: Option<TreacheryCard>, defense: Option<TreacheryCard>, banker_boost: i32| DialEstimate {
            dial_needed,
            weapon,
            defense,
            hero,
            messiah,
            is_traitor,
            lasgun_shield,
            stone_burner,
            banker_boost,
        };

        if Self::can_only_use_traitors_or_facedancers(game, opponent) {
            return estimate(0.5, None, None, 0);
        }

        if is_traitor {
            return estimate(99.0, None, None, 0);
        }

        if lasgun_shield {
            if weapon.is_some_and(|w| !w.is_laser()) && Self::may_play_no_weapon(game, player, defense) {
                weapon = None;
            }
            return estimate(0.5, weapon, defense, 0);
        }

        let banker_boost = if game.skilled_as(hero.or_null(), LeaderSkill::Banker) && !i_assume_my_leader_will_die {
            game.player(player).resources.min(3)
        } else {
            0
        };

        if hero.is_some_and(|h| h.as_treachery_card().is_some())
            && defense.is_some_and(|d| !d.is_useless())
            && Self::may_play_no_defense(game, player, weapon)
        {
            defense = None;
        }

        let opponent_leader = match &opponent_plan {
            Some(plan) if prescience == PrescienceAspect::Leader => plan.hero(),
            _ => {
                // The opponent's strongest leader that isn't my traitor.
                let traitors = &game.player(player).traitors;
                Self::heroes_for_battle(game, opponent, true)
                    .into_iter()
                    .filter(|l| !traitors.contains(l))
                    .min_by_key(|l| Reverse(l.value_in_combat_against(hero)))
            }
        };
        let opponent_leader_value = opponent_leader.map(|l| l.value_in_combat_against(hero)).unwrap_or(0);
        let opponent_messiah_bonus = if Battle::messiah_available_for_battle(game, opponent) { 2 } else { 0 };
        let max_reinforcements = if take_reinforcements_into_account {
            (Self::max_reinforced_dial_to(game, player, opponent, territory) as f64).ceil() as i32
        } else {
            0
        };
        let opponent_dial = match &opponent_plan {
            Some(plan) if prescience == PrescienceAspect::Dial => plan.dial(game, game.player(player).faction),
            _ => Self::max_dial(game, opponent, territory, Some(player), false),
        };
        let my_homeworld_bonus = game.player(player).get_homeworld_battle_contribution_and_lasgun_shield_limit(game, territory);
        let opponent_homeworld_bonus = game.player(opponent).get_homeworld_battle_contribution_and_lasgun_shield_limit(game, territory);

        let aggressor_term: f32 = if i_am_aggressor { 0.0 } else { 0.5 };
        // What the enemy and my leader add to the dial, unless I assume they die.
        let leader_terms = |enemy_leader_dies: bool, my_leader_dies: bool| {
            let enemy = if enemy_leader_dies { 0 } else { opponent_leader_value + opponent_messiah_bonus };
            let mine = if my_leader_dies { 0 } else { my_hero_value + opponent_penalty + my_messiah_bonus };
            (enemy as f32, mine as f32)
        };

        let (enemy_term, my_term) = leader_terms(i_assume_enemy_leader_will_die, i_assume_my_leader_will_die);
        let mut result = opponent_dial + opponent_homeworld_bonus as f32 + max_reinforcements as f32 + enemy_term + aggressor_term
            - my_term
            - my_homeworld_bonus as f32;

        if Self::max_dial(game, player, territory, Some(opponent), false) - result >= 5.0 {
            // I only need a small part of my forces, so I can be more careful about the leaders.
            let my_leader_dies = i_assume_my_leader_will_die || (chance_of_my_hero_surviving as f64) < 0.8;
            let enemy_leader_dies = i_assume_enemy_leader_will_die && !((chance_of_enemy_hero_surviving as f64) > 0.1);
            let (enemy_term, my_term) = leader_terms(enemy_leader_dies, my_leader_dies);
            result = opponent_dial + max_reinforcements as f32 + enemy_term + aggressor_term - my_term;
        }

        estimate(result, weapon, defense, banker_boost)
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

    fn has_lasgun_shield(my_weapon: Option<TreacheryCard>, my_defense: Option<TreacheryCard>, enemy_weapon: Option<TreacheryCard>, enemy_defense: Option<TreacheryCard>) -> bool {
        (my_weapon.is_some_and(|w| w.is_laser()) && enemy_defense.is_some_and(|d| d.is_shield()))
            || (enemy_weapon.is_some_and(|w| w.is_laser()) && my_defense.is_some_and(|d| d.is_shield()))
    }

    /// Not pure: working out the battle outcome on the live plans changes the game
    /// (bots-plan.md, "Bots are not pure").
    pub fn determine_rock_was_melted(&self, game: &mut Game) -> Rc<RockWasMelted> {
        let agg = game.aggressor_plan.clone();
        let def = game.defender_plan.clone();
        let territory = game.current_battle.as_ref().nre().territory(game).or_null();
        let outcome = Battle::determine_battle_outcome(agg.as_ref().nre(), def.as_ref().nre(), territory, game);
        // Building the message can throw, which cancels the decision.
        let _ = outcome.get_message(game);
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
        let portable_antidote = game.player(self.player).treachery_cards.iter().copied().find(|c| c.is_portable_antidote());

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
        let mut valid_cards: Vec<TreacheryCard> = Thought::valid_cards(game).collect();
        valid_cards.sort_by_cached_key(|c| Reverse(self.card_quality(game, *c, opponent)));

        // The best card I don't know the whereabouts of, else the best card.
        let card = valid_cards
            .iter()
            .copied()
            .find(|x| !Self::is_known_to(game, self.player, *x))
            .or(valid_cards.first().copied())?;

        let mut result = Thought::new(game, self.faction(game));
        result.set_card(Some(card));
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
        use StrongholdAdvantage as A;
        let plan = self.determine_battle_plan(game, false);
        let valid = HMSAdvantageChosen::valid_advantages(game, self.player);
        // In order of preference, each with the condition under which I want it.
        let preferences = [
            (!plan.has_poison() && !plan.has_antidote(), A::CountDefensesAsAntidote),
            (self.resources(game) < 5, A::FreeResourcesForBattles),
            (true, A::CollectResourcesForDial),
            (!plan.has_useless(), A::CollectResourcesForUseless),
            (true, A::FreeResourcesForBattles),
            (true, A::WinTies),
        ];
        let advantage = preferences
            .into_iter()
            .find(|(wanted, advantage)| *wanted && valid.contains(advantage))
            .map(|(_, advantage)| advantage)
            .or(valid.first().copied())
            .unwrap_or(A::None);

        let mut result = HMSAdvantageChosen::new(game, self.faction(game));
        result.advantage = advantage;
        Rc::new(result)
    }

    pub fn determine_retreat(&self, game: &Game) -> Option<Rc<Retreat>> {
        let mut forces_to_retreat = Retreat::max_forces(game, self.player);
        let mut special_forces_to_retreat = Retreat::max_special_forces(game, self.player);
        let max_forces = Retreat::max_total_forces(game, self.player);
        while forces_to_retreat + special_forces_to_retreat > max_forces {
            if forces_to_retreat > 0 {
                forces_to_retreat -= 1;
            } else {
                special_forces_to_retreat -= 1;
            }
        }

        let to = Retreat::valid_targets(game, self.player)
            .filter(|l| self.resources_in(game, *l) > 0)
            .highest_or_default(&self.random, |l| self.resources_in(game, *l))
            .or_else(|| Retreat::valid_targets(game, self.player).find(|l| l.is_protected_from_storm()))
            .or_else(|| Retreat::valid_targets(game, self.player).next());

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
        // My best card, if it is worth keeping.
        let to_keep = LoserConcluded::cards_loser_may_keep(game)
            .into_iter()
            .map(|c| (self.card_quality(game, c, Some(self.player)), c))
            .filter(|(quality, _)| *quality > 2)
            .min_by_key(|(quality, _)| Reverse(*quality))
            .map(|(_, c)| c);
        let mut result = LoserConcluded::new(game, self.faction(game));
        result.set_kept_card(to_keep);
        result.assassinate = LoserConcluded::can_assassinate(game, self.player);
        Rc::new(result)
    }
}
