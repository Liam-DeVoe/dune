use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct Battle {
    pub hero: Option<Hero>,
    pub messiah: bool,
    pub forces: u16,
    pub forces_at_half_strength: u16,
    pub special_forces: u16,
    pub special_forces_at_half_strength: u16,
    pub ally_contribution_amount: u16,
    /// Interior mutability: written after the event is stored in game state.
    pub weapon: Cell<Option<TreacheryCard>>,
    /// Interior mutability: written after the event is stored in game state.
    pub defense: Cell<Option<TreacheryCard>>,
    pub banker_bonus: u16,
    /// Interior mutability: written after the event is stored in game state.
    pub original_weapon: Cell<Option<TreacheryCard>>,
    /// Interior mutability: written after the event is stored in game state.
    pub original_defense: Cell<Option<TreacheryCard>>,
    /// The fields of `Event` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

/// What swapping in a plan's dynamic weapons changed that the game records.
struct DynamicWeaponSwap {
    mirror_weapon_used: bool,
    /// The defense a Diplomat replaced, if the plan's player used one.
    card_used_by_diplomat: Option<Option<TreacheryCard>>,
}

impl Default for Battle {
    fn default() -> Self {
        Battle {
            hero: TreacheryCardManager::get(0).map(Hero::TreacheryCard),
            messiah: Default::default(),
            forces: Default::default(),
            forces_at_half_strength: Default::default(),
            special_forces: Default::default(),
            special_forces_at_half_strength: Default::default(),
            ally_contribution_amount: Default::default(),
            weapon: Cell::new(TreacheryCardManager::get(0)),
            defense: Cell::new(TreacheryCardManager::get(0)),
            banker_bonus: Default::default(),
            original_weapon: Cell::new(Default::default()),
            original_defense: Cell::new(Default::default()),
            base: Default::default(),
        }
    }
}

/// `vec![Part::from(a), ...]`: the `params object?[]` of `Log(...)`.
macro_rules! le {
    ($($e:expr),* $(,)?) => {
        vec![$($crate::messages::part::Part::from($e)),*]
    };
}

impl Battle {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn weapon(&self) -> Option<TreacheryCard> {
        self.weapon.get()
    }

    pub fn set_weapon(&self, value: Option<TreacheryCard>) {
        self.weapon.set(value);
    }

    pub fn defense(&self) -> Option<TreacheryCard> {
        self.defense.get()
    }

    pub fn set_defense(&self, value: Option<TreacheryCard>) {
        self.defense.set(value);
    }

    pub fn total_forces(&self) -> i32 {
        (i32::from(self.forces) + i32::from(self.forces_at_half_strength) + i32::from(self.special_forces)) + i32::from(self.special_forces_at_half_strength)
    }

    pub fn has_poison(&self) -> bool {
        self.weapon().is_some_and(|w| w.is_poison_weapon())
    }

    pub fn has_projectile(&self) -> bool {
        self.weapon().is_some_and(|w| w.is_projectile_weapon())
    }

    pub fn has_projectile_defense(&self) -> bool {
        self.defense().is_some_and(|d| d.is_projectile_defense())
    }

    pub fn has_shield(&self) -> bool {
        self.defense().is_some_and(|d| d.is_shield())
    }

    pub fn has_non_antidote_poison_defense(&self) -> bool {
        self.defense().is_some_and(|d| d.is_non_antidote_poison_defense())
    }

    pub fn has_antidote(&self) -> bool {
        self.defense().is_some_and(|d| d.is_poison_defense())
    }

    pub fn has_laser(&self) -> bool {
        self.weapon().is_some_and(|w| w.is_laser())
    }

    pub fn has_poison_tooth(&self) -> bool {
        self.weapon().is_some_and(|w| w.is_poison_tooth())
    }

    pub fn has_artillery(&self) -> bool {
        self.weapon().is_some_and(|w| w.is_artillery())
    }

    pub fn has_reinforcements(&self) -> bool {
        self.weapon().is_some_and(|w| w.type_() == TreacheryCardType::Reinforcements)
            || self.defense().is_some_and(|d| d.type_() == TreacheryCardType::Reinforcements)
    }

    pub fn has_rock_melter(&self) -> bool {
        self.weapon().is_some_and(|w| w.is_rock_melter())
    }

    pub fn has_useless(&self) -> bool {
        self.weapon().is_some_and(|w| w.is_useless()) || self.defense().is_some_and(|d| d.is_useless())
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let Some(current_battle) = game.battle.current_battle.clone() else {
            game_error!("Battle event executed when there is no current battle");
        };

        if self.initiator() == current_battle.aggressor(game) {
            game.battle.aggressor_plan = Some(self.clone());
        } else if self.initiator() == current_battle.defender(game) {
            game.battle.defender_plan = Some(self.clone());
        }

        if let (Some(aggressor_plan), Some(defender_plan)) = (game.battle.aggressor_plan.clone(), game.battle.defender_plan.clone()) {
            let battle_territory = current_battle.territory;
            if let Some(battle_territory) = battle_territory {
                let supplier = Battle::determine_force_supplier(game, aggressor_plan.player());
                game.reveal_current_no_field_in_territory(supplier, battle_territory)?;
                let supplier = Battle::determine_force_supplier(game, defender_plan.player());
                game.reveal_current_no_field_in_territory(supplier, battle_territory)?;
            }

            let m = express!(aggressor_plan.try_get_battle_plan_message(game)?, "; ", defender_plan.try_get_battle_plan_message(game)?)
                .with_plans(aggressor_plan.get_plan_summary(game), defender_plan.get_plan_summary(game));
            self.log_objects(game, le![m]);

            self.register_known_cards(game, &aggressor_plan);
            self.register_known_cards(game, &defender_plan);

            self.pass_tleilaxu_traitor_action(game);

            let melting = aggressor_plan.has_rock_melter() || defender_plan.has_rock_melter();
            game.enter_either(melting, Phase::MeltingRock, Phase::CallTraitorOrPass);
        }
        Ok(())
    }

    pub fn register_known_cards(&self, game: &mut Game, battle: &Battle) {
        if let Some(w) = battle.weapon() {
            game.register_known(w);
        }
        if let Some(d) = battle.defense() {
            game.register_known(d);
        }
    }

    pub fn pass_tleilaxu_traitor_action(&self, game: &mut Game) {
        let Some(current_battle) = game.battle.current_battle.clone() else {
            panic!("Battle event executed when there is no current battle");
        };

        // `GetPlayer(Faction.Tleilaxu)?.Ally != Faction.Harkonnen` is true when Tleilaxu is not playing.
        let tleilaxu_ally_is_not_harkonnen =
            |game: &Game, me: &Battle| me.get_player(game, Some(Faction::Tleilaxu)).map(|p| game.player(p).ally) != Some(Some(Faction::Harkonnen));

        if current_battle.aggressor(game) == Some(Faction::Tleilaxu)
            && (tleilaxu_ally_is_not_harkonnen(game, self) || game.prevented(FactionAdvantage::HarkonnenCallTraitorForAlly))
        {
            let mut action = TreacheryCalled::new(game, Faction::Tleilaxu);
            action.traitor_called = false;
            game.battle.aggressor_traitor_action = Some(Rc::new(action));
        } else if current_battle.defender(game) == Some(Faction::Tleilaxu)
            && (tleilaxu_ally_is_not_harkonnen(game, self) || game.prevented(FactionAdvantage::HarkonnenCallTraitorForAlly))
        {
            let mut action = TreacheryCalled::new(game, Faction::Tleilaxu);
            action.traitor_called = false;
            game.battle.defender_traitor_action = Some(Rc::new(action));
        }
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " finalize their battle plan")
    }

    /// The plan as the log shows it, or why it can't be shown: the game has no battle against an
    /// opponent (an undo replays the plan into a game whose battle is over).
    pub fn try_get_battle_plan_message(&self, game: &Game) -> GameResult<Message> {
        let opponent = game.battle.current_battle.as_ref().required("current battle")?.opponent_of_faction(game, self.initiator()).required("opponent")?;
        Ok(express!(
            self.initiator(),
            " Plan → leader: ",
            self.hero,
            ", dial: ",
            self.dial(game, game.player(opponent).faction.required("faction")?),
            express_part_if!(game.applicable(Rule::AdvancedCombat); Payment::of(self.cost(game))),
            express_part_if!(i32::from(self.ally_contribution_amount) > 0; " (", Payment::of_by(i32::from(self.ally_contribution_amount), game.player(self.player()).ally), ")"),
            ", weapon: ",
            self.weapon(),
            ", defense: ",
            self.defense()
        ))
    }

    pub fn get_plan_summary(&self, game: &Game) -> PlanSummary {
        let faction = self.initiator();
        let leader = self.hero;
        let messiah = self.messiah;
        let opponent = game.battle.current_battle.as_ref().expect("current battle").opponent_of_faction(game, self.initiator()).expect("opponent of faction");
        let dial = self.dial(game, game.player(opponent).faction.expect("faction"));
        let spice = if game.applicable(Rule::AdvancedCombat) { Some(self.cost(game)) } else { None };
        let ally_contribution = i32::from(self.ally_contribution_amount);
        let ally = game.player(self.player()).ally;
        PlanSummary {
            faction: faction.expect("faction"),
            leader,
            messiah,
            dial,
            spice,
            ally_contribution,
            ally,
            weapon: self.weapon(),
            defense: self.defense(),
        }
    }

    pub fn activate_dynamic_weapons(&self, game: &mut Game, opponent_weapon: Option<TreacheryCard>, opponent_defense: Option<TreacheryCard>) {
        let swap = self.swap_in_dynamic_weapons(game, opponent_weapon, opponent_defense);
        if swap.mirror_weapon_used {
            self.log_objects(game, le![self.original_weapon.get(), " becomes a ", self.weapon()]);
        }
        if let Some(card) = swap.card_used_by_diplomat {
            game.battle.card_used_by_diplomat = card;
        }
    }

    /// Puts the cards a Mirror Weapon, a Diplomat or a Portable Antidote stand for into this
    /// plan, without changing the game. `activate_dynamic_weapons` also records them in the game.
    fn swap_in_dynamic_weapons(&self, game: &Game, opponent_weapon: Option<TreacheryCard>, opponent_defense: Option<TreacheryCard>) -> DynamicWeaponSwap {
        let mut swap = DynamicWeaponSwap { mirror_weapon_used: false, card_used_by_diplomat: None };
        if self.weapon().is_some_and(|w| w.type_() == TreacheryCardType::MirrorWeapon) {
            self.original_weapon.set(self.weapon());
            self.set_weapon(opponent_weapon);
            swap.mirror_weapon_used = true;
        }

        if game.battle.current_diplomacy.as_ref().is_some_and(|d| d.by(self.initiator())) {
            self.original_defense.set(self.defense());
            self.set_defense(opponent_defense);
            swap.card_used_by_diplomat = Some(self.original_defense.get());
        }

        if Battle::is_using_portable_antidote(game, self.initiator().expect("faction")) {
            self.original_defense.set(self.defense());
            let antidote = game.decks.treachery_discard_pile.as_ref().expect("treachery discard pile").items.iter().copied().find(|c| c.is_portable_antidote());
            self.set_defense(antidote);
        }
        swap
    }

    pub fn deactivate_dynamic_weapons(&self) {
        if self.original_weapon.get().is_some() {
            self.set_weapon(self.original_weapon.get());
            self.original_weapon.set(None);
        }

        if self.original_defense.get().is_some() {
            self.set_defense(self.original_defense.get());
            self.original_defense.set(None);
        }
    }

    pub fn dial(&self, g: &Game, opponent: Faction) -> HalfPoints {
        Battle::dial_of_forces(
            g,
            self.player(),
            opponent,
            i32::from(self.forces),
            i32::from(self.special_forces),
            i32::from(self.forces_at_half_strength),
            i32::from(self.special_forces_at_half_strength),
        )
    }

    pub fn dial_of_forces(g: &Game, p: PlayerId, opponent: Faction, forces: i32, special_forces: i32, forces_at_half_strength: i32, special_forces_at_half_strength: i32) -> HalfPoints {
        let pl = g.player(p);
        let ecaz_battle_contribution = if !g.prevented(FactionAdvantage::EcazOccupation) && (pl.faction == Some(Faction::Ecaz) || pl.ally == Some(Faction::Ecaz)) {
            g.battle.current_ecaz_battle_contribution
        } else {
            0
        };

        // `CurrentEcazOrAllyFighter == Faction.None` is false while it is null.
        if g.battle.current_ecaz_or_ally_fighter == Some(None) || pl.faction != Some(Faction::Ecaz) {
            return Battle::force_value(g, pl.faction.expect("faction"), opponent, forces, special_forces, forces_at_half_strength, special_forces_at_half_strength)
                + HalfPoints::whole(ecaz_battle_contribution);
        }

        Battle::force_value(g, pl.ally.expect("faction"), opponent, forces, special_forces, forces_at_half_strength, special_forces_at_half_strength) + HalfPoints::whole(ecaz_battle_contribution)
    }

    pub fn cost(&self, g: &Game) -> i32 {
        let mut _paid = 0;
        Battle::cost_of_forces(g, self.player(), i32::from(self.forces), i32::from(self.special_forces), &mut _paid)
    }

    pub fn cost_with_arrakeen_share(&self, g: &Game, paid_by_arrakeen: &mut i32) -> i32 {
        Battle::cost_of_forces(g, self.player(), i32::from(self.forces), i32::from(self.special_forces), paid_by_arrakeen)
    }

    pub fn determine_battle_outcome(agg: &Rc<Battle>, def: &Rc<Battle>, territory: Territory, game: &mut Game) -> BattleOutcome {
        if agg.hero.is_some() {
            agg.activate_dynamic_weapons(game, def.weapon(), def.defense());
        }
        if def.hero.is_some() {
            def.activate_dynamic_weapons(game, agg.weapon(), agg.defense());
        }
        Battle::outcome_with_dynamic_weapons_active(agg, def, territory, game)
    }

    /// The outcome `determine_battle_outcome` would give, without changing the game: the Mirror
    /// Weapon message and the Diplomat's card aren't recorded. The plans' cards are swapped while
    /// it runs and swapped back at the end, as in `determine_battle_outcome`.
    pub fn predict_battle_outcome(agg: &Rc<Battle>, def: &Rc<Battle>, territory: Territory, game: &Game) -> BattleOutcome {
        if agg.hero.is_some() {
            agg.swap_in_dynamic_weapons(game, def.weapon(), def.defense());
        }
        if def.hero.is_some() {
            def.swap_in_dynamic_weapons(game, agg.weapon(), agg.defense());
        }
        Battle::outcome_with_dynamic_weapons_active(agg, def, territory, game)
    }

    fn outcome_with_dynamic_weapons_active(agg: &Rc<Battle>, def: &Rc<Battle>, territory: Territory, game: &Game) -> BattleOutcome {
        let mut result = BattleOutcome::new(agg.player(), def.player());

        let poison_tooth_used = !game.battle.poison_tooth_cancelled && (agg.has_poison_tooth() || def.has_poison_tooth());
        let artillery_used = agg.has_artillery() || def.has_artillery();
        let rock_melter_used = agg.has_rock_melter() || def.has_rock_melter();
        let rock_melter_used_to_kill = game.battle.current_rock_was_melted.as_ref().is_some_and(|r| r.kill);

        result.agg_hero_killed = false;
        result.agg_hero_cause_of_death = None;

        let protected = game.is_protected_by_carthag_advantage(agg, territory);
        Battle::determine_cause_of_death(
            agg,
            def,
            agg.hero,
            poison_tooth_used,
            artillery_used,
            rock_melter_used && rock_melter_used_to_kill,
            protected,
            &mut result.agg_hero_killed,
            &mut result.agg_hero_cause_of_death,
            &mut result.agg_saved_by_carthag,
        );

        result.def_hero_killed = false;
        result.def_hero_cause_of_death = None;
        let protected = game.is_protected_by_carthag_advantage(def, territory);
        Battle::determine_cause_of_death(
            def,
            agg,
            def.hero,
            poison_tooth_used,
            artillery_used,
            rock_melter_used && rock_melter_used_to_kill,
            protected,
            &mut result.def_hero_killed,
            &mut result.def_hero_cause_of_death,
            &mut result.def_saved_by_carthag,
        );

        let hero_strength_counts_to_total = !artillery_used && !(rock_melter_used && !rock_melter_used_to_kill);

        result.agg_hero_effective_strength = match agg.hero {
            Some(h) if hero_strength_counts_to_total => h.value_in_combat_against(def.hero),
            _ => 0,
        };
        result.def_hero_effective_strength = match def.hero {
            Some(h) if hero_strength_counts_to_total => h.value_in_combat_against(agg.hero),
            _ => 0,
        };

        if hero_strength_counts_to_total {
            result.agg_hero_skill_bonus = Battle::determine_skill_bonus(game, agg, &mut result.agg_activated_bonus_skill);
            result.agg_battle_penalty = if !result.def_hero_killed {
                Battle::determine_skill_penalty(game, def, result.aggressor, &mut result.def_activated_penalty_skill)
            } else {
                0
            };
            result.agg_messiah_contribution = if agg.messiah && agg.hero.is_some() { 2 } else { 0 };

            result.def_hero_skill_bonus = Battle::determine_skill_bonus(game, def, &mut result.def_activated_bonus_skill);
            result.def_battle_penalty = if !result.agg_hero_killed {
                Battle::determine_skill_penalty(game, agg, result.defender, &mut result.agg_activated_penalty_skill)
            } else {
                0
            };
            result.def_messiah_contribution = if def.messiah && def.hero.is_some() { 2 } else { 0 };
        }

        let agg_hero_contribution = if result.agg_hero_killed {
            0
        } else {
            (result
                .agg_hero_effective_strength + result.agg_hero_skill_bonus) + result.agg_messiah_contribution
        };
        let def_hero_contribution = if result.def_hero_killed {
            0
        } else {
            (result
                .def_hero_effective_strength + result.def_hero_skill_bonus) + result.def_messiah_contribution
        };

        let agg_ecaz_karma_contribution = if agg.initiator() == Some(Faction::Ecaz) { game.battle.ecaz_karma_bonus } else { 0 };
        let def_ecaz_karma_contribution = if def.initiator() == Some(Faction::Ecaz) { game.battle.ecaz_karma_bonus } else { 0 };

        let agg_force_supplier = Battle::determine_force_supplier(game, result.aggressor);
        result.agg_undialed_forces = game.player(agg_force_supplier).any_forces_in_territory(Some(territory)) - agg.total_forces();

        let def_force_supplier = Battle::determine_force_supplier(game, result.defender);
        result.def_undialed_forces = game.player(def_force_supplier).any_forces_in_territory(Some(territory)) - def.total_forces();

        if !rock_melter_used {
            result.agg_reinforcements_contribution = if agg.has_reinforcements() { 2 } else { 0 };
            result.def_reinforcements_contribution = if def.has_reinforcements() { 2 } else { 0 };

            result.agg_homeworld_contribution = game.player(agg.player()).get_homeworld_battle_contribution_and_lasgun_shield_limit(game, territory);
            result.def_homeworld_contribution = game.player(def.player()).get_homeworld_battle_contribution_and_lasgun_shield_limit(game, territory);

            let defender_faction = game.player(result.defender).faction;
            result.agg_total = agg.dial(game, defender_faction.expect("faction"))
                + HalfPoints::whole(
                    agg_hero_contribution + agg_ecaz_karma_contribution + result.agg_homeworld_contribution + result.agg_reinforcements_contribution
                        - result.agg_battle_penalty,
                );
            let aggressor_faction = game.player(result.aggressor).faction;
            result.def_total = def.dial(game, aggressor_faction.expect("faction"))
                + HalfPoints::whole(
                    def_hero_contribution + def_ecaz_karma_contribution + result.def_homeworld_contribution + result.def_reinforcements_contribution
                        - result.def_battle_penalty,
                );
        } else {
            let ecaz = game.get_player(Some(Faction::Ecaz));

            result.agg_total = HalfPoints::whole(result.agg_undialed_forces);
            if Some(game.player(result.aggressor).faction) == game.battle.current_ecaz_or_ally_fighter {
                let ecaz_forces = ecaz.map(|p| game.player(p).any_forces_in_territory(Some(territory))).unwrap_or(0);
                result.agg_total += HalfPoints::whole(half_rounded_up(ecaz_forces));
            }

            result.def_total = HalfPoints::whole(result.def_undialed_forces);
            if Some(game.player(result.defender).faction) == game.battle.current_ecaz_or_ally_fighter {
                let ecaz_forces = ecaz.map(|p| game.player(p).any_forces_in_territory(Some(territory))).unwrap_or(0);
                result.def_total += HalfPoints::whole(half_rounded_up(ecaz_forces));
            }
        }

        agg.deactivate_dynamic_weapons();
        def.deactivate_dynamic_weapons();

        let defender_faction = game.player(result.defender).faction;
        let aggressor_faction = game.player(result.aggressor).faction;
        let mut aggressor_wins_ties = !game.has_stronghold_advantage(defender_faction, StrongholdAdvantage::WinTies, territory);

        if BattleInitiated::is_aggressor_by_juice(game, defender_faction)
            && !game.has_stronghold_advantage(aggressor_faction, StrongholdAdvantage::WinTies, territory)
        {
            aggressor_wins_ties = false;
        }

        result.winner = Some(if aggressor_wins_ties {
            if result.agg_total >= result.def_total { result.aggressor } else { result.defender }
        } else if result.def_total >= result.agg_total {
            result.defender
        } else {
            result.aggressor
        });

        result.winner_battle_plan = Some(if result.winner == Some(result.aggressor) { agg.clone() } else { def.clone() });
        result.loser = Some(if result.winner == Some(result.aggressor) { result.defender } else { result.aggressor });
        result.loser_battle_plan = Some(if result.loser == Some(result.aggressor) { agg.clone() } else { def.clone() });

        result
    }

    pub fn determine_cause_of_death(player_plan: &Battle, opponent_plan: &Battle, the_hero: Option<Hero>, poison_tooth_used: bool, artillery_used: bool, rock_melter_was_used_to_kill: bool, is_protected_by_carthag_advantage: bool, hero_dies: &mut bool, cause_of_death: &mut Option<TreacheryCardType>, saved_by_carthag: &mut bool) {
        *hero_dies = false;
        *cause_of_death = None;
        *saved_by_carthag = is_protected_by_carthag_advantage && opponent_plan.has_poison() && !player_plan.has_antidote();

        Battle::determine_death_by(the_hero, TreacheryCardType::Rockmelter, rock_melter_was_used_to_kill, hero_dies, cause_of_death);
        Battle::determine_death_by(the_hero, TreacheryCardType::ArtilleryStrike, artillery_used && !player_plan.has_shield(), hero_dies, cause_of_death);
        Battle::determine_death_by(the_hero, TreacheryCardType::PoisonTooth, poison_tooth_used && !player_plan.has_non_antidote_poison_defense(), hero_dies, cause_of_death);
        Battle::determine_death_by(the_hero, TreacheryCardType::Laser, opponent_plan.has_laser(), hero_dies, cause_of_death);
        Battle::determine_death_by(the_hero, TreacheryCardType::Poison, opponent_plan.has_poison() && !(player_plan.has_antidote() || is_protected_by_carthag_advantage), hero_dies, cause_of_death);
        Battle::determine_death_by(the_hero, TreacheryCardType::Projectile, opponent_plan.has_projectile() && !player_plan.has_projectile_defense(), hero_dies, cause_of_death);
    }

    pub fn determine_death_by(hero: Option<Hero>, by_weapon: TreacheryCardType, weapon_has_effect: bool, hero_is_killed: &mut bool, cause_of_death: &mut Option<TreacheryCardType>) {
        if !*hero_is_killed && hero.is_some() && weapon_has_effect {
            *hero_is_killed = true;
            *cause_of_death = Some(by_weapon);
        }
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let p = self.player();
        let pl = game.player(p);
        if (i32::from(self.forces) + i32::from(self.forces_at_half_strength)) > Battle::max_forces(game, p, false) {
            return Some(express!("Too many ", pl.force(), " selected"));
        }
        if (i32::from(self.special_forces) + i32::from(self.special_forces_at_half_strength)) > Battle::max_forces(game, p, true) {
            return Some(express!("Too many ", pl.special_force(), " selected"));
        }
        let mut _paid = 0;
        let cost = Battle::cost_of_forces(game, p, i32::from(self.forces), i32::from(self.special_forces), &mut _paid);
        if i32::from(self.ally_contribution_amount) > cost {
            return Some(express!("Your ally is paying more than needed"));
        }
        if i32::from(self.ally_contribution_amount) > Battle::max_ally_resources(game, p, i32::from(self.forces), i32::from(self.special_forces)) {
            return Some(express!("Your ally won't pay that much"));
        }
        if cost > (i32::from(pl.resources) + i32::from(self.ally_contribution_amount)) {
            return Some(express!(
                "You can't pay ",
                Payment::of(cost),
                " to fight with ",
                i32::from(self.forces) + i32::from(self.special_forces),
                " forces at full strength"
            ));
        }
        if self.hero.is_none() && !Battle::valid_battle_heroes(game, p).is_empty() && !game.applicable(Rule::BattleWithoutLeader) {
            return Some(express!("You must select a leader"));
        }
        if let Some(hero) = self.hero {
            if !Battle::valid_battle_heroes(game, p).contains(&hero) {
                return Some(express!("Invalid leader"));
            }
        }
        if self.weapon().is_some() && self.weapon() == self.defense() {
            return Some(express!("Can't use the same card as weapon and defense"));
        }
        if self.hero.is_none() && (self.weapon().is_some() || self.defense().is_some()) {
            return Some(express!("Can't use treachery cards without a leader"));
        }
        if let Some(hero @ Hero::Leader(_)) = self.hero {
            if let Some(t) = game.battle.current_battle.as_ref().expect("current battle").territory {
                if !game.can_fight_in(hero, t) {
                    return Some(express!("Selected leader already fought in another territory"));
                }
            }
        }
        if self.hero.is_none() && self.messiah {
            return Some(express!("Can't use ", Concept::Messiah, " without a leader"));
        }
        if self.messiah && !Battle::messiah_may_be_used_in_battle(game, p) {
            return Some(express!(Concept::Messiah, " is not available"));
        }
        if self.weapon().is_none() && self.defense().is_some_and(|d| d.type_() == TreacheryCardType::WeirdingWay) {
            return Some(express!("You can't use ", TreacheryCardType::WeirdingWay, " as defense without using a weapon"));
        }
        if self.defense().is_none() && self.weapon().is_some_and(|w| w.type_() == TreacheryCardType::Chemistry) {
            return Some(express!("You can't use ", TreacheryCardType::Chemistry, " as weapon without using a defense"));
        }
        let battle_territory = game.battle.current_battle.as_ref().expect("current battle").territory;
        if !Battle::valid_weapons(game, p, self.defense(), self.hero, battle_territory, true).contains(&self.weapon()) {
            return Some(express!("Invalid weapon"));
        }
        let battle_territory = game.battle.current_battle.as_ref().expect("current battle").territory;
        if !Battle::valid_defenses(game, p, self.weapon(), battle_territory, true).contains(&self.defense()) {
            return Some(express!("Invalid defense"));
        }
        if game.is_in_front_of_shield(self.hero) {
            return Some(express!(self.hero, " is currently in front of your player shield"));
        }
        if let Some(hero) = self.hero {
            if i32::from(self.banker_bonus) > 0 && !game.skilled_as(hero, LeaderSkill::Banker) {
                return Some(express!("Only a leader skilled as ", LeaderSkill::Banker, " can be boosted by ", Concept::Resource));
            }
        }
        if i32::from(self.banker_bonus) > Battle::max_banker_boost(game, self.player(), self.hero) {
            return Some(express!("You cannot boost your leader this much"));
        }
        if (cost + i32::from(self.banker_bonus)) > (i32::from(pl.resources) + i32::from(self.ally_contribution_amount)) {
            return Some(express!("You can't pay this ", LeaderSkill::Banker, " bonus"));
        }

        // var opponent = Game.CurrentBattle.OpponentOf(p)!.Faction;  evaluated even with no seen plans.
        let opponent = game.player(game.battle.current_battle.as_ref().expect("current battle").opponent_of(game, p).expect("opponent of")).faction;
        for (aspect, seen) in game.battle.prescience_seen_plans.iter() {
            if seen.initiator() == self.initiator() && !self.matches(game, seen, *aspect, opponent.expect("faction")) {
                return Some(express!("Your ", *aspect, " was revealed by prescience and can't change"));
            }
        }

        if self.hero.is_some() && Battle::affected_by_voice(game, self.player(), game.battle.current_voice.as_ref()) {
            if let Some(current_voice) = game.battle.current_voice.as_ref().filter(|v| v.must) {
                let vt = current_voice.type_;
                if vt != Some(TreacheryCardType::Mercenary)
                    && pl
                        .treachery_cards
                        .iter()
                        .any(|c| Voice::is_voiced_by(game, true, true, c.type_(), vt) || Voice::is_voiced_by(game, false, true, c.type_(), vt))
                    && self.weapon().map_or(true, |w| !Voice::is_voiced_by(game, true, true, w.type_(), vt))
                    && self.defense().map_or(true, |d| !Voice::is_voiced_by(game, false, true, d.type_(), vt))
                {
                    return Some(express!("You must use a ", vt));
                }
            }
        }

        None
    }

    pub fn determine_force_supplier(g: &Game, player_that_fights: PlayerId) -> PlayerId {
        let p = g.player(player_that_fights);
        if p.is(Faction::Ecaz) && g.battle.current_ecaz_or_ally_fighter != Some(None) && !g.prevented(FactionAdvantage::EcazOccupation) {
            return p.allied_player(g).unwrap_or(player_that_fights);
        }

        player_that_fights
    }

    pub fn force_value(g: &Game, player: Faction, opponent: Faction, forces: i32, special_forces: i32, forces_at_half_strength: i32, special_forces_at_half_strength: i32) -> HalfPoints {
        let emperor_cunning = g.turn.current_emperor_cunning.as_ref().is_some_and(|c| c.initiator() == Some(player));

        let nr_of_forces_to_count_as_special_due_to_emperor_cunning = if emperor_cunning { 5.min(forces) } else { 0 };
        let forces_adjusted_for_cunning = forces - nr_of_forces_to_count_as_special_due_to_emperor_cunning;
        let special_forces_adjusted_for_cunning = special_forces + nr_of_forces_to_count_as_special_due_to_emperor_cunning;

        let nr_of_forces_at_half_strength_to_count_as_special_due_to_emperor_cunning = if g.turn.current_emperor_cunning.as_ref().is_some_and(|c| c.initiator() == Some(player)) {
            0.max(5.min(forces_at_half_strength) - nr_of_forces_to_count_as_special_due_to_emperor_cunning)
        } else {
            0
        };
        let forces_at_half_strength_adjusted_for_cunning = forces_at_half_strength - nr_of_forces_at_half_strength_to_count_as_special_due_to_emperor_cunning;
        let special_forces_at_half_strength_adjusted_for_cunning =
            special_forces_at_half_strength + nr_of_forces_at_half_strength_to_count_as_special_due_to_emperor_cunning;

        let special_force_strength = Battle::determine_special_force_strength(g, player, opponent);
        let normal_force_strength = Battle::determine_normal_force_strength(g, player);

        normal_force_strength * forces_adjusted_for_cunning
            + special_force_strength * special_forces_adjusted_for_cunning
            + Battle::determine_normal_force_strength_without_spice(g, player) * forces_at_half_strength_adjusted_for_cunning
            + Battle::determine_special_force_strength_without_spice(g, player, opponent) * special_forces_at_half_strength_adjusted_for_cunning
    }

    /// The elements of this player's plan that the other side's prescience lets them see. Eager: the only
    /// caller (`BattleRevision`) enumerates once, and its loop body writes only `PrescienceSeenPlans`, which
    /// this query does not read. Nothing in here throws once `CurrentBattle` is set.
    pub fn aspects_seen_by_opponent(g: &Game, p: PlayerId) -> Vec<Option<PrescienceAspect>> {
        let battle = g.battle.current_battle.as_ref().expect("current battle");
        let mut result = Vec::new();
        if let Some(prescience) = g.battle.current_prescience.as_ref() {
            if battle.opponent_of_faction(g, prescience.initiator()) == Some(p) {
                result.push(prescience.aspect);
            }
        }
        if let Some(nexus_prescience) = g.turn.current_nexus_prescience.as_ref() {
            if battle.opponent_of_faction(g, nexus_prescience.initiator()) == Some(p) {
                result.push(nexus_prescience.atreides_prescience_aspect);
            }
        }
        if g.battle.atreides_karma && battle.opponent_of_faction(g, Faction::Atreides) == Some(p) {
            result.extend([Some(PrescienceAspect::Leader), Some(PrescienceAspect::Weapon), Some(PrescienceAspect::Defense), Some(PrescienceAspect::Dial)]);
        }
        result
    }

    fn matches(&self, game: &Game, seen: &Battle, aspect: Option<PrescienceAspect>, opponent: Faction) -> bool {
        match aspect {
            Some(PrescienceAspect::Leader) => self.hero == seen.hero,
            Some(PrescienceAspect::Weapon) => self.weapon() == seen.weapon(),
            Some(PrescienceAspect::Defense) => self.defense() == seen.defense(),
            Some(PrescienceAspect::Dial) => self.dial(game, opponent) == seen.dial(game, opponent),
            None => true,
        }
    }

    pub fn is_using_portable_antidote(g: &Game, faction: Faction) -> bool {
        g.battle.current_portable_antidote_used.as_ref().map(|e| e.initiator()) == Some(Some(faction))
    }

    /// The strength of one special force with spice support.
    pub fn determine_special_force_strength(g: &Game, player: Faction, opponent: Faction) -> HalfPoints {
        if player == Faction::Fremen && g.prevented(FactionAdvantage::FremenSpecialForceBonus) {
            return HalfPoints::whole(1);
        }
        if player == Faction::Emperor
            && (g.prevented(FactionAdvantage::EmperorSpecialForceBonus) || opponent == Faction::Fremen || g.occupier_of(World::SalusaSecundus).is_some())
        {
            return HalfPoints::whole(1);
        }
        if player == Faction::Ixian && g.prevented(FactionAdvantage::IxianSpecialForceBonus) {
            return HalfPoints::whole(1);
        }
        if player == Faction::BeneGesserit {
            return HalfPoints::whole(1);
        }

        HalfPoints::whole(2)
    }

    /// The strength of one special force without spice support: half, or full for the Emperor
    /// at high threshold on Salusa Secundus.
    pub fn determine_special_force_strength_without_spice(g: &Game, player: Faction, opponent: Faction) -> HalfPoints {
        let full = Battle::determine_special_force_strength(g, player, opponent);
        if player == Faction::Emperor && g.has_high_threshold_in(player, World::SalusaSecundus) {
            return full;
        }
        full.halved().expect("a special force's strength is whole")
    }

    /// The strength of one normal force with spice support.
    pub fn determine_normal_force_strength(g: &Game, player: Faction) -> HalfPoints {
        if player == Faction::Ixian && g.turn.current_ixian_cunning.is_none() {
            return HalfPoints::HALF;
        }

        HalfPoints::whole(1)
    }

    /// The strength of one normal force without spice support: half, or full for the Ixians,
    /// whose suboids don't need spice.
    pub fn determine_normal_force_strength_without_spice(g: &Game, player: Faction) -> HalfPoints {
        let full = Battle::determine_normal_force_strength(g, player);
        if player == Faction::Ixian {
            return full;
        }
        full.halved().expect("only Ixian forces are worth half a point")
    }

    pub fn must_pay_for_any_forces_in_battle(g: &Game, p: PlayerId) -> bool {
        let pl = g.player(p);
        let does_not_need_to_support_forces = !g.prevented(FactionAdvantage::FremenNotPayingForBattles)
            && (pl.faction == Some(Faction::Fremen) || (pl.faction == Some(Faction::Ecaz) && pl.ally == Some(Faction::Fremen)));

        g.applicable(Rule::AdvancedCombat) && !does_not_need_to_support_forces
    }

    pub fn must_pay_for_special_forces_in_battle(g: &Game, p: PlayerId) -> bool {
        Battle::must_pay_for_any_forces_in_battle(g, p)
            && !(g.player(p).is(Faction::Emperor) && g.has_high_threshold_in(Faction::Emperor, World::SalusaSecundus))
    }

    pub fn messiah_may_be_used_in_battle(g: &Game, p: PlayerId) -> bool {
        Battle::messiah_available_for_battle(g, p)
    }

    pub fn cost_of_forces(g: &Game, p: PlayerId, amount_of_forces_at_full_strength: i32, amount_of_special_forces_at_full_strength: i32, paid_by_arrakeen: &mut i32) -> i32 {
        let cost = (amount_of_forces_at_full_strength * Battle::normal_force_cost(g, p)) + (amount_of_special_forces_at_full_strength * Battle::special_force_cost(g, p));
        *paid_by_arrakeen = Battle::cost_reduction(g, p).min(cost);
        cost - (*paid_by_arrakeen)
    }

    pub fn cost_reduction(g: &Game, p: PlayerId) -> i32 {
        match g.battle.current_battle.as_ref().and_then(|b| b.territory) {
            Some(t) if g.has_stronghold_advantage(g.player(p).faction, StrongholdAdvantage::FreeResourcesForBattles, t) => 2,
            _ => 0,
        }
    }

    pub fn normal_force_cost(g: &Game, p: PlayerId) -> i32 {
        if Battle::must_pay_for_any_forces_in_battle(g, p) {
            if g.player(p).faction == Some(Faction::Ixian) {
                return 0;
            }
            return 1;
        }

        0
    }

    pub fn special_force_cost(g: &Game, p: PlayerId) -> i32 {
        if g.player(p).is(Faction::Emperor) && g.player(p).has_high_threshold(g, World::SalusaSecundus) {
            return 0;
        }

        if Battle::must_pay_for_any_forces_in_battle(g, p) {
            return 1;
        }

        0
    }

    pub fn max_resources(g: &Game, p: PlayerId, forces: i32, special_forces: i32) -> i32 {
        i32::from(g.player(p).resources).min(0.max(forces + special_forces - Battle::cost_reduction(g, p)))
    }

    pub fn max_ally_resources(g: &Game, p: PlayerId, forces: i32, special_forces: i32) -> i32 {
        g.resources_your_ally_can_pay(p).min(0.max(forces + special_forces - Battle::cost_reduction(g, p)))
    }

    pub fn max_forces(g: &Game, p: PlayerId, special_forces: bool) -> i32 {
        let force_supplier = Battle::determine_force_supplier(g, p);

        let Some(territory) = g.battle.current_battle.as_ref().and_then(|b| b.territory) else {
            return 0;
        };
        let fs = g.player(force_supplier);

        if !special_forces {
            if fs.faction == Some(Faction::Richese) && fs.special_forces_in_territory(Some(territory)) > 0 {
                return fs.forces_in_reserve().min(g.ship_move.current_no_field_value) + fs.forces_in_territory(Some(territory));
            }
            return fs.forces_in_territory(Some(territory));
        }

        if fs.faction != Some(Faction::Richese) {
            return fs.special_forces_in_territory(Some(territory));
        }
        0
    }

    pub fn battles_to_be_fought(g: &Game, player: PlayerId, return_only_one_battle: bool) -> Vec<Fight> {
        let mut result: Vec<Fight> = Vec::new();

        let may_battle_under_storm = g.applicable(Rule::BattlesUnderStorm);
        let ecaz = g.get_player(Some(Faction::Ecaz));
        let pl = g.player(player);

        let occupied_locations = pl.occupied_locations(g);
        for occupied_location in occupied_locations
            .into_iter()
            .filter(|l| (may_battle_under_storm || !Sector::in_storm(l.sector(&g.board.map), g.board.sector_in_storm)) && l.id() != g.board.map.polar_sink().id())
        {
            let locations_within_range = g.board.map.find_neighbours_within_territory(occupied_location, false, g.board.sector_in_storm);

            let occupying_forces = g.occupying_forces_on_planet();
            for (key, battalions) in occupying_forces
                .iter()
                .filter(|(k, _)| (may_battle_under_storm || !Sector::in_storm(k.sector(&g.board.map), g.board.sector_in_storm)) && locations_within_range.contains(k))
            {
                let _ = key;
                let defenders = battalions.iter().filter(|b| Some(b.faction) != pl.faction && Some(b.faction) != pl.ally);

                for faction in defenders.map(|b| b.faction) {
                    if let Some(ecaz) = ecaz {
                        let ecaz_ally = g.player(ecaz).ally;
                        if (faction == Faction::Ecaz && result.iter().any(|f| Some(f.faction) == ecaz_ally))
                            || (Some(faction) == ecaz_ally && result.iter().any(|f| f.faction == Faction::Ecaz))
                        {
                            continue;
                        }
                    }

                    result.push(Fight::new(occupied_location.territory(), faction));

                    // C# parity: `break` only leaves the innermost loop.
                    if return_only_one_battle {
                        break;
                    }
                }
            }
        }

        result
    }

    pub fn affected_by_voice(g: &Game, p: PlayerId, voice: Option<&Rc<Voice>>) -> bool {
        let Some(voice) = voice else {
            return false;
        };
        let opponent = g.battle.current_battle.as_ref().and_then(|b| b.opponent_of(g, p));
        match opponent {
            Some(o) => voice.initiator() == g.player(o).faction || voice.initiator() == g.player(o).ally,
            None => false,
        }
    }

    pub fn valid_battle_heroes(g: &Game, p: PlayerId) -> Vec<Hero> {
        let affected_by_voice = Battle::affected_by_voice(g, p, g.battle.current_voice.as_ref());
        let must_use_cheap_hero =
            affected_by_voice && g.battle.current_voice.as_ref().is_some_and(|v| v.must && v.type_ == Some(TreacheryCardType::Mercenary));
        let may_not_use_cheap_hero =
            affected_by_voice && g.battle.current_voice.as_ref().is_some_and(|v| v.may_not() && v.type_ == Some(TreacheryCardType::Mercenary));

        let mut result: Vec<Hero> = Vec::new();

        if must_use_cheap_hero && !Battle::cheap_heroes(g, p).next().is_none() {
            result.extend(Battle::cheap_heroes(g, p).into_iter().map(Hero::TreacheryCard));
        } else {
            result.extend(
                g.player(p)
                    .leaders
                    .iter()
                    .copied()
                    .filter(|l| g.is_alive(Hero::Leader(*l)) && g.can_join_current_battle(Hero::Leader(*l)))
                    .map(Hero::Leader),
            );
            if !may_not_use_cheap_hero {
                result.extend(Battle::cheap_heroes(g, p).into_iter().map(Hero::TreacheryCard));
            }
        }

        result
    }

    pub fn valid_weapons(g: &Game, p: PlayerId, selected_defense: Option<TreacheryCard>, selected_hero: Option<Hero>, territory_of_battle: Option<Territory>, including_none: bool) -> Vec<Option<TreacheryCard>> {
        let mut result: Vec<Option<TreacheryCard>> = Vec::new();

        if Battle::valid_battle_heroes(g, p).is_empty() {
            if including_none {
                result.push(None);
            }
            return result;
        }

        let with_planetologist = selected_hero.is_some_and(|h| g.skilled_as(h, LeaderSkill::Planetologist));
        let playable_weapons = Battle::cards_playable_as_weapon(g, p, selected_defense, territory_of_battle, with_planetologist);

        let mut resolved_by_voice_rule = false;

        if Battle::affected_by_voice(g, p, g.battle.current_voice.as_ref()) {
            let voice = g.battle.current_voice.as_ref().expect("current voice");
            if voice.must {
                let vt = voice.type_;
                if selected_defense.is_some_and(|d| Some(d.type_()) == vt) {
                    result.extend(playable_weapons.iter().copied().map(Some));
                    if including_none {
                        result.push(None);
                    }
                    resolved_by_voice_rule = true;
                } else if playable_weapons.iter().any(|w| Voice::is_voiced_by(g, true, true, w.type_(), vt)) {
                    result.extend(playable_weapons.iter().copied().filter(|w| Voice::is_voiced_by(g, true, true, w.type_(), vt)).map(Some));
                    resolved_by_voice_rule = true;
                }
            } else if voice.may_not() {
                let vt = voice.type_;
                result.extend(playable_weapons.iter().copied().filter(|w| !Voice::is_voiced_by(g, true, false, w.type_(), vt)).map(Some));
                if including_none {
                    result.push(None);
                }
                resolved_by_voice_rule = true;
            }
        }

        if !resolved_by_voice_rule {
            result.extend(playable_weapons.iter().copied().map(Some));
            if including_none {
                result.push(None);
            }
        }

        result
    }

    pub fn valid_defenses(g: &Game, p: PlayerId, selected_weapon: Option<TreacheryCard>, territory_of_battle: Option<Territory>, including_none: bool) -> Vec<Option<TreacheryCard>> {
        let mut result: Vec<Option<TreacheryCard>> = Vec::new();

        if Battle::valid_battle_heroes(g, p).is_empty() {
            if including_none {
                result.push(None);
            }
            return result;
        }

        let playable_defenses = Battle::cards_playable_as_defense(g, p, selected_weapon, territory_of_battle);

        let mut resolved_by_voice_rule = false;

        if Battle::affected_by_voice(g, p, g.battle.current_voice.as_ref()) {
            let voice = g.battle.current_voice.as_ref().expect("current voice");
            if voice.must {
                let vt = voice.type_;
                if selected_weapon.is_some_and(|w| Some(w.type_()) == vt) {
                    result.extend(playable_defenses.iter().copied().map(Some));
                    if including_none {
                        result.push(None);
                    }
                    resolved_by_voice_rule = true;
                } else if playable_defenses.iter().any(|w| Voice::is_voiced_by(g, false, true, w.type_(), vt)) {
                    result.extend(playable_defenses.iter().copied().filter(|w| Voice::is_voiced_by(g, false, true, w.type_(), vt)).map(Some));
                    resolved_by_voice_rule = true;
                }
            } else if voice.may_not() {
                let vt = voice.type_;
                result.extend(playable_defenses.iter().copied().filter(|w| !Voice::is_voiced_by(g, false, false, w.type_(), vt)).map(Some));
                if including_none {
                    result.push(None);
                }
                resolved_by_voice_rule = true;
            }
        }

        if !resolved_by_voice_rule {
            result.extend(playable_defenses.iter().copied().map(Some));
            if including_none {
                result.push(None);
            }
        }

        result
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn cheap_heroes(game: &Game, p: PlayerId) -> impl Iterator<Item = TreacheryCard> {
        game.player(p).treachery_cards.iter().copied().filter(move |c| c.type_() == TreacheryCardType::Mercenary)
    }

    pub fn cards_playable_as_weapon(g: &Game, p: PlayerId, with_defense: Option<TreacheryCard>, territory_of_battle: Option<Territory>, with_planetologist: bool) -> Vec<TreacheryCard> {
        let pl = g.player(p);
        let fighting_on_own_homeworld = territory_of_battle.is_some_and(|t| pl.is_native(t));
        let forces_to_reveal_under_no_field =
            if territory_of_battle.is_some() && pl.is(Faction::Richese) && pl.special_forces_in_territory(territory_of_battle) != 0 {
                pl.forces_in_reserve().min(g.ship_move.current_no_field_value)
            } else {
                0
            };

        pl.treachery_cards
            .iter()
            .copied()
            .filter(|c| {
                (c.type_() != TreacheryCardType::Chemistry && (c.is_weapon() || c.type_() == TreacheryCardType::Useless))
                    || (c.type_() == TreacheryCardType::Chemistry
                        && with_defense.is_some_and(|d| d.is_defense())
                        && with_defense.expect("with defense").type_() != TreacheryCardType::WeirdingWay)
                    || (with_planetologist && c.is_atreides())
                    || (c.type_() == TreacheryCardType::Reinforcements
                        && (pl.forces_in_reserve() + pl.special_forces_in_reserve() - forces_to_reveal_under_no_field) >= 3)
                    || (!fighting_on_own_homeworld && c.type_() == TreacheryCardType::HarassAndWithdraw)
            })
            .collect()
    }

    pub fn max_banker_boost(g: &Game, p: PlayerId, hero: Option<Hero>) -> i32 {
        if hero.is_some_and(|h| g.skilled_as(h, LeaderSkill::Banker)) {
            i32::from(g.player(p).resources).min(3)
        } else {
            0
        }
    }

    pub fn cards_playable_as_defense(g: &Game, p: PlayerId, with_weapon: Option<TreacheryCard>, territory_of_battle: Option<Territory>) -> Vec<TreacheryCard> {
        let pl = g.player(p);
        let fighting_on_own_homeworld = territory_of_battle.is_some_and(|t| pl.is_native(t));
        let forces_to_reveal_under_no_field =
            if territory_of_battle.is_some() && pl.is(Faction::Richese) && pl.special_forces_in_territory(territory_of_battle) != 0 {
                pl.forces_in_reserve().min(g.ship_move.current_no_field_value)
            } else {
                0
            };

        pl.treachery_cards
            .iter()
            .copied()
            .filter(|c| {
                (c.type_() != TreacheryCardType::WeirdingWay && (c.is_defense() || c.type_() == TreacheryCardType::Useless))
                    || (c.type_() == TreacheryCardType::WeirdingWay
                        && with_weapon.is_some_and(|w| w.is_weapon())
                        && with_weapon.expect("with weapon").type_() != TreacheryCardType::Chemistry)
                    || (c.type_() == TreacheryCardType::Reinforcements
                        && (pl.forces_in_reserve() + pl.special_forces_in_reserve() - forces_to_reveal_under_no_field) >= 3)
                    || (!fighting_on_own_homeworld && c.type_() == TreacheryCardType::HarassAndWithdraw)
            })
            .collect()
    }

    pub fn messiah_available_for_battle(g: &Game, p: PlayerId) -> bool {
        g.player(p).messiah_available(g)
            && g.can_join_current_battle(LeaderManager::messiah())
            && !g.prevented(FactionAdvantage::AtreidesUseMessiah)
            && g.applicable(Rule::AtreidesMessiah)
    }

    pub fn determine_skill_bonus(g: &Game, plan: &Battle, activated_skill: &mut Option<LeaderSkill>) -> i32 {
        Battle::determine_skill_bonus_of_parts(
            g,
            plan.player(),
            plan.hero,
            plan.weapon(),
            plan.defense(),
            i32::from(plan.banker_bonus),
            activated_skill,
        )
    }

    pub fn determine_skill_bonus_of_parts(g: &Game, player: PlayerId, hero: Option<Hero>, weapon: Option<TreacheryCard>, defense: Option<TreacheryCard>, banker_bonus: i32, activated_skill: &mut Option<LeaderSkill>) -> i32 {
        let hero_skilled = |skill: LeaderSkill| hero.is_some_and(|h| g.skilled_as(h, skill));

        if hero_skilled(LeaderSkill::Thinker) {
            *activated_skill = Some(LeaderSkill::Thinker);
            return 2;
        }

        if hero_skilled(LeaderSkill::Banker) {
            *activated_skill = Some(LeaderSkill::Banker);
            return banker_bonus;
        }

        if weapon.is_some_and(|w| w.is_useless()) || defense.is_some_and(|d| d.is_useless()) {
            if hero_skilled(LeaderSkill::Warmaster) {
                *activated_skill = Some(LeaderSkill::Warmaster);
                return 3;
            }

            if g.has_skill_in_front_of_shield(player, LeaderSkill::Warmaster) {
                *activated_skill = Some(LeaderSkill::Warmaster);
                return 1;
            }
        }

        if defense.is_some_and(|d| d.is_projectile_defense()) {
            if hero_skilled(LeaderSkill::Adept) {
                *activated_skill = Some(LeaderSkill::Adept);
                return 3;
            }

            if g.has_skill_in_front_of_shield(player, LeaderSkill::Adept) {
                *activated_skill = Some(LeaderSkill::Adept);
                return 1;
            }
        }

        if weapon.is_some_and(|w| w.is_projectile_weapon()) {
            if hero_skilled(LeaderSkill::Swordmaster) {
                *activated_skill = Some(LeaderSkill::Swordmaster);
                return 3;
            }

            if g.has_skill_in_front_of_shield(player, LeaderSkill::Swordmaster) {
                *activated_skill = Some(LeaderSkill::Swordmaster);
                return 1;
            }
        }

        if weapon.is_some_and(|w| w.is_atreides()) {
            if hero_skilled(LeaderSkill::Planetologist) {
                *activated_skill = Some(LeaderSkill::Planetologist);
                return 2;
            }
        }

        if defense.is_some_and(|d| d.is_poison_defense())
            || g.battle.current_portable_antidote_used.as_ref().is_some_and(|e| e.player_opt() == Some(player))
        {
            if hero_skilled(LeaderSkill::KillerMedic) {
                *activated_skill = Some(LeaderSkill::KillerMedic);
                return 3;
            }

            if g.has_skill_in_front_of_shield(player, LeaderSkill::KillerMedic) {
                *activated_skill = Some(LeaderSkill::KillerMedic);
                return 1;
            }
        }

        if weapon.is_some_and(|w| w.is_poison_weapon() || w.is_poison_tooth()) {
            if hero_skilled(LeaderSkill::MasterOfAssassins) {
                *activated_skill = Some(LeaderSkill::MasterOfAssassins);
                return 3;
            }

            if g.has_skill_in_front_of_shield(player, LeaderSkill::MasterOfAssassins) {
                *activated_skill = Some(LeaderSkill::MasterOfAssassins);
                return 1;
            }
        }

        *activated_skill = None;
        0
    }

    pub fn determine_skill_penalty(g: &Game, plan: &Battle, opponent: PlayerId, activated_skill: &mut Option<LeaderSkill>) -> i32 {
        Battle::determine_skill_penalty_of_hero(g, plan.hero, opponent, activated_skill)
    }

    pub fn determine_skill_penalty_of_hero(g: &Game, hero: Option<Hero>, opponent: PlayerId, activated_skill: &mut Option<LeaderSkill>) -> i32 {
        if hero.is_some_and(|h| g.skilled_as(h, LeaderSkill::Bureaucrat)) {
            *activated_skill = Some(LeaderSkill::Bureaucrat);
            let o = g.player(opponent);
            return g.board.map.strongholds().filter(|l| o.occupies(g, Some(*l))).count() as i32;
        }

        *activated_skill = None;
        0
    }
}

impl GameEventMethods for Battle {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> EventType {
        EventType::Battle
    }

    fn into_event(self: Rc<Self>) -> Event {
        Event::Battle(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        Battle::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        Battle::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        Battle::get_message(self, game)
    }

}
