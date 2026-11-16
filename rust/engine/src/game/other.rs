use crate::prelude::*;
use crate::messages::part::Part;

impl Game {
    pub fn reveal_current_no_field(&mut self, player: PlayerId, in_location: Option<Location>) -> GameResult {
        if self.player(player).faction != Some(Faction::Richese) {
            return Ok(());
        }

        // FirstOrDefault(...).Key: the default KeyValuePair has a null key.
        let no_field_location = self
            .player(player)
            .forces_in_locations
            .iter()
            .find(|(_, v)| v.amount_of_special_forces > 0)
            .map(|(k, _)| *k);

        let Some(no_field_location) = no_field_location else { return Ok(()) };

        if let Some(in_location) = in_location {
            if in_location != no_field_location {
                return Ok(());
            }
        }

        self.ship_move.latest_revealed_no_field_value = self.ship_move.current_no_field_value;
        Player::special_forces_to_reserves(self, player, no_field_location, 1)?;
        let nr_of_forces = self.player(player).forces_in_reserve().min(self.ship_move.current_no_field_value);
        Player::ship_forces(self, player, no_field_location, nr_of_forces)?;
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

        if self.ship_move.current_no_field_value == 0 {
            self.flip_bene_gesserit_advisors_when_alone()?;
        }

        self.ship_move.current_no_field_value = -1;
        Ok(())
    }

    pub fn reveal_current_no_field_in_territory(&mut self, player: PlayerId, in_territory: Territory) -> GameResult {
        if !self.player(player).is(Faction::Richese) {
            return Ok(());
        }
        for l in in_territory.locations().iter().copied() {
            self.reveal_current_no_field(player, Some(l))?;
        }
        Ok(())
    }

    pub fn exchange_resources_in_bribe(&mut self, from: PlayerId, target: PlayerId, amount: i32) -> GameResult {
        {
            let p = self.player_mut(from);
            crate::game::amounts::spend(&mut p.resources, amount, "spice")?;
        }

        if self.bribes_during_mentat() {
            if self.player(from).faction == Some(Faction::Emperor) && self.player(target).faction == self.player(from).ally {
                let t = self.player_mut(target);
                crate::game::amounts::gain(&mut t.resources, amount, "spice")?;
            } else {
                let t = self.player_mut(target);
                crate::game::amounts::gain(&mut t.bribes, amount, "spice")?;
            }
        } else {
            let t = self.player_mut(target);
            crate::game::amounts::gain(&mut t.resources, amount, "spice")?;
        }
        Ok(())
    }

    pub fn revoke_plan_if_needed(&mut self, f: Faction) {
        let plan = match self.current_battle().cloned() {
            Some(b) => b.plan_of_faction(self, f),
            None => None,
        };
        if let Some(plan) = plan {
            self.revoke_plan(&plan);
        }
    }

    pub fn revoke_plan(&mut self, plan: &BattlePlan) {
        if self.aggressor_plan() == Some(plan) {
            self.fight_mut().aggressor_plan = None;
        } else if self.defender_plan() == Some(plan) {
            self.fight_mut().defender_plan = None;
        }
    }

    pub fn prevent(&mut self, initiator: Faction, advantage: FactionAdvantage, log: bool) {
        if log {
            self.log(vec![initiator.into(), " prevent ".into(), advantage.into()]);
        }

        if !self.turn.prevented_advantages.contains(&advantage) {
            self.turn.prevented_advantages.push(advantage);
        }
    }

    pub fn allow(&mut self, advantage: FactionAdvantage) {
        if self.turn.prevented_advantages.contains(&advantage) {
            if let Some(i) = self.turn.prevented_advantages.iter().position(|a| *a == advantage) {
                self.turn.prevented_advantages.remove(i);
            }
            self.log(vec![express!(advantage, " is no longer prevented").with_log(AsideLog::after()).into()]);
        }
    }

    pub fn apply_bureaucracy(&mut self, payer: Option<Faction>, receiver: Option<Faction>) {
        if !self.turn.bureaucrat_was_used_this_phase {
            let bureaucrat = self.player_skilled_as(LeaderSkill::Bureaucrat);
            if let Some(bureaucrat) = bureaucrat {
                if self.player(bureaucrat).faction != payer && self.player(bureaucrat).faction != receiver {
                    self.turn.target_of_bureaucracy = receiver;
                    self.interrupt(Interrupt::Bureaucracy);
                }
            }
        }
    }

    pub fn activate_banker(&mut self, player_who_paid: PlayerId) {
        if !self.turn.banker_was_used_this_phase {
            let banker = self.player_skilled_as(LeaderSkill::Banker);
            if let Some(banker) = banker {
                if banker != player_who_paid {
                    self.turn.banker_was_used_this_phase = true;
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
                    crate::game::amounts::gain(&mut self.player_mut(banker).banked_resources, 1, "spice").expect("banked spice");
                }
            }
        }
    }

    pub fn play_nexus_card(&mut self, initiator: PlayerId, message_elements: Vec<Part>) {
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
        let karama_or_type: Part = match karama {
            Some(c) => c.into(),
            None => TreacheryCardType::Karma.into(),
        };
        self.log(vec![express!(karama_or_type, " prevents ", prevented).with_log(AsideLog::aside()).into()]);
    }

    pub fn log_prevention_by_low_threshold(&mut self, prevented: FactionAdvantage) {
        self.log(vec!["Low Threshold prevents ".into(), prevented.into()]);
    }

    pub fn prevented(&self, advantage: FactionAdvantage) -> bool {
        self.turn.prevented_advantages.contains(&advantage)
    }

    pub fn bribes_during_mentat(&self) -> bool {
        !self.applicable(Rule::BribesAreImmediate)
    }

    pub fn karma_prevented(&self, f: impl Into<Option<Faction>>) -> bool {
        let f: Option<Faction> = f.into();
        self.turn.current_karma_prevention.as_ref().is_some_and(|p| p.target == f)
    }

    pub fn juice_forces_first_player(&self) -> bool {
        self.turn.current_juice.as_ref().is_some_and(|j| j.type_ == Some(JuiceType::GoFirst))
    }

    pub fn juice_forces_last_player(&self) -> bool {
        self.turn.current_juice.as_ref().is_some_and(|j| j.type_ == Some(JuiceType::GoLast))
    }

}

#[derive(Clone, Debug)]
pub struct TriggeredBureaucracy {
    pub payment_from: Option<Faction>,
    pub payment_to: Option<Faction>,
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

/// A Clairvoyance question to another faction (`ClairVoyancePlayed` with a target), kept while it
/// waits for its answer and after.
#[derive(Clone, Debug, PartialEq)]
pub struct ClairvoyanceAsked {
    pub submitter: By,
    pub target: Option<Faction>,
    pub question: Option<ClairvoyanceQuestion>,
    pub question_parameter1: Option<String>,
    pub question_parameter2: Option<String>,
}

impl ClairvoyanceAsked {
    pub fn initiator(&self) -> Option<Faction> {
        self.submitter.faction
    }

    /// The question as the event that asked it, to read its parameters.
    pub fn played(&self) -> ClairVoyancePlayed {
        ClairVoyancePlayed {
            question: self.question,
            target: self.target,
            question_parameter1: self.question_parameter1.clone(),
            question_parameter2: self.question_parameter2.clone(),
        }
    }

    /// See [`ClairVoyancePlayed::parameter1`].
    pub fn parameter1(&self, game: &Game) -> crate::messages::part::Part {
        self.played().parameter1(game)
    }

    /// See [`ClairVoyancePlayed::parameter2`].
    pub fn parameter2(&self) -> crate::messages::part::Part {
        self.played().parameter2()
    }

    pub fn is_about(&self, game: &Game, type_: TreacheryCardType) -> bool {
        self.played().is_about(game, type_)
    }

    pub fn get_question(&self, game: &Game) -> Message {
        self.played().get_question(game)
    }
}

/// The answer to a Clairvoyance question (`ClairVoyanceAnswered`).
#[derive(Clone, Debug, PartialEq)]
pub struct ClairvoyanceAnswerGiven {
    pub submitter: By,
    pub answer: Option<ClairVoyanceAnswer>,
}

impl ClairvoyanceAnswerGiven {
    pub fn initiator(&self) -> Option<Faction> {
        self.submitter.faction
    }

    pub fn is_no(&self) -> bool {
        self.answer == Some(ClairVoyanceAnswer::No)
    }

    pub fn is_yes(&self) -> bool {
        self.answer == Some(ClairVoyanceAnswer::Yes)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClairVoyanceQandA {
    /// `Option` because the answer is recorded with `Game.LatestClairvoyance`, which can be
    /// null; nothing throws in that case.
    pub question: Option<ClairvoyanceAsked>,
    pub answer: ClairvoyanceAnswerGiven,
}
