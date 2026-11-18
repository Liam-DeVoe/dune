use crate::prelude::*;

/// A request of `initiator` to Tleilaxu to revive `hero` early (`RequestTleilaxuRevival`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RevivalRequest {
    pub initiator: Option<Faction>,
    /// The submitter's player when the request was made.
    pub player: Option<PlayerId>,
    pub hero: Option<Hero>,
}

/// A Karama (or CHOAM) prevention of `target`'s revival this turn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RevivalPrevention {
    pub target: Option<Faction>,
}

/// The revival phase (`PhaseState::Revival`): its step, Recruits, the requests to Tleilaxu and
/// the early revivals Tleilaxu offered, the CHOAM prevention of free revival, Tleilaxu's Karama
/// prevention of revival and the Graveyard tech token's income. It is
/// dropped when the phase ends. What other phases read is in `turn.revivals`.
#[derive(Debug)]
pub struct RevivalState {
    pub step: RevivalStep,
    pub revival_tech_token_income: bool,
    pub tleilaxu_started_revival_with_low_threshold: bool,
    /// Whether Recruits were played this turn (`RecruitsPlayed`). Cleared again when the
    /// revivals end, before the report.
    pub current_recruits_played: bool,
    // In the order Tleilaxu made the offers.
    pub early_revivals_offers: IndexMap<Hero, i32>,
    pub current_free_revival_prevention: Option<RevivalPrevention>,
    /// Leader revivals requested from Tleilaxu that they haven't answered yet.
    pub current_revival_requests: Vec<RevivalRequest>,
    /// Tleilaxu's Karama prevention of a faction's revival. Cleared when the revivals end,
    /// before the report.
    pub current_karma_revival_prevention: Option<RevivalPrevention>,
}

impl RevivalState {
    pub fn new(step: RevivalStep, tleilaxu_started_revival_with_low_threshold: bool) -> Self {
        RevivalState {
            step,
            revival_tech_token_income: false,
            tleilaxu_started_revival_with_low_threshold,
            current_recruits_played: false,
            early_revivals_offers: Default::default(),
            current_free_revival_prevention: None,
            current_revival_requests: Vec::new(),
            current_karma_revival_prevention: None,
        }
    }
}

/// Where the revival phase is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevivalStep {
    /// Before the revivals: Recruits, and Tleilaxu raising revival limits.
    Beginning,
    /// Factions revive forces and leaders.
    Reviving,
    Report,
}

impl RevivalStep {
    pub fn phase(self) -> Phase {
        match self {
            RevivalStep::Beginning => Phase::BeginningOfResurrection,
            RevivalStep::Reviving => Phase::Resurrection,
            RevivalStep::Report => Phase::ResurrectionReport,
        }
    }

}

/// What revivals leave for other phases (`turn.revivals`). The counts start again when the
/// revival phase does; Raise Dead and Nexus cards revive in other phases, and shipments, bots
/// and views read the counts and limits.
#[derive(Debug)]
pub struct Revivals {
    pub total_revivals_this_turn: IndexMap<Faction, i32>,
    pub free_revivals_this_turn: IndexMap<Faction, i32>,
    /// Cleared when the storm phase starts, not the revival phase.
    pub factions_that_revived_special_forces_this_turn: Vec<Faction>,
    pub leader_revivals_this_turn: IndexMap<Faction, i32>,
    /// Set in the bidding report or the revival phase, cleared when ship and move starts.
    /// Null after `SetIncreasedRevivalLimits` with `"Factions":null` executed without validation.
    pub factions_with_increased_revival_limits: Option<Vec<Faction>>,
    /// Ecaz ambassadors placed this turn, in the revival report.
    pub ambassadors_placed_this_turn: i32,
}

impl Revivals {
    pub fn new() -> Self {
        Revivals {
            total_revivals_this_turn: Default::default(),
            free_revivals_this_turn: Default::default(),
            factions_that_revived_special_forces_this_turn: Vec::new(),
            leader_revivals_this_turn: Default::default(),
            factions_with_increased_revival_limits: Some(Vec::new()),
            ambassadors_placed_this_turn: 0,
        }
    }
}

impl Game {
    /// The revival phase's state, while the game is in it (an interrupt may suspend it).
    pub fn revival(&self) -> Option<&RevivalState> {
        match &self.phase {
            PhaseState::Revival(r) => Some(r),
            _ => None,
        }
    }

    pub fn revival_mut(&mut self) -> Option<&mut RevivalState> {
        match &mut self.phase {
            PhaseState::Revival(r) => Some(r),
            _ => None,
        }
    }

    /// Starts the revival phase (`EndPhase::enter_revival_phase` does the rest).
    pub fn begin_revival(&mut self) {
        let tleilaxu_low = self.has_low_threshold(Faction::Tleilaxu);
        self.enter_state(PhaseState::Revival(RevivalState::new(RevivalStep::Beginning, tleilaxu_low)));
    }

    /// Moves the revival phase to `step`, dropping any interrupt, as entering a phase did.
    pub fn enter_revival(&mut self, step: RevivalStep) {
        match self.revival_mut() {
            Some(r) => {
                r.step = step;
                self.interrupts.clear();
                self.remove_ended_deals(step.phase());
            }
            None => {
                let tleilaxu_low = self.has_low_threshold(Faction::Tleilaxu);
                self.enter_state(PhaseState::Revival(RevivalState::new(step, tleilaxu_low)));
            }
        }
    }

    /// Whether Recruits double the revivals now.
    pub fn recruits_played(&self) -> bool {
        self.revival().is_some_and(|r| r.current_recruits_played)
    }

    /// The early revivals Tleilaxu offered in this revival phase, by hero.
    pub fn early_revival_offer(&self, hero: Hero) -> Option<i32> {
        self.revival().and_then(|r| r.early_revivals_offers.get(&hero).copied())
    }

    pub fn prepare_skill_assignment_to_revived_leader(&mut self, player: PlayerId, leader: Option<Leader>) {
        let already_assigning = self.phase() == Phase::AssigningSkill;

        self.player_mut(player).most_recently_revived_leader = leader;
        self.decks.skill_deck.as_mut().expect("skill deck").shuffle(&mut self.rng);
        self.stone(Milestone::Shuffled);
        let skill = self.decks.skill_deck.as_mut().expect("skill deck").draw();
        self.player_mut(player).skills_to_choose_from.push(skill);
        let skill = self.decks.skill_deck.as_mut().expect("skill deck").draw();
        self.player_mut(player).skills_to_choose_from.push(skill);
        if already_assigning {
            self.remove_ended_deals(Phase::AssigningSkill);
        } else {
            self.interrupt(Interrupt::SkillAssignment);
        }
    }

    pub fn free_revivals(&self, player: PlayerId, uses_emperor_secret_ally: bool) -> i32 {
        let p = self.player(player);
        if self.free_revival_prevented(p.faction.expect("faction")) {
            return 0;
        }

        let mut nr_of_free_revivals: i32 = 0;

        match p.faction {
            Some(Faction::Fremen) => nr_of_free_revivals = 3,
            Some(Faction::Atreides) => nr_of_free_revivals = 2,
            Some(Faction::Harkonnen) => nr_of_free_revivals = 2,
            Some(Faction::Emperor) => nr_of_free_revivals = 1,
            Some(Faction::Guild) => nr_of_free_revivals = 1,
            Some(Faction::BeneGesserit) => nr_of_free_revivals = 1,
            Some(Faction::Ixian) => nr_of_free_revivals = 1,
            Some(Faction::Tleilaxu) => nr_of_free_revivals = 2,

            Some(Faction::Choam) => nr_of_free_revivals = 0,
            Some(Faction::Richese) => nr_of_free_revivals = 2,
            Some(Faction::Ecaz) => nr_of_free_revivals = 2,
            Some(Faction::Moritani) => nr_of_free_revivals = 2,
            _ => {}
        }

        if (p.ally == Some(Faction::Fremen) && self.alliances.fremen_allows_three_free_revivals)
            || self.turn.current_fremen_secret_ally.as_ref().is_some_and(|e| e.player_opt() == Some(player))
        {
            nr_of_free_revivals = 3;
        }

        if uses_emperor_secret_ally {
            nr_of_free_revivals += 3;
        }

        if self.gets_extra_charity_and_free_revival_due_to_low_threshold(player) {
            nr_of_free_revivals += 1;
        }

        if self.recruits_played() {
            nr_of_free_revivals *= 2;
        }

        std::cmp::max(
            nr_of_free_revivals - self.turn.revivals.free_revivals_this_turn.get_opt(p.faction).copied().unwrap_or(0),
            0,
        )
    }

    pub fn gets_extra_charity_and_free_revival_due_to_low_threshold(&self, player: PlayerId) -> bool {
        let p = self.player(player);
        (!(p.is(Faction::Emperor) || p.is(Faction::Choam)) && p.has_any_low_threshold(self))
            || (p.is(Faction::Emperor) && p.has_low_threshold(self, World::Emperor))
            || (p.is(Faction::Choam) && p.has_any_low_threshold(self) && self.occupier_of(World::Choam).is_none())
    }

    /// Reads `g` for some values and `self` for others (kept deliberately); every caller passes the same game.
    pub fn get_max_revivals(&self, g: &Game, p: PlayerId, emperor_secret_ally_used: bool) -> i32 {
        let pl = g.player(p);
        if pl.is(Faction::Tleilaxu) || (pl.is(Faction::Choam) && !g.prevented(FactionAdvantage::ChoamRevival)) {
            return i32::MAX;
        }

        let emperor_secret_ally: i32 = if emperor_secret_ally_used { 3 } else { 0 };
        // Low threshold's free revival raises the limit too. The Emperor gets it from Kaitain only:
        // Salusa Secundus at low threshold gives no free revival.
        let faction = pl.faction.expect("faction");
        let low_threshold_applies = if faction == Faction::Emperor { pl.has_low_threshold(g, World::Emperor) } else { g.has_low_threshold(faction) };
        let low_threshold: i32 = if low_threshold_applies { 1 } else { 0 };
        let already_revived = g.turn.revivals.total_revivals_this_turn.get_opt(pl.faction).copied().unwrap_or(0);

        if self.recruits_played() {
            return std::cmp::max(7 + emperor_secret_ally + low_threshold - already_revived, 0);
        }

        // No list of increased limits means nobody has one.
        if self.turn.revivals.factions_with_increased_revival_limits.as_deref().unwrap_or(&[]).contains_opt(pl.faction) {
            return std::cmp::max(5 + low_threshold + emperor_secret_ally - already_revived, 0);
        }

        std::cmp::max(3 + low_threshold + emperor_secret_ally - already_revived, 0)
    }

    pub fn free_revival_prevented(&self, f: Faction) -> bool {
        self.revival().and_then(|r| r.current_free_revival_prevention.as_ref()).map(|e| e.target) == Some(Some(f))
    }

    pub fn prevented_from_reviving(&self, f: impl Into<Option<Faction>>) -> bool {
        let f: Option<Faction> = f.into();
        self.revival().and_then(|r| r.current_karma_revival_prevention.as_ref()).is_some_and(|e| e.target == f)
    }

    pub fn ambassador_in(&self, t: Territory) -> Option<Ambassador> {
        self.board.ambassadors_on_planet.get(&t).copied()
    }

    pub fn claim_free_revival(&mut self, player: PlayerId) -> GameResult {
        let max_free_revivals = self.free_revivals(player, false);

        if max_free_revivals <= 0 {
            return Ok(());
        }

        let mut free_revived_special_forces: i32 = 0;
        let mut free_revived_normal_forces: i32 = 0;

        let max_revivable_special_forces = Revival::valid_max_revivals(self, player, true, false);
        let max_revivable_normal_forces = Revival::valid_max_revivals(self, player, false, false);

        while free_revived_special_forces < max_free_revivals && free_revived_special_forces < max_revivable_special_forces {
            free_revived_special_forces += 1;
        }

        while free_revived_special_forces + free_revived_normal_forces < max_free_revivals
            && free_revived_normal_forces < max_revivable_normal_forces
        {
            free_revived_normal_forces += 1;
        }

        if free_revived_normal_forces + free_revived_special_forces <= 0 {
            return Ok(());
        }

        if free_revived_normal_forces > 0 {
            Player::revive_forces(self, player, free_revived_normal_forces)?;
        }

        if free_revived_special_forces > 0 {
            Player::revive_special_forces(self, player, free_revived_special_forces)?;
        }

        let faction = self.player(player).faction;
        if faction != Some(Faction::Tleilaxu) {
            self.revival_mut().expect("in the revival phase").revival_tech_token_income = true;
        }

        let tleilaxu = self.get_player(Some(Faction::Tleilaxu));
        let mut tleilaxu_receives_income = false;

        if let Some(tleilaxu) = tleilaxu {
            // Tleilaxu earn this from their own free revival too, which is checked and correct
            // (known-issues.md, "Checked and correct"): "for each faction using free revival or a
            // ghola card, you take 1 spice from the Spice Bank" (Ixians & Tleilaxu rulebook). Only
            // the tech token's income above excludes Tleilaxu.
            tleilaxu_receives_income =
                !self.revival().expect("in the revival phase").tleilaxu_started_revival_with_low_threshold && !self.prevented(FactionAdvantage::TleilaxuReceiveRevive);

            if tleilaxu_receives_income {
                let r = &mut self.player_mut(tleilaxu).resources;
                *r += 1;
            }
        }

        let v = self
            .turn.revivals.free_revivals_this_turn
            .get_opt(faction).copied().unwrap_or(0) + free_revived_normal_forces + free_revived_special_forces;
        self.turn.revivals.free_revivals_this_turn.insert(faction.expect("faction"), v);
        let v = self
            .turn.revivals.total_revivals_this_turn
            .get_opt(faction).copied().unwrap_or(0) + free_revived_normal_forces + free_revived_special_forces;
        self.turn.revivals.total_revivals_this_turn.insert(faction.expect("faction"), v);

        let p = self.player(player);
        let mut row = RevivalRow::new(faction.expect("faction"), free_revived_normal_forces, free_revived_special_forces, 0);
        row.tleilaxu_income = if tleilaxu_receives_income { 1 } else { 0 };
        let message = express!(
            faction,
            " revive ",
            express_part_if!(free_revived_special_forces > 0; free_revived_special_forces, p.special_force()),
            express_part_if!(free_revived_normal_forces > 0; free_revived_normal_forces, p.force()),
            " for free",
            express_part_if!(tleilaxu_receives_income; Faction::Tleilaxu, " receive ", Payment::of(1))
        )
        .with_log(row);
        self.log(vec![message.into()]);

        self.stone(Milestone::Revival);
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct RevivalCost {
    pub cost_for_force_revival_for_player: i32,
    pub cost_for_emperor: i32,
    pub cost_to_revive_hero: i32,
    pub can_be_paid: bool,
    pub includes_costs_for_special_forces: bool,
    pub number_of_forces_revived_for_free: i32,
}

impl Default for RevivalCost {
    fn default() -> Self {
        RevivalCost {
            cost_for_force_revival_for_player: Default::default(),
            cost_for_emperor: Default::default(),
            cost_to_revive_hero: Default::default(),
            can_be_paid: Default::default(),
            includes_costs_for_special_forces: Default::default(),
            number_of_forces_revived_for_free: Default::default(),
        }
    }
}

impl RevivalCost {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn total_cost(&self) -> i32 {
        self.total_cost_for_player() + self.cost_for_emperor
    }

    pub fn total_cost_for_player(&self) -> i32 {
        self.cost_for_force_revival_for_player + self.cost_to_revive_hero
    }

    pub fn total_cost_for_force_revival(&self) -> i32 {
        self.cost_for_force_revival_for_player + self.cost_for_emperor
    }

}

impl GameStatus {
    /// The status in the revival phase, or `None` in another phase.
    pub(crate) fn revival_status(_game: &Game, _me: Option<PlayerId>, phase: Phase) -> Option<GameStatus> {
        Some(match phase {
            Phase::Resurrection => Self::status("Factions may now reclaim forces and leaders.", None),

            _ => return None,
        })
    }
}

impl Game {
    /// What `player` may do in the revival phase (see `add_player_actions`).
    pub(crate) fn add_revival_actions(&self, player: PlayerId, _is_host: bool, result: &mut Vec<EventType>) {
        let faction = self.player(player).faction;
        match self.phase() {
            Phase::BeginningOfResurrection => {
                if faction == Some(Faction::Tleilaxu)
                    && self.players.len() > 1
                    && !self.prevented(FactionAdvantage::TleilaxuIncreasingRevivalLimits)
                {
                    result.push(EventType::SetIncreasedRevivalLimits);
                }
                if RecruitsPlayed::is_applicable(self, player) {
                    result.push(EventType::RecruitsPlayed);
                }
            }

            Phase::Resurrection => {
                if self.is_playing(Faction::Tleilaxu)
                    && faction != Some(Faction::Tleilaxu)
                    && !self.killed_heroes(player).is_empty()
                    && Revival::normally_revivable_heroes(self, player).is_empty()
                {
                    result.push(EventType::RequestTleilaxuRevival);
                }

                if faction == Some(Faction::Tleilaxu) && self.revival().is_some_and(|r| !r.current_revival_requests.is_empty() || !r.early_revivals_offers.is_empty()) {
                    result.push(EventType::AcceptOrCancelTleilaxuRevival);
                }
                if self.has_something_to_revive(player) && !self.prevented_from_reviving(faction) {
                    result.push(EventType::Revival);
                }
                if faction == Some(Faction::Tleilaxu)
                    && self.players.len() > 1
                    && !self.prevented(FactionAdvantage::TleilaxuIncreasingRevivalLimits)
                {
                    result.push(EventType::SetIncreasedRevivalLimits);
                }
            }

            Phase::ResurrectionReport => {
                if faction == Some(Faction::Ecaz) && AmbassadorPlaced::is_applicable(self, player) {
                    result.push(EventType::AmbassadorPlaced);
                }
            }

            _ => {}
        }
    }
}
