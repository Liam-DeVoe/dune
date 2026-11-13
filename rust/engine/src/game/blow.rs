use crate::map::discovery::Discovery;
use crate::log::blow::{
    BlowCardLog, BlowDeckLog, BlowReshuffleLog, WormAppearedLog, WormDevouredForcesLog, WormDevouredSpiceLog, WormIgnoredLog,
    WormSurvivedLog,
};
use crate::prelude::*;

/// `params object?[]` element list for `Log(...)`.
macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(ExpressionElement::from($e)),*] };
}


impl Game {
    pub fn enter_spice_blow_phase(&mut self) {
        self.main_phase_start(MainPhase::Blow, true);
        self.ignored_monsters.clear();
        self.ignored_sandtrout = None;
        self.has_acted_or_passed.clear();

        self.enter(Phase::Thumper, true);
    }

    pub fn draw_resource_card(&mut self) {
        let mut drawn: Option<ResourceCard> = None;
        loop {
            // while (ThumperCalledShaiHulud || !(drawn = DrawAndDiscardResourceCard(CurrentDiscardPile!)).IsSpiceBlow)
            let condition = if self.thumper_called_shai_hulud {
                true
            } else {
                let d = self.draw_and_discard_resource_card(self.current_phase == Phase::BlowA);
                drawn = Some(d);
                !d.is_spice_blow()
            };
            if !condition {
                break;
            }

            if drawn.is_some_and(|d| d.is_shai_hulud() || d.is_great_maker()) && self.current_turn == 1 {
                self.log_ignored_monster(drawn.expect("drawn").is_great_maker());
                let c = self.current_discard_pile_mut().expect("current discard pile").draw();
                self.ignored_monsters.push(c);
            } else if self.thumper_called_shai_hulud
                || drawn.is_some_and(|d| d.is_shai_hulud())
                || drawn.is_some_and(|d| d.is_great_maker())
            {
                if !self.thumper_called_shai_hulud {
                    self.stone(if drawn.is_some_and(|d| d.is_shai_hulud()) { Milestone::Monster } else { Milestone::GreatMonster });
                }

                if !self.sand_trout_occured() {
                    self.sand_trout_doubles_resources = false;
                    self.number_of_monsters += 1;
                    let t = self.previous_monster_territory();
                    let is_great = !self.thumper_called_shai_hulud && drawn.is_some_and(|d| d.is_great_maker());
                    self.let_monster_appear(t, is_great);
                    // C# parity: this break skips the `ThumperCalledShaiHulud = false` below.
                    if matches!(self.current_phase, Phase::FremenSendingMonsterA | Phase::FremenSendingMonsterB) {
                        break;
                    }
                } else {
                    //Sandtrout triggers
                    if self.sand_trout.is_some() {
                        let sand_trout = self.sand_trout.expect("sand trout");
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

                    self.sand_trout = None;
                    self.sand_trout_doubles_resources = true;
                    self.log(elems![Concept::Monster, " is ignored due to ", Concept::BabyMonster]);
                }
            } else if drawn.is_some_and(|d| d.is_sand_trout()) {
                let d = drawn.expect("drawn");
                if self.current_turn > 1 {
                    self.stone(Milestone::BabyMonster);
                    self.log(elems![Concept::BabyMonster, " detected! All alliances are cancelled."]);
                    self.cancel_all_alliances();
                    let pile = self.current_discard_pile_mut().expect("current discard pile");
                    if let Some(i) = pile.items.iter().position(|c| *c == d) {
                        pile.items.remove(i);
                    }
                    self.sand_trout = Some(d);
                } else {
                    self.log(elems![Concept::BabyMonster, " on turn 1 was ignored"]);
                    self.ignored_sandtrout = Some(self.current_discard_pile_mut().expect("current discard pile").draw());
                }
            }

            self.thumper_called_shai_hulud = false;
        }

        if self.current_phase != Phase::FremenSendingMonsterA && self.current_phase != Phase::FremenSendingMonsterB {
            if let Some(d) = drawn {
                self.set_previous_blow_card(Some(d));
                self.process_blow_card(d);
            }
        }
    }

    /// `PreviousBlowCard == null || PreviousBlowCard.IsShaiHulud || PreviousBlowCard.IsGreatMaker ? null : PreviousBlowCard.Territory`
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
            if p.ally != Faction::None {
                let f = p.faction;
                self.break_alliance(f);
            }
        }
    }

    /// Discards to the current discard pile; `discard_pile_a` says which pile that is
    /// (`ResourceCardDiscardPileA` when true). Nothing in the body changes the phase.
    pub fn draw_and_discard_resource_card(&mut self, discard_pile_a: bool) -> ResourceCard {
        if self.resource_card_deck.as_ref().expect("resource card deck").is_empty() {
            self.reshuffle_resource_deck();
        }

        let drawn = self.resource_card_deck.as_mut().expect("resource card deck").draw();
        let pile = if discard_pile_a { &mut self.resource_card_discard_pile_a } else { &mut self.resource_card_discard_pile_b };
        pile.as_mut().expect("pile").put_on_top(drawn);
        drawn
    }

    pub fn reshuffle_resource_deck(&mut self) {
        if self.applicable(Rule::IncreasedResourceFlow) {
            let count_a = self.resource_card_discard_pile_a.as_ref().expect("resource card discard pile a").items.len() as i32;
            let count_b = self.resource_card_discard_pile_b.as_ref().expect("resource card discard pile b").items.len() as i32;
            // C# parity: "A en B" typo.
            self.log(elems![count_a + count_b, " cards were shuffled from ", Concept::Resource, " discard piles A en B into a new deck."]);
        } else {
            let count_a = self.resource_card_discard_pile_a.as_ref().expect("resource card discard pile a").items.len() as i32;
            let msg = express!(count_a, " cards were shuffled from the ", Concept::Resource, " discard pile into a new deck")
                .with_log(BlowReshuffleLog::new(count_a));
            self.log(elems![msg]);
        }

        if self.resource_card_discard_pile_a.as_ref().expect("resource card discard pile a").items.iter().any(|c| c.is_discovery())
            || self.resource_card_discard_pile_b.as_ref().expect("resource card discard pile b").items.iter().any(|c| c.is_discovery())
        {
            // C# parity: "Discovery were removed" grammar.
            self.log(elems!["Discovery were removed from the game"]);
        }

        // foreach over a lazy Where: the predicate runs on each card just before its Add, so a
        // card that throws leaves the earlier ones added (docs/lazy-queries.md).
        let count = self.resource_card_discard_pile_a.as_ref().expect("resource card discard pile a").items.len();
        for idx in 0..count {
            let i = self.resource_card_discard_pile_a.as_ref().expect("resource card discard pile a").items[idx];
            if !i.is_discovery() {
                self.resource_card_deck.as_mut().expect("resource card deck").items.push(i);
            }
        }

        self.resource_card_discard_pile_a.as_mut().expect("resource card discard pile a").clear();

        // foreach over a lazy Where: the predicate runs on each card just before its Add, so a
        // card that throws leaves the earlier ones added (docs/lazy-queries.md).
        let count = self.resource_card_discard_pile_b.as_ref().expect("resource card discard pile b").items.len();
        for idx in 0..count {
            let i = self.resource_card_discard_pile_b.as_ref().expect("resource card discard pile b").items[idx];
            if !i.is_discovery() {
                self.resource_card_deck.as_mut().expect("resource card deck").items.push(i);
            }
        }

        self.resource_card_discard_pile_b.as_mut().expect("resource card discard pile b").clear();

        self.resource_card_deck.as_mut().expect("resource card deck").shuffle();
        self.stone(Milestone::Shuffled);
    }

    pub fn process_blow_card(&mut self, blow_card: ResourceCard) {
        self.stone(Milestone::Resource);

        if blow_card.is_discovery() {
            let territory = blow_card.territory().expect("territory");
            if self.any_forces_in(territory) {
                self.kill_all_forces_in(territory, false);
            }

            let devoured_resources = self.remove_resources_territory(territory);
            self.log_if(devoured_resources > 0, elems![Payment::of(devoured_resources), " in ", territory, " is destroyed"]);

            let mut drawn_token = DiscoveryToken::None;
            let discovery_location = blow_card.discovery_location().expect("discovery location");
            if discovery_location.discovery_token_type() == DiscoveryTokenType::Guild {
                if !self.guild_discovery_tokens.as_ref().expect("guild discovery tokens").is_empty() {
                    drawn_token = self.guild_discovery_tokens.as_mut().expect("guild discovery tokens").draw();
                    self.discoveries_on_planet.insert_new(discovery_location, Discovery::new(drawn_token, DiscoveryTokenType::Guild, discovery_location));
                } else {
                    self.log(elems!["There are no more ", DiscoveryTokenType::Guild, " discoveries left"]);
                }
            } else if discovery_location.discovery_token_type() == DiscoveryTokenType::Fremen {
                if !self.fremen_discovery_tokens.as_ref().expect("fremen discovery tokens").is_empty() {
                    drawn_token = self.fremen_discovery_tokens.as_mut().expect("fremen discovery tokens").draw();
                    self.discoveries_on_planet.insert_new(discovery_location, Discovery::new(drawn_token, DiscoveryTokenType::Fremen, discovery_location));
                } else {
                    self.log(elems!["There are no more ", DiscoveryTokenType::Fremen, " discoveries left"]);
                }
            }

            if drawn_token != DiscoveryToken::None {
                self.stone(Milestone::DiscoveryAppeared);
                self.log(elems!["A ", discovery_location.discovery_token_type(), " discovery awaits in ", discovery_location.territory(), "..."]);
            }
        }

        let spice_factor = if self.sand_trout_doubles_resources { 2 } else { 1 };
        let location = blow_card.location().expect("location");
        let spice_amount = spice_factor * location.spice_blow_amount();

        if location.sector(&self.map) != self.sector_in_storm {
            let msg = express!(Payment::of(spice_amount), " detected in ", location, Game::sandtrout_message(self.sand_trout_doubles_resources))
                .with_log(BlowCardLog::new(blow_card, location, spice_amount, false, self.sand_trout_doubles_resources));
            self.log(elems![msg]);
            self.sand_trout_doubles_resources = false;
            self.change_resources_on_planet(location, spice_amount);
        } else {
            // C# parity: SandTroutDoublesResources is not reset when the doubled blow is lost
            // to the storm (hazards 18.2).
            let msg = express!(Payment::of(spice_amount), " in ", location, " is lost in the storm")
                .with_log(BlowCardLog::new(blow_card, location, spice_amount, true, false));
            self.log(elems![msg]);
        }

        let condition = self.applicable(Rule::ExpansionTreacheryCardsExceptPBandSSandAmal);
        let phase = if self.current_phase == Phase::BlowA { Phase::HarvesterA } else { Phase::HarvesterB };
        self.enter_boolean_phase_action(condition, phase, Game::move_to_next_phase_after_resource_blow);
    }

    pub fn sandtrout_message(sand_trout_doubles_resources: bool) -> MessagePart {
        express_part_if!(sand_trout_doubles_resources; ", doubled by ", Concept::BabyMonster)
    }

    pub fn move_to_next_phase_after_resource_blow(&mut self) {
        if self.monsters.is_empty() {
            // Static `Enter(bool, Action, Action)`, inlined.
            if (self.current_phase == Phase::BlowA || self.current_phase == Phase::HarvesterA) && self.applicable(Rule::IncreasedResourceFlow) {
                self.enter_blow_b();
            } else {
                self.start_nexus_card_phase();
            }
        } else {
            self.current_alliance_offers.clear();

            if self.applicable(Rule::BreakAlliancesOnAlliancePhase) {
                self.break_all_alliances();
            }

            let is_a = matches!(self.current_phase, Phase::BlowA | Phase::HarvesterA);
            if self.monsters.len() == 1 && self.monsters[0].is_great_monster {
                self.nexus_votes.clear();
                self.enter_boolean_phase_phase(is_a, Phase::VoteAllianceA, Phase::VoteAllianceB);
            } else {
                self.enter_alliance_phase(is_a);
            }
        }
    }

    pub fn let_monster_appear(&mut self, t: Option<Territory>, is_great_monster: bool) {
        match t {
            Some(t) if self.current_turn != 1 => {
                let m = MonsterAppearence::new(t, is_great_monster);

                let appeared = WormAppearedLog::new(m.territory, (self.monsters.len() as i32) + 1);
                if m.is_great_monster {
                    self.log(elems![Concept::GreatMonster, " appears in ", m.territory]);
                } else if !self.monsters.is_empty() {
                    let msg = express!(
                        Concept::Monster,
                        " appears a ",
                        Game::natural((self.monsters.len() as i32) + 1),
                        " time during this ",
                        MainPhase::Blow
                    )
                    .with_log(appeared);
                    self.log(elems![msg]);
                } else {
                    let msg = express!(Concept::Monster, " appears in ", m.territory).with_log(appeared);
                    self.log(elems![msg]);
                }

                if !self.monsters.is_empty() {
                    if self.is_playing(Faction::Fremen) && self.applicable(Rule::FremenSendingMonster) {
                        if !self.prevented(FactionAdvantage::FremenControlsMonster) {
                            let condition = self.current_phase == Phase::BlowA;
                            self.enter_boolean_phase_phase(condition, Phase::FremenSendingMonsterA, Phase::FremenSendingMonsterB);
                        } else {
                            self.monsters.push(m);
                            self.log_prevention_by_karma(FactionAdvantage::FremenControlsMonster);
                            if !self.applicable(Rule::FullPhaseKarma) {
                                self.allow(FactionAdvantage::FremenControlsMonster);
                            }
                        }
                    } else {
                        self.monsters.push(m);
                    }
                } else if self.monsters.is_empty() {
                    self.monsters.push(m.clone());
                    self.perform_monster(&m);
                }
            }
            _ => self.log_ignored_monster(is_great_monster),
        }
    }

    pub fn perform_monster(&mut self, m: &MonsterAppearence) {
        for &l in m.territory.locations() {
            for p in 0..self.players.len() {
                let p = self.player_at(p);
                if self.player(p).any_forces_in(Some(l)) > 0 {
                    if !self.protected_from_monster(p) {
                        self.reveal_current_no_field(p, None);

                        let faction = self.player(p).faction;
                        let mut devoured =
                            express!(m.describing_concept(), " devours ", self.player(p).any_forces_in(Some(l)), faction, " forces in ", l);
                        if !m.is_great_monster {
                            devoured = devoured.with_log(WormDevouredForcesLog::new(
                                faction,
                                self.player(p).forces_in(Some(l)),
                                self.player(p).special_forces_in(Some(l)),
                                l,
                            ));
                        }
                        self.log(elems![devoured]);
                        Player::kill_all_forces(self, p, l, false);

                        if self.player(p).is(Faction::Fremen) {
                            if !self.applicable(Rule::FullPhaseKarma) {
                                self.allow(FactionAdvantage::FremenProtectedFromMonster);
                            }
                        } else if self.player(p).ally == Faction::Fremen && self.fremen_will_protect_from_monster {
                            if !self.applicable(Rule::FullPhaseKarma) {
                                self.allow(FactionAdvantage::FremenProtectedFromMonsterAlly);
                            }
                        }
                    } else {
                        let faction = self.player(p).faction;
                        let mut survived = express!(faction, " survive ", m.describing_concept(), " in ", l);
                        if !m.is_great_monster {
                            survived = survived.with_log(WormSurvivedLog::new(faction, l));
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

        self.flip_bene_gesserit_advisors_when_alone();
    }

    pub fn enter_blow_a(&mut self) {
        self.monsters.clear();
        self.enter(Phase::BlowA, true);
        if self.applicable(Rule::IncreasedResourceFlow) {
            let msg = express!("Deck A").as_heading(Concept::Resource).with_log(BlowDeckLog::new("A".to_string()));
            self.log(elems![msg]);
        }
        self.draw_resource_card();
        self.let_factions_discard_surplus_cards();
    }

    pub fn enter_blow_b(&mut self) {
        self.monsters.clear();
        self.enter(Phase::BlowB, true);
        let msg = express!("Deck B").as_heading(Concept::Resource).with_log(BlowDeckLog::new("B".to_string()));
        self.log(elems![msg]);
        self.draw_resource_card();
        self.let_factions_discard_surplus_cards();
    }

    pub fn end_worm_ride_during_phase(&mut self, phase: Phase) {
        self.current_phase = phase;

        if FremenRidesMonster::is_applicable(self) {
            let condition = self.current_phase == Phase::AllianceA || self.current_phase == Phase::FremenRidingMonsterA;
            self.enter_boolean_phase_phase(condition, Phase::FremenRidingMonsterA, Phase::FremenRidingMonsterB);
        } else if self.current_phase == Phase::FremenRidingMonsterA && self.applicable(Rule::IncreasedResourceFlow) {
            self.enter_blow_b();
        } else {
            self.start_nexus_card_phase();
        }
    }

    pub fn enter_alliance_phase(&mut self, is_a: bool) {
        // Every faction must pass or form an alliance before the phase ends.
        self.has_acted_or_passed.clear();
        self.enter_boolean_phase_phase(is_a, Phase::AllianceA, Phase::AllianceB);
    }

    pub fn end_nexus(&mut self) {
        self.nexus_has_occured = true;
        self.current_alliance_offers.clear();

        if FremenRidesMonster::is_applicable(self) {
            let condition = self.current_phase == Phase::AllianceA;
            self.enter_boolean_phase_phase(condition, Phase::FremenRidingMonsterA, Phase::FremenRidingMonsterB);
        } else {
            if self.current_phase == Phase::AllianceA {
                let condition = self.applicable(Rule::IncreasedResourceFlow);
                Game::enter_boolean_action_action(self, condition, Game::enter_blow_b, Game::start_nexus_card_phase);
            } else if self.current_phase == Phase::AllianceB {
                self.start_nexus_card_phase();
            }
        }
    }

    pub fn end_nexus_if_everyone_is_done(&mut self) {
        if matches!(self.current_phase, Phase::AllianceA | Phase::AllianceB) && self.everyone_acted_or_passed() {
            self.end_nexus();
        }
    }

    pub fn start_nexus_card_phase(&mut self) {
        if self.nexus_has_occured
            && self.applicable(Rule::NexusCards)
            && self.players.iter().any(|p| p.has_ally())
            && self.players.iter().any(|p| !p.has_ally())
        {
            self.enter_nexus_card_phase();
        } else {
            self.end_blow_phase();
        }

        self.nexus_has_occured = false;
    }

    pub fn enter_nexus_card_phase(&mut self) {
        self.factions_that_may_draw_nexus_card = self.players.iter().filter(|p| !p.has_ally()).map(|p| p.faction).collect();
        self.factions_that_drew_nexus_card.clear();
        self.enter(Phase::NexusCards, true);
    }

    pub fn discard_nexus_card(&mut self, p: PlayerIndex) {
        let nexus = self.player(p).nexus;
        if nexus != Faction::None {
            let faction = self.player(p).faction;
            self.log(elems![faction, " discard the ", nexus, " Nexus Card"]);
            self.stone(Milestone::NexusPlayed);
            self.nexus_discard_pile.push(nexus);
            self.player_mut(p).nexus = Faction::None;
        }
    }

    pub fn end_blow_phase(&mut self) {
        self.factions_that_may_draw_nexus_card.clear();
        self.current_fremen_secret_ally = None;
        self.has_acted_or_passed.clear();
        self.reshuffle_ignored_monsters();
        self.main_phase_end();
        self.enter(Phase::BlowReport, true);
    }

    pub fn reshuffle_ignored_monsters(&mut self) {
        if !self.ignored_monsters.is_empty() || self.ignored_sandtrout.is_some() {
            if !self.ignored_monsters.is_empty() {
                self.log(elems![
                    self.ignored_monsters.len() as i32,
                    " ignored ",
                    Concept::Monster,
                    " cards were shuffled back into the ",
                    Concept::Resource,
                    " deck"
                ]);
                let ignored = self.ignored_monsters.clone();
                for c in ignored {
                    self.resource_card_deck.as_mut().expect("resource card deck").items.push(c);
                }
            }

            if let Some(sandtrout) = self.ignored_sandtrout {
                self.log(elems![Concept::BabyMonster, " card was shuffled back into the ", Concept::Resource, " deck"]);
                self.resource_card_deck.as_mut().expect("resource card deck").items.push(sandtrout);
            }

            self.resource_card_deck.as_mut().expect("resource card deck").shuffle();
            self.stone(Milestone::Shuffled);
        }
    }

    pub fn sand_trout_occured(&self) -> bool {
        self.sand_trout.is_some()
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
            // C# parity: "11th", "21th".
            _ => format!("{}th", count),
        }
    }

    pub fn current_discard_pile(&self) -> Option<&Deck<ResourceCard>> {
        if self.current_phase == Phase::BlowA { self.resource_card_discard_pile_a.as_ref() } else { self.resource_card_discard_pile_b.as_ref() }
    }

    /// `CurrentDiscardPile`, for callers that change the pile.
    pub fn current_discard_pile_mut(&mut self) -> Option<&mut Deck<ResourceCard>> {
        if self.current_phase == Phase::BlowA { self.resource_card_discard_pile_a.as_mut() } else { self.resource_card_discard_pile_b.as_mut() }
    }

    pub fn previous_blow_card(&self) -> Option<ResourceCard> {
        if self.current_phase == Phase::BlowA { self.latest_spice_card_a } else { self.latest_spice_card_b }
    }

    pub fn set_previous_blow_card(&mut self, value: Option<ResourceCard>) {
        if self.current_phase == Phase::BlowA {
            self.latest_spice_card_a = value;
        } else {
            self.latest_spice_card_b = value;
        }
    }

    pub fn protected_from_monster(&self, p: PlayerIndex) -> bool {
        if let Some(ally) = &self.current_fremen_secret_ally {
            if ally.player_opt() == Some(p) {
                return true;
            }
        }
        if self.player(p).is(Faction::Fremen) {
            return !self.prevented(FactionAdvantage::FremenProtectedFromMonster);
        }
        self.player(p).ally == Faction::Fremen && self.fremen_will_protect_from_monster && !self.prevented(FactionAdvantage::FremenProtectedFromMonsterAlly)
    }
}

#[derive(Clone, Debug, Canon)]
pub struct MonsterAppearence {
    pub territory: Territory,
    pub is_great_monster: bool,
}

impl MonsterAppearence {
    pub fn new(territory: Territory, is_great_monster: bool) -> Self {
        MonsterAppearence { territory, is_great_monster }
    }

    pub fn describing_concept(&self) -> Concept {
        if self.is_great_monster { Concept::GreatMonster } else { Concept::Monster }
    }

}
