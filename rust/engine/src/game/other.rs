use crate::prelude::*;

impl Game {
    pub fn reveal_current_no_field(&mut self, player: PlayerIndex, in_location: Option<Location>) {
        let in_location = in_location.and_then(crate::compat::cs::CsNull::opt);
        if self.player(player).faction != Faction::Richese {
            return;
        }

        // FirstOrDefault(...).Key: the default KeyValuePair has a null key.
        let no_field_location = self
            .player(player)
            .forces_in_locations
            .iter()
            .find(|(_, v)| v.amount_of_special_forces > 0)
            .map(|(k, _)| *k);

        let Some(no_field_location) = no_field_location else { return };

        if let Some(in_location) = in_location {
            if in_location != no_field_location {
                return;
            }
        }

        self.latest_revealed_no_field_value = self.current_no_field_value;
        Player::special_forces_to_reserves(self, player, no_field_location, 1);
        let nr_of_forces = self.player(player).forces_in_reserve().min(self.current_no_field_value);
        Player::ship_forces(self, player, no_field_location, nr_of_forces);
        let faction = self.player(player).faction;
        self.log(vec![
            faction.into(),
            " reveal ".into(),
            nr_of_forces.into(),
            FactionForce::Richese.into(),
            " under ".into(),
            FactionSpecialForce::Richese.into(),
            " in ".into(),
            no_field_location.into(),
        ]);

        if self.current_no_field_value == 0 {
            self.flip_bene_gesserit_advisors_when_alone();
        }

        self.current_no_field_value = -1;
    }

    pub fn reveal_current_no_field_player_territory(&mut self, player: PlayerIndex, in_territory: Territory) {
        if !self.player(player).is(Faction::Richese) {
            return;
        }
        for l in in_territory.locations().iter().copied() {
            self.reveal_current_no_field(player, Some(l));
        }
    }

    pub fn exchange_resources_in_bribe(&mut self, from: PlayerIndex, target: PlayerIndex, amount: i32) {
        {
            let p = self.player_mut(from);
            p.resources = p.resources.wrapping_sub(amount);
        }

        if self.bribes_during_mentat() {
            if self.player(from).faction == Faction::Emperor && self.player(target).faction == self.player(from).ally {
                let t = self.player_mut(target);
                t.resources = t.resources.wrapping_add(amount);
            } else {
                let t = self.player_mut(target);
                t.bribes = t.bribes.wrapping_add(amount);
            }
        } else {
            let t = self.player_mut(target);
            t.resources = t.resources.wrapping_add(amount);
        }
    }

    pub fn revoke_plan_if_needed(&mut self, f: Faction) {
        let plan = match self.current_battle.clone() {
            Some(b) => b.plan_of_faction(self, f),
            None => None,
        };
        if let Some(plan) = plan {
            self.revoke_plan(&plan);
        }
    }

    pub fn revoke_plan(&mut self, plan: &Rc<Battle>) {
        // Event == is reference equality.
        if self.aggressor_plan.as_ref().is_some_and(|a| Rc::ptr_eq(a, plan)) {
            self.aggressor_plan = None;
        } else if self.defender_plan.as_ref().is_some_and(|d| Rc::ptr_eq(d, plan)) {
            self.defender_plan = None;
        }
    }

    pub fn prevent(&mut self, initiator: Faction, advantage: FactionAdvantage, log: bool) {
        if log {
            self.log(vec![initiator.into(), " prevent ".into(), advantage.into()]);
        }

        if !self.prevented_advantages.contains(&advantage) {
            self.prevented_advantages.push(advantage);
        }
    }

    pub fn allow(&mut self, advantage: FactionAdvantage) {
        if self.prevented_advantages.contains(&advantage) {
            if let Some(i) = self.prevented_advantages.iter().position(|a| *a == advantage) {
                self.prevented_advantages.remove(i);
            }
            self.log(vec![express!(advantage, " is no longer prevented").with_log(AsideLog::after()).into()]);
        }
    }

    pub fn apply_bureaucracy(&mut self, payer: Faction, receiver: Faction) {
        if !self.bureaucrat_was_used_this_phase {
            let bureaucrat = self.player_skilled_as(LeaderSkill::Bureaucrat);
            if let Some(bureaucrat) = bureaucrat {
                if self.player(bureaucrat).faction != payer && self.player(bureaucrat).faction != receiver {
                    self.phase_before_bureaucrat_was_activated = self.current_phase;
                    self.target_of_bureaucracy = receiver;
                    self.enter(Phase::Bureaucracy, true);
                }
            }
        }
    }

    pub fn activate_banker(&mut self, player_who_paid: PlayerIndex) {
        if !self.banker_was_used_this_phase {
            let banker = self.player_skilled_as(LeaderSkill::Banker);
            if let Some(banker) = banker {
                if banker != player_who_paid {
                    self.banker_was_used_this_phase = true;
                    let faction = self.player(banker).faction;
                    self.log(vec![
                        faction.into(),
                        " will receive ".into(),
                        Payment::of(1).into(),
                        " from ".into(),
                        LeaderSkill::Banker.into(),
                        " at ".into(),
                        MainPhase::Collection.into(),
                    ]);
                    let b = self.player_mut(banker);
                    b.banked_resources = b.banked_resources.wrapping_add(1);
                }
            }
        }
    }

    pub fn play_nexus_card(&mut self, initiator: PlayerIndex, message_elements: Vec<ExpressionElement>) {
        self.stone(Milestone::NexusPlayed);

        let type_of_nexus = if NexusPlayed::can_use_cunning(self, initiator) {
            "Cunning"
        } else if NexusPlayed::can_use_betrayal(self, initiator) {
            "Betrayal"
        } else {
            "Secret Ally"
        };

        let faction = self.player(initiator).faction;
        let nexus = self.player(initiator).nexus;
        if message_elements.len() > 0 {
            self.log(vec![
                faction.into(),
                " play ".into(),
                nexus.into(),
                format!(" Nexus {} to ", type_of_nexus).into(),
                MessagePart::express(message_elements).into(),
            ]);
        } else {
            self.log(vec![faction.into(), " play ".into(), nexus.into(), format!(" Nexus {}", type_of_nexus).into()]);
        }

        self.discard_nexus_card(initiator);
    }

    pub fn log_prevention_by_karma(&mut self, prevented: FactionAdvantage) {
        // A Karama card rather than the card type, so it shows as a card pill.
        let karama = TreacheryCardManager::get_cards_in_and_outside_play()
            .iter()
            .copied()
            .find(|c| c.type_() == TreacheryCardType::Karma);
        let karama_or_type: ExpressionElement = match karama {
            Some(c) => c.into(),
            None => TreacheryCardType::Karma.into(),
        };
        self.log(vec![express!(karama_or_type, " prevents ", prevented).with_log(AsideLog::aside()).into()]);
    }

    pub fn log_prevention_by_low_threshold(&mut self, prevented: FactionAdvantage) {
        self.log(vec!["Low Threshold prevents ".into(), prevented.into()]);
    }

    pub fn prevented(&self, advantage: FactionAdvantage) -> bool {
        self.prevented_advantages.contains(&advantage)
    }

    pub fn bribes_during_mentat(&self) -> bool {
        !self.applicable(Rule::BribesAreImmediate)
    }

    pub fn karma_prevented(&self, f: Faction) -> bool {
        self.current_karma_prevention.as_ref().is_some_and(|p| p.target == f)
    }

    pub fn juice_forces_first_player(&self) -> bool {
        self.current_juice.as_ref().is_some_and(|j| j.type_ == JuiceType::GoFirst)
    }

    pub fn juice_forces_last_player(&self) -> bool {
        self.current_juice.as_ref().is_some_and(|j| j.type_ == JuiceType::GoLast)
    }

    pub fn yields_secrets(&self, p: PlayerIndex) -> bool {
        !self.secrets_remain_hidden.contains(&self.player(p).faction)
    }

    pub fn yields_secrets_faction(&self, f: Faction) -> bool {
        !self.secrets_remain_hidden.contains(&f)
    }
}

#[derive(Clone, Debug, Canon)]
pub struct TriggeredBureaucracy {
    pub payment_from: Faction,
    pub payment_to: Faction,
}

impl Default for TriggeredBureaucracy {
    fn default() -> Self {
        TriggeredBureaucracy {
            payment_from: Default::default(),
            payment_to: Default::default(),
        }
    }
}

impl TriggeredBureaucracy {
    pub fn new() -> Self {
        Self::default()
    }

}

#[derive(Clone, Debug, Canon)]
pub struct ClairVoyanceQandA {
    /// `Option` because the answer is recorded with `Game.LatestClairvoyance`, which can be
    /// null; nothing throws in that case.
    pub question: Option<Rc<ClairVoyancePlayed>>,
    pub answer: Rc<ClairVoyanceAnswered>,
}

impl ClairVoyanceQandA {
    pub fn new(question: Option<Rc<ClairVoyancePlayed>>, answer: &Rc<ClairVoyanceAnswered>) -> Self {
        ClairVoyanceQandA { question, answer: answer.clone() }
    }
}
