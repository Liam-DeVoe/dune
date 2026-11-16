use crate::prelude::*;

/// What the game does next when a phase ends: enter a phase, or run a method that decides.
#[derive(Clone, Copy)]
pub enum Step {
    Enter(Phase),
    Run(fn(&mut Game) -> GameResult),
}

/// A turn of the game, numbered from 1. Turn 0 is [`Turn::BEFORE_START`]: the setup, before
/// the first storm. It is a real moment of the game, with its own report and moments, so it is a
/// value of the type rather than `Option<Turn>`. A game has at most 255 turns.
///
/// Event JSON, views and the log write the number. Event JSON refuses a number outside 0 to 255.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Turn(u8);

impl Turn {
    /// The setup, before the first storm.
    pub const BEFORE_START: Turn = Turn(0);
    pub const FIRST: Turn = Turn(1);

    pub const fn new(number: u8) -> Turn {
        Turn(number)
    }

    pub fn number(self) -> u8 {
        self.0
    }

    /// The turn after this one, or `None` after turn 255.
    pub fn next(self) -> Option<Turn> {
        self.0.checked_add(1).map(Turn)
    }
}

impl From<Turn> for i32 {
    fn from(t: Turn) -> i32 {
        t.0 as i32
    }
}

impl std::fmt::Display for Turn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Where the game is (turn, main phase, phase) and what lasts only for the current turn or main
/// phase: who has acted, payments and milestones, prevented advantages, cards and Nexus cards in
/// effect, and the phases paused by Clairvoyance, discarding and Bureaucracy.
pub struct TurnState {
    pub current_turn: Turn,
    pub current_main_phase: MainPhase,
    pub current_moment: Option<MainPhaseMoment>,
    pub current_phase: Phase,
    pub has_acted_or_passed: Vec<Faction>,
    pub recent_milestones: Vec<Milestone>,
    pub recently_paid: Vec<Payment>,
    pub stored_recently_paid: Vec<Payment>,
    pub prevented_advantages: Vec<FactionAdvantage>,
    pub phase_before_discarding: Option<Phase>,
    pub factions_that_must_discard: Vec<Faction>,
    pub phase_before_discarding_traitor: Option<Phase>,
    pub faction_that_must_discard_traitor: Option<Faction>,
    pub number_of_traitors_to_discard: i32,
    pub phase_before_searching_discarded: Option<Phase>,
    pub phase_paused_by_clairvoyance: Option<Phase>,
    pub latest_clairvoyance: Option<Rc<ClairVoyancePlayed>>,
    pub latest_clairvoyance_qn_a: Option<ClairVoyanceQandA>,
    pub latest_clairvoyance_battle: Option<Rc<BattleInitiated>>,
    pub resource_audited_factions: Vec<Faction>,
    pub current_juice: Option<Rc<JuicePlayed>>,
    pub current_karma_prevention: Option<Rc<ChoamKarmaPrevention>>,
    pub current_planetology: Option<Rc<Planetology>>,
    pub bureaucrat_was_used_this_phase: bool,
    pub phase_before_bureaucrat_was_activated: Option<Phase>,
    pub target_of_bureaucracy: Option<Faction>,
    pub was_victim_of_bureaucracy: Option<Faction>,
    pub banker_was_used_this_phase: bool,
    pub current_nexus_prescience: Option<Rc<NexusPlayed>>,
    pub current_fremen_secret_ally: Option<Rc<NexusPlayed>>,
    pub current_emperor_cunning: Option<Rc<NexusPlayed>>,
    pub current_guild_cunning: Option<Rc<NexusPlayed>>,
    pub current_guild_secret_ally: Option<Rc<NexusPlayed>>,
    pub current_bene_gesserit_cunning: Option<Rc<NexusPlayed>>,
    pub current_ixian_cunning: Option<Rc<NexusPlayed>>,
    pub harkonnen_traitor_was_cancelled: bool,
    pub facedancer_was_cancelled: bool,
}

impl TurnState {
    pub fn new() -> Self {
        TurnState {
            current_turn: Default::default(),
            current_main_phase: MainPhase::Started,
            current_moment: None,
            current_phase: Phase::AwaitingPlayers,
            has_acted_or_passed: Vec::new(),
            recent_milestones: Vec::new(),
            recently_paid: Vec::new(),
            stored_recently_paid: Vec::new(),
            prevented_advantages: Vec::new(),
            phase_before_discarding: Default::default(),
            factions_that_must_discard: Vec::new(),
            phase_before_discarding_traitor: Default::default(),
            faction_that_must_discard_traitor: Default::default(),
            number_of_traitors_to_discard: Default::default(),
            phase_before_searching_discarded: Default::default(),
            phase_paused_by_clairvoyance: Default::default(),
            latest_clairvoyance: None,
            latest_clairvoyance_qn_a: None,
            latest_clairvoyance_battle: None,
            resource_audited_factions: Vec::new(),
            current_juice: None,
            current_karma_prevention: None,
            current_planetology: None,
            bureaucrat_was_used_this_phase: Default::default(),
            phase_before_bureaucrat_was_activated: Default::default(),
            target_of_bureaucracy: Default::default(),
            was_victim_of_bureaucracy: Default::default(),
            banker_was_used_this_phase: Default::default(),
            current_nexus_prescience: None,
            current_fremen_secret_ally: None,
            current_emperor_cunning: None,
            current_guild_cunning: None,
            current_guild_secret_ally: None,
            current_bene_gesserit_cunning: None,
            current_ixian_cunning: None,
            harkonnen_traitor_was_cancelled: Default::default(),
            facedancer_was_cancelled: Default::default(),
        }
    }
}

impl Game {
    pub fn main_phase_start(&mut self, phase: MainPhase, start_report: bool) {
        self.turn.current_main_phase = phase;
        self.turn.current_moment = Some(MainPhaseMoment::Start);
        if start_report {
            self.start_report(phase);
        }
        self.turn.current_karma_prevention = None;
        self.turn.current_juice = None;
        self.turn.bureaucrat_was_used_this_phase = false;
        self.turn.banker_was_used_this_phase = false;
        // An intrusion is answered right after the shipment or move that caused it. One still
        // queued when a new phase starts can no longer be answered, and would take the place of
        // the next real one (escapes.md, "A Bene Gesserit intrusion left over from another phase").
        self.ship_move.intrusions.clear();
    }

    pub fn main_phase_middle(&mut self) {
        if self.turn.current_moment == Some(MainPhaseMoment::Start) {
            self.turn.current_moment = Some(MainPhaseMoment::Middle);
        }
    }

    pub fn main_phase_end(&mut self) {
        self.turn.current_moment = Some(MainPhaseMoment::End);

        let mut exceptions_to_allowing: Vec<FactionAdvantage> = Vec::new();

        if self.turn.current_main_phase == MainPhase::Bidding && self.prevented(FactionAdvantage::TleilaxuIncreasingRevivalLimits) {
            exceptions_to_allowing.push(FactionAdvantage::TleilaxuIncreasingRevivalLimits);
        }

        if self.turn.current_main_phase == MainPhase::Resurrection && self.prevented(FactionAdvantage::AtreidesSpiceBlowPrescience) {
            exceptions_to_allowing.push(FactionAdvantage::AtreidesSpiceBlowPrescience);
        }

        self.turn.was_victim_of_bureaucracy = None;

        self.allow_all_prevented_faction_advantages(&exceptions_to_allowing);
    }

    pub fn allow_all_prevented_faction_advantages(&mut self, exceptions: &[FactionAdvantage]) {
        for adv in get_values::<FactionAdvantage>() {
            if !exceptions.contains(&adv) {
                self.allow(adv);
            }
        }
    }

    pub fn enter(&mut self, phase: Phase, cleanup_deals: bool) {
        self.turn.current_phase = phase;

        if cleanup_deals {
            self.remove_ended_deals(phase);
        }
    }

    pub fn enter_either(&mut self, condition: bool, phase_if_true: Phase, phase_if_false: Phase) {
        self.enter(if condition { phase_if_true } else { phase_if_false }, true);
    }

    pub fn enter_or_else(&mut self, condition: bool, phase_if_true: Phase, method_otherwise: fn(&mut Game)) {
        if condition {
            self.enter(phase_if_true, true);
        } else {
            method_otherwise(self);
        }
    }

    /// As [`Game::enter_or_else`], for a method that can fail.
    pub fn enter_or_else_try(&mut self, condition: bool, phase_if_true: Phase, method_otherwise: fn(&mut Game) -> GameResult) -> GameResult {
        if condition {
            self.enter(phase_if_true, true);
            Ok(())
        } else {
            method_otherwise(self)
        }
    }

    /// The game is passed to the actions.
    pub fn run_either(game: &mut Game, condition: bool, method_if_true: fn(&mut Game), method_otherwise: fn(&mut Game)) {
        if condition {
            method_if_true(game);
        } else {
            method_otherwise(game);
        }
    }

    /// Takes the step of the first choice whose condition holds, or `otherwise`.
    pub fn enter_first(&mut self, choices: &[(bool, Step)], otherwise: Step) -> GameResult {
        let step = choices.iter().find(|(condition, _)| *condition).map_or(otherwise, |(_, step)| *step);
        match step {
            Step::Enter(phase) => {
                self.enter(phase, true);
                Ok(())
            }
            Step::Run(method) => method(self),
        }
    }

    pub fn in_timed_phase(&self) -> bool {
        self.turn.current_phase == Phase::Bidding
            || self.turn.current_phase == Phase::GuildShip
            || self.turn.current_phase == Phase::GuildMove
            || self.turn.current_phase == Phase::NonGuildShip
            || self.turn.current_phase == Phase::NonGuildMove
            || self.turn.current_phase == Phase::BattlePhase
    }

    pub fn everyone_acted_or_passed(&self) -> bool {
        self.turn.has_acted_or_passed.len() == self.players.len()
    }

    pub fn stone(&mut self, m: Milestone) {
        self.turn.recent_milestones.push(m);
    }

    pub fn set_recent_payment(&mut self, amount: i32, by: Option<Faction>, to: Option<Faction>, reason: &Event) {
        if amount > 0 {
            self.turn.recently_paid.push(Payment::new_full(amount, by, to, Some(reason.clone())));
        }
    }

    pub fn set_recent_payment_to_bank(&mut self, amount: i32, by: Option<Faction>, reason: &Event) {
        self.set_recent_payment(amount, by, None, reason);
    }

    pub fn clear_recent_payments(&mut self) {
        // StoredRecentlyPaid = RecentlyPaid (a reference swap); RecentlyPaid = []
        self.turn.stored_recently_paid = std::mem::take(&mut self.turn.recently_paid);
    }

    pub fn has_recent_payment_for(&self, t: EventType) -> bool {
        self.turn.recently_paid.iter().any(|p| p.reason.as_ref().map_or(false, |r| r.event_type() == t))
    }

    pub fn has_quite_recent_payment_to(&self, f: Faction) -> bool {
        self.turn.recently_paid.iter().any(|p| p.to == Some(f)) || self.turn.stored_recently_paid.iter().any(|p| p.to == Some(f))
    }

    pub fn quite_recent_payment_to(&self, f: Faction) -> Option<Payment> {
        let res = self
            .turn.recently_paid
            .iter()
            .find(|p| p.to == Some(f))
            .or_else(|| self.turn.stored_recently_paid.iter().find(|p| p.to == Some(f)));
        res.cloned()
    }

    pub fn recently_paid_total_amount(&self) -> i32 {
        self.turn.recently_paid.iter().map(|p| p.amount).sum::<i32>()
    }
}
