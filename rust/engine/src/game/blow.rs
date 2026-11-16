use crate::map::discovery::Discovery;
use crate::log::blow::{
    BlowCardLog, BlowDeckLog, BlowReshuffleLog, WormAppearedLog, WormDevouredForcesLog, WormDevouredSpiceLog, WormIgnoredLog,
    WormSurvivedLog,
};
use crate::prelude::*;
use crate::messages::part::Part;

/// One faction's vote in a Nexus (`NexusVoted`): `passed` is a vote against alliances.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NexusVote {
    pub initiator: Option<Faction>,
    pub passed: bool,
}

/// The spice blow phase (`PhaseState::Blow`): its step, the cards set aside on the first turn,
/// the Thumper, and the Nexus (votes, and who may draw a Nexus card). It is dropped when the
/// phase ends. What outlives it is on the board: the latest spice cards, the worms and the
/// sandtrout.
pub struct BlowState {
    pub step: BlowStep,
    pub ignored_monsters: Vec<ResourceCard>,
    pub ignored_sandtrout: Option<ResourceCard>,
    pub thumper_called_shai_hulud: bool,
    pub nexus_votes: Vec<NexusVote>,
    pub nexus_has_occurred: bool,
    pub factions_that_may_draw_nexus_card: Vec<Faction>,
    pub factions_that_drew_nexus_card: Vec<Faction>,
}

impl BlowState {
    pub fn new(step: BlowStep) -> Self {
        BlowState {
            step,
            ignored_monsters: Vec::new(),
            ignored_sandtrout: None,
            thumper_called_shai_hulud: false,
            nexus_votes: Vec::new(),
            nexus_has_occurred: false,
            factions_that_may_draw_nexus_card: Vec::new(),
            factions_that_drew_nexus_card: Vec::new(),
        }
    }
}

/// Where the spice blow phase is. A and B are the two spice decks (B only with
/// `IncreasedResourceFlow`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlowStep {
    Thumper,
    BlowA,
    HarvesterA,
    BlowB,
    HarvesterB,
    FremenSendingMonsterA,
    VoteAllianceA,
    AllianceA,
    FremenRidingMonsterA,
    FremenSendingMonsterB,
    VoteAllianceB,
    AllianceB,
    FremenRidingMonsterB,
    NexusCards,
    Report,
}

impl BlowStep {
    pub fn phase(self) -> Phase {
        match self {
            BlowStep::Thumper => Phase::Thumper,
            BlowStep::BlowA => Phase::BlowA,
            BlowStep::HarvesterA => Phase::HarvesterA,
            BlowStep::BlowB => Phase::BlowB,
            BlowStep::HarvesterB => Phase::HarvesterB,
            BlowStep::FremenSendingMonsterA => Phase::FremenSendingMonsterA,
            BlowStep::VoteAllianceA => Phase::VoteAllianceA,
            BlowStep::AllianceA => Phase::AllianceA,
            BlowStep::FremenRidingMonsterA => Phase::FremenRidingMonsterA,
            BlowStep::FremenSendingMonsterB => Phase::FremenSendingMonsterB,
            BlowStep::VoteAllianceB => Phase::VoteAllianceB,
            BlowStep::AllianceB => Phase::AllianceB,
            BlowStep::FremenRidingMonsterB => Phase::FremenRidingMonsterB,
            BlowStep::NexusCards => Phase::NexusCards,
            BlowStep::Report => Phase::BlowReport,
        }
    }

    pub fn of(phase: Phase) -> Option<BlowStep> {
        let step = match phase {
            Phase::Thumper => BlowStep::Thumper,
            Phase::BlowA => BlowStep::BlowA,
            Phase::HarvesterA => BlowStep::HarvesterA,
            Phase::BlowB => BlowStep::BlowB,
            Phase::HarvesterB => BlowStep::HarvesterB,
            Phase::FremenSendingMonsterA => BlowStep::FremenSendingMonsterA,
            Phase::VoteAllianceA => BlowStep::VoteAllianceA,
            Phase::AllianceA => BlowStep::AllianceA,
            Phase::FremenRidingMonsterA => BlowStep::FremenRidingMonsterA,
            Phase::FremenSendingMonsterB => BlowStep::FremenSendingMonsterB,
            Phase::VoteAllianceB => BlowStep::VoteAllianceB,
            Phase::AllianceB => BlowStep::AllianceB,
            Phase::FremenRidingMonsterB => BlowStep::FremenRidingMonsterB,
            Phase::NexusCards => BlowStep::NexusCards,
            Phase::BlowReport => BlowStep::Report,
            _ => return None,
        };
        Some(step)
    }

    /// The step for deck A if `a`, otherwise for deck B: `either(a, BlowStep::AllianceA, BlowStep::AllianceB)`.
    pub fn either(a: bool, step_a: BlowStep, step_b: BlowStep) -> BlowStep {
        if a { step_a } else { step_b }
    }
}

/// `params object?[]` element list for `Log(...)`.
macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(Part::from($e)),*] };
}

impl Game {
    /// The spice blow phase's state, while the game is in it (an interrupt may suspend it).
    pub fn blow(&self) -> Option<&BlowState> {
        match &self.phase {
            PhaseState::Blow(b) => Some(b),
            _ => None,
        }
    }

    pub fn blow_mut(&mut self) -> Option<&mut BlowState> {
        match &mut self.phase {
            PhaseState::Blow(b) => Some(b),
            _ => None,
        }
    }

    /// The spice blow phase's state, for code that only runs in the spice blow phase.
    pub(crate) fn blow_state(&mut self) -> &mut BlowState {
        self.blow_mut().expect("the spice blow phase")
    }

    /// Moves the spice blow phase to `step`, dropping any interrupt, as entering a phase did.
    /// Starts the phase if the game isn't in it yet.
    pub fn enter_blow(&mut self, step: BlowStep) {
        match self.blow_mut() {
            Some(b) => {
                b.step = step;
                self.interrupts.clear();
                self.remove_ended_deals(step.phase());
            }
            None => self.enter_state(PhaseState::Blow(BlowState::new(step))),
        }
    }

    /// Back to the spice blow step under the interrupts (a worm ride's intrusions), without
    /// ending any deals.
    fn resume_blow(&mut self, step: BlowStep) {
        assert_eq!(self.blow().map(|b| b.step), Some(step), "resuming the spice blow phase");
        self.interrupts.clear();
    }

    pub fn enter_spice_blow_phase(&mut self) {
        self.main_phase_start(MainPhase::Blow, true);
        self.turn.has_acted_or_passed.clear();

        // The new state has no ignored worms or sandtrout yet.
        self.enter_blow(BlowStep::Thumper);
    }

    pub fn draw_resource_card(&mut self) -> GameResult {
        let mut drawn: Option<ResourceCard> = None;
        loop {
            // while (ThumperCalledShaiHulud || !(drawn = DrawAndDiscardResourceCard(CurrentDiscardPile!)).IsSpiceBlow)
            let condition = if self.blow_state().thumper_called_shai_hulud {
                true
            } else {
                let d = self.draw_and_discard_resource_card(self.phase() == Phase::BlowA);
                drawn = Some(d);
                !d.is_spice_blow()
            };
            if !condition {
                break;
            }

            if drawn.is_some_and(|d| d.is_shai_hulud() || d.is_great_maker()) && self.turn.current_turn == Turn::FIRST {
                self.log_ignored_monster(drawn.expect("drawn").is_great_maker());
                let c = self.current_discard_pile_mut().expect("current discard pile").draw();
                self.blow_state().ignored_monsters.push(c);
            } else if self.blow_state().thumper_called_shai_hulud
                || drawn.is_some_and(|d| d.is_shai_hulud())
                || drawn.is_some_and(|d| d.is_great_maker())
            {
                if self.monster_drawn(drawn)? {
                    // C# parity: this break skips the `ThumperCalledShaiHulud = false` below.
                    break;
                }
            } else if let Some(d) = drawn.filter(|d| d.is_sand_trout()) {
                self.sand_trout_drawn(d);
            }

            self.blow_state().thumper_called_shai_hulud = false;
        }

        if self.phase() != Phase::FremenSendingMonsterA && self.phase() != Phase::FremenSendingMonsterB {
            if let Some(d) = drawn {
                self.set_previous_blow_card(Some(d));
                self.process_blow_card(d)?;
            }
        }
        Ok(())
    }

    /// A worm drawn, or called by a Thumper (`drawn` is then `None`): it appears, unless a
    /// sandtrout cancels it. True if Fremen are to send it, which stops the drawing.
    fn monster_drawn(&mut self, drawn: Option<ResourceCard>) -> GameResult<bool> {
        if !self.blow_state().thumper_called_shai_hulud {
            self.stone(if drawn.is_some_and(|d| d.is_shai_hulud()) { Milestone::Monster } else { Milestone::GreatMonster });
        }

        if !self.sand_trout_occurred() {
            self.board.sand_trout_doubles_resources = false;
            self.board.number_of_monsters += 1;
            let t = self.previous_monster_territory();
            let is_great = !self.blow_state().thumper_called_shai_hulud && drawn.is_some_and(|d| d.is_great_maker());
            self.let_monster_appear(t, is_great)?;
            return Ok(matches!(self.phase(), Phase::FremenSendingMonsterA | Phase::FremenSendingMonsterB));
        }

        //Sandtrout triggers
        if let Some(sand_trout) = self.board.sand_trout {
            let pile = self.current_discard_pile_mut().expect("current discard pile");
            if let Some(d) = drawn {
                if let Some(i) = pile.items.iter().position(|c| *c == d) {
                    pile.items.remove(i);
                }
            }
            pile.put_on_top(sand_trout);
            if let Some(d) = drawn {
                pile.put_on_top(d);
            }
        }

        self.board.sand_trout = None;
        self.board.sand_trout_doubles_resources = true;
        self.log(elems![Concept::Monster, " is ignored due to ", Concept::BabyMonster]);
        Ok(false)
    }

    /// A sandtrout drawn: it cancels all alliances and waits for the next worm, or on the first
    /// turn is set aside.
    fn sand_trout_drawn(&mut self, d: ResourceCard) {
        if self.turn.current_turn > Turn::FIRST {
            self.stone(Milestone::BabyMonster);
            self.log(elems![Concept::BabyMonster, " detected! All alliances are cancelled."]);
            self.cancel_all_alliances();
            let pile = self.current_discard_pile_mut().expect("current discard pile");
            if let Some(i) = pile.items.iter().position(|c| *c == d) {
                pile.items.remove(i);
            }
            self.board.sand_trout = Some(d);
        } else {
            self.log(elems![Concept::BabyMonster, " on turn 1 was ignored"]);
            let c = self.current_discard_pile_mut().expect("current discard pile").draw();
            self.blow_state().ignored_sandtrout = Some(c);
        }
    }

    fn previous_monster_territory(&self) -> Option<Territory> {
        match self.previous_blow_card() {
            None => None,
            Some(c) if c.is_shai_hulud() || c.is_great_maker() => None,
            Some(c) => c.territory(),
        }
    }

    pub fn log_ignored_monster(&mut self, is_great_monster: bool) {
        if is_great_monster {
            self.log(elems![Concept::GreatMonster, " on turn 1 was ignored"]);
        } else {
            let msg = express!(Concept::Monster, " on turn 1 was ignored").with_log(WormIgnoredLog::default());
            self.log(elems![msg]);
        }
    }

    pub fn cancel_all_alliances(&mut self) {
        for i in 0..self.players.len() {
            let p = &self.players[i];
            if p.ally != None {
                let f = p.faction;
                self.break_alliance(f.expect("faction"));
            }
        }
    }

    /// Discards to the current discard pile; `discard_pile_a` says which pile that is
    /// (`ResourceCardDiscardPileA` when true). Nothing in the body changes the phase.
    pub fn draw_and_discard_resource_card(&mut self, discard_pile_a: bool) -> ResourceCard {
        if self.decks.resource_card_deck.as_ref().expect("resource card deck").is_empty() {
            self.reshuffle_resource_deck();
        }

        let drawn = self.decks.resource_card_deck.as_mut().expect("resource card deck").draw();
        let pile = if discard_pile_a { &mut self.decks.resource_card_discard_pile_a } else { &mut self.decks.resource_card_discard_pile_b };
        pile.as_mut().expect("pile").put_on_top(drawn);
        drawn
    }

    pub fn reshuffle_resource_deck(&mut self) {
        if self.applicable(Rule::IncreasedResourceFlow) {
            let count_a = self.decks.resource_card_discard_pile_a.as_ref().expect("resource card discard pile a").items.len() as i32;
            let count_b = self.decks.resource_card_discard_pile_b.as_ref().expect("resource card discard pile b").items.len() as i32;
            self.log(elems![count_a + count_b, " cards were shuffled from ", Concept::Resource, " discard piles A and B into a new deck."]);
        } else {
            let count_a = self.decks.resource_card_discard_pile_a.as_ref().expect("resource card discard pile a").items.len() as i32;
            let msg = express!(count_a, " cards were shuffled from the ", Concept::Resource, " discard pile into a new deck")
                .with_log(BlowReshuffleLog::new(count_a));
            self.log(elems![msg]);
        }

        if self.decks.resource_card_discard_pile_a.as_ref().expect("resource card discard pile a").items.iter().any(|c| c.is_discovery())
            || self.decks.resource_card_discard_pile_b.as_ref().expect("resource card discard pile b").items.iter().any(|c| c.is_discovery())
        {
            self.log(elems!["Discovery cards were removed from the game"]);
        }

        // foreach over a lazy Where: the predicate runs on each card just before its Add, so a
        // card that throws leaves the earlier ones added.
        let count = self.decks.resource_card_discard_pile_a.as_ref().expect("resource card discard pile a").items.len();
        for idx in 0..count {
            let i = self.decks.resource_card_discard_pile_a.as_ref().expect("resource card discard pile a").items[idx];
            if !i.is_discovery() {
                self.decks.resource_card_deck.as_mut().expect("resource card deck").items.push(i);
            }
        }

        self.decks.resource_card_discard_pile_a.as_mut().expect("resource card discard pile a").clear();

        // foreach over a lazy Where: the predicate runs on each card just before its Add, so a
        // card that throws leaves the earlier ones added.
        let count = self.decks.resource_card_discard_pile_b.as_ref().expect("resource card discard pile b").items.len();
        for idx in 0..count {
            let i = self.decks.resource_card_discard_pile_b.as_ref().expect("resource card discard pile b").items[idx];
            if !i.is_discovery() {
                self.decks.resource_card_deck.as_mut().expect("resource card deck").items.push(i);
            }
        }

        self.decks.resource_card_discard_pile_b.as_mut().expect("resource card discard pile b").clear();

        self.decks.resource_card_deck.as_mut().expect("resource card deck").shuffle(&mut self.rng);
        self.stone(Milestone::Shuffled);
    }

    pub fn process_blow_card(&mut self, blow_card: ResourceCard) -> GameResult {
        self.stone(Milestone::Resource);

        if blow_card.is_discovery() {
            let territory = blow_card.territory().expect("territory");
            if self.any_forces_in(territory) {
                self.kill_all_forces_in(territory, false)?;
            }

            let devoured_resources = self.remove_resources_territory(territory);
            self.log_if(devoured_resources > 0, elems![Payment::of(devoured_resources), " in ", territory, " is destroyed"]);

            let mut drawn_token = None;
            let discovery_location = blow_card.discovery_location().expect("discovery location");
            if discovery_location.discovery_token_type() == Some(DiscoveryTokenType::Guild) {
                if !self.decks.guild_discovery_tokens.as_ref().expect("guild discovery tokens").is_empty() {
                    let token = self.decks.guild_discovery_tokens.as_mut().expect("guild discovery tokens").draw();
                    drawn_token = Some(token);
                    self.board.discoveries_on_planet.insert_new(discovery_location, Discovery::new(token, DiscoveryTokenType::Guild, discovery_location));
                } else {
                    self.log(elems!["There are no more ", DiscoveryTokenType::Guild, " discoveries left"]);
                }
            } else if discovery_location.discovery_token_type() == Some(DiscoveryTokenType::Fremen) {
                if !self.decks.fremen_discovery_tokens.as_ref().expect("fremen discovery tokens").is_empty() {
                    let token = self.decks.fremen_discovery_tokens.as_mut().expect("fremen discovery tokens").draw();
                    drawn_token = Some(token);
                    self.board.discoveries_on_planet.insert_new(discovery_location, Discovery::new(token, DiscoveryTokenType::Fremen, discovery_location));
                } else {
                    self.log(elems!["There are no more ", DiscoveryTokenType::Fremen, " discoveries left"]);
                }
            }

            if drawn_token != None {
                self.stone(Milestone::DiscoveryAppeared);
                self.log(elems!["A ", discovery_location.discovery_token_type(), " discovery awaits in ", discovery_location.territory(), "..."]);
            }
        }

        let spice_factor = if self.board.sand_trout_doubles_resources { 2 } else { 1 };
        let location = blow_card.location().expect("location");
        let spice_amount = spice_factor * location.spice_blow_amount();

        if !Sector::in_storm(location.sector(&self.board.map), self.board.sector_in_storm) {
            let msg = express!(Payment::of(spice_amount), " detected in ", location, Game::sandtrout_message(self.board.sand_trout_doubles_resources))
                .with_log(BlowCardLog::new(blow_card, location, spice_amount, false, self.board.sand_trout_doubles_resources));
            self.log(elems![msg]);
            self.board.sand_trout_doubles_resources = false;
            self.change_resources_on_planet(location, spice_amount)?;
        } else {
            // C# parity: SandTroutDoublesResources is not reset when the doubled blow is lost
            // to the storm.
            let msg = express!(Payment::of(spice_amount), " in ", location, " is lost in the storm")
                .with_log(BlowCardLog::new(blow_card, location, spice_amount, true, false));
            self.log(elems![msg]);
        }

        if self.applicable(Rule::ExpansionTreacheryCardsExceptPBandSSandAmal) {
            self.enter_blow(BlowStep::either(self.phase() == Phase::BlowA, BlowStep::HarvesterA, BlowStep::HarvesterB));
            Ok(())
        } else {
            self.move_to_next_phase_after_resource_blow()
        }
    }

    pub fn sandtrout_message(sand_trout_doubles_resources: bool) -> MessagePart {
        express_part_if!(sand_trout_doubles_resources; ", doubled by ", Concept::BabyMonster)
    }

    pub fn move_to_next_phase_after_resource_blow(&mut self) -> GameResult {
        if self.board.monsters.is_empty() {
            // Static `Enter(bool, Action, Action)`, inlined.
            if (self.phase() == Phase::BlowA || self.phase() == Phase::HarvesterA) && self.applicable(Rule::IncreasedResourceFlow) {
                self.enter_blow_b()?;
            } else {
                self.start_nexus_card_phase();
            }
        } else {
            self.alliances.current_alliance_offers.clear();

            if self.applicable(Rule::BreakAlliancesOnAlliancePhase) {
                self.break_all_alliances();
            }

            let is_a = matches!(self.phase(), Phase::BlowA | Phase::HarvesterA);
            if self.board.monsters.len() == 1 && self.board.monsters[0].is_great_monster {
                self.blow_state().nexus_votes.clear();
                self.enter_blow(BlowStep::either(is_a, BlowStep::VoteAllianceA, BlowStep::VoteAllianceB));
            } else {
                self.enter_alliance_phase(is_a);
            }
        }
        Ok(())
    }

    pub fn let_monster_appear(&mut self, t: Option<Territory>, is_great_monster: bool) -> GameResult {
        match t {
            Some(t) if self.turn.current_turn != Turn::FIRST => {
                let m = MonsterAppearance::new(t, is_great_monster);

                let appeared = WormAppearedLog::new(m.territory, (self.board.monsters.len() as i32) + 1);
                if m.is_great_monster {
                    self.log(elems![Concept::GreatMonster, " appears in ", m.territory]);
                } else if !self.board.monsters.is_empty() {
                    let msg = express!(
                        Concept::Monster,
                        " appears a ",
                        Game::natural((self.board.monsters.len() as i32) + 1),
                        " time during this ",
                        MainPhase::Blow
                    )
                    .with_log(appeared);
                    self.log(elems![msg]);
                } else {
                    let msg = express!(Concept::Monster, " appears in ", m.territory).with_log(appeared);
                    self.log(elems![msg]);
                }

                if !self.board.monsters.is_empty() {
                    if self.is_playing(Faction::Fremen) && self.applicable(Rule::FremenSendingMonster) {
                        if !self.prevented(FactionAdvantage::FremenControlsMonster) {
                            let condition = self.phase() == Phase::BlowA;
                            self.enter_blow(BlowStep::either(condition, BlowStep::FremenSendingMonsterA, BlowStep::FremenSendingMonsterB));
                        } else {
                            self.board.monsters.push(m);
                            self.log_prevention_by_karma(FactionAdvantage::FremenControlsMonster);
                            if !self.applicable(Rule::FullPhaseKarma) {
                                self.allow(FactionAdvantage::FremenControlsMonster);
                            }
                        }
                    } else {
                        self.board.monsters.push(m);
                    }
                } else if self.board.monsters.is_empty() {
                    self.board.monsters.push(m.clone());
                    self.perform_monster(&m)?;
                }
            }
            _ => self.log_ignored_monster(is_great_monster),
        }
        Ok(())
    }

    pub fn perform_monster(&mut self, m: &MonsterAppearance) -> GameResult {
        for &l in m.territory.locations() {
            for p in 0..self.players.len() {
                let p = self.player_at(p);
                if self.player(p).any_forces_in(Some(l)) > 0 {
                    if !self.protected_from_monster(p) {
                        self.reveal_current_no_field(p, None)?;

                        let faction = self.player(p).faction;
                        let mut devoured =
                            express!(m.describing_concept(), " devours ", self.player(p).any_forces_in(Some(l)), faction, " forces in ", l);
                        if !m.is_great_monster {
                            devoured = devoured.with_log(WormDevouredForcesLog::new(
                                faction.expect("faction"),
                                self.player(p).forces_in(Some(l)),
                                self.player(p).special_forces_in(Some(l)),
                                l,
                            ));
                        }
                        self.log(elems![devoured]);
                        Player::kill_all_forces(self, p, l, false)?;

                        if self.player(p).is(Faction::Fremen) {
                            if !self.applicable(Rule::FullPhaseKarma) {
                                self.allow(FactionAdvantage::FremenProtectedFromMonster);
                            }
                        } else if self.player(p).ally == Some(Faction::Fremen) && self.alliances.fremen_will_protect_from_monster {
                            if !self.applicable(Rule::FullPhaseKarma) {
                                self.allow(FactionAdvantage::FremenProtectedFromMonsterAlly);
                            }
                        }
                    } else {
                        let faction = self.player(p).faction;
                        let mut survived = express!(faction, " survive ", m.describing_concept(), " in ", l);
                        if !m.is_great_monster {
                            survived = survived.with_log(WormSurvivedLog::new(faction.expect("faction"), l));
                        }
                        self.log(elems![survived]);
                    }
                }
            }
        }

        let devoured_resources = self.remove_resources_territory(m.territory);
        if devoured_resources > 0 {
            let mut devoured = express!(m.describing_concept(), " devours ", Payment::of(devoured_resources), " in ", m.territory);
            if !m.is_great_monster {
                devoured = devoured.with_log(WormDevouredSpiceLog::new(devoured_resources, m.territory));
            }
            self.log(elems![devoured]);
        }

        self.flip_bene_gesserit_advisors_when_alone()?;
        Ok(())
    }

    pub fn enter_blow_a(&mut self) -> GameResult {
        self.board.monsters.clear();
        self.enter_blow(BlowStep::BlowA);
        if self.applicable(Rule::IncreasedResourceFlow) {
            let msg = express!("Deck A").as_heading(Concept::Resource).with_log(BlowDeckLog::new("A".to_string()));
            self.log(elems![msg]);
        }
        self.draw_resource_card()?;
        self.let_factions_discard_surplus_cards();
        Ok(())
    }

    pub fn enter_blow_b(&mut self) -> GameResult {
        self.board.monsters.clear();
        self.enter_blow(BlowStep::BlowB);
        let msg = express!("Deck B").as_heading(Concept::Resource).with_log(BlowDeckLog::new("B".to_string()));
        self.log(elems![msg]);
        self.draw_resource_card()?;
        self.let_factions_discard_surplus_cards();
        Ok(())
    }

    /// Ends a worm ride in `phase` (`FremenRidingMonsterA` or `B`), after any intrusion it caused.
    pub fn end_worm_ride_during_phase(&mut self, phase: Phase) -> GameResult {
        self.resume_blow(BlowStep::of(phase).expect("a worm ride step"));

        if FremenRidesMonster::is_applicable(self) {
            let condition = self.phase() == Phase::AllianceA || self.phase() == Phase::FremenRidingMonsterA;
            self.enter_blow(BlowStep::either(condition, BlowStep::FremenRidingMonsterA, BlowStep::FremenRidingMonsterB));
        } else if self.phase() == Phase::FremenRidingMonsterA && self.applicable(Rule::IncreasedResourceFlow) {
            self.enter_blow_b()?;
        } else {
            self.start_nexus_card_phase();
        }
        Ok(())
    }

    pub fn enter_alliance_phase(&mut self, is_a: bool) {
        // Every faction must pass or form an alliance before the phase ends.
        self.turn.has_acted_or_passed.clear();
        self.enter_blow(BlowStep::either(is_a, BlowStep::AllianceA, BlowStep::AllianceB));
    }

    pub fn end_nexus(&mut self) -> GameResult {
        self.blow_state().nexus_has_occurred = true;
        self.alliances.current_alliance_offers.clear();

        if FremenRidesMonster::is_applicable(self) {
            let condition = self.phase() == Phase::AllianceA;
            self.enter_blow(BlowStep::either(condition, BlowStep::FremenRidingMonsterA, BlowStep::FremenRidingMonsterB));
        } else {
            if self.phase() == Phase::AllianceA {
                let condition = self.applicable(Rule::IncreasedResourceFlow);
                if condition { self.enter_blow_b()? } else { self.start_nexus_card_phase() }
            } else if self.phase() == Phase::AllianceB {
                self.start_nexus_card_phase();
            }
        }
        Ok(())
    }

    pub fn end_nexus_if_everyone_is_done(&mut self) -> GameResult {
        if matches!(self.phase(), Phase::AllianceA | Phase::AllianceB) && self.everyone_acted_or_passed() {
            self.end_nexus()?;
        }
        Ok(())
    }

    pub fn start_nexus_card_phase(&mut self) {
        if self.blow_state().nexus_has_occurred
            && self.applicable(Rule::NexusCards)
            && self.players.iter().any(|p| p.has_ally())
            && self.players.iter().any(|p| !p.has_ally())
        {
            self.enter_nexus_card_phase();
        } else {
            self.end_blow_phase();
        }

        self.blow_state().nexus_has_occurred = false;
    }

    pub fn enter_nexus_card_phase(&mut self) {
        let may_draw = self.players.iter().filter(|p| !p.has_ally()).filter_map(|p| p.faction).collect();
        let blow = self.blow_state();
        blow.factions_that_may_draw_nexus_card = may_draw;
        blow.factions_that_drew_nexus_card.clear();
        self.enter_blow(BlowStep::NexusCards);
    }

    pub fn discard_nexus_card(&mut self, p: PlayerId) {
        let nexus = self.player(p).nexus;
        if nexus != None {
            let faction = self.player(p).faction;
            self.log(elems![faction, " discard the ", nexus, " Nexus Card"]);
            self.stone(Milestone::NexusPlayed);
            self.decks.nexus_discard_pile.push(nexus.expect("faction"));
            self.player_mut(p).nexus = None;
        }
    }

    pub fn end_blow_phase(&mut self) {
        self.blow_state().factions_that_may_draw_nexus_card.clear();
        self.turn.current_fremen_secret_ally = None;
        self.turn.has_acted_or_passed.clear();
        self.reshuffle_ignored_monsters();
        self.main_phase_end();
        self.enter_blow(BlowStep::Report);
    }

    pub fn reshuffle_ignored_monsters(&mut self) {
        let ignored = self.blow_state().ignored_monsters.clone();
        let ignored_sandtrout = self.blow_state().ignored_sandtrout;
        if !ignored.is_empty() || ignored_sandtrout.is_some() {
            if !ignored.is_empty() {
                self.log(elems![
                    ignored.len() as i32,
                    " ignored ",
                    Concept::Monster,
                    " cards were shuffled back into the ",
                    Concept::Resource,
                    " deck"
                ]);
                for c in ignored {
                    self.decks.resource_card_deck.as_mut().expect("resource card deck").items.push(c);
                }
            }

            if let Some(sandtrout) = ignored_sandtrout {
                self.log(elems![Concept::BabyMonster, " card was shuffled back into the ", Concept::Resource, " deck"]);
                self.decks.resource_card_deck.as_mut().expect("resource card deck").items.push(sandtrout);
            }

            self.decks.resource_card_deck.as_mut().expect("resource card deck").shuffle(&mut self.rng);
            self.stone(Milestone::Shuffled);
        }
    }

    pub fn sand_trout_occurred(&self) -> bool {
        self.board.sand_trout.is_some()
    }

    pub fn natural(count: i32) -> String {
        match count {
            1 => "first".to_string(),
            2 => "second".to_string(),
            3 => "third".to_string(),
            4 => "fourth".to_string(),
            5 => "fifth".to_string(),
            6 => "sixth".to_string(),
            7 => "seventh".to_string(),
            8 => "eighth".to_string(),
            9 => "ninth".to_string(),
            10 => "tenth".to_string(),
            _ => {
                let suffix = match (count % 10, count % 100) {
                    (_, 11..=13) => "th",
                    (1, _) => "st",
                    (2, _) => "nd",
                    (3, _) => "rd",
                    _ => "th",
                };
                format!("{count}{suffix}")
            }
        }
    }

    /// `CurrentDiscardPile`, for callers that change the pile.
    pub fn current_discard_pile_mut(&mut self) -> Option<&mut Deck<ResourceCard>> {
        if self.phase() == Phase::BlowA { self.decks.resource_card_discard_pile_a.as_mut() } else { self.decks.resource_card_discard_pile_b.as_mut() }
    }

    pub fn previous_blow_card(&self) -> Option<ResourceCard> {
        if self.phase() == Phase::BlowA { self.board.latest_spice_card_a } else { self.board.latest_spice_card_b }
    }

    pub fn set_previous_blow_card(&mut self, value: Option<ResourceCard>) {
        if self.phase() == Phase::BlowA {
            self.board.latest_spice_card_a = value;
        } else {
            self.board.latest_spice_card_b = value;
        }
    }

    pub fn protected_from_monster(&self, p: PlayerId) -> bool {
        if let Some(ally) = &self.turn.current_fremen_secret_ally {
            if ally.player_opt() == Some(p) {
                return true;
            }
        }
        if self.player(p).is(Faction::Fremen) {
            return !self.prevented(FactionAdvantage::FremenProtectedFromMonster);
        }
        self.player(p).ally == Some(Faction::Fremen) && self.alliances.fremen_will_protect_from_monster && !self.prevented(FactionAdvantage::FremenProtectedFromMonsterAlly)
    }
}

#[derive(Clone, Debug)]
pub struct MonsterAppearance {
    pub territory: Territory,
    pub is_great_monster: bool,
}

impl MonsterAppearance {
    pub fn new(territory: Territory, is_great_monster: bool) -> Self {
        MonsterAppearance { territory, is_great_monster }
    }

    pub fn describing_concept(&self) -> Concept {
        if self.is_great_monster { Concept::GreatMonster } else { Concept::Monster }
    }

}

impl GameStatus {
    /// The status in the spice blow phase, or `None` in another phase.
    pub(crate) fn blow_status(game: &Game, _me: Option<PlayerId>, phase: Phase) -> Option<GameStatus> {
        let g = game;
        Some(match phase {
            Phase::FremenSendingMonsterA | Phase::FremenSendingMonsterB => {
                let a = express!("Please decide where to let ", Concept::Monster, " appear.");
                let w = express!(Faction::Fremen, " are deciding where to let ", Concept::Monster, " appear...");
                Self::faction_status(game, a, w, Some(Faction::Fremen), None)
            }

            Phase::FremenRidingMonsterA | Phase::FremenRidingMonsterB => {
                let a = express!("Please decide where to travel with ", Concept::Monster, ".");
                let w = express!(Faction::Fremen, " are deciding where to travel with ", Concept::Monster, "...");
                Self::faction_status(game, a, w, Some(Faction::Fremen), None)
            }

            Phase::BlowA | Phase::HarvesterA => {
                let t = game.board.latest_spice_card_a.expect("latest spice card a").location().expect("location").territory();
                Self::status(express!("Factions may use a ", TreacheryCardType::Harvester, " to double the ", Concept::Resource, " blow in ", t, "..."), None)
            }

            Phase::BlowB | Phase::HarvesterB => {
                let t = game.board.latest_spice_card_b.expect("latest spice card b").location().expect("location").territory();
                Self::status(express!("Factions may use a ", TreacheryCardType::Harvester, " to double the ", Concept::Resource, " blow in ", t, "..."), None)
            }

            Phase::VoteAllianceA | Phase::VoteAllianceB => {
                let players = Self::players_that_have_not_acted_or_passed(game);
                Self::players_status("Please vote Yes or No to a Nexus.", "Factions are voting on a Nexus...", players, None)
            }

            Phase::AllianceA | Phase::AllianceB => {
                let players = Self::players_that_have_not_acted_or_passed(game);
                Self::players_status(
                    "You may now make or break an alliance.",
                    "Factions are making and breaking alliances...",
                    players,
                    None,
                )
            }

            Phase::NexusCards => {
                let players: Vec<Option<PlayerId>> = g.player_indices().filter(|p| NexusCardDrawn::applicable(g, *p)).map(Some).collect();
                Self::players_status("Do you wish to draw a Nexus card in case you have none or have your own faction?", "Factions are thinking about drawing Nexus cards...", players, None)
            }

            _ => return None,
        })
    }
}

impl Game {
    /// What `player` may do in the spice blow phase (see `add_player_actions`).
    pub(crate) fn add_blow_actions(&self, player: PlayerId, _is_host: bool, result: &mut Vec<EventType>) {
        let faction = self.player(player).faction;
        match self.phase() {
            Phase::Thumper => {
                if self.player(player).has_treachery_card_type(TreacheryCardType::Thumper) {
                    result.push(EventType::ThumperPlayed);
                }
            }

            Phase::VoteAllianceA | Phase::VoteAllianceB => {
                if self.blow().is_some_and(|b| b.nexus_votes.iter().all(|v| v.initiator != faction)) {
                    result.push(EventType::NexusVoted);
                }
            }

            Phase::HarvesterA | Phase::HarvesterB => {
                if self.player(player).has_treachery_card_type(TreacheryCardType::Harvester) {
                    result.push(EventType::HarvesterPlayed);
                }
            }

            Phase::FremenSendingMonsterA | Phase::FremenSendingMonsterB => {
                if faction == Some(Faction::Fremen) {
                    result.push(EventType::FremenSentMonster);
                }
            }

            Phase::FremenRidingMonsterA | Phase::FremenRidingMonsterB => {
                if faction == Some(Faction::Fremen) {
                    result.push(EventType::FremenRidesMonster);
                }
            }

            Phase::AllianceA | Phase::AllianceB => {
                if !self.turn.has_acted_or_passed.contains_opt(faction) {
                    if self.player(player).ally == None && self.players.len() > 1 {
                        result.push(EventType::AllianceOffered);
                    }
                    if self.player(player).ally != None {
                        result.push(EventType::AllianceBroken);
                    }
                    result.push(EventType::AlliancePassed);
                }
            }

            Phase::NexusCards => {
                if NexusCardDrawn::applicable(self, player) {
                    result.push(EventType::NexusCardDrawn);
                }
            }

            Phase::BlowReport => {}

            _ => {}
        }
    }
}
