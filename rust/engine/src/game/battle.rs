use crate::prelude::*;
use crate::messages::part::Part;

/// State of the battle phase: the battle sequence, the current battle with its plans, traitor
/// calls, voice and prescience, and what the winner and loser still have to do.
pub struct BattleState {
    pub battle_sequence: Option<PlayerSequence>,
    pub battle_about_to_start: Option<InitiatedBattle>,
    pub current_battle: Option<InitiatedBattle>,
    pub aggressor_plan: Option<BattlePlan>,
    pub previous_aggressor_plan: Option<BattlePlan>,
    pub aggressor_traitor_action: Option<TraitorCall>,
    pub defender_plan: Option<BattlePlan>,
    pub previous_defender_plan: Option<BattlePlan>,
    pub defender_traitor_action: Option<TraitorCall>,
    pub battle_outcome: Option<BattleOutcome>,
    /// `Some(None)`: decided, and nobody.
    pub battle_winner: Option<Option<Faction>>,
    /// `Some(None)`: decided, and nobody.
    pub battle_loser: Option<Option<Faction>>,
    pub nr_of_battles_fought: i32,
    /// `Some(None)`: decided, and nobody.
    pub current_ecaz_or_ally_fighter: Option<Option<Faction>>,
    pub current_ecaz_battle_contribution: i32,
    pub current_voice: Option<VoiceUsed>,
    /// Someone who may use Voice declined it this battle (`VoiceDeclined`), so it is closed.
    pub voice_declined: bool,
    pub current_prescience: Option<PrescienceUsed>,
    /// The plans as they were when a revision withdrew them.
    pub prescience_seen_plans: IndexMap<Option<PrescienceAspect>, BattlePlan>,
    pub current_thought: Option<ThoughtAsked>,
    // C#: `StrongholdAdvantage?`. Outside a battle it is `None`; in one, `Some(None)` until the
    // advantage is chosen.
    pub chosen_hms_advantage: Option<Option<StrongholdAdvantage>>,
    pub current_portable_antidote_used: Option<AntidoteEquipped>,
    pub poison_tooth_cancelled: bool,
    pub current_rock_was_melted: Option<RockMelted>,
    pub traitors_decipherer_can_look_at: Vec<Hero>,
    pub decipherer_may_replace_traitor: bool,
    pub loser_may_try_to_assassinate: bool,
    pub battle_winner_may_choose_to_discard: bool,
    pub secret_ally_allows_keeping_cards_after_losing_battle: bool,
    pub cards_to_be_discarded_by_loser_after_battle: Vec<TreacheryCard>,
    pub current_diplomacy: Option<DiplomacyUsed>,
    pub audited_cards: Vec<TreacheryCard>,
    pub harkonnen_victim: Option<Leader>,
    pub ixian_special_force_losses_to_take: i32,
    pub battle_triggered_bureaucracy: Option<TriggeredBureaucracy>,
    pub card_used_by_diplomat: Option<TreacheryCard>,
    pub auditor_survived_battle: bool,
    pub harkonnen_do_not_have_to_return_used_captured_leader: bool,
    pub battle_was_concluded_by_winner: bool,
    pub atreides_karma: bool,
    pub ecaz_karma_bonus: i32,
}

impl BattleState {
    pub fn new() -> Self {
        BattleState {
            battle_sequence: None,
            battle_about_to_start: None,
            current_battle: None,
            aggressor_plan: None,
            previous_aggressor_plan: None,
            aggressor_traitor_action: None,
            defender_plan: None,
            previous_defender_plan: None,
            defender_traitor_action: None,
            battle_outcome: None,
            battle_winner: None,
            battle_loser: None,
            nr_of_battles_fought: Default::default(),
            current_ecaz_or_ally_fighter: None,
            current_ecaz_battle_contribution: Default::default(),
            current_voice: None,
            voice_declined: false,
            current_prescience: None,
            prescience_seen_plans: IndexMap::new(),
            current_thought: None,
            chosen_hms_advantage: None,
            current_portable_antidote_used: None,
            poison_tooth_cancelled: Default::default(),
            current_rock_was_melted: None,
            traitors_decipherer_can_look_at: Vec::new(),
            decipherer_may_replace_traitor: Default::default(),
            loser_may_try_to_assassinate: Default::default(),
            battle_winner_may_choose_to_discard: true,
            secret_ally_allows_keeping_cards_after_losing_battle: Default::default(),
            cards_to_be_discarded_by_loser_after_battle: Vec::new(),
            current_diplomacy: None,
            audited_cards: Vec::new(),
            harkonnen_victim: None,
            ixian_special_force_losses_to_take: Default::default(),
            battle_triggered_bureaucracy: None,
            card_used_by_diplomat: None,
            auditor_survived_battle: Default::default(),
            harkonnen_do_not_have_to_return_used_captured_leader: Default::default(),
            battle_was_concluded_by_winner: Default::default(),
            atreides_karma: Default::default(),
            ecaz_karma_bonus: Default::default(),
        }
    }
}

/// The methods that read who played an event, for the state that a played event leaves.
macro_rules! played_by {
    ($($t:ty),* $(,)?) => {$(
        impl $t {
            pub fn initiator(&self) -> Option<Faction> {
                self.submitter.faction
            }

            #[track_caller]
            pub fn player(&self) -> PlayerId {
                self.submitter.player()
            }

            pub fn player_opt(&self) -> Option<PlayerId> {
                self.submitter.player
            }

            pub fn by(&self, f: impl Into<Option<Faction>>) -> bool {
                self.submitter.is(f)
            }
        }
    )*};
}

played_by!(InitiatedBattle, BattlePlan, TraitorCall, VoiceUsed, PrescienceUsed, ThoughtAsked, DiplomacyUsed, AntidoteEquipped, RockMelted);

/// A battle that was initiated (`BattleInitiated`): who started it, against whom, and where.
#[derive(Clone, Debug, PartialEq)]
pub struct InitiatedBattle {
    pub submitter: By,
    pub territory: Option<Territory>,
    pub target: Option<Faction>,
    /// Tells two battles with the same factions and territory apart: the history entry of the
    /// `BattleInitiated` that started it. C# compared the events by reference, so a battle
    /// initiated again after an undo is a different battle, at the same place in the history.
    pub id: EntryId,
}

/// A finalized battle plan (`Battle`). Revealing the plans can swap in the cards a Mirror Weapon,
/// a Diplomat or a Portable Antidote stand for; `original_weapon` and `original_defense` keep
/// the cards they replaced while that lasts.
#[derive(Clone, Debug, PartialEq)]
pub struct BattlePlan {
    pub submitter: By,
    pub hero: Option<Hero>,
    pub messiah: bool,
    pub forces: u16,
    pub forces_at_half_strength: u16,
    pub special_forces: u16,
    pub special_forces_at_half_strength: u16,
    pub ally_contribution_amount: u16,
    pub weapon: Option<TreacheryCard>,
    pub defense: Option<TreacheryCard>,
    pub banker_bonus: u16,
    pub original_weapon: Option<TreacheryCard>,
    pub original_defense: Option<TreacheryCard>,
}

/// Whether a faction called treachery (`TreacheryCalled`).
#[derive(Clone, Debug, PartialEq)]
pub struct TraitorCall {
    pub submitter: By,
    pub traitor_called: bool,
}

/// The Voice used in the current battle (`Voice`).
#[derive(Clone, Debug, PartialEq)]
pub struct VoiceUsed {
    pub submitter: By,
    pub must: bool,
    pub type_: Option<TreacheryCardType>,
}

/// The Prescience used in the current battle (`Prescience`).
#[derive(Clone, Debug, PartialEq)]
pub struct PrescienceUsed {
    pub submitter: By,
    pub aspect: Option<PrescienceAspect>,
}

/// The question a Thinker asked in the current battle (`Thought`).
#[derive(Clone, Debug, PartialEq)]
pub struct ThoughtAsked {
    pub submitter: By,
    pub card: Option<TreacheryCard>,
}

/// The card a Diplomat turns into the opponent's defense (`Diplomacy`).
#[derive(Clone, Debug, PartialEq)]
pub struct DiplomacyUsed {
    pub submitter: By,
    pub card: Option<TreacheryCard>,
}

/// Who equipped a Portable Antidote in the current battle (`PortableAntidoteUsed`).
#[derive(Clone, Debug, PartialEq)]
pub struct AntidoteEquipped {
    pub submitter: By,
}

/// How a Rock Melter was used in the current battle (`RockWasMelted`).
#[derive(Clone, Debug, PartialEq)]
pub struct RockMelted {
    pub submitter: By,
    pub kill: bool,
}

impl TraitorCall {
    pub fn succeeded(&self, game: &Game) -> bool {
        self.traitor_called && (self.initiator() != Some(Faction::Harkonnen) || !game.turn.harkonnen_traitor_was_cancelled)
    }
}

impl VoiceUsed {
    pub fn may_not(&self) -> bool {
        !self.must
    }
}

/// `vec![Part::from(a), ...]`: the `params object?[]` of `Log(...)`.
macro_rules! le {
    ($($e:expr),* $(,)?) => {
        vec![$($crate::messages::part::Part::from($e)),*]
    };
}

impl Game {
    pub fn initiate_battle(&mut self) {
        let Some(current_battle) = self.battle.battle_about_to_start.clone() else {
            panic!("Cannot initiate battle when BattleAboutToStart is null");
        };
        self.battle.current_battle = Some(current_battle.clone());
        self.update_battle_report_participants(&current_battle);

        self.battle.chosen_hms_advantage = Some(None);
        self.battle.battle_outcome = None;
        self.battle.nr_of_battles_fought += 1;

        let aggressive_player = current_battle.aggressive_player(self);
        self.announce_hero_availability(aggressive_player);
        let defending_player = current_battle.defending_player(self);
        self.announce_hero_availability(defending_player);
        let aggressive_player = current_battle.aggressive_player(self);
        let defending_player = current_battle.defending_player(self);
        self.assign_battle_wheels(aggressive_player, defending_player);
    }

    pub fn update_battle_report_participants(&mut self, battle: &InitiatedBattle) {
        if !self.current_report().is_battle() {
            return;
        }
        let aggressor = battle.aggressor(self);
        self.current_report_mut().aggressor = aggressor;
        let defender = battle.defender(self);
        self.current_report_mut().defender = defender;
        let territory = battle.territory;
        self.current_report_mut().territory = territory;
    }

    pub fn assign_battle_wheels(&mut self, player1: Option<PlayerId>, player2: Option<PlayerId>) {
        self.board.has_battle_wheel.clear();
        if let Some(player1) = player1 {
            let f = self.player(player1).faction;
            self.board.has_battle_wheel.push(f.expect("faction"));
        }
        if let Some(player2) = player2 {
            let f = self.player(player2).faction;
            self.board.has_battle_wheel.push(f.expect("faction"));
        }
    }

    pub fn announce_hero_availability(&mut self, p: Option<PlayerId>) {
        let Some(p) = p else { return };
        if Battle::valid_battle_heroes(self, p).is_empty() {
            let f = self.player(p).faction;
            self.log(le![f, " have no leaders available for this battle"]);
        }
    }

    pub fn handle_revealed_battle_plans(&mut self) -> GameResult {
        let (Some(current_battle), Some(mut aggressor_plan), Some(mut defender_plan), Some(aggressor_traitor_action), Some(defender_traitor_action)) = (
            self.battle.current_battle.clone(),
            self.battle.aggressor_plan.clone(),
            self.battle.defender_plan.clone(),
            self.battle.aggressor_traitor_action.clone(),
            self.battle.defender_traitor_action.clone(),
        ) else {
            return Ok(());
        };

        self.resolve_effect_of_owned_tueks_sietch(&aggressor_plan);
        self.resolve_effect_of_owned_tueks_sietch(&defender_plan);

        self.discard_one_time_cards_used_in_battle(&aggressor_traitor_action, &defender_traitor_action)?;

        self.resolve_battle(&current_battle, &mut aggressor_plan, &mut defender_plan, &aggressor_traitor_action, &defender_traitor_action)?;
        self.record_battle_outcome_in_report();

        if self.battle.card_used_by_diplomat.is_some() {
            let card = self.battle.card_used_by_diplomat;
            self.discard_card(card, true)?;
            self.battle.card_used_by_diplomat = None;
        }

        if Some(aggressor_plan.initiator()) == self.battle.battle_winner {
            self.activate_decipherer_if_applicable(&aggressor_plan);
        }
        if Some(defender_plan.initiator()) == self.battle.battle_winner {
            self.activate_decipherer_if_applicable(&defender_plan);
        }

        if Some(aggressor_plan.initiator()) == self.battle.battle_winner {
            self.activate_sandmaster_if_applicable(&aggressor_plan);
        }
        if Some(defender_plan.initiator()) == self.battle.battle_winner {
            self.activate_sandmaster_if_applicable(&defender_plan);
        }

        if Some(aggressor_plan.initiator()) == self.battle.battle_winner {
            self.resolve_effect_of_owned_sietch_tabr(&aggressor_plan, &defender_plan);
        }
        if Some(defender_plan.initiator()) == self.battle.battle_winner {
            self.resolve_effect_of_owned_sietch_tabr(&defender_plan, &aggressor_plan);
        }

        if let Some(outcome) = self.battle.battle_outcome.clone() {
            if Some(aggressor_plan.initiator()) == self.battle.battle_winner {
                self.resolve_effect_of_occupied_jacurutu(&aggressor_plan, outcome.def_undialed_forces);
            }
            if Some(defender_plan.initiator()) == self.battle.battle_winner {
                self.resolve_effect_of_occupied_jacurutu(&defender_plan, outcome.agg_undialed_forces);
            }
        }

        self.flip_bene_gesserit_advisors_when_alone()?;
        if let Some(territory) = current_battle.territory {
            self.determine_occupation(territory);
        }

        if let Some(bureaucracy) = self.battle.battle_triggered_bureaucracy.clone() {
            self.apply_bureaucracy(bureaucracy.payment_from, bureaucracy.payment_to);
            self.battle.battle_triggered_bureaucracy = None;
        }

        if self.phase() != Phase::Retreating {
            self.determine_how_to_proceed_after_revealing_battle_plans()?;
        }
        Ok(())
    }

    pub fn discard_one_time_cards_used_in_battle(&mut self, aggressor_call: &TraitorCall, defender_call: &TraitorCall) -> GameResult {
        let (Some(aggressor_plan), Some(defender_plan)) = (self.battle.aggressor_plan.clone(), self.battle.defender_plan.clone()) else {
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

    /// Works out the battle between the plans the game holds, `agg` and `def`. A Diplomat's or
    /// Portable Antidote's card can stay in a plan after the battle is worked out (see
    /// `BattlePlan::deactivate_dynamic_weapons`); the game's plans and `agg` and `def` keep it.
    pub fn resolve_battle(&mut self, b: &InitiatedBattle, agg: &mut BattlePlan, def: &mut BattlePlan, aggressor_treachery: &TraitorCall, defender_treachery: &TraitorCall) -> GameResult {
        let territory = b.territory.required("territory")?;
        let outcome = Battle::determine_battle_outcome(agg, def, territory, self);
        self.battle.aggressor_plan = Some(agg.clone());
        self.battle.defender_plan = Some(def.clone());
        let (agg, def) = (&*agg, &*def);
        self.battle.battle_outcome = Some(outcome.clone());

        let lasgun_shield = !aggressor_treachery.succeeded(self)
            && !defender_treachery.succeeded(self)
            && (agg.has_laser() || def.has_laser())
            && (agg.has_shield() || def.has_shield());

        self.activate_smuggler(aggressor_treachery, defender_treachery, &outcome, lasgun_shield);

        self.handle_reinforcements(agg)?;
        self.handle_reinforcements(def)?;

        if aggressor_treachery.succeeded(self) || defender_treachery.succeeded(self) {
            self.traitor_called(b, agg, def, defender_treachery, agg.hero, def.hero)?;
        } else if lasgun_shield {
            let territory = b.territory.required("territory")?;
            self.lasgun_shield_explosion(agg, def, agg.player(), def.player(), territory, agg.hero, def.hero)?;
        } else {
            let territory = b.territory.required("territory")?;
            self.set_hero_locations(agg, territory);
            let territory = b.territory.required("territory")?;
            self.set_hero_locations(def, territory);
            let territory = b.territory.required("territory")?;
            self.handle_battle_outcome(agg, def, territory)?;
        }

        self.determine_if_captured_leaders_must_be_released();
        self.battle.auditor_survived_battle = (agg.hero.is_some_and(|h| h.hero_type() == HeroType::Auditor) && self.is_alive(agg.hero.required("hero")?))
            || (def.hero.is_some_and(|h| h.hero_type() == HeroType::Auditor) && self.is_alive(def.hero.required("hero")?));
        Ok(())
    }

    pub fn record_battle_outcome_in_report(&mut self) {
        if !self.current_report().is_battle() {
            return;
        }
        self.current_report_mut().battle_decided = true;
        let winner = self.battle.battle_winner.unwrap_or(None);
        self.current_report_mut().battle_winner = winner;
    }

    pub fn harkonnen_must_decide_to_capture(&self) -> bool {
        self.battle.battle_winner == Some(Some(Faction::Harkonnen))
            && self.applicable(Rule::HarkonnenCapturesOrKillsLeaders)
            && !self.prevented(FactionAdvantage::HarkonnenCaptureLeader)
    }

    pub fn capture_leader(&mut self) {
        let (Some(battle_winner), Some(aggressor_plan), Some(defender_plan)) = (self.battle.battle_winner, self.battle.aggressor_plan.clone(), self.battle.defender_plan.clone()) else {
            return;
        };
        let victim = if aggressor_plan.by(battle_winner) { defender_plan } else { aggressor_plan };
        self.select_victim_of_harkonnen_winner(&victim);
    }

    pub fn handle_reinforcements(&mut self, plan: &BattlePlan) -> GameResult {
        if plan.has_reinforcements() {
            let p = plan.player();
            let forces_to_remove = std::cmp::min(3, self.player(p).forces_in_reserve());
            Player::kill_forces_in_reserves(self, p, forces_to_remove, false)?;

            let special_forces_to_remove = 3 - forces_to_remove;
            if special_forces_to_remove > 0 {
                Player::kill_forces_in_reserves(self, p, special_forces_to_remove, true)?;
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
        Ok(())
    }

    pub fn activate_smuggler(&mut self, aggressor_treachery: &TraitorCall, defender_treachery: &TraitorCall, outcome: &BattleOutcome, lasgun_shield: bool) {
        let (Some(aggressor_plan), Some(defender_plan), Some(current_battle)) = (self.battle.aggressor_plan.clone(), self.battle.defender_plan.clone(), self.battle.current_battle.clone()) else {
            return;
        };

        let agg_hero_survives = !defender_treachery.succeeded(self) && (aggressor_treachery.succeeded(self) || (!lasgun_shield && !outcome.agg_hero_killed));
        let def_hero_survives = !aggressor_treachery.succeeded(self) && (defender_treachery.succeeded(self) || (!lasgun_shield && !outcome.def_hero_killed));

        if agg_hero_survives && aggressor_plan.hero.is_some() {
            let territory = current_battle.territory.expect("territory");
            self.activate_smuggler_if_applicable(aggressor_plan.player(), aggressor_plan.hero, defender_plan.hero, territory);
        }

        if def_hero_survives && defender_plan.hero.is_some() {
            let territory = current_battle.territory.expect("territory");
            self.activate_smuggler_if_applicable(defender_plan.player(), defender_plan.hero, aggressor_plan.hero, territory);
        }
    }

    pub fn discard_one_time_cards(&mut self, plan: &BattlePlan) -> GameResult {
        if let Some(card) = plan.hero.and_then(|h| h.as_treachery_card()) {
            self.discard_card(Some(card), true)?;
        }

        if let Some(weapon) = plan.weapon() {
            if weapon.is_artillery()
                || weapon.is_mirror_weapon()
                || weapon.is_rock_melter()
                || (weapon.is_poison_tooth() && !self.battle.poison_tooth_cancelled)
                || !(weapon.is_weapon() || weapon.is_defense() || weapon.is_useless())
                || self
                    .battle.current_diplomacy
                    .as_ref()
                    .is_some_and(|d| plan.initiator() == d.initiator() && Some(weapon) == d.card)
            {
                self.discard_card(plan.weapon(), true)?;
            }
        }

        if let Some(defense) = plan.defense() {
            if defense.is_portable_antidote() || !(defense.is_weapon() || defense.is_defense() || defense.is_useless()) {
                self.discard_card(plan.defense(), true)?;
            }
        }

        if let Some(used) = self.battle.current_portable_antidote_used.clone() {
            if used.player() == plan.player() {
                let portable_antidote = self.player(used.player()).card(TreacheryCardType::PortableAntidote);
                if portable_antidote.is_some() {
                    self.discard_card(portable_antidote, true)?;
                }
            }
        }

        if let Some(melted) = self.battle.current_rock_was_melted.clone() {
            if melted.player() == plan.player() {
                let melted_card = self.player(melted.player()).card(TreacheryCardType::Rockmelter);
                if melted_card.is_some() {
                    self.discard_card(melted_card, true)?;
                }
            }
        }
        Ok(())
    }

    pub fn activate_sandmaster_if_applicable(&mut self, plan: &BattlePlan) {
        let Some(current_battle) = self.battle.current_battle.clone() else {
            return;
        };

        let territory = current_battle.territory.expect("territory");
        let location_with_resources = territory.locations().iter().copied().find(|l| self.board.resources_on_planet.contains_key(l));

        if location_with_resources.is_some()
            && plan.hero.is_some()
            && self.skilled_as(plan.hero.expect("hero"), LeaderSkill::Sandmaster)
            && self.player(plan.player()).any_forces_in_territory(current_battle.territory) > 0
        {
            let territory = current_battle.territory;
            self.log(le![LeaderSkill::Sandmaster, " adds ", Payment::of(3), " to ", territory]);
            crate::game::amounts::gain(self.board.resources_on_planet.get_mut(&location_with_resources.expect("location with resources")).expect("spice there"), 3, "spice")
                .expect("spice on the board");
        }
    }

    pub fn activate_smuggler_if_applicable(&mut self, player: PlayerId, hero: Option<Hero>, opponent_hero: Option<Hero>, territory: Territory) {
        let Some(hero) = hero else { return };

        if self.skilled_as(hero, LeaderSkill::Smuggler) {
            let location_with_resources = territory.locations().iter().copied().find(|l| self.board.resources_on_planet.contains_key(l));
            if let Some(location_with_resources) = location_with_resources {
                let collected = std::cmp::min(self.resources_in(location_with_resources), hero.value_in_combat_against(opponent_hero));
                if collected > 0 {
                    let f = self.player(player).faction;
                    self.log(le![f, LeaderSkill::Smuggler, " collects ", Payment::of(collected), " from ", territory]);
                    self.change_resources_on_planet(location_with_resources, -collected).expect("at most the spice there");
                    let pl = self.player_mut(player);
                    crate::game::amounts::gain(&mut pl.resources, collected, "spice").expect("collected spice");
                }
            }
        }
    }

    pub fn activate_decipherer_if_applicable(&mut self, plan: &BattlePlan) {
        let player_is_skilled = self.has_skill_in_front_of_shield(plan.player(), LeaderSkill::Decipherer);
        let leader_is_skilled = plan.hero.is_some() && self.skilled_as(plan.hero.expect("hero"), LeaderSkill::Decipherer);

        if player_is_skilled || leader_is_skilled {
            let p = plan.player();
            let traitor = self.decks.traitor_deck.as_mut().expect("traitor deck").draw();
            self.battle.traitors_decipherer_can_look_at.push(traitor);
            self.player_mut(p).known_non_traitors.insert(traitor);

            let traitor = self.decks.traitor_deck.as_mut().expect("traitor deck").draw();
            self.battle.traitors_decipherer_can_look_at.push(traitor);
            self.player_mut(p).known_non_traitors.insert(traitor);

            self.battle.decipherer_may_replace_traitor = leader_is_skilled && !BattleConcluded::valid_traitors_to_replace(self, p).next().is_none();
        }
    }

    pub fn finish_decipherer_if_applicable(&mut self) {
        if self.battle.traitors_decipherer_can_look_at.len() > 0 {
            let items = self.battle.traitors_decipherer_can_look_at.clone();
            for item in items {
                self.decks.traitor_deck.as_mut().expect("traitor deck").put_on_top(item);
            }

            self.decks.traitor_deck.as_mut().expect("traitor deck").shuffle(&mut self.rng);
            self.stone(Milestone::Shuffled);
            self.battle.traitors_decipherer_can_look_at.clear();
        }
    }

    pub fn resolve_effect_of_owned_tueks_sietch(&mut self, player_plan: &BattlePlan) {
        let Some(current_battle) = self.battle.current_battle.clone() else {
            return;
        };

        let territory = current_battle.territory.expect("territory");
        if self.has_stronghold_advantage(player_plan.initiator(), StrongholdAdvantage::CollectResourcesForUseless, territory) {
            self.collect_tueks_sietch_bonus(player_plan.player(), player_plan.weapon());
            self.collect_tueks_sietch_bonus(player_plan.player(), player_plan.defense());
        }
    }

    pub fn collect_tueks_sietch_bonus(&mut self, player: PlayerId, card: Option<TreacheryCard>) {
        if let Some(card) = card.filter(|c| c.type_() == TreacheryCardType::Useless) {
            let f = self.player(player).faction;
            let tueks = self.board.map.tueks_sietch();
            self.log(le![tueks, " stronghold advantage: ", f, " collect ", Payment::of(2), " for playing ", card]);
            crate::game::amounts::gain(&mut self.player_mut(player).resources, 2, "spice").expect("Tuek's Sietch spice");
        }
    }

    pub fn resolve_effect_of_owned_sietch_tabr(&mut self, winner_plan: &BattlePlan, opponent_plan: &BattlePlan) {
        let Some(current_battle) = self.battle.current_battle.clone() else {
            return;
        };

        let territory = current_battle.territory.expect("territory");
        if !self.has_stronghold_advantage(winner_plan.initiator(), StrongholdAdvantage::CollectResourcesForDial, territory) {
            return;
        }

        let collected = opponent_plan.dial(self, winner_plan.initiator().expect("faction")).floor();
        if collected <= 0 {
            return;
        }

        let tabr = self.board.map.sietch_tabr();
        self.log(le![tabr, " stronghold advantage: ", winner_plan.initiator(), " collect ", Payment::of(collected), " from enemy force dial"]);
        let pl = self.player_mut(winner_plan.player());
        crate::game::amounts::gain(&mut pl.resources, collected, "spice").expect("collected spice");
    }

    pub fn resolve_effect_of_occupied_jacurutu(&mut self, winner_plan: &BattlePlan, opponent_undialed_forces: i32) {
        let Some(current_battle) = self.battle.current_battle.clone() else {
            return;
        };

        let jacurutu = self.board.map.jacurutu();
        if current_battle.territory != Some(jacurutu.territory()) || opponent_undialed_forces <= 0 {
            return;
        }

        self.log(le![winner_plan.initiator(), " get ", Payment::of(opponent_undialed_forces), " from winning a fight in ", jacurutu]);
        let pl = self.player_mut(winner_plan.player());
        crate::game::amounts::gain(&mut pl.resources, opponent_undialed_forces, "spice").expect("spice from Jacurutu");
    }

    pub fn determine_how_to_proceed_after_revealing_battle_plans(&mut self) -> GameResult {
        if self.auditee().is_some() && !self.choam_leader_was_revealed_as_traitor() {
            self.prepare_audit()?;
        } else {
            let harkonnen_must_decide = self.harkonnen_must_decide_to_capture();
            // Enter(condition, FinishBattle, ...), with FinishBattle's failure passed on.
            if self.battle.battle_winner == Some(None) {
                self.finish_battle()?;
            } else {
                self.enter_either(harkonnen_must_decide, Phase::CaptureDecision, Phase::BattleConclusion);
            }
            self.let_factions_discard_surplus_cards();
        }
        Ok(())
    }

    pub fn prepare_audit(&mut self) -> GameResult {
        let Some(auditee) = self.auditee() else {
            return Ok(());
        };

        let mut auditable_cards = Deck::new(AuditCancelled::get_cards_that_may_be_audited(self));

        if auditable_cards.items.len() > 0 {
            let nr_of_audited_cards = AuditCancelled::get_number_of_cards_that_may_be_audited(self);
            self.battle.audited_cards.clear();
            auditable_cards.shuffle(&mut self.rng);
            let mut i = 0;
            while i < nr_of_audited_cards {
                let card = auditable_cards.try_draw()?;
                self.battle.audited_cards.push(card);
                i += 1;
            }

            self.enter(Phase::AvoidingAudit, true);
        } else {
            let f = self.player(auditee).faction;
            self.log(le![f, " don't have cards to audit"]);
            let harkonnen_must_decide = self.harkonnen_must_decide_to_capture();
            // Enter(condition, FinishBattle, ...), with FinishBattle's failure passed on.
            if self.battle.battle_winner == Some(None) {
                self.finish_battle()?;
            } else {
                self.enter_either(harkonnen_must_decide, Phase::CaptureDecision, Phase::BattleConclusion);
            }
        }

        self.let_factions_discard_surplus_cards();
        Ok(())
    }

    pub fn determine_if_captured_leaders_must_be_released(&mut self) {
        let Some(current_battle) = self.battle.current_battle.clone() else {
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
            .filter(|l| self.leaders.captured_leaders.contains_key(l) && !self.is_alive(Hero::Leader(*l)))
            .collect();
        for captive in dead_captives {
            self.return_captured_leader(harkonnen, captive);
        }

        //DetermineIfLeaderUsedInBattleMustBeReleased
        if !self.battle.harkonnen_do_not_have_to_return_used_captured_leader {
            let used_leader_in_battle = current_battle.plan_of(self, Some(harkonnen)).and_then(|p| p.hero);
            if let Some(leader) = used_leader_in_battle.and_then(|h| h.as_leader()) {
                if self.leaders.captured_leaders.contains_key(&leader) {
                    self.return_captured_leader(harkonnen, leader);
                }
            }
        }

        //DetermineIfCapturedLeadersMustBeReleasedWhenHarkonnenHasNoLeadersLeft
        if !self
            .player(harkonnen)
            .leaders
            .iter()
            .any(|l| !self.leaders.captured_leaders.contains_key(l) && self.is_alive(Hero::Leader(*l)))
        {
            let captives: Vec<Leader> = self.player(harkonnen).leaders.iter().copied().filter(|l| self.leaders.captured_leaders.contains_key(l)).collect();
            for captive in captives {
                self.return_captured_leader(harkonnen, captive);
            }
        }
    }

    pub fn return_captured_leader(&mut self, current_owner: PlayerId, to_return: Leader) {
        if let Some(value) = self.leaders.captured_leaders.get(&to_return).copied() {
            let original_player = self.get_player(Some(value)).expect("player");
            self.player_mut(original_player).leaders.push(to_return);
            let leaders = &mut self.player_mut(current_owner).leaders;
            if let Some(pos) = leaders.iter().position(|l| *l == to_return) {
                leaders.remove(pos);
            }
            self.leaders.captured_leaders.shift_remove(&to_return);

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
        match &self.battle.current_battle {
            Some(current_battle) => {
                current_battle.is_aggressor_or_defender(self, choam)
                    && current_battle.treachery_of_opponent(self, choam).is_some_and(|t| t.succeeded(self))
            }
            None => false,
        }
    }

    pub fn handle_battle_outcome(&mut self, agg: &BattlePlan, def: &BattlePlan, territory: Territory) -> GameResult {
        let Some(outcome) = self.battle.battle_outcome.clone() else {
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
            le![agg.hero, " ", outcome.agg_activated_bonus_skill, " bonus: ", outcome.agg_hero_skill_bonus],
        );
        self.log_if(
            outcome.def_hero_skill_bonus != 0,
            le![def.hero, " ", outcome.def_activated_bonus_skill, " bonus: ", outcome.def_hero_skill_bonus],
        );

        self.log_if(
            outcome.agg_battle_penalty != 0,
            le![agg.hero, " ", outcome.def_activated_penalty_skill, " penalty: ", outcome.agg_battle_penalty],
        );
        self.log_if(
            outcome.def_battle_penalty != 0,
            le![def.hero, " ", outcome.agg_activated_penalty_skill, " penalty: ", outcome.def_battle_penalty],
        );

        // The Kwisatz Haderach bonus shows on the leader in the revealed plans.

        let winner_faction = self.player(winner).faction;
        self.battle.battle_winner = Some(winner_faction);
        self.battle.battle_loser = Some(self.player(loser).faction);

        if outcome.agg_hero_killed && agg.hero.is_some() {
            self.kill_leader_in_battle(agg.hero.required("hero")?, outcome.agg_hero_cause_of_death, winner, outcome.agg_hero_effective_strength);
        } else {
            let carthag = self.board.map.carthag();
            self.log_if(
                outcome.agg_saved_by_carthag,
                le![carthag, " stronghold advantage saves ", agg.hero, " from death by ", TreacheryCardType::Poison],
            );
        }

        if outcome.def_hero_killed && def.hero.is_some() {
            self.kill_leader_in_battle(def.hero.required("hero")?, outcome.def_hero_cause_of_death, winner, outcome.def_hero_effective_strength);
        } else {
            let carthag = self.board.map.carthag();
            self.log_if(
                outcome.def_saved_by_carthag,
                le![carthag, " stronghold advantage saves ", def.hero, " from death by ", TreacheryCardType::Poison],
            );
        }

        let def_faction = self.player(def.player()).faction;
        if BattleInitiated::is_aggressor_by_juice(self, def_faction) {
            self.log(le![agg.initiator(), " strength: ", outcome.agg_total]);
            self.log(le![def.initiator(), " strength: ", outcome.def_total, " (aggressor by ", TreacheryCardType::Juice, ")"]);
        } else {
            self.log_strength(agg.initiator().required("faction")?, outcome.agg_total);
            self.log_strength(def.initiator().required("faction")?, outcome.def_total);
        }

        self.battle.loser_may_try_to_assassinate = self.battle.battle_loser == Some(Some(Faction::Moritani))
            && self.applicable(Rule::MoritaniAssassinate)
            && self.leaders.assassinated.iter().all(|l| Some(l.faction()) != self.battle.battle_winner.flatten())
            && winner_battle_plan.hero.is_some_and(|h| h.as_leader().is_some())
            && self.is_alive(winner_battle_plan.hero.required("hero")?);

        let message = express!(winner_faction, " WIN THE BATTLE").with_fact(BattleFact::new(BattleFactKind::Winner, winner_faction.required("faction")?));
        self.log(le![message]);

        self.handle_harass_and_withdraw(agg, territory)?;
        self.handle_harass_and_withdraw(def, territory)?;

        let loser_may_retreat = !outcome.loser_hero_killed()
            && loser_battle_plan.hero.is_some()
            && self.skilled_as(loser_battle_plan.hero.required("hero")?, LeaderSkill::Diplomat)
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

    pub fn handle_harass_and_withdraw(&mut self, plan: &BattlePlan, territory: Territory) -> GameResult {
        let Some(current_battle) = self.battle.current_battle.clone() else {
            return Ok(());
        };
        if !plan.weapon().is_some_and(|w| w.type_() == TreacheryCardType::HarassAndWithdraw)
            && !plan.defense().is_some_and(|d| d.type_() == TreacheryCardType::HarassAndWithdraw)
        {
            return Ok(());
        }

        let force_supplier = Battle::determine_force_supplier(self, plan.player());
        let battle_territory = current_battle.territory.expect("territory");
        let undialed_normal_forces = self
            .player(force_supplier)
            .forces_in_territory(Some(battle_territory)) - i32::from(plan.forces) - i32::from(plan.forces_at_half_strength);
        let battle_territory = current_battle.territory.expect("territory");
        let undialed_special_forces = self
            .player(force_supplier)
            .special_forces_in_territory(Some(battle_territory)) - i32::from(plan.special_forces) - i32::from(plan.special_forces_at_half_strength);
        Player::some_forces_in_territory_to_reserves(self, force_supplier, territory, undialed_normal_forces, false)?;
        Player::some_forces_in_territory_to_reserves(self, force_supplier, territory, undialed_special_forces, true)?;

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
        Ok(())
    }

    pub fn handle_losses(&mut self) -> GameResult {
        let (Some(current_battle), Some(outcome)) = (self.battle.current_battle.clone(), self.battle.battle_outcome.clone()) else {
            return Ok(());
        };

        let (Some(winner), Some(winner_plan), Some(loser), Some(loser_plan)) =
            (outcome.winner, outcome.winner_battle_plan.clone(), outcome.loser, outcome.loser_battle_plan.clone())
        else {
            return Ok(());
        };

        let territory = current_battle.territory.required("territory")?;
        self.process_winner_losses(territory, winner, &winner_plan, false)?;
        let territory = current_battle.territory.required("territory")?;
        self.process_loser_losses(territory, loser, &loser_plan)?;
        Ok(())
    }

    pub fn is_protected_by_carthag_advantage(&self, plan: &BattlePlan, territory: Territory) -> bool {
        self.has_stronghold_advantage(plan.initiator(), StrongholdAdvantage::CountDefensesAsAntidote, territory)
            && !plan.has_poison()
            && !plan.has_poison_tooth()
            && plan.defense().is_some_and(|d| d.is_defense())
    }

    pub fn process_loser_losses(&mut self, territory: Territory, loser: PlayerId, loser_gambit: &BattlePlan) -> GameResult {
        let had_messiah_before_losses = self.player(loser).messiah_available(self);
        let force_supplier = Battle::determine_force_supplier(self, loser);
        self.process_ecaz_occupation_loser_losses(territory, loser)?;

        self.log_all_forces_lost(force_supplier, territory);
        self.pay_dialed_spice(loser, loser_gambit, false);
        Player::kill_all_forces_in_territory(self, force_supplier, territory, true)?;
        let may_keep = self.may_keep_cards_after_losing_battle(loser);
        self.lose_cards(loser_gambit, may_keep)?;

        if self.player(loser).messiah_available(self) && !had_messiah_before_losses {
            self.stone(Milestone::Messiah);
        }
        Ok(())
    }

    pub fn process_ecaz_occupation_winner_losses(&mut self, territory: Territory, winner: PlayerId) -> GameResult {
        if self.battle.current_ecaz_battle_contribution > 0 && self.player(winner).or_ally_is(Faction::Ecaz) {
            let ecaz = self.get_player(Some(Faction::Ecaz));
            let Some(ecaz) = ecaz else { return Ok(()) };
            let amount = self.battle.current_ecaz_battle_contribution;
            Player::kill_forces_in_territory(self, ecaz, territory, amount, false, true)?;
            let force = self.player(ecaz).force();
            let amount = self.battle.current_ecaz_battle_contribution;
            self.log(le![Faction::Ecaz, " lose ", amount, force]);
        }
        Ok(())
    }

    pub fn process_ecaz_occupation_loser_losses(&mut self, territory: Territory, loser: PlayerId) -> GameResult {
        if self.battle.current_ecaz_battle_contribution > 0 && self.player(loser).or_ally_is(Faction::Ecaz) {
            let ecaz = self.get_player(Some(Faction::Ecaz));
            let Some(ecaz) = ecaz else { return Ok(()) };
            self.log_all_forces_lost(ecaz, territory);
            Player::kill_all_forces_in_territory(self, ecaz, territory, true)?;
        }
        Ok(())
    }

    pub fn may_keep_cards_after_losing_battle(&self, p: PlayerId) -> bool {
        (self.player(p).ally == Some(Faction::Moritani) && self.alliances.moritani_allows_keeping_cards)
            || (self.player(p).nexus == Some(Faction::Moritani) && NexusPlayed::can_use_secret_ally(self, p))
    }

    pub fn dialled_resources_are_refunded(&self, p: PlayerId) -> bool {
        self.applicable(Rule::FremenAllyGetsDialedResourcesRefunded) && self.player(p).ally == Some(Faction::Fremen) && self.alliances.fremen_refunds_battle_dial
    }

    pub fn pay_dialed_spice(&mut self, p: PlayerId, plan: &BattlePlan, traitor_was_revealed: bool) {
        let mut paid_by_arrakeen: i32 = 0;
        let cost = plan.cost_with_arrakeen_share(self, &mut paid_by_arrakeen);
        let cost_to_choam = if self.player(p).ally == Some(Faction::Choam) { i32::from(plan.ally_contribution_amount) } else { 0 };

        if paid_by_arrakeen > 0 {
            let arrakeen = self.board.map.arrakeen();
            self.log(le![arrakeen, " stronghold advantage supports ", Payment::of(paid_by_arrakeen)]);
        }

        if (cost + paid_by_arrakeen) > 0 {
            let cost_for_player = cost - i32::from(plan.ally_contribution_amount);
            let mut refunded_resources = 0;

            if cost_for_player > 0 {
                let pl = self.player_mut(p);
                crate::game::amounts::spend(&mut pl.resources, cost_for_player, "spice").expect("spice dialled in battle");

                if self.dialled_resources_are_refunded(p) {
                    self.log(le![Payment::of(cost_for_player), " dialled in battle will be refunded in the ", MainPhase::Contemplate, " phase"]);
                    refunded_resources = cost_for_player;
                    let pl = self.player_mut(p);
                    crate::game::amounts::gain(&mut pl.bribes, cost_for_player, "spice").expect("refunded spice");
                }
            }

            if i32::from(plan.ally_contribution_amount) > 0 {
                if let Some(allied_player) = self.player(p).allied_player(self) {
                    let ap = self.player_mut(allied_player);
                    crate::game::amounts::spend(&mut ap.resources, i32::from(plan.ally_contribution_amount), "spice").expect("the ally's spice dialled in battle");
                    let f = self.player(p).faction;
                    self.decrease_permitted_use_of_ally_spice(f.expect("faction"), i32::from(plan.ally_contribution_amount));
                }
            }

            let dialled_resources_relevant_for_choam =
                (cost - cost_to_choam - refunded_resources) + paid_by_arrakeen;

            let receiver_profit = self.handle_choam_income(p, dialled_resources_relevant_for_choam, traitor_was_revealed);

            if (cost - receiver_profit) >= 4 {
                self.activate_banker(p);
            }
        }

        if i32::from(plan.banker_bonus) > 0 {
            let pl = self.player_mut(p);
            crate::game::amounts::spend(&mut pl.resources, i32::from(plan.banker_bonus), "spice").expect("spice paid for the Banker");
            let f = self.player(p).faction;
            self.log(le![f, " paid ", Payment::of(i32::from(plan.banker_bonus)), " as ", LeaderSkill::Banker]);
        }
    }

    pub fn handle_choam_income(&mut self, paid_by: PlayerId, costs_excluding_payment_by_choam_ally: i32, traitor_was_revealed: bool) -> i32 {
        let mut result = 0;

        let choam = self.get_player(Some(Faction::Choam));
        if let Some(choam) = choam {
            if self.player(paid_by).faction != Some(Faction::Choam) && !traitor_was_revealed {
                // Half, rounded down.
                result = costs_excluding_payment_by_choam_ally.div_euclid(2);

                if result > 0 {
                    if !self.prevented(FactionAdvantage::ChoamReceiveForcePayment) {
                        let b = self.player_mut(choam);
                        crate::game::amounts::gain(&mut b.resources, result, "spice").expect("CHOAM's income");
                        self.log(le![Faction::Choam, " get ", Payment::of(result), " from supported forces"]);

                        if result >= 5 {
                            self.battle.battle_triggered_bureaucracy = Some(TriggeredBureaucracy {
                                payment_from: self.player(paid_by).faction,
                                payment_to: Some(Faction::Choam),
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

    pub fn process_winner_losses(&mut self, territory: Territory, winner: PlayerId, plan: &BattlePlan, traitor_was_revealed: bool) -> GameResult {
        self.pay_dialed_spice(winner, plan, traitor_was_revealed);
        self.process_winner_force_losses(territory, winner, plan)?;
        Ok(())
    }

    pub fn process_winner_force_losses(&mut self, territory: Territory, winner: PlayerId, plan: &BattlePlan) -> GameResult {
        let force_supplier = Battle::determine_force_supplier(self, winner);
        self.process_ecaz_occupation_winner_losses(territory, winner)?;

        let special_forces_to_lose = i32::from(plan.special_forces) + i32::from(plan.special_forces_at_half_strength);
        let forces_to_lose = i32::from(plan.forces) + i32::from(plan.forces_at_half_strength);

        let mut special_forces_to_save_to_reserves = 0;
        let mut forces_to_save_to_reserves = 0;
        let mut special_forces_to_save_in_territory = 0;
        let mut forces_to_save_in_territory = 0;

        let nr_of_special_force_losses_that_can_be_replaced_by_remaining_normal_forces = std::cmp::min(
            special_forces_to_lose,
            (self.player(force_supplier)
                .forces_in_territory(Some(territory)) - i32::from(plan.forces)) - i32::from(plan.forces_at_half_strength),
        );

        if !self.may_substitute_force_losses(force_supplier) || nr_of_special_force_losses_that_can_be_replaced_by_remaining_normal_forces == 0 {
            if plan.hero.is_some() && self.skilled_as(plan.hero.expect("hero"), LeaderSkill::Graduate) {
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
            } else if self.has_skill_in_front_of_shield(winner, LeaderSkill::Graduate) {
                special_forces_to_save_to_reserves = std::cmp::min(special_forces_to_lose, 1);
                forces_to_save_to_reserves = std::cmp::max(0, std::cmp::min(forces_to_lose, 1 - special_forces_to_save_to_reserves));
            }
        }

        if special_forces_to_save_in_territory + forces_to_save_in_territory + special_forces_to_save_to_reserves + forces_to_save_to_reserves > 0 {
            if special_forces_to_save_to_reserves > 0 {
                Player::some_forces_in_territory_to_reserves(self, force_supplier, territory, special_forces_to_save_to_reserves, true)?;
            }

            if forces_to_save_to_reserves > 0 {
                Player::some_forces_in_territory_to_reserves(self, force_supplier, territory, forces_to_save_to_reserves, false)?;
            }

            let force = self.player(force_supplier).force();
            let special_force = self.player(force_supplier).special_force();
            self.log(le![
                LeaderSkill::Graduate,
                " rescues ",
                express_part_if!(forces_to_save_in_territory > 0; forces_to_save_in_territory, force),
                express_part_if!(special_forces_to_save_in_territory > 0; special_forces_to_save_in_territory, special_force),
                express_part_if!(forces_to_save_in_territory > 0 || special_forces_to_save_in_territory > 0; " on site"),
                express_part_if!((forces_to_save_in_territory > 0 || special_forces_to_save_in_territory > 0) && (forces_to_save_to_reserves > 0 || special_forces_to_save_to_reserves > 0); " and "),
                express_part_if!(forces_to_save_to_reserves > 0; forces_to_save_to_reserves, force),
                express_part_if!(special_forces_to_save_to_reserves > 0; special_forces_to_save_to_reserves, special_force),
                express_part_if!(forces_to_save_to_reserves > 0 || special_forces_to_save_to_reserves > 0; " to reserves")
            ]);
        }

        if !self.may_substitute_force_losses(force_supplier)
            || (special_forces_to_lose - special_forces_to_save_to_reserves - special_forces_to_save_in_territory)
                == 0
            || self.player(force_supplier).forces_in_territory(Some(territory)) <= (i32::from(plan.forces) + i32::from(plan.forces_at_half_strength))
        {
            let winner_forces_lost = forces_to_lose - forces_to_save_to_reserves - forces_to_save_in_territory;
            let winner_special_forces_lost = special_forces_to_lose - special_forces_to_save_to_reserves - special_forces_to_save_in_territory;
            self.handle_force_losses(territory, force_supplier, winner_forces_lost, winner_special_forces_lost)?;
        } else {
            self.battle.ixian_special_force_losses_to_take = special_forces_to_lose - special_forces_to_save_to_reserves - special_forces_to_save_in_territory;
        }
        Ok(())
    }

    pub fn may_substitute_force_losses(&self, p: PlayerId) -> bool {
        self.player(p).faction == Some(Faction::Ixian) && !self.prevented(FactionAdvantage::IxianReplacingSpecialForces)
    }

    pub fn handle_force_losses(&mut self, territory: Territory, player: PlayerId, forces_lost: i32, special_forces_lost: i32) -> GameResult {
        let had_messiah_before_losses = self.player(player).messiah_available(self);

        Player::kill_forces_in_territory(self, player, territory, forces_lost, false, true)?;
        Player::kill_forces_in_territory(self, player, territory, special_forces_lost, true, true)?;

        self.log_losses(player, forces_lost, special_forces_lost);

        if self.player(player).messiah_available(self) && !had_messiah_before_losses {
            self.stone(Milestone::Messiah);
        }
        Ok(())
    }

    pub fn kill_leader_in_battle(&mut self, killed_hero: Hero, cause_of_death: Option<TreacheryCardType>, winner: PlayerId, hero_value: i32) {
        let winner_faction = self.player(winner).faction;
        let mut fact = BattleFact::new(BattleFactKind::Killed, winner_faction.expect("faction"));
        fact.hero = Some(killed_hero);
        fact.cause = Some(Part::from(cause_of_death));
        fact.value = hero_value;
        let message = express!(cause_of_death, " kills ", killed_hero, " → ", winner_faction, " get ", Payment::of(hero_value)).with_fact(fact);
        self.log(le![message]);
        if killed_hero.as_leader().is_some() {
            self.kill_hero(killed_hero);
        }
        let pl = self.player_mut(winner);
        crate::game::amounts::gain(&mut pl.resources, hero_value, "spice").expect("the killed leader's value");
    }

    pub fn log_all_forces_lost(&mut self, player: PlayerId, territory: Territory) {
        let forces = self.player(player).forces_in_territory(Some(territory));
        let special_forces = self.player(player).special_forces_in_territory(Some(territory));
        let faction = self.player(player).faction;
        let force = self.player(player).force();
        let special_force = self.player(player).special_force();
        let mut fact = BattleFact::new(BattleFactKind::ForcesLost, faction.expect("faction"));
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

    pub fn log_strength(&mut self, faction: Faction, strength: HalfPoints) {
        let mut fact = BattleFact::new(BattleFactKind::Strength, faction);
        fact.strength = strength;
        let message = express!(faction, " strength: ", strength).with_fact(fact);
        self.log(le![message]);
    }

    pub fn log_losses(&mut self, player: PlayerId, forces_lost: i32, special_forces_lost: i32) {
        if forces_lost > 0 || special_forces_lost > 0 {
            let faction = self.player(player).faction;
            let force = self.player(player).force();
            let special_force = self.player(player).special_force();
            let mut fact = BattleFact::new(BattleFactKind::ForcesLost, faction.expect("faction"));
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

    pub fn set_hero_locations(&mut self, b: &BattlePlan, territory: Territory) {
        if let Some(hero) = b.hero.filter(|h| h.as_leader().is_some()) {
            self.leaders.leader_state[&hero].current_territory = Some(territory);
        }

        if b.messiah {
            self.leaders.leader_state[&LeaderManager::messiah()].current_territory = Some(territory);
        }
    }

    pub fn traitor_called(&mut self, b: &InitiatedBattle, agg: &BattlePlan, def: &BattlePlan, defender_treachery: &TraitorCall, agg_leader: Option<Hero>, def_leader: Option<Hero>) -> GameResult {
        let aggressor_traitor_action = self.battle.aggressor_traitor_action.clone();
        if aggressor_traitor_action.as_ref().is_some_and(|a| a.succeeded(self)) && defender_treachery.succeeded(self) {
            let (Some(agg_leader), Some(def_leader)) = (agg_leader, def_leader) else {
                game_error!("A required value is missing");
            };

            let territory = b.territory.required("territory")?;
            self.two_traitors_called(agg, def, agg.player(), def.player(), territory, agg_leader, def_leader)?;
        } else if let Some(aggressor_traitor_action) = aggressor_traitor_action {
            let succeeded = aggressor_traitor_action.succeeded(self);
            let winner = if succeeded { agg.player() } else { def.player() };
            let loser = if succeeded { def.player() } else { agg.player() };
            let loser_gambit = if aggressor_traitor_action.succeeded(self) { def } else { agg };
            let winner_gambit = if aggressor_traitor_action.succeeded(self) { agg } else { def };
            if self.player(winner).is(Faction::Harkonnen) {
                self.battle.harkonnen_do_not_have_to_return_used_captured_leader = true;
            }
            let territory = b.territory.required("territory")?;
            self.one_traitor_called(territory, winner, loser, loser_gambit, winner_gambit)?;
        }
        Ok(())
    }

    pub fn one_traitor_called(&mut self, territory: Territory, winner: PlayerId, loser: PlayerId, loser_plan: &BattlePlan, winner_plan: &BattlePlan) -> GameResult {
        let Some(traitor) = loser_plan.hero else {
            game_error!("Traitor cannot be null in OneTraitorCalled");
        };

        let had_messiah_before_losses = self.player(loser).messiah_available(self);
        let traitor_value = traitor.value_in_combat_against(winner_plan.hero);
        let traitor_owner = if self.player(winner).traitors.iter().any(|t| t.is_traitor(traitor)) {
            self.player(winner).faction
        } else {
            Some(Faction::Harkonnen)
        };

        // The traitor decides the battle, but the strengths were worked out
        // already, and the log shows them.
        if let (Some(current_battle), Some(outcome)) = (self.battle.current_battle.clone(), self.battle.battle_outcome.clone()) {
            let aggressor = current_battle.aggressor(self);
            self.log_strength(aggressor.required("faction")?, outcome.agg_total);
            let defender = current_battle.defender(self);
            self.log_strength(defender.required("faction")?, outcome.def_total);
        }

        let loser_faction = self.player(loser).faction;
        let mut fact = BattleFact::new(BattleFactKind::Traitor, traitor_owner.required("faction")?);
        fact.hero = Some(traitor);
        let message = express!(traitor, " is a ", traitor_owner, " traitor! ", loser_faction, " lose everything").with_fact(fact);
        self.log(le![message]);

        if traitor.as_leader().is_some() {
            let winner_faction = self.player(winner).faction;
            let mut fact = BattleFact::new(BattleFactKind::Killed, winner_faction.required("faction")?);
            fact.hero = Some(traitor);
            fact.cause = Some(Part::from("Treachery"));
            fact.value = traitor_value;
            let message = express!("Treachery kills ", traitor, " → ", winner_faction, " get ", Payment::of(traitor_value)).with_fact(fact);
            self.log(le![message]);
            self.kill_hero(traitor);
            let pl = self.player_mut(winner);
            crate::game::amounts::gain(&mut pl.resources, traitor_value, "spice")?;
        }

        self.battle.battle_winner = Some(self.player(winner).faction);
        self.battle.battle_loser = Some(self.player(loser).faction);

        let force_supplier_of_loser = Battle::determine_force_supplier(self, loser);

        self.process_ecaz_occupation_loser_losses(territory, loser)?;

        if force_supplier_of_loser != loser {
            self.log_all_forces_lost(force_supplier_of_loser, territory);
            Player::kill_all_forces_in_territory(self, force_supplier_of_loser, territory, true)?;
        }

        self.log_all_forces_lost(loser, territory);
        Player::kill_all_forces_in_territory(self, loser, territory, true)?;
        let may_keep = self.may_keep_cards_after_losing_battle(loser);
        self.lose_cards(loser_plan, may_keep)?;
        self.pay_dialed_spice(loser, loser_plan, true);

        if self.player(loser).messiah_available(self) && !had_messiah_before_losses {
            self.stone(Milestone::Messiah);
        }
        Ok(())
    }

    /// C# parity: never sets BattleWinner or BattleLoser.
    pub fn two_traitors_called(&mut self, agg: &BattlePlan, def: &BattlePlan, aggressor: PlayerId, defender: PlayerId, territory: Territory, agg_leader: Hero, def_leader: Hero) -> GameResult {
        let had_messiah_before_losses = self.player(aggressor).messiah_available(self) || self.player(defender).messiah_available(self);

        self.log(le!["Treachery kills both ", def_leader, " and ", agg_leader]);
        self.kill_hero(def_leader);
        self.kill_hero(agg_leader);

        let force_supplier_of_defender = Battle::determine_force_supplier(self, defender);
        self.process_ecaz_occupation_loser_losses(territory, defender)?;
        if force_supplier_of_defender != defender {
            self.log_all_forces_lost(force_supplier_of_defender, territory);
            Player::kill_all_forces_in_territory(self, force_supplier_of_defender, territory, true)?;
        }

        self.log_all_forces_lost(defender, territory);
        Player::kill_all_forces_in_territory(self, defender, territory, true)?;

        let force_supplier_of_aggressor = Battle::determine_force_supplier(self, aggressor);
        self.process_ecaz_occupation_loser_losses(territory, aggressor)?;
        if force_supplier_of_aggressor != aggressor {
            self.log_all_forces_lost(force_supplier_of_aggressor, territory);
            Player::kill_all_forces_in_territory(self, force_supplier_of_aggressor, territory, true)?;
        }

        self.log_all_forces_lost(aggressor, territory);
        Player::kill_all_forces_in_territory(self, aggressor, territory, true)?;

        self.lose_cards(def, false)?;
        self.pay_dialed_spice(defender, def, true);

        self.lose_cards(agg, false)?;
        self.pay_dialed_spice(aggressor, agg, true);

        if (self.player(aggressor).messiah_available(self) || self.player(defender).messiah_available(self)) && !had_messiah_before_losses {
            self.stone(Milestone::Messiah);
        }
        Ok(())
    }

    pub fn lasgun_shield_explosion(&mut self, agg: &BattlePlan, def: &BattlePlan, aggressor: PlayerId, defender: PlayerId, territory: Territory, agg_leader: Option<Hero>, def_leader: Option<Hero>) -> GameResult {
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

        self.kill_all_forces_in(territory, true)?;
        self.kill_ambassador_in(territory);

        if (self.player(aggressor).messiah_available(self) || self.player(defender).messiah_available(self)) && !had_messiah_before_losses {
            self.stone(Milestone::Messiah);
        }
        Ok(())
    }

    pub fn kill_all_forces_in(&mut self, territory: Territory, in_battle: bool) -> GameResult {
        let mut i = 0;
        while i < self.players.len() {
            let p = self.player_at(i);
            // `territory` may be null: AnyForcesIn(null) is 0.
            if self.player(p).any_forces_in_territory(Some(territory)) > 0 {
                self.reveal_current_no_field_in_territory(p, territory)?;

                let homeworld_kill_limit = if in_battle {
                    self.player(p).get_homeworld_battle_contribution_and_lasgun_shield_limit(self, territory)
                } else {
                    0
                };
                if homeworld_kill_limit == 0 {
                    let f = self.player(p).faction;
                    self.log(le!["All ", f, " forces in ", territory, " were killed"]);
                    Player::kill_all_forces_in_territory(self, p, territory, in_battle)?;
                } else {
                    let normal_forces_to_kill = std::cmp::min(self.player(p).forces_in_territory(Some(territory)), homeworld_kill_limit);
                    let special_forces_to_kill = std::cmp::min(
                        self.player(p).special_forces_in_territory(Some(territory)),
                        homeworld_kill_limit - normal_forces_to_kill,
                    );

                    if normal_forces_to_kill > 0 {
                        Player::kill_forces_in_territory(self, p, territory, normal_forces_to_kill, false, in_battle)?;
                    }
                    if special_forces_to_kill > 0 {
                        Player::kill_forces_in_territory(self, p, territory, special_forces_to_kill, true, in_battle)?;
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
        Ok(())
    }

    pub fn select_victim_of_harkonnen_winner(&mut self, victim_action: &BattlePlan) {
        let victim = self.get_player(victim_action.initiator());
        let Some(victim) = victim else { return };

        // Get all living leaders from the opponent that haven't fought in another territory this turn
        let leaders: Vec<Leader> = self
            .player(victim)
            .leaders
            .iter()
            .copied()
            .filter(|l| {
                l.hero_type() != HeroType::Auditor && self.leaders.leader_state[&Hero::Leader(*l)].alive() && self.can_join_current_battle(Hero::Leader(*l))
            })
            .collect();
        let mut available_leaders: Deck<Leader> = Deck::new(leaders);

        if !available_leaders.is_empty() {
            available_leaders.shuffle(&mut self.rng);
            self.battle.harkonnen_victim = Some(available_leaders.draw());
        } else {
            self.battle.harkonnen_victim = None;
            let f = self.player(victim).faction;
            self.log(le![f, " don't have any leaders for ", Faction::Harkonnen, " to capture or kill"]);
        }
    }

    pub fn finish_battle(&mut self) -> GameResult {
        let Some(aggressor_plan) = self.battle.aggressor_plan.clone() else {
            game_error!("Aggressor plan is null");
        };
        let Some(defender_plan) = self.battle.defender_plan.clone() else {
            game_error!("Defender plan is null");
        };
        let Some(aggressor_traitor_action) = self.battle.aggressor_traitor_action.clone() else {
            game_error!("Aggressor traitor action is null");
        };
        let Some(defender_traitor_action) = self.battle.defender_traitor_action.clone() else {
            game_error!("Defender traitor action is null");
        };
        let Some(current_battle) = self.battle.current_battle.clone() else {
            game_error!("Current battle is null");
        };

        // `IHero? == Leader?`: true when both are null.
        let vidal = self.vidal().map(Hero::Leader);
        if aggressor_plan.hero == vidal
            && self.leaders.when_to_set_aside_vidal == Some(VidalMoment::AfterUsedInBattle)
            && !(aggressor_traitor_action.succeeded(self) && !defender_traitor_action.succeeded(self))
        {
            self.set_aside_vidal();
        }

        let vidal = self.vidal().map(Hero::Leader);
        if defender_plan.hero == vidal
            && self.leaders.when_to_set_aside_vidal == Some(VidalMoment::AfterUsedInBattle)
            && !(defender_traitor_action.succeeded(self) && !aggressor_traitor_action.succeeded(self))
        {
            self.set_aside_vidal();
        }

        self.return_skilled_leaders_in_front_of_shield_after_battle();
        let territory = current_battle.territory.expect("territory");
        self.determine_occupation(territory);
        if !self.applicable(Rule::FullPhaseKarma) {
            self.allow_prevented_battle_faction_advantages();
        }
        if self.turn.current_juice.as_ref().is_some_and(|j| j.type_ == Some(JuiceType::Aggressor)) {
            self.turn.current_juice = None;
        }
        self.battle.current_diplomacy = None;
        self.battle.current_rock_was_melted = None;
        self.battle.current_portable_antidote_used = None;
        self.battle.harkonnen_do_not_have_to_return_used_captured_leader = false;
        self.finish_decipherer_if_applicable();
        if self.advance_battle_sequence().is_none() {
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

    /// C# parity: a different occupation rule from `DetermineOccupationAtStartOrEndOfTurn`.
    pub fn determine_occupation_location(&mut self, location: Option<Location>) {
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
            self.board.homeworld_occupation.shift_remove(&hw);

            if !self.player(sole_player_on_planet).is_native_homeworld(hw) {
                let f = self.player(sole_player_on_planet).faction;
                self.board.homeworld_occupation.insert_new(hw, f.expect("faction"));
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
        let Some(current_battle) = self.battle.current_battle.clone() else {
            return;
        };

        // The body changes only values, never the keys.
        let keys: Vec<Hero> = self.leaders.leader_state.keys().copied().collect();
        for key in keys {
            let Some(leader) = key.as_leader() else { continue };
            if !(self.is_skilled(Some(key)) && !self.leaders.leader_state[&key].in_front_of_shield) {
                continue;
            }

            let current_owner = self.player_indices().find(|p| self.player(*p).leaders.contains(&leader));

            let take = match current_owner {
                None => true,
                Some(current_owner) => {
                    !self.leaders.captured_leaders.contains_key(&leader)
                        && !(self.player(current_owner).faction != Some(Faction::Ecaz) && leader.hero_type() == HeroType::Vidal)
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

    pub fn lose_cards(&mut self, plan: &BattlePlan, may_choose_to_keep_one: bool) -> GameResult {
        let p = plan.player();
        if !(self.player(p).ally == Some(Faction::Moritani) && self.alliances.moritani_allows_keeping_cards)
            && self.player(p).nexus == Some(Faction::Moritani)
            && NexusPlayed::can_use_secret_ally(self, p)
        {
            self.battle.secret_ally_allows_keeping_cards_after_losing_battle = true;
        }

        if may_choose_to_keep_one {
            if let Some(weapon) = plan.weapon() {
                if self.player(p).has(Some(weapon)) {
                    self.battle.cards_to_be_discarded_by_loser_after_battle.push(weapon);
                }
            }
            if let Some(defense) = plan.defense() {
                if self.player(p).has(Some(defense)) {
                    self.battle.cards_to_be_discarded_by_loser_after_battle.push(defense);
                }
            }
        } else {
            if plan.weapon().is_some() {
                self.discard_card(plan.weapon(), true)?;
            }
            if plan.defense().is_some() {
                self.discard_card(plan.defense(), true)?;
            }
        }
        Ok(())
    }

    /// The player whose turn it is to initiate a battle: the current player of the battle
    /// sequence, or the first after them who has a battle to fight. `None` when nobody has one.
    /// Reads the sequence without changing it; [`Game::advance_battle_sequence`] moves it on.
    pub fn next_player_to_battle(&self) -> Option<PlayerId> {
        let mut seq = self.battle.battle_sequence.clone()?;
        self.skip_players_without_battles(&mut seq)
    }

    /// Moves the battle sequence past the players who have no battle to fight, to the player
    /// [`Game::next_player_to_battle`] names. The battle phase does this when it starts, after
    /// each battle, and after each event while a player is to initiate a battle or the battle
    /// report is shown (`Game::play`), so a player passed over stays passed over.
    pub fn advance_battle_sequence(&mut self) -> Option<PlayerId> {
        let mut seq = self.battle.battle_sequence.take()?;
        let next = self.skip_players_without_battles(&mut seq);
        self.battle.battle_sequence = Some(seq);
        next
    }

    /// Advances `seq` (this game's battle sequence) until its current player has a battle to
    /// fight, at most once around the table.
    fn skip_players_without_battles(&self, seq: &mut PlayerSequence) -> Option<PlayerId> {
        for _ in 0..self.players.len() {
            let player_to_check = seq.current_player(self);
            // BattlesToBeFought dereferences the player before anything else that has effects.
            if !Battle::battles_to_be_fought(self, player_to_check.expect("player to check"), false).is_empty() {
                return player_to_check;
            }
            seq.next_player(self);
        }
        None
    }

    pub fn auditee(&self) -> Option<PlayerId> {
        if self.applicable(Rule::ChoamAuditor) && !self.prevented(FactionAdvantage::ChoamAudit) {
            if self.battle.aggressor_plan.as_ref().is_some_and(|p| p.hero.is_some_and(|h| h.hero_type() == HeroType::Auditor)) {
                if let Some(defender_plan) = &self.battle.defender_plan {
                    return Some(defender_plan.player());
                }
            }

            if self.battle.defender_plan.as_ref().is_some_and(|p| p.hero.is_some_and(|h| h.hero_type() == HeroType::Auditor)) {
                if let Some(aggressor_plan) = &self.battle.aggressor_plan {
                    return Some(aggressor_plan.player());
                }
            }
        }

        None
    }

    pub fn winner_hero(&self) -> Option<Hero> {
        // `Faction? != Faction.None`: true when BattleWinner is null.
        if self.battle.battle_winner != Some(None) {
            let winner_gambit = if self.battle.battle_winner == self.battle.aggressor_plan.as_ref().map(|p| p.initiator()) {
                self.battle.aggressor_plan.as_ref()
            } else {
                self.battle.defender_plan.as_ref()
            };
            return winner_gambit.and_then(|p| p.hero);
        }

        None
    }

    pub fn winner_battle_action(&self) -> Option<BattlePlan> {
        if let Some(aggressor_plan) = &self.battle.aggressor_plan {
            if Some(aggressor_plan.initiator()) == self.battle.battle_winner {
                return Some(aggressor_plan.clone());
            }
        }
        if let Some(defender_plan) = &self.battle.defender_plan {
            if Some(defender_plan.initiator()) == self.battle.battle_winner {
                return Some(defender_plan.clone());
            }
        }

        None
    }

    pub fn can_join_current_battle(&self, hero: Hero) -> bool {
        let current_territory = self.leaders.leader_state[&hero].current_territory;
        current_territory.is_none() || current_territory == self.battle.current_battle.as_ref().and_then(|b| b.territory)
    }
}

#[derive(Clone, Debug)]
pub struct Fight {
    pub territory: Territory,
    pub faction: Faction,
}

impl Fight {
    pub fn id(&self) -> i32 {
        self.territory.id() + (1000i32 * self.faction as i32)
    }

    pub fn new(territory: Territory, faction: Faction) -> Self {
        Fight { territory, faction }
    }

}

#[derive(Clone, Debug)]
pub struct LossToTake {
    pub faction: Option<Faction>,
    /// `Option` because `new LossToTake()` leaves it null (`TakeLosses.LossesToTake` on an empty list).
    pub location: Option<Location>,
    pub amount: i32,
}

impl LossToTake {
    pub fn new() -> Self {
        LossToTake { faction: None, location: None, amount: 0 }
    }

}

impl GameStatus {
    /// The status in the battle phase, or `None` in another phase.
    pub(crate) fn battle_status(game: &Game, me_: Option<PlayerId>, phase: Phase) -> Option<GameStatus> {
        let g = game;
        Some(match phase {
            Phase::BeginningOfBattle => Self::host_status("You may proceed to the first battle when ready.", "Waiting for the host to proceed to the first battle...", None),

            Phase::ClaimingBattle => {
                let a = express!("You may now decide who will fight in ", g.battle.battle_about_to_start.as_ref().expect("battle about to start").territory);
                let battle = g.battle.battle_about_to_start.clone().expect("battle about to start");
                let f: Option<Option<Faction>> = if g.has_low_threshold(Faction::Ecaz) { battle.opponent_of_faction(g, Faction::Ecaz).map(|p| g.player(p).faction) } else { Some(Some(Faction::Ecaz)) };
                let w = express!(f, " are deciding who will fight in ", battle.territory, "...");
                let p = if g.has_low_threshold(Faction::Ecaz) { battle.opponent_of_faction(g, Faction::Ecaz) } else { g.get_player(Some(Faction::Ecaz)) };
                Self::player_status(a, w, p, None)
            }

            Phase::Thought => {
                let a = express!(g.battle.current_thought.as_ref().expect("current thought").initiator(), " asked you a question and are waiting for your answer.");
                let battle = g.battle.current_battle.clone().expect("current battle");
                let thought_initiator = g.battle.current_thought.as_ref().expect("current thought").initiator();
                let w = express!("Waiting for ", battle.opponent_of_faction(g, thought_initiator).map(|p| g.player(p).faction), " to answer a question...");
                let battle = g.battle.current_battle.clone().expect("current battle");
                let p = battle.opponent_of_faction(g, g.battle.current_thought.as_ref().expect("current thought").initiator());
                Self::player_status(a, w, p, None)
            }

            Phase::BattlePhase => Self::determine_battle_status(game, me_),

            Phase::MeltingRock => {
                let a = express!("Please decide how to use your ", TreacheryCardType::Rockmelter, ".");
                let w = express!("Waiting for a decision on how they wish to use their ", TreacheryCardType::Rockmelter, "...");
                let p = if g.battle.aggressor_plan.as_ref().expect("aggressor plan").has_rock_melter() {
                    Some(g.battle.current_battle.as_ref().expect("current battle").player())
                } else {
                    g.battle.current_battle.as_ref().expect("current battle").defending_player(g)
                };
                Self::player_status(a, w, p, None)
            }

            Phase::CallTraitorOrPass => {
                let players = Self::players_that_need_to_call_traitor(game);
                Self::players_status("You may now call TREACHERY if the enemy leader is a traitor under your command.", "Waiting for a faction to call TREACHERY...", players, None)
            }

            Phase::CancellingTraitor => Self::host_status("You may proceed when done waiting for players to cancel a traitor call.", "Players may now try to cancel a traitor call...", None),

            Phase::Retreating => {
                let a = express!("Please decide about retreating forces from ", g.battle.current_battle.as_ref().expect("current battle").territory, ".");
                let w = express!(g.battle.battle_loser, " are thinking about retreating forces from ", g.battle.current_battle.as_ref().expect("current battle").territory, ".");
                Self::faction_status(g, a, w, g.battle.battle_loser.flatten(), None)
            }

            Phase::AvoidingAudit => {
                let a = express!("Please decide if you wish to avoid being audited.");
                let w = express!(g.player(g.auditee().expect("auditee")).faction, " are thinking about avoiding a scheduled audit...");
                let p = g.auditee();
                Self::player_status(a, w, p, None)
            }

            Phase::CaptureDecision => {
                let a = express!("Please decide if you wish to capture a leader.");
                let w = express!(Faction::Harkonnen, " are deciding about capturing a leader...");
                Self::faction_status(game, a, w, Some(Faction::Harkonnen), None)
            }

            Phase::Auditing => {
                let a = express!("Please conclude the audit when done inspecting your opponent's cards.");
                let w = express!(Faction::Choam, " are performing their audit...");
                Self::faction_status(game, a, w, Some(Faction::Choam), None)
            }

            Phase::BattleConclusion => {
                let a = express!("You won!");
                let w = express!(game.battle.battle_winner, " are celebrating their victory in battle...");
                let f = game.battle.battle_winner;
                Self::faction_status(game, a, w, f.flatten(), None)
            }

            Phase::RevealingFacedancer => {
                let a = express!("You may reveal a leader to be one of your face dancers.");
                let w = express!("Waiting for ", Faction::Tleilaxu, " to reveal a face dancer...");
                Self::faction_status(game, a, w, Some(Faction::Tleilaxu), None)
            }

            Phase::Facedancing => {
                let a = express!("You may now replace opponent forces with your own.");
                let w = express!("Waiting for ", Faction::Tleilaxu, " to replace forces...");
                Self::faction_status(game, a, w, Some(Faction::Tleilaxu), None)
            }

            Phase::BattleReport if game.next_player_to_battle().is_some() => {
                Self::status(express!("Factions may now review the battle report before the next battle begins..."), None)
            }

            Phase::BattleReport if game.next_player_to_battle().is_none() => Self::status(express!(game.turn.current_main_phase, " phase ended."), None),

            Phase::ReplacingFaceDancer => Self::faction_status(
                game,
                "You may replace an unrevealed Face Dancer with a new one from the Traitor Deck.",
                "Waiting for a Face Dancer to be replaced...",
                Some(Faction::Tleilaxu),
                None,
            ),

            _ => return None,
        })
    }
}

impl Game {
    /// What `player` may do in the battle phase (see `add_player_actions`).
    pub(crate) fn add_battle_actions(&self, player: PlayerId, _is_host: bool, result: &mut Vec<EventType>) {
        let faction = self.player(player).faction;
        match self.phase() {
            Phase::BattlePhase => {
                if self.battle.current_battle.is_none() && Some(player) == self.next_player_to_battle() {
                    result.push(EventType::BattleInitiated);
                }

                if self.battle.current_battle.is_some() && SwitchedSkilledLeader::can_be_played(self, player) {
                    result.push(EventType::SwitchedSkilledLeader);
                }

                if HMSAdvantageChosen::can_be_played(self, player) {
                    result.push(EventType::HMSAdvantageChosen);
                }

                if Voice::may_use_voice(self, player) {
                    result.push(EventType::Voice);
                }

                if VoiceDeclined::may_decline(self, player) {
                    result.push(EventType::VoiceDeclined);
                }

                if Thought::may_be_used(self, player) {
                    result.push(EventType::Thought);
                }

                if Prescience::may_use_prescience(self, player) {
                    result.push(EventType::Prescience);
                }

                if let Some(cb) = self.battle.current_battle.clone() {
                    if faction == cb.aggressor(self) && self.battle.aggressor_plan.is_none() {
                        result.push(EventType::Battle);
                    } else if faction == cb.defender(self) && self.battle.defender_plan.is_none() {
                        result.push(EventType::Battle);
                    }

                    if faction == cb.aggressor(self) && self.battle.aggressor_plan.is_some() {
                        result.push(EventType::BattleRevision);
                    } else if faction == cb.defender(self) && self.battle.defender_plan.is_some() {
                        result.push(EventType::BattleRevision);
                    }

                    if ResidualPlayed::may_play(self, player) {
                        result.push(EventType::ResidualPlayed);
                    }
                }

            }

            Phase::ClaimingBattle => {
                if self.has_low_threshold(Faction::Ecaz) {
                    let opponent = match &self.battle.battle_about_to_start {
                        Some(b) => b.opponent_of_faction(self, Faction::Ecaz),
                        None => None,
                    };
                    if opponent.is_some_and(|o| self.player(o).is(faction)) {
                        result.push(EventType::BattleClaimed);
                    }
                } else if faction == Some(Faction::Ecaz) {
                    result.push(EventType::BattleClaimed);
                }
            }

            Phase::CallTraitorOrPass => {
                if let (Some(cb), Some(ap), Some(dp)) = (&self.battle.current_battle, &self.battle.aggressor_plan, &self.battle.defender_plan) {
                    let traitors = &self.player(player).traitors;
                    let ally_of = |f: Faction| self.get_player(Some(f)).map(|p| self.player(p).ally);
                    if (self.battle.aggressor_traitor_action.is_none() && faction == cb.aggressor(self))
                        || (self.battle.aggressor_traitor_action.is_none()
                            && faction == Some(Faction::Harkonnen)
                            && ally_of(ap.initiator().expect("faction")) == Some(Some(Faction::Harkonnen))
                            && dp.hero.is_some()
                            && traitors.contains(&dp.hero.expect("hero")))
                        || (self.battle.defender_traitor_action.is_none() && faction == cb.defender(self))
                        || (self.battle.defender_traitor_action.is_none()
                            && faction == Some(Faction::Harkonnen)
                            && ally_of(dp.initiator().expect("faction")) == Some(Some(Faction::Harkonnen))
                            && ap.hero.is_some()
                            && traitors.contains(&ap.hero.expect("hero")))
                    {
                        result.push(EventType::TreacheryCalled);
                    }
                }

                if let Some(ap) = &self.battle.aggressor_plan {
                    if faction == ap.initiator()
                        && ap.weapon().is_some_and(|w| w.type_() == TreacheryCardType::PoisonTooth)
                        && !self.battle.poison_tooth_cancelled
                    {
                        result.push(EventType::PoisonToothCancelled);
                    }
                }

                if let Some(dp) = &self.battle.defender_plan {
                    if faction == dp.initiator()
                        && dp.weapon().is_some_and(|w| w.type_() == TreacheryCardType::PoisonTooth)
                        && !self.battle.poison_tooth_cancelled
                    {
                        result.push(EventType::PoisonToothCancelled);
                    }
                }

                if PortableAntidoteUsed::can_be_played(self, player) {
                    result.push(EventType::PortableAntidoteUsed);
                }

                if Diplomacy::can_be_played(self, player) {
                    result.push(EventType::Diplomacy);
                }
            }

            Phase::Retreating => {
                if Some(faction) == self.battle.battle_loser {
                    result.push(EventType::Retreat);
                }
            }

            Phase::MeltingRock => {
                if RockWasMelted::can_be_played(self, player) {
                    result.push(EventType::RockWasMelted);
                }
            }

            Phase::CaptureDecision => {
                if faction == Some(Faction::Harkonnen) {
                    result.push(EventType::Captured);
                }
            }

            Phase::BattleConclusion => {
                if Some(faction) == self.battle.battle_winner && !self.battle.battle_was_concluded_by_winner {
                    result.push(EventType::BattleConcluded);
                }
                if LoserConcluded::is_applicable(self, Some(player)) {
                    result.push(EventType::LoserConcluded);
                }
            }

            Phase::AvoidingAudit => {
                if Some(player) == self.auditee() {
                    result.push(EventType::AuditCancelled);
                }
            }

            Phase::Auditing => {
                if faction == Some(Faction::Choam) {
                    result.push(EventType::Audited);
                }
            }

            Phase::RevealingFacedancer => {
                if faction == Some(Faction::Tleilaxu) {
                    result.push(EventType::FaceDancerRevealed);
                }
            }

            Phase::Facedancing => {
                if faction == Some(Faction::Tleilaxu) {
                    result.push(EventType::FaceDanced);
                }
            }

            Phase::BattleReport => {
                // NextPlayerToBattle mutates BattleSequence; only read when the
                // preceding conditions hold.
            }

            Phase::ReplacingFaceDancer => {
                if faction == Some(Faction::Tleilaxu) {
                    result.push(EventType::FaceDancerReplaced);
                }
            }

            Phase::Thought => {
                // `CurrentBattle?.OpponentOf(CurrentThought!.Initiator)`: the argument (and its
                // NullReferenceException) is only evaluated when CurrentBattle is not null.
                let opponent = match &self.battle.current_battle {
                    Some(cb) => cb.opponent_of_faction(self, self.battle.current_thought.as_ref().expect("current thought").initiator()),
                    None => None,
                };
                if Some(player) == opponent {
                    result.push(EventType::ThoughtAnswered);
                }
            }

            _ => {}
        }
    }
}
