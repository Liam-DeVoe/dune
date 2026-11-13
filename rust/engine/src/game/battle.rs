use crate::prelude::*;
use dotnet_compat::math::f64_to_i32;

/// `vec![ExpressionElement::from(a), ...]`: the `params object?[]` of `Log(...)`.
macro_rules! le {
    ($($e:expr),* $(,)?) => {
        vec![$($crate::messages::expression::ExpressionElement::from($e)),*]
    };
}

impl Game {
    pub fn initiate_battle(&mut self) {
        let Some(current_battle) = self.battle_about_to_start.clone() else {
            panic!("Cannot initiate battle when BattleAboutToStart is null");
        };
        self.current_battle = Some(current_battle.clone());
        self.update_battle_report_participants(&current_battle);

        self.chosen_hms_advantage = Some(StrongholdAdvantage::None);
        self.battle_outcome = None;
        self.nr_of_battles_fought += 1;

        let aggressive_player = current_battle.aggressive_player(self);
        self.announce_hero_availability(aggressive_player);
        let defending_player = current_battle.defending_player(self);
        self.announce_hero_availability(defending_player);
        let aggressive_player = current_battle.aggressive_player(self);
        let defending_player = current_battle.defending_player(self);
        self.assign_battle_wheels(aggressive_player, defending_player);
    }

    pub fn update_battle_report_participants(&mut self, battle: &BattleInitiated) {
        if !self.current_report().is_battle() {
            return;
        }
        let aggressor = battle.aggressor(self);
        self.current_report_mut().aggressor = aggressor;
        let defender = battle.defender(self);
        self.current_report_mut().defender = defender;
        let territory = battle.territory(self);
        self.current_report_mut().territory = territory;
    }

    pub fn assign_battle_wheels(&mut self, player1: Option<PlayerIndex>, player2: Option<PlayerIndex>) {
        let player1 = player1.and_then(crate::compat::cs::CsNull::opt);
        let player2 = player2.and_then(crate::compat::cs::CsNull::opt);
        self.has_battle_wheel.clear();
        if let Some(player1) = player1 {
            let f = self.player(player1).faction;
            self.has_battle_wheel.push(f);
        }
        if let Some(player2) = player2 {
            let f = self.player(player2).faction;
            self.has_battle_wheel.push(f);
        }
    }

    pub fn announce_hero_availability(&mut self, p: Option<PlayerIndex>) {
        let p = p.and_then(crate::compat::cs::CsNull::opt);
        let Some(p) = p else { return };
        if Battle::valid_battle_heroes(self, p).is_empty() {
            let f = self.player(p).faction;
            self.log(le![f, " have no leaders available for this battle"]);
        }
    }

    pub fn handle_revealed_battle_plans(&mut self) -> GameResult {
        let (Some(current_battle), Some(aggressor_plan), Some(defender_plan), Some(aggressor_traitor_action), Some(defender_traitor_action)) = (
            self.current_battle.clone(),
            self.aggressor_plan.clone(),
            self.defender_plan.clone(),
            self.aggressor_traitor_action.clone(),
            self.defender_traitor_action.clone(),
        ) else {
            return Ok(());
        };

        self.resolve_effect_of_owned_tueks_sietch(&aggressor_plan);
        self.resolve_effect_of_owned_tueks_sietch(&defender_plan);

        self.discard_one_time_cards_used_in_battle(&aggressor_traitor_action, &defender_traitor_action)?;

        self.resolve_battle(&current_battle, &aggressor_plan, &defender_plan, &aggressor_traitor_action, &defender_traitor_action)?;
        self.record_battle_outcome_in_report();

        if self.card_used_by_diplomat.is_some() {
            let card = self.card_used_by_diplomat;
            self.discard_treachery_card_boolean(card, true)?;
            self.card_used_by_diplomat = None;
        }

        if Some(aggressor_plan.initiator()) == self.battle_winner {
            self.activate_decipherer_if_applicable(&aggressor_plan);
        }
        if Some(defender_plan.initiator()) == self.battle_winner {
            self.activate_decipherer_if_applicable(&defender_plan);
        }

        if Some(aggressor_plan.initiator()) == self.battle_winner {
            self.activate_sandmaster_if_applicable(&aggressor_plan);
        }
        if Some(defender_plan.initiator()) == self.battle_winner {
            self.activate_sandmaster_if_applicable(&defender_plan);
        }

        if Some(aggressor_plan.initiator()) == self.battle_winner {
            self.resolve_effect_of_owned_sietch_tabr(&aggressor_plan, &defender_plan);
        }
        if Some(defender_plan.initiator()) == self.battle_winner {
            self.resolve_effect_of_owned_sietch_tabr(&defender_plan, &aggressor_plan);
        }

        if let Some(outcome) = self.battle_outcome.clone() {
            if Some(aggressor_plan.initiator()) == self.battle_winner {
                self.resolve_effect_of_occupied_jacurutu(&aggressor_plan, outcome.def_undialed_forces);
            }
            if Some(defender_plan.initiator()) == self.battle_winner {
                self.resolve_effect_of_occupied_jacurutu(&defender_plan, outcome.agg_undialed_forces);
            }
        }

        self.flip_bene_gesserit_advisors_when_alone();
        if let Some(territory) = current_battle.territory(self) {
            self.determine_occupation(territory);
        }

        if let Some(bureaucracy) = self.battle_triggered_bureaucracy.clone() {
            self.apply_bureaucracy(bureaucracy.payment_from, bureaucracy.payment_to);
            self.battle_triggered_bureaucracy = None;
        }

        if self.current_phase != Phase::Retreating {
            self.determine_how_to_proceed_after_revealing_battle_plans()?;
        }
        Ok(())
    }

    pub fn discard_one_time_cards_used_in_battle(&mut self, aggressor_call: &TreacheryCalled, defender_call: &TreacheryCalled) -> GameResult {
        let (Some(aggressor_plan), Some(defender_plan)) = (self.aggressor_plan.clone(), self.defender_plan.clone()) else {
            return Ok(());
        };

        let aggressor_keeps_cards = aggressor_call.succeeded(self) && !defender_call.succeeded(self);
        if !aggressor_keeps_cards {
            self.discard_one_time_cards(&aggressor_plan)?;
        }

        let defender_keeps_cards = defender_call.succeeded(self) && !aggressor_call.succeeded(self);
        if !defender_keeps_cards {
            self.discard_one_time_cards(&defender_plan)?;
        }
        Ok(())
    }

    pub fn resolve_battle(&mut self, b: &BattleInitiated, agg: &Rc<Battle>, def: &Rc<Battle>, aggressor_treachery: &TreacheryCalled, defender_treachery: &TreacheryCalled) -> GameResult {
        let territory = b.territory(self).required("territory")?;
        let outcome = Battle::determine_battle_outcome(agg, def, territory, self);
        self.battle_outcome = Some(outcome.clone());

        let lasgun_shield = !aggressor_treachery.succeeded(self)
            && !defender_treachery.succeeded(self)
            && (agg.has_laser() || def.has_laser())
            && (agg.has_shield() || def.has_shield());

        self.activate_smuggler(aggressor_treachery, defender_treachery, &outcome, lasgun_shield);

        self.handle_reinforcements(agg);
        self.handle_reinforcements(def);

        if aggressor_treachery.succeeded(self) || defender_treachery.succeeded(self) {
            self.traitor_called(b, agg, def, defender_treachery, agg.hero(), def.hero())?;
        } else if lasgun_shield {
            let territory = b.territory(self).required("territory")?;
            self.lasgun_shield_explosion(agg, def, agg.player(), def.player(), territory, agg.hero(), def.hero())?;
        } else {
            let territory = b.territory(self).required("territory")?;
            self.set_hero_locations(agg, territory);
            let territory = b.territory(self).required("territory")?;
            self.set_hero_locations(def, territory);
            let territory = b.territory(self).required("territory")?;
            self.handle_battle_outcome(agg, def, territory)?;
        }

        self.determine_if_captured_leaders_must_be_released();
        self.auditor_survived_battle = (agg.hero().is_some_and(|h| h.hero_type() == HeroType::Auditor) && self.is_alive(agg.hero().required("hero")?))
            || (def.hero().is_some_and(|h| h.hero_type() == HeroType::Auditor) && self.is_alive(def.hero().required("hero")?));
        Ok(())
    }

    pub fn record_battle_outcome_in_report(&mut self) {
        if !self.current_report().is_battle() {
            return;
        }
        self.current_report_mut().battle_decided = true;
        let winner = self.battle_winner.unwrap_or(Faction::None);
        self.current_report_mut().battle_winner = winner;
    }

    pub fn harkonnen_must_decide_to_capture(&self) -> bool {
        self.battle_winner == Some(Faction::Harkonnen)
            && self.applicable(Rule::HarkonnenCapturesOrKillsLeaders)
            && !self.prevented(FactionAdvantage::HarkonnenCaptureLeader)
    }

    pub fn capture_leader(&mut self) {
        let (Some(battle_winner), Some(aggressor_plan), Some(defender_plan)) = (self.battle_winner, self.aggressor_plan.clone(), self.defender_plan.clone()) else {
            return;
        };
        let victim = if aggressor_plan.by(battle_winner) { defender_plan } else { aggressor_plan };
        self.select_victim_of_harkonnen_winner(&victim);
    }

    pub fn handle_reinforcements(&mut self, plan: &Battle) {
        if plan.has_reinforcements() {
            let p = plan.player();
            let forces_to_remove = std::cmp::min(3, self.player(p).forces_in_reserve());
            Player::add_forces_to_reserves(self, p, -forces_to_remove);
            let pl = self.player_mut(p);
            pl.forces_killed += forces_to_remove;

            let special_forces_to_remove = 3 - forces_to_remove;
            if special_forces_to_remove > 0 {
                Player::add_special_forces_to_reserves(self, p, -special_forces_to_remove);
                let pl = self.player_mut(p);
                pl.special_forces_killed += special_forces_to_remove;
            }

            let force = self.player(p).force();
            let special_force = self.player(p).special_force();
            self.log(le![
                plan.initiator(),
                express_part_if!(forces_to_remove > 0; forces_to_remove, force),
                express_part_if!(forces_to_remove > 0 && special_forces_to_remove > 0; " and "),
                express_part_if!(special_forces_to_remove > 0; special_forces_to_remove, special_force),
                " reinforcements from reserves were killed"
            ]);
        }
    }

    pub fn activate_smuggler(&mut self, aggressor_treachery: &TreacheryCalled, defender_treachery: &TreacheryCalled, outcome: &BattleOutcome, lasgun_shield: bool) {
        let (Some(aggressor_plan), Some(defender_plan), Some(current_battle)) = (self.aggressor_plan.clone(), self.defender_plan.clone(), self.current_battle.clone()) else {
            return;
        };

        let agg_hero_survives = !defender_treachery.succeeded(self) && (aggressor_treachery.succeeded(self) || (!lasgun_shield && !outcome.agg_hero_killed));
        let def_hero_survives = !aggressor_treachery.succeeded(self) && (defender_treachery.succeeded(self) || (!lasgun_shield && !outcome.def_hero_killed));

        if agg_hero_survives && aggressor_plan.hero().is_some() {
            let territory = current_battle.territory(self).nre();
            self.activate_smuggler_if_applicable(aggressor_plan.player(), aggressor_plan.hero(), defender_plan.hero(), territory);
        }

        if def_hero_survives && defender_plan.hero().is_some() {
            let territory = current_battle.territory(self).nre();
            self.activate_smuggler_if_applicable(defender_plan.player(), defender_plan.hero(), aggressor_plan.hero(), territory);
        }
    }

    pub fn discard_one_time_cards(&mut self, plan: &Battle) -> GameResult {
        if let Some(card) = plan.hero().and_then(|h| h.as_treachery_card()) {
            self.discard_treachery_card_boolean(Some(card), true)?;
        }

        if let Some(weapon) = plan.weapon() {
            if weapon.is_artillery()
                || weapon.is_mirror_weapon()
                || weapon.is_rock_melter()
                || (weapon.is_poison_tooth() && !self.poison_tooth_cancelled)
                || !(weapon.is_weapon() || weapon.is_defense() || weapon.is_useless())
                || self
                    .current_diplomacy
                    .as_ref()
                    .is_some_and(|d| plan.initiator() == d.initiator() && Some(weapon) == d.card())
            {
                self.discard_treachery_card_boolean(plan.weapon(), true)?;
            }
        }

        if let Some(defense) = plan.defense() {
            if defense.is_portable_antidote() || !(defense.is_weapon() || defense.is_defense() || defense.is_useless()) {
                self.discard_treachery_card_boolean(plan.defense(), true)?;
            }
        }

        if let Some(used) = self.current_portable_antidote_used.clone() {
            if used.player() == plan.player() {
                let portable_antidote = self.player(used.player()).card(TreacheryCardType::PortableAntidote);
                if portable_antidote.is_some() {
                    self.discard_treachery_card_boolean(portable_antidote, true)?;
                }
            }
        }

        if let Some(melted) = self.current_rock_was_melted.clone() {
            if melted.player() == plan.player() {
                let melted_card = self.player(melted.player()).card(TreacheryCardType::Rockmelter);
                if melted_card.is_some() {
                    self.discard_treachery_card_boolean(melted_card, true)?;
                }
            }
        }
        Ok(())
    }

    pub fn activate_sandmaster_if_applicable(&mut self, plan: &Battle) {
        let Some(current_battle) = self.current_battle.clone() else {
            return;
        };

        let territory = current_battle.territory(self).nre();
        let location_with_resources = territory.locations().iter().copied().find(|l| self.resources_on_planet.contains_key(l));

        if location_with_resources.is_some()
            && plan.hero().is_some()
            && self.skilled_as(plan.hero().nre(), LeaderSkill::Sandmaster)
            && self.player(plan.player()).any_forces_in_territory(current_battle.territory(self)) > 0
        {
            let territory = current_battle.territory(self);
            self.log(le![LeaderSkill::Sandmaster, " adds ", Payment::of(3), " to ", territory]);
            self.change_resources_on_planet(location_with_resources.nre(), 3);
        }
    }

    pub fn activate_smuggler_if_applicable(&mut self, player: PlayerIndex, hero: Option<Hero>, opponent_hero: Option<Hero>, territory: Territory) {
        let hero = hero.and_then(crate::compat::cs::CsNull::opt);
        let opponent_hero = opponent_hero.and_then(crate::compat::cs::CsNull::opt);
        let Some(hero) = hero else { return };

        if self.skilled_as(hero, LeaderSkill::Smuggler) {
            let location_with_resources = territory.locations().iter().copied().find(|l| self.resources_on_planet.contains_key(l));
            if let Some(location_with_resources) = location_with_resources {
                let collected = std::cmp::min(self.resources_on_planet[&location_with_resources], hero.value_in_combat_against(opponent_hero));
                if collected > 0 {
                    let f = self.player(player).faction;
                    self.log(le![f, LeaderSkill::Smuggler, " collects ", Payment::of(collected), " from ", territory]);
                    self.change_resources_on_planet(location_with_resources, -collected);
                    let pl = self.player_mut(player);
                    pl.resources += collected;
                }
            }
        }
    }

    pub fn activate_decipherer_if_applicable(&mut self, plan: &Battle) {
        let player_is_skilled = self.skilled_as_player_leader_skill(plan.player(), LeaderSkill::Decipherer);
        let leader_is_skilled = plan.hero().is_some() && self.skilled_as(plan.hero().nre(), LeaderSkill::Decipherer);

        if player_is_skilled || leader_is_skilled {
            let p = plan.player();
            let traitor = self.traitor_deck.as_mut().nre().draw();
            self.traitors_decipherer_can_look_at.push(traitor);
            self.player_mut(p).known_non_traitors.insert(traitor);

            let traitor = self.traitor_deck.as_mut().nre().draw();
            self.traitors_decipherer_can_look_at.push(traitor);
            self.player_mut(p).known_non_traitors.insert(traitor);

            self.decipherer_may_replace_traitor = leader_is_skilled && !BattleConcluded::valid_traitors_to_replace(self, p).next().is_none();
        }
    }

    pub fn finish_decipherer_if_applicable(&mut self) {
        if self.traitors_decipherer_can_look_at.len() > 0 {
            let items = self.traitors_decipherer_can_look_at.clone();
            for item in items {
                self.traitor_deck.as_mut().nre().put_on_top(item);
            }

            self.traitor_deck.as_mut().nre().shuffle();
            self.stone(Milestone::Shuffled);
            self.traitors_decipherer_can_look_at.clear();
        }
    }

    pub fn resolve_effect_of_owned_tueks_sietch(&mut self, player_plan: &Battle) {
        let Some(current_battle) = self.current_battle.clone() else {
            return;
        };

        let territory = current_battle.territory(self).nre();
        if self.has_stronghold_advantage(player_plan.initiator(), StrongholdAdvantage::CollectResourcesForUseless, territory) {
            self.collect_tueks_sietch_bonus(player_plan.player(), player_plan.weapon());
            self.collect_tueks_sietch_bonus(player_plan.player(), player_plan.defense());
        }
    }

    pub fn collect_tueks_sietch_bonus(&mut self, player: PlayerIndex, card: Option<TreacheryCard>) {
        let card = card.and_then(crate::compat::cs::CsNull::opt);
        if let Some(card) = card.filter(|c| c.type_() == TreacheryCardType::Useless) {
            let f = self.player(player).faction;
            let tueks = self.map.tueks_sietch();
            self.log(le![tueks, " stronghold advantage: ", f, " collect ", Payment::of(2), " for playing ", card]);
            let pl = self.player_mut(player);
            pl.resources += 2;
        }
    }

    pub fn resolve_effect_of_owned_sietch_tabr(&mut self, winner_plan: &Battle, opponent_plan: &Battle) {
        let Some(current_battle) = self.current_battle.clone() else {
            return;
        };

        let territory = current_battle.territory(self).nre();
        if !self.has_stronghold_advantage(winner_plan.initiator(), StrongholdAdvantage::CollectResourcesForDial, territory) {
            return;
        }

        // (int)Math.Floor(float): the float widens to double.
        let collected = f64_to_i32((opponent_plan.dial(self, winner_plan.initiator()) as f64).floor());
        if collected <= 0 {
            return;
        }

        let tabr = self.map.sietch_tabr();
        self.log(le![tabr, " stronghold advantage: ", winner_plan.initiator(), " collect ", Payment::of(collected), " from enemy force dial"]);
        let pl = self.player_mut(winner_plan.player());
        pl.resources += collected;
    }

    pub fn resolve_effect_of_occupied_jacurutu(&mut self, winner_plan: &Battle, opponent_undialed_forces: i32) {
        let Some(current_battle) = self.current_battle.clone() else {
            return;
        };

        let jacurutu = self.map.jacurutu();
        if current_battle.territory(self) != Some(jacurutu.territory()) || opponent_undialed_forces <= 0 {
            return;
        }

        self.log(le![winner_plan.initiator(), " get ", Payment::of(opponent_undialed_forces), " from winning a fight in ", jacurutu]);
        let pl = self.player_mut(winner_plan.player());
        pl.resources += opponent_undialed_forces;
    }

    pub fn determine_how_to_proceed_after_revealing_battle_plans(&mut self) -> GameResult {
        if self.auditee().is_some() && !self.choam_leader_was_revealed_as_traitor() {
            self.prepare_audit()?;
        } else {
            let harkonnen_must_decide = self.harkonnen_must_decide_to_capture();
            // Enter(condition, FinishBattle, ...), with FinishBattle's failure passed on.
            if self.battle_winner == Some(Faction::None) {
                self.finish_battle()?;
            } else {
                self.enter_boolean_phase_phase(harkonnen_must_decide, Phase::CaptureDecision, Phase::BattleConclusion);
            }
            self.let_factions_discard_surplus_cards();
        }
        Ok(())
    }

    pub fn prepare_audit(&mut self) -> GameResult {
        let Some(auditee) = self.auditee() else {
            return Ok(());
        };

        let Some(random) = self.random.clone() else {
            panic!("Random is not initialized");
        };
        let mut auditable_cards = Deck::new(AuditCancelled::get_cards_that_may_be_audited(self), Some(random));

        if auditable_cards.items.len() > 0 {
            let nr_of_audited_cards = AuditCancelled::get_number_of_cards_that_may_be_audited(self);
            self.audited_cards.clear();
            auditable_cards.shuffle();
            let mut i = 0;
            while i < nr_of_audited_cards {
                let card = auditable_cards.try_draw()?;
                self.audited_cards.push(card);
                i += 1;
            }

            self.enter(Phase::AvoidingAudit, true);
        } else {
            let f = self.player(auditee).faction;
            self.log(le![f, " don't have cards to audit"]);
            let harkonnen_must_decide = self.harkonnen_must_decide_to_capture();
            // Enter(condition, FinishBattle, ...), with FinishBattle's failure passed on.
            if self.battle_winner == Some(Faction::None) {
                self.finish_battle()?;
            } else {
                self.enter_boolean_phase_phase(harkonnen_must_decide, Phase::CaptureDecision, Phase::BattleConclusion);
            }
        }

        self.let_factions_discard_surplus_cards();
        Ok(())
    }

    pub fn determine_if_captured_leaders_must_be_released(&mut self) {
        let Some(current_battle) = self.current_battle.clone() else {
            return;
        };

        let harkonnen = self.get_player(Some(Faction::Harkonnen));
        let Some(harkonnen) = harkonnen else { return };
        if !current_battle.is_aggressor_or_defender_faction(self, Faction::Harkonnen) {
            return;
        }

        //DetermineIfDeadLeaderMustBeReleased
        let dead_captives: Vec<Leader> = self
            .player(harkonnen)
            .leaders
            .iter()
            .copied()
            .filter(|l| self.captured_leaders.contains_key(l) && !self.is_alive(Hero::Leader(*l)))
            .collect();
        for captive in dead_captives {
            self.return_captured_leader(harkonnen, captive);
        }

        //DetermineIfLeaderUsedInBattleMustBeReleased
        if !self.harkonnen_do_not_have_to_return_used_captured_leader {
            let used_leader_in_battle = current_battle.plan_of(self, Some(harkonnen)).and_then(|p| p.hero());
            if let Some(leader) = used_leader_in_battle.and_then(|h| h.as_leader()) {
                if self.captured_leaders.contains_key(&leader) {
                    self.return_captured_leader(harkonnen, leader);
                }
            }
        }

        //DetermineIfCapturedLeadersMustBeReleasedWhenHarkonnenHasNoLeadersLeft
        if !self
            .player(harkonnen)
            .leaders
            .iter()
            .any(|l| !self.captured_leaders.contains_key(l) && self.is_alive(Hero::Leader(*l)))
        {
            let captives: Vec<Leader> = self.player(harkonnen).leaders.iter().copied().filter(|l| self.captured_leaders.contains_key(l)).collect();
            for captive in captives {
                self.return_captured_leader(harkonnen, captive);
            }
        }
    }

    pub fn return_captured_leader(&mut self, current_owner: PlayerIndex, to_return: Leader) {
        if let Some(value) = self.captured_leaders.get(&to_return).copied() {
            let original_player = self.get_player(Some(value)).nre();
            self.player_mut(original_player).leaders.push(to_return);
            let leaders = &mut self.player_mut(current_owner).leaders;
            if let Some(pos) = leaders.iter().position(|l| *l == to_return) {
                leaders.remove(pos);
            }
            self.captured_leaders.shift_remove(&to_return);

            if !self.is_alive(Hero::Leader(to_return)) {
                return;
            }

            if self.is_skilled(Some(Hero::Leader(to_return))) {
                self.set_in_front_of_shield(to_return, true);
            }
            let original_faction = self.player(original_player).faction;
            let owner_faction = self.player(current_owner).faction;
            self.log(le![to_return, " returns to ", original_faction, " after working for ", owner_faction]);
        }
    }

    pub fn choam_leader_was_revealed_as_traitor(&self) -> bool {
        let choam = self.get_player(Some(Faction::Choam));
        let Some(choam) = choam else { return false };
        match &self.current_battle {
            Some(current_battle) => {
                current_battle.is_aggressor_or_defender(self, choam)
                    && current_battle.treachery_of_opponent(self, choam).is_some_and(|t| t.succeeded(self))
            }
            None => false,
        }
    }

    pub fn handle_battle_outcome(&mut self, agg: &Battle, def: &Battle, territory: Territory) -> GameResult {
        let Some(outcome) = self.battle_outcome.clone() else {
            game_error!("A required value is missing");
        };

        let Some(winner) = outcome.winner else {
            game_error!("Battle winner is null");
        };
        let Some(loser) = outcome.loser else {
            game_error!("Battle loser is null");
        };
        let Some(winner_battle_plan) = outcome.winner_battle_plan.clone() else {
            game_error!("Winning battle plan is null");
        };
        let Some(loser_battle_plan) = outcome.loser_battle_plan.clone() else {
            game_error!("Losing battle plan is null");
        };

        self.log_if(
            outcome.agg_hero_skill_bonus != 0,
            le![agg.hero(), " ", outcome.agg_activated_bonus_skill, " bonus: ", outcome.agg_hero_skill_bonus],
        );
        self.log_if(
            outcome.def_hero_skill_bonus != 0,
            le![def.hero(), " ", outcome.def_activated_bonus_skill, " bonus: ", outcome.def_hero_skill_bonus],
        );

        self.log_if(
            outcome.agg_battle_penalty != 0,
            le![agg.hero(), " ", outcome.def_activated_penalty_skill, " penalty: ", outcome.agg_battle_penalty],
        );
        self.log_if(
            outcome.def_battle_penalty != 0,
            le![def.hero(), " ", outcome.agg_activated_penalty_skill, " penalty: ", outcome.def_battle_penalty],
        );

        // The Kwisatz Haderach bonus shows on the leader in the revealed plans.

        let winner_faction = self.player(winner).faction;
        self.battle_winner = Some(winner_faction);
        self.battle_loser = Some(self.player(loser).faction);

        if outcome.agg_hero_killed && agg.hero().is_some() {
            self.kill_leader_in_battle(agg.hero().required("hero")?, outcome.agg_hero_cause_of_death, winner, outcome.agg_hero_effective_strength);
        } else {
            let carthag = self.map.carthag();
            self.log_if(
                outcome.agg_saved_by_carthag,
                le![carthag, " stronghold advantage saves ", agg.hero(), " from death by ", TreacheryCardType::Poison],
            );
        }

        if outcome.def_hero_killed && def.hero().is_some() {
            self.kill_leader_in_battle(def.hero().required("hero")?, outcome.def_hero_cause_of_death, winner, outcome.def_hero_effective_strength);
        } else {
            let carthag = self.map.carthag();
            self.log_if(
                outcome.def_saved_by_carthag,
                le![carthag, " stronghold advantage saves ", def.hero(), " from death by ", TreacheryCardType::Poison],
            );
        }

        let def_faction = self.player(def.player()).faction;
        if BattleInitiated::is_aggressor_by_juice(self, def_faction) {
            self.log(le![agg.initiator(), " strength: ", outcome.agg_total]);
            self.log(le![def.initiator(), " strength: ", outcome.def_total, " (aggressor by ", TreacheryCardType::Juice, ")"]);
        } else {
            self.log_strength(agg.initiator(), outcome.agg_total);
            self.log_strength(def.initiator(), outcome.def_total);
        }

        self.loser_may_try_to_assassinate = self.battle_loser == Some(Faction::Moritani)
            && self.applicable(Rule::MoritaniAssassinate)
            && self.assassinated.iter().all(|l| Some(l.faction()) != self.battle_winner)
            && winner_battle_plan.hero().is_some_and(|h| h.as_leader().is_some())
            && self.is_alive(winner_battle_plan.hero().required("hero")?);

        let message = express!(winner_faction, " WIN THE BATTLE").with_fact(BattleFact::new(BattleFactKind::Winner, winner_faction));
        self.log(le![message]);

        self.handle_harass_and_withdraw(agg, territory);
        self.handle_harass_and_withdraw(def, territory);

        let loser_may_retreat = !outcome.loser_hero_killed()
            && loser_battle_plan.hero().is_some()
            && self.skilled_as(loser_battle_plan.hero().required("hero")?, LeaderSkill::Diplomat)
            && Retreat::max_total_forces(self, loser) > 0
            && (Retreat::max_forces(self, loser) > 0 || Retreat::max_special_forces(self, loser) > 0)
            && !Retreat::valid_targets(self, loser).next().is_none();

        if loser_may_retreat {
            self.enter(Phase::Retreating, true);
        } else {
            self.handle_losses()?;
        }
        Ok(())
    }

    pub fn handle_harass_and_withdraw(&mut self, plan: &Battle, territory: Territory) {
        let Some(current_battle) = self.current_battle.clone() else {
            return;
        };
        if !plan.weapon().is_some_and(|w| w.type_() == TreacheryCardType::HarassAndWithdraw)
            && !plan.defense().is_some_and(|d| d.type_() == TreacheryCardType::HarassAndWithdraw)
        {
            return;
        }

        let force_supplier = Battle::determine_force_supplier(self, plan.player());
        let battle_territory = current_battle.territory(self).nre();
        let undialed_normal_forces = self
            .player(force_supplier)
            .forces_in_territory(Some(battle_territory)) - plan.forces - plan.forces_at_half_strength;
        let battle_territory = current_battle.territory(self).nre();
        let undialed_special_forces = self
            .player(force_supplier)
            .special_forces_in_territory(Some(battle_territory)) - plan.special_forces - plan.special_forces_at_half_strength;
        Player::forces_to_reserves_territory_int32_boolean(self, force_supplier, territory, undialed_normal_forces, false);
        Player::forces_to_reserves_territory_int32_boolean(self, force_supplier, territory, undialed_special_forces, true);

        if (undialed_normal_forces + undialed_special_forces) > 0 {
            let force = self.player(force_supplier).force();
            let special_force = self.player(force_supplier).special_force();
            self.log(le![
                plan.initiator(),
                " withdraw ",
                express_part_if!(undialed_normal_forces > 0; undialed_normal_forces, force),
                express_part_if!(undialed_normal_forces > 0 && undialed_special_forces > 0; " and "),
                express_part_if!(undialed_special_forces > 0; undialed_special_forces, special_force),
                " to reserves"
            ]);
        }
    }

    pub fn handle_losses(&mut self) -> GameResult {
        let (Some(current_battle), Some(outcome)) = (self.current_battle.clone(), self.battle_outcome.clone()) else {
            return Ok(());
        };

        let (Some(winner), Some(winner_plan), Some(loser), Some(loser_plan)) =
            (outcome.winner, outcome.winner_battle_plan.clone(), outcome.loser, outcome.loser_battle_plan.clone())
        else {
            return Ok(());
        };

        let territory = current_battle.territory(self).required("territory")?;
        self.process_winner_losses(territory, winner, &winner_plan, false);
        let territory = current_battle.territory(self).required("territory")?;
        self.process_loser_losses(territory, loser, &loser_plan)?;
        Ok(())
    }

    pub fn is_protected_by_carthag_advantage(&self, plan: &Battle, territory: Territory) -> bool {
        self.has_stronghold_advantage(plan.initiator(), StrongholdAdvantage::CountDefensesAsAntidote, territory)
            && !plan.has_poison()
            && !plan.has_poison_tooth()
            && plan.defense().is_some_and(|d| d.is_defense())
    }

    pub fn process_loser_losses(&mut self, territory: Territory, loser: PlayerIndex, loser_gambit: &Battle) -> GameResult {
        let had_messiah_before_losses = self.player(loser).messiah_available(self);
        let force_supplier = Battle::determine_force_supplier(self, loser);
        self.process_ecaz_occupation_loser_losses(territory, loser);

        self.log_all_forces_lost(force_supplier, territory);
        self.pay_dialed_spice(loser, loser_gambit, false);
        Player::kill_all_forces_territory_boolean(self, force_supplier, territory, true);
        let may_keep = self.may_keep_cards_after_losing_battle(loser);
        self.lose_cards(loser_gambit, may_keep)?;

        if self.player(loser).messiah_available(self) && !had_messiah_before_losses {
            self.stone(Milestone::Messiah);
        }
        Ok(())
    }

    pub fn process_ecaz_occupation_winner_losses(&mut self, territory: Territory, winner: PlayerIndex) {
        if self.current_ecaz_battle_contribution > 0 && self.player(winner).or_ally_is(Faction::Ecaz) {
            let ecaz = self.get_player(Some(Faction::Ecaz));
            let Some(ecaz) = ecaz else { return };
            let amount = self.current_ecaz_battle_contribution;
            Player::kill_forces_territory_int32_boolean_boolean(self, ecaz, territory, amount, false, true);
            let force = self.player(ecaz).force();
            let amount = self.current_ecaz_battle_contribution;
            self.log(le![Faction::Ecaz, " lose ", amount, force]);
        }
    }

    pub fn process_ecaz_occupation_loser_losses(&mut self, territory: Territory, loser: PlayerIndex) {
        if self.current_ecaz_battle_contribution > 0 && self.player(loser).or_ally_is(Faction::Ecaz) {
            let ecaz = self.get_player(Some(Faction::Ecaz));
            let Some(ecaz) = ecaz else { return };
            self.log_all_forces_lost(ecaz, territory);
            Player::kill_all_forces_territory_boolean(self, ecaz, territory, true);
        }
    }

    pub fn may_keep_cards_after_losing_battle(&self, p: PlayerIndex) -> bool {
        (self.player(p).ally == Faction::Moritani && self.moritani_allows_keeping_cards)
            || (self.player(p).nexus == Faction::Moritani && NexusPlayed::can_use_secret_ally(self, p))
    }

    pub fn dialled_resources_are_refunded(&self, p: PlayerIndex) -> bool {
        self.applicable(Rule::FremenAllyGetsDialedResourcesRefunded) && self.player(p).ally == Faction::Fremen && self.fremen_refunds_battle_dial
    }

    pub fn pay_dialed_spice(&mut self, p: PlayerIndex, plan: &Battle, traitor_was_revealed: bool) {
        let mut paid_by_arrakeen: i32 = 0;
        let cost = plan.cost_game_int32(self, &mut paid_by_arrakeen);
        let cost_to_choam = if self.player(p).ally == Faction::Choam { plan.ally_contribution_amount } else { 0 };

        if paid_by_arrakeen > 0 {
            let arrakeen = self.map.arrakeen();
            self.log(le![arrakeen, " stronghold advantage supports ", Payment::of(paid_by_arrakeen)]);
        }

        if (cost + paid_by_arrakeen) > 0 {
            let cost_for_player = cost - plan.ally_contribution_amount;
            let mut refunded_resources = 0;

            if cost_for_player > 0 {
                let pl = self.player_mut(p);
                pl.resources -= cost_for_player;

                if self.dialled_resources_are_refunded(p) {
                    self.log(le![Payment::of(cost_for_player), " dialled in battle will be refunded in the ", MainPhase::Contemplate, " phase"]);
                    refunded_resources = cost_for_player;
                    let pl = self.player_mut(p);
                    pl.bribes += cost_for_player;
                }
            }

            if plan.ally_contribution_amount > 0 {
                if let Some(allied_player) = self.player(p).allied_player(self) {
                    let ap = self.player_mut(allied_player);
                    ap.resources -= plan.ally_contribution_amount;
                    let f = self.player(p).faction;
                    self.decrease_permitted_use_of_ally_spice(f, plan.ally_contribution_amount);
                }
            }

            let dialled_resources_relevant_for_choam =
                (cost - cost_to_choam - refunded_resources) + paid_by_arrakeen;

            let receiver_profit = self.handle_choam_income(p, dialled_resources_relevant_for_choam, traitor_was_revealed);

            if (cost - receiver_profit) >= 4 {
                self.activate_banker(p);
            }
        }

        if plan.banker_bonus > 0 {
            let pl = self.player_mut(p);
            pl.resources -= plan.banker_bonus;
            let f = self.player(p).faction;
            self.log(le![f, " paid ", Payment::of(plan.banker_bonus), " as ", LeaderSkill::Banker]);
        }
    }

    pub fn handle_choam_income(&mut self, paid_by: PlayerIndex, costs_excluding_payment_by_choam_ally: i32, traitor_was_revealed: bool) -> i32 {
        let mut result = 0;

        let choam = self.get_player(Some(Faction::Choam));
        if let Some(choam) = choam {
            if self.player(paid_by).faction != Faction::Choam && !traitor_was_revealed {
                // (int)Math.Floor(0.5f * int): float multiply, then widened to double.
                result = f64_to_i32(((0.5f32 * costs_excluding_payment_by_choam_ally as f32) as f64).floor());

                if result > 0 {
                    if !self.prevented(FactionAdvantage::ChoamReceiveForcePayment) {
                        let b = self.player_mut(choam);
                        b.resources += result;
                        self.log(le![Faction::Choam, " get ", Payment::of(result), " from supported forces"]);

                        if result >= 5 {
                            self.battle_triggered_bureaucracy = Some(TriggeredBureaucracy {
                                payment_from: self.player(paid_by).faction,
                                payment_to: Faction::Choam,
                                ..Default::default()
                            });
                        }
                    } else {
                        self.log_prevention_by_karma(FactionAdvantage::ChoamReceiveForcePayment);
                    }
                }
            }
        }

        result
    }

    pub fn process_winner_losses(&mut self, territory: Territory, winner: PlayerIndex, plan: &Battle, traitor_was_revealed: bool) {
        self.pay_dialed_spice(winner, plan, traitor_was_revealed);
        self.process_winner_force_losses(territory, winner, plan);
    }

    pub fn process_winner_force_losses(&mut self, territory: Territory, winner: PlayerIndex, plan: &Battle) {
        let force_supplier = Battle::determine_force_supplier(self, winner);
        self.process_ecaz_occupation_winner_losses(territory, winner);

        let special_forces_to_lose = plan.special_forces + plan.special_forces_at_half_strength;
        let forces_to_lose = plan.forces + plan.forces_at_half_strength;

        let mut special_forces_to_save_to_reserves = 0;
        let mut forces_to_save_to_reserves = 0;
        let mut special_forces_to_save_in_territory = 0;
        let mut forces_to_save_in_territory = 0;

        let nr_of_special_force_losses_that_can_be_replaced_by_remaining_normal_forces = std::cmp::min(
            special_forces_to_lose,
            (self.player(force_supplier)
                .forces_in_territory(Some(territory)) - plan.forces) - plan.forces_at_half_strength,
        );

        if !self.may_substitute_force_losses(force_supplier) || nr_of_special_force_losses_that_can_be_replaced_by_remaining_normal_forces == 0 {
            if plan.hero().is_some() && self.skilled_as(plan.hero().nre(), LeaderSkill::Graduate) {
                special_forces_to_save_in_territory = std::cmp::min(special_forces_to_lose, 1);
                forces_to_save_in_territory = std::cmp::max(0, std::cmp::min(forces_to_lose, 1 - special_forces_to_save_in_territory));

                special_forces_to_save_to_reserves = std::cmp::max(
                    0,
                    std::cmp::min(
                        (special_forces_to_lose - special_forces_to_save_in_territory) - forces_to_save_in_territory,
                        2,
                    ),
                );
                forces_to_save_to_reserves = std::cmp::max(
                    0,
                    std::cmp::min(forces_to_lose - forces_to_save_in_territory, 2 - special_forces_to_save_to_reserves),
                );
            } else if self.skilled_as_player_leader_skill(winner, LeaderSkill::Graduate) {
                special_forces_to_save_to_reserves = std::cmp::min(special_forces_to_lose, 1);
                forces_to_save_to_reserves = std::cmp::max(0, std::cmp::min(forces_to_lose, 1 - special_forces_to_save_to_reserves));
            }
        }

        if special_forces_to_save_in_territory + forces_to_save_in_territory + special_forces_to_save_to_reserves + forces_to_save_to_reserves > 0 {
            if special_forces_to_save_to_reserves > 0 {
                Player::forces_to_reserves_territory_int32_boolean(self, force_supplier, territory, special_forces_to_save_to_reserves, true);
            }

            if forces_to_save_to_reserves > 0 {
                Player::forces_to_reserves_territory_int32_boolean(self, force_supplier, territory, forces_to_save_to_reserves, false);
            }

            let force = self.player(force_supplier).force();
            let special_force = self.player(force_supplier).special_force();
            self.log(le![
                LeaderSkill::Graduate,
                " rescues ",
                express_part_if!(forces_to_save_in_territory > 0; forces_to_save_in_territory, force),
                express_part_if!(special_forces_to_save_in_territory > 0; special_forces_to_save_in_territory, special_force),
                express_part_if!(forces_to_save_in_territory > 0 || special_forces_to_save_in_territory > 0; " on site"),
                express_part_if!(forces_to_save_to_reserves > 0 || special_forces_to_save_to_reserves > 0; " and "),
                express_part_if!(forces_to_save_to_reserves > 0; forces_to_save_to_reserves, force),
                express_part_if!(special_forces_to_save_to_reserves > 0; special_forces_to_save_to_reserves, special_force),
                express_part_if!(forces_to_save_to_reserves > 0 || special_forces_to_save_to_reserves > 0; " to reserves")
            ]);
        }

        if !self.may_substitute_force_losses(force_supplier)
            || (special_forces_to_lose - special_forces_to_save_to_reserves - special_forces_to_save_in_territory)
                == 0
            || self.player(force_supplier).forces_in_territory(Some(territory)) <= (plan.forces + plan.forces_at_half_strength)
        {
            let winner_forces_lost = forces_to_lose - forces_to_save_to_reserves - forces_to_save_in_territory;
            let winner_special_forces_lost = special_forces_to_lose - special_forces_to_save_to_reserves - special_forces_to_save_in_territory;
            self.handle_force_losses(territory, force_supplier, winner_forces_lost, winner_special_forces_lost);
        } else {
            self.ixian_special_force_losses_to_take = special_forces_to_lose - special_forces_to_save_to_reserves - special_forces_to_save_in_territory;
        }
    }

    pub fn may_substitute_force_losses(&self, p: PlayerIndex) -> bool {
        self.player(p).faction == Faction::Ixian && !self.prevented(FactionAdvantage::IxianReplacingSpecialForces)
    }

    pub fn handle_force_losses(&mut self, territory: Territory, player: PlayerIndex, forces_lost: i32, special_forces_lost: i32) {
        let had_messiah_before_losses = self.player(player).messiah_available(self);

        Player::kill_forces_territory_int32_boolean_boolean(self, player, territory, forces_lost, false, true);
        Player::kill_forces_territory_int32_boolean_boolean(self, player, territory, special_forces_lost, true, true);

        self.log_losses(player, forces_lost, special_forces_lost);

        if self.player(player).messiah_available(self) && !had_messiah_before_losses {
            self.stone(Milestone::Messiah);
        }
    }

    pub fn kill_leader_in_battle(&mut self, killed_hero: Hero, cause_of_death: TreacheryCardType, winner: PlayerIndex, hero_value: i32) {
        let winner_faction = self.player(winner).faction;
        let mut fact = BattleFact::new(BattleFactKind::Killed, winner_faction);
        fact.hero = Some(killed_hero);
        fact.cause = Some(ExpressionElement::from(cause_of_death));
        fact.value = hero_value;
        let message = express!(cause_of_death, " kills ", killed_hero, " → ", winner_faction, " get ", Payment::of(hero_value)).with_fact(fact);
        self.log(le![message]);
        if killed_hero.as_leader().is_some() {
            self.kill_hero(killed_hero);
        }
        let pl = self.player_mut(winner);
        pl.resources += hero_value;
    }

    pub fn log_all_forces_lost(&mut self, player: PlayerIndex, territory: Territory) {
        let forces = self.player(player).forces_in_territory(Some(territory));
        let special_forces = self.player(player).special_forces_in_territory(Some(territory));
        let faction = self.player(player).faction;
        let force = self.player(player).force();
        let special_force = self.player(player).special_force();
        let mut fact = BattleFact::new(BattleFactKind::ForcesLost, faction);
        fact.forces = forces;
        fact.special_forces = special_forces;
        fact.all = true;
        let message = express!(
            faction,
            " lose all ",
            express_part_if!(forces > 0; forces, force),
            express_part_if!(special_forces > 0; special_forces, special_force)
        )
        .with_fact(fact);
        self.log(le![message]);
    }

    pub fn log_strength(&mut self, faction: Faction, strength: f32) {
        let mut fact = BattleFact::new(BattleFactKind::Strength, faction);
        fact.strength = strength;
        let message = express!(faction, " strength: ", strength).with_fact(fact);
        self.log(le![message]);
    }

    pub fn log_losses(&mut self, player: PlayerIndex, forces_lost: i32, special_forces_lost: i32) {
        if forces_lost > 0 || special_forces_lost > 0 {
            let faction = self.player(player).faction;
            let force = self.player(player).force();
            let special_force = self.player(player).special_force();
            let mut fact = BattleFact::new(BattleFactKind::ForcesLost, faction);
            fact.forces = forces_lost;
            fact.special_forces = special_forces_lost;
            let message = express!(
                faction,
                " lose ",
                express_part_if!(forces_lost > 0; forces_lost, force),
                express_part_if!(special_forces_lost > 0; special_forces_lost, special_force)
            )
            .with_fact(fact);
            self.log(le![message]);
        }
    }

    pub fn set_hero_locations(&mut self, b: &Battle, territory: Territory) {
        if let Some(hero) = b.hero().filter(|h| h.as_leader().is_some()) {
            self.leader_state[&hero].current_territory = Some(territory);
        }

        if b.messiah {
            self.leader_state[&LeaderManager::messiah()].current_territory = Some(territory);
        }
    }

    pub fn traitor_called(&mut self, b: &BattleInitiated, agg: &Rc<Battle>, def: &Rc<Battle>, defender_treachery: &TreacheryCalled, agg_leader: Option<Hero>, def_leader: Option<Hero>) -> GameResult {
        let agg_leader = agg_leader.and_then(crate::compat::cs::CsNull::opt);
        let def_leader = def_leader.and_then(crate::compat::cs::CsNull::opt);
        let aggressor_traitor_action = self.aggressor_traitor_action.clone();
        if aggressor_traitor_action.as_ref().is_some_and(|a| a.succeeded(self)) && defender_treachery.succeeded(self) {
            let (Some(agg_leader), Some(def_leader)) = (agg_leader, def_leader) else {
                game_error!("A required value is missing");
            };

            let territory = b.territory(self).required("territory")?;
            self.two_traitors_called(agg, def, agg.player(), def.player(), territory, agg_leader, def_leader)?;
        } else if let Some(aggressor_traitor_action) = aggressor_traitor_action {
            let succeeded = aggressor_traitor_action.succeeded(self);
            let winner = if succeeded { agg.player() } else { def.player() };
            let loser = if succeeded { def.player() } else { agg.player() };
            let loser_gambit = if aggressor_traitor_action.succeeded(self) { def } else { agg };
            let winner_gambit = if aggressor_traitor_action.succeeded(self) { agg } else { def };
            if self.player(winner).is(Faction::Harkonnen) {
                self.harkonnen_do_not_have_to_return_used_captured_leader = true;
            }
            let territory = b.territory(self).required("territory")?;
            self.one_traitor_called(territory, winner, loser, loser_gambit, winner_gambit)?;
        }
        Ok(())
    }

    pub fn one_traitor_called(&mut self, territory: Territory, winner: PlayerIndex, loser: PlayerIndex, loser_plan: &Battle, winner_plan: &Battle) -> GameResult {
        let Some(traitor) = loser_plan.hero() else {
            game_error!("Traitor cannot be null in OneTraitorCalled");
        };

        let had_messiah_before_losses = self.player(loser).messiah_available(self);
        let traitor_value = traitor.value_in_combat_against(winner_plan.hero());
        let traitor_owner = if self.player(winner).traitors.iter().any(|t| t.is_traitor(traitor)) {
            self.player(winner).faction
        } else {
            Faction::Harkonnen
        };

        // The traitor decides the battle, but the strengths were worked out
        // already, and the log shows them.
        if let (Some(current_battle), Some(outcome)) = (self.current_battle.clone(), self.battle_outcome.clone()) {
            let aggressor = current_battle.aggressor(self);
            self.log_strength(aggressor, outcome.agg_total);
            let defender = current_battle.defender(self);
            self.log_strength(defender, outcome.def_total);
        }

        let loser_faction = self.player(loser).faction;
        let mut fact = BattleFact::new(BattleFactKind::Traitor, traitor_owner);
        fact.hero = Some(traitor);
        let message = express!(traitor, " is a ", traitor_owner, " traitor! ", loser_faction, " lose everything").with_fact(fact);
        self.log(le![message]);

        if traitor.as_leader().is_some() {
            let winner_faction = self.player(winner).faction;
            let mut fact = BattleFact::new(BattleFactKind::Killed, winner_faction);
            fact.hero = Some(traitor);
            fact.cause = Some(ExpressionElement::from("Treachery"));
            fact.value = traitor_value;
            let message = express!("Treachery kills ", traitor, " → ", winner_faction, " get ", Payment::of(traitor_value)).with_fact(fact);
            self.log(le![message]);
            self.kill_hero(traitor);
            let pl = self.player_mut(winner);
            pl.resources += traitor_value;
        }

        self.battle_winner = Some(self.player(winner).faction);
        self.battle_loser = Some(self.player(loser).faction);

        let force_supplier_of_loser = Battle::determine_force_supplier(self, loser);

        self.process_ecaz_occupation_loser_losses(territory, loser);

        if force_supplier_of_loser != loser {
            self.log_all_forces_lost(force_supplier_of_loser, territory);
            Player::kill_all_forces_territory_boolean(self, force_supplier_of_loser, territory, true);
        }

        self.log_all_forces_lost(loser, territory);
        Player::kill_all_forces_territory_boolean(self, loser, territory, true);
        let may_keep = self.may_keep_cards_after_losing_battle(loser);
        self.lose_cards(loser_plan, may_keep)?;
        self.pay_dialed_spice(loser, loser_plan, true);

        if self.player(loser).messiah_available(self) && !had_messiah_before_losses {
            self.stone(Milestone::Messiah);
        }
        Ok(())
    }

    /// C# parity: never sets BattleWinner or BattleLoser (hazards 18.2).
    pub fn two_traitors_called(&mut self, agg: &Battle, def: &Battle, aggressor: PlayerIndex, defender: PlayerIndex, territory: Territory, agg_leader: Hero, def_leader: Hero) -> GameResult {
        let had_messiah_before_losses = self.player(aggressor).messiah_available(self) || self.player(defender).messiah_available(self);

        self.log(le!["Treachery kills both ", def_leader, " and ", agg_leader]);
        self.kill_hero(def_leader);
        self.kill_hero(agg_leader);

        let force_supplier_of_defender = Battle::determine_force_supplier(self, defender);
        self.process_ecaz_occupation_loser_losses(territory, defender);
        if force_supplier_of_defender != defender {
            self.log_all_forces_lost(force_supplier_of_defender, territory);
            Player::kill_all_forces_territory_boolean(self, force_supplier_of_defender, territory, true);
        }

        self.log_all_forces_lost(defender, territory);
        Player::kill_all_forces_territory_boolean(self, defender, territory, true);

        let force_supplier_of_aggressor = Battle::determine_force_supplier(self, aggressor);
        self.process_ecaz_occupation_loser_losses(territory, aggressor);
        if force_supplier_of_aggressor != aggressor {
            self.log_all_forces_lost(force_supplier_of_aggressor, territory);
            Player::kill_all_forces_territory_boolean(self, force_supplier_of_aggressor, territory, true);
        }

        self.log_all_forces_lost(aggressor, territory);
        Player::kill_all_forces_territory_boolean(self, aggressor, territory, true);

        self.lose_cards(def, false)?;
        self.pay_dialed_spice(defender, def, true);

        self.lose_cards(agg, false)?;
        self.pay_dialed_spice(aggressor, agg, true);

        if (self.player(aggressor).messiah_available(self) || self.player(defender).messiah_available(self)) && !had_messiah_before_losses {
            self.stone(Milestone::Messiah);
        }
        Ok(())
    }

    pub fn lasgun_shield_explosion(&mut self, agg: &Battle, def: &Battle, aggressor: PlayerIndex, defender: PlayerIndex, territory: Territory, agg_leader: Option<Hero>, def_leader: Option<Hero>) -> GameResult {
        let agg_leader = agg_leader.and_then(crate::compat::cs::CsNull::opt);
        let def_leader = def_leader.and_then(crate::compat::cs::CsNull::opt);
        let had_messiah_before_losses = self.player(aggressor).messiah_available(self) || self.player(defender).messiah_available(self);

        self.log(le!["A ", TreacheryCardType::Laser, "/", TreacheryCardType::Shield, " explosion occurs!"]);
        self.stone(Milestone::Explosion);

        if let Some(agg_leader) = agg_leader {
            self.log(le!["The explosion kills ", agg_leader]);
            self.kill_hero(agg_leader);
        }

        if let Some(def_leader) = def_leader {
            self.log(le!["The explosion kills ", def_leader]);
            self.kill_hero(def_leader);
        }

        if agg.messiah || def.messiah {
            self.log(le!["The explosion kills the ", Concept::Messiah]);
            self.kill_hero(LeaderManager::messiah());
        }

        self.lose_cards(agg, false)?;
        self.pay_dialed_spice(aggressor, agg, false);

        self.lose_cards(def, false)?;
        self.pay_dialed_spice(defender, def, false);

        let removed = self.remove_resources_territory(territory);
        if removed > 0 {
            self.log(le!["The explosion destroys ", Payment::of(removed), " in ", territory]);
        }

        self.kill_all_forces_in(territory, true);
        self.kill_ambassador_in(territory);

        if (self.player(aggressor).messiah_available(self) || self.player(defender).messiah_available(self)) && !had_messiah_before_losses {
            self.stone(Milestone::Messiah);
        }
        Ok(())
    }

    pub fn kill_all_forces_in(&mut self, territory: Territory, in_battle: bool) {
        let mut i = 0;
        while i < self.players.len() {
            let p = self.player_at(i);
            // `territory` may be null: AnyForcesIn(null) is 0.
            if self.player(p).any_forces_in_territory(territory.opt()) > 0 {
                self.reveal_current_no_field_player_territory(p, territory);

                let homeworld_kill_limit = if in_battle {
                    self.player(p).get_homeworld_battle_contribution_and_lasgun_shield_limit(self, territory)
                } else {
                    0
                };
                if homeworld_kill_limit == 0 {
                    let f = self.player(p).faction;
                    self.log(le!["All ", f, " forces in ", territory, " were killed"]);
                    Player::kill_all_forces_territory_boolean(self, p, territory, in_battle);
                } else {
                    let normal_forces_to_kill = std::cmp::min(self.player(p).forces_in_territory(Some(territory)), homeworld_kill_limit);
                    let special_forces_to_kill = std::cmp::min(
                        self.player(p).special_forces_in_territory(Some(territory)),
                        homeworld_kill_limit - normal_forces_to_kill,
                    );

                    if normal_forces_to_kill > 0 {
                        Player::kill_forces_territory_int32_boolean_boolean(self, p, territory, normal_forces_to_kill, false, in_battle);
                    }
                    if special_forces_to_kill > 0 {
                        Player::kill_forces_territory_int32_boolean_boolean(self, p, territory, special_forces_to_kill, true, in_battle);
                    }

                    let force = self.player(p).force();
                    let special_force = self.player(p).special_force();
                    self.log(le![
                        express_part_if!(normal_forces_to_kill > 0; normal_forces_to_kill, force),
                        express_part_if!(normal_forces_to_kill > 0 && special_forces_to_kill > 0; " and "),
                        express_part_if!(special_forces_to_kill > 0; special_forces_to_kill, special_force),
                        " in ",
                        territory,
                        " were killed"
                    ]);
                }
            }
            i += 1;
        }
    }

    pub fn select_victim_of_harkonnen_winner(&mut self, victim_action: &Battle) {
        let victim = self.get_player(Some(victim_action.initiator()));
        let Some(victim) = victim else { return };

        // Get all living leaders from the opponent that haven't fought in another territory this turn
        let leaders: Vec<Leader> = self
            .player(victim)
            .leaders
            .iter()
            .copied()
            .filter(|l| {
                l.hero_type() != HeroType::Auditor && self.leader_state[&Hero::Leader(*l)].alive() && self.can_join_current_battle(Hero::Leader(*l))
            })
            .collect();
        let mut available_leaders: Deck<Leader> = Deck::new(leaders, self.random.clone());

        if !available_leaders.is_empty() {
            available_leaders.shuffle();
            self.harkonnen_victim = Some(available_leaders.draw());
        } else {
            self.harkonnen_victim = None;
            let f = self.player(victim).faction;
            self.log(le![f, " don't have any leaders for ", Faction::Harkonnen, " to capture or kill"]);
        }
    }

    pub fn finish_battle(&mut self) -> GameResult {
        let Some(aggressor_plan) = self.aggressor_plan.clone() else {
            game_error!("Aggressor plan is null");
        };
        let Some(defender_plan) = self.defender_plan.clone() else {
            game_error!("Defender plan is null");
        };
        let Some(aggressor_traitor_action) = self.aggressor_traitor_action.clone() else {
            game_error!("Aggressor traitor action is null");
        };
        let Some(defender_traitor_action) = self.defender_traitor_action.clone() else {
            game_error!("Defender traitor action is null");
        };
        let Some(current_battle) = self.current_battle.clone() else {
            game_error!("Current battle is null");
        };

        // `IHero? == Leader?`: true when both are null.
        let vidal = self.vidal().map(Hero::Leader);
        if aggressor_plan.hero() == vidal
            && self.when_to_set_aside_vidal == VidalMoment::AfterUsedInBattle
            && !(aggressor_traitor_action.succeeded(self) && !defender_traitor_action.succeeded(self))
        {
            self.set_aside_vidal();
        }

        let vidal = self.vidal().map(Hero::Leader);
        if defender_plan.hero() == vidal
            && self.when_to_set_aside_vidal == VidalMoment::AfterUsedInBattle
            && !(defender_traitor_action.succeeded(self) && !aggressor_traitor_action.succeeded(self))
        {
            self.set_aside_vidal();
        }

        self.return_skilled_leaders_in_front_of_shield_after_battle();
        let territory = current_battle.territory(self).nre();
        self.determine_occupation(territory);
        if !self.applicable(Rule::FullPhaseKarma) {
            self.allow_prevented_battle_faction_advantages();
        }
        if self.current_juice.as_ref().is_some_and(|j| j.type_ == JuiceType::Aggressor) {
            self.current_juice = None;
        }
        self.current_diplomacy = None;
        self.current_rock_was_melted = None;
        self.current_portable_antidote_used = None;
        self.harkonnen_do_not_have_to_return_used_captured_leader = false;
        self.finish_decipherer_if_applicable();
        if self.next_player_to_battle().is_none() {
            self.main_phase_end();
        }
        self.enter(Phase::BattleReport, true);
        Ok(())
    }

    pub fn determine_occupation(&mut self, territory: Territory) {
        for location in territory.locations().iter().copied() {
            self.determine_occupation_location(Some(location));
        }
    }

    /// C# parity: a different occupation rule from `DetermineOccupationAtStartOrEndOfTurn` (hazards 18.2).
    pub fn determine_occupation_location(&mut self, location: Option<Location>) {
        let location = location.and_then(crate::compat::cs::CsNull::opt);
        let Some(hw) = location.and_then(|l| l.as_homeworld()) else {
            return;
        };

        let previous_occupier = self.occupier_of(hw.world());
        let battalions = self.battalions_in(hw.into());
        let sole_player_on_planet = if battalions.len() == 1 {
            let battalions = self.battalions_in(hw.into());
            let first = battalions.into_iter().next().expect("a battalion");
            self.get_player(Some(first.faction))
        } else {
            None
        };

        if let Some(sole_player_on_planet) = sole_player_on_planet {
            self.homeworld_occupation.remove(&hw);

            if !self.player(sole_player_on_planet).is_native_homeworld(hw) {
                let f = self.player(sole_player_on_planet).faction;
                self.homeworld_occupation.insert_new(hw, f);
                self.log(le![f, " now occupy ", hw]);
            } else if let Some(previous_occupier) = previous_occupier {
                let f = self.player(previous_occupier).faction;
                self.log(le![f, " no longer occupy ", hw]);
            }

            self.check_if_shipment_permissions_should_be_revoked();

            if hw.world() == World::Ecaz {
                self.check_if_occupier_takes_vidal(previous_occupier);
            }
        }
    }

    pub fn return_skilled_leaders_in_front_of_shield_after_battle(&mut self) {
        let Some(current_battle) = self.current_battle.clone() else {
            return;
        };

        // The C# loop filters LeaderState lazily. The body changes only values, never
        // the keys, so walking a snapshot of the keys and testing each one when it is
        // reached is the same.
        let keys: Vec<Hero> = self.leader_state.keys().copied().collect();
        for key in keys {
            let Some(leader) = key.as_leader() else { continue };
            if !(self.is_skilled(Some(key)) && !self.leader_state[&key].in_front_of_shield) {
                continue;
            }

            let current_owner = self.player_indices().find(|p| self.player(*p).leaders.contains(&leader));

            let take = match current_owner {
                None => true,
                Some(current_owner) => {
                    !self.captured_leaders.contains_key(&leader)
                        && !(self.player(current_owner).faction != Faction::Ecaz && leader.hero_type() == HeroType::Vidal)
                        && current_battle.is_aggressor_or_defender(self, current_owner)
                }
            };
            if take {
                self.set_in_front_of_shield(leader, true);

                if self.is_alive(key) {
                    let skill = self.skill(Some(key));
                    self.log(le![skill, " ", leader, " is placed back in front of shield"]);
                }
            }
        }
    }

    pub fn allow_prevented_battle_faction_advantages(&mut self) {
        self.allow(FactionAdvantage::AtreidesUseMessiah);
        self.allow(FactionAdvantage::AtreidesBattlePlanPrescience);
        self.allow(FactionAdvantage::BeneGesseritUsingVoice);
        self.allow(FactionAdvantage::FremenSpecialForceBonus);
        self.allow(FactionAdvantage::FremenNotPayingForBattles);
        self.allow(FactionAdvantage::EmperorSpecialForceBonus);
        self.allow(FactionAdvantage::IxianSpecialForceBonus);
        self.allow(FactionAdvantage::IxianReplacingSpecialForces);
        self.allow(FactionAdvantage::HarkonnenCallTraitorForAlly);
        self.allow(FactionAdvantage::HarkonnenCaptureLeader);
        self.allow(FactionAdvantage::ChoamReceiveForcePayment);
    }

    pub fn lose_cards(&mut self, plan: &Battle, may_choose_to_keep_one: bool) -> GameResult {
        let p = plan.player();
        if !(self.player(p).ally == Faction::Moritani && self.moritani_allows_keeping_cards)
            && self.player(p).nexus == Faction::Moritani
            && NexusPlayed::can_use_secret_ally(self, p)
        {
            self.secret_ally_allows_keeping_cards_after_losing_battle = true;
        }

        if may_choose_to_keep_one {
            if let Some(weapon) = plan.weapon() {
                if self.player(p).has(Some(weapon)) {
                    self.cards_to_be_discarded_by_loser_after_battle.push(weapon);
                }
            }
            if let Some(defense) = plan.defense() {
                if self.player(p).has(Some(defense)) {
                    self.cards_to_be_discarded_by_loser_after_battle.push(defense);
                }
            }
        } else {
            if plan.weapon().is_some() {
                self.discard_treachery_card_boolean(plan.weapon(), true)?;
            }
            if plan.defense().is_some() {
                self.discard_treachery_card_boolean(plan.defense(), true)?;
            }
        }
        Ok(())
    }

    /// Mutates `BattleSequence` when read (hazards 4.3).
    pub fn next_player_to_battle(&mut self) -> Option<PlayerIndex> {
        if self.battle_sequence.is_none() {
            return None;
        }

        let mut i = 0;
        while i < self.players.len() {
            let player_to_check = self.battle_sequence.as_ref().nre().current_player(self);
            // BattlesToBeFought dereferences the player before anything else that has effects.
            if !Battle::battles_to_be_fought(self, player_to_check.nre(), false).is_empty() {
                return player_to_check;
            }

            let mut seq = self.battle_sequence.take().nre();
            seq.next_player(self);
            self.battle_sequence = Some(seq);
            i += 1;
        }

        None
    }

    pub fn auditee(&self) -> Option<PlayerIndex> {
        if self.applicable(Rule::ChoamAuditor) && !self.prevented(FactionAdvantage::ChoamAudit) {
            if self.aggressor_plan.as_ref().is_some_and(|p| p.hero().is_some_and(|h| h.hero_type() == HeroType::Auditor)) {
                if let Some(defender_plan) = &self.defender_plan {
                    return Some(defender_plan.player());
                }
            }

            if self.defender_plan.as_ref().is_some_and(|p| p.hero().is_some_and(|h| h.hero_type() == HeroType::Auditor)) {
                if let Some(aggressor_plan) = &self.aggressor_plan {
                    return Some(aggressor_plan.player());
                }
            }
        }

        None
    }

    pub fn winner_hero(&self) -> Option<Hero> {
        // `Faction? != Faction.None`: true when BattleWinner is null (hazards 14).
        if self.battle_winner != Some(Faction::None) {
            let winner_gambit = if self.battle_winner == self.aggressor_plan.as_ref().map(|p| p.initiator()) {
                self.aggressor_plan.as_ref()
            } else {
                self.defender_plan.as_ref()
            };
            return winner_gambit.and_then(|p| p.hero());
        }

        None
    }

    pub fn winner_battle_action(&self) -> Option<Rc<Battle>> {
        if let Some(aggressor_plan) = &self.aggressor_plan {
            if Some(aggressor_plan.initiator()) == self.battle_winner {
                return Some(aggressor_plan.clone());
            }
        }
        if let Some(defender_plan) = &self.defender_plan {
            if Some(defender_plan.initiator()) == self.battle_winner {
                return Some(defender_plan.clone());
            }
        }

        None
    }

    pub fn can_join_current_battle(&self, hero: Hero) -> bool {
        let current_territory = self.leader_state[&hero].current_territory;
        current_territory.is_none() || current_territory == self.current_battle.as_ref().and_then(|b| b.territory(self))
    }
}

#[derive(Clone, Debug, Canon)]
pub struct Fight {
    pub territory: Territory,
    pub faction: Faction,
}

impl Fight {
    pub fn id(&self) -> i32 {
        self.territory.id() + (1000i32 * self.faction.0)
    }

    pub fn new(territory: Territory, faction: Faction) -> Self {
        Fight { territory, faction }
    }

}

#[derive(Clone, Debug, Canon)]
pub struct LossToTake {
    pub faction: Faction,
    /// `Option` because `new LossToTake()` leaves it null (`TakeLosses.LossesToTake` on an empty list).
    pub location: Option<Location>,
    pub amount: i32,
}

impl LossToTake {
    pub fn new() -> Self {
        LossToTake { faction: Faction::None, location: None, amount: 0 }
    }

}
