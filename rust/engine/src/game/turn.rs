use crate::prelude::*;

/// What the game does next when a phase ends: enter a phase, or run a method that decides.
#[derive(Clone, Copy)]
pub enum Step {
    Enter(Phase),
    Run(fn(&mut Game) -> GameResult),
}

/// CHOAM's prevention of `target`'s Karama this phase (`ChoamKarmaPrevention`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KarmaPrevention {
    pub target: Option<Faction>,
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

/// Where the game is (turn and main phase; the phase is `game.phase`) and what lasts only for
/// the current turn or main phase: who has acted, payments and milestones, prevented advantages,
/// and cards and Nexus cards in effect.
pub struct TurnState {
    pub current_turn: Turn,
    pub current_main_phase: MainPhase,
    pub current_moment: Option<MainPhaseMoment>,
    pub has_acted_or_passed: Vec<Faction>,
    pub recent_milestones: Vec<Milestone>,
    pub recently_paid: Vec<Payment>,
    pub stored_recently_paid: Vec<Payment>,
    pub prevented_advantages: Vec<FactionAdvantage>,
    pub latest_clairvoyance: Option<ClairvoyanceAsked>,
    pub latest_clairvoyance_qn_a: Option<ClairVoyanceQandA>,
    pub latest_clairvoyance_battle: Option<InitiatedBattle>,
    pub resource_audited_factions: Vec<Faction>,
    pub current_juice: Option<JuiceUsed>,
    pub current_karma_prevention: Option<KarmaPrevention>,
    pub current_planetology: Option<PlanetologyUse>,
    pub bureaucrat_was_used_this_phase: bool,
    pub was_victim_of_bureaucracy: Option<Faction>,
    pub banker_was_used_this_phase: bool,
    pub current_nexus_prescience: Option<NexusCardPlayed>,
    pub current_fremen_secret_ally: Option<NexusCardPlayed>,
    pub current_emperor_cunning: Option<NexusCardPlayed>,
    pub current_guild_cunning: Option<NexusCardPlayed>,
    pub current_guild_secret_ally: Option<NexusCardPlayed>,
    pub current_bene_gesserit_cunning: Option<NexusCardPlayed>,
    pub current_ixian_cunning: Option<NexusCardPlayed>,
    pub harkonnen_traitor_was_cancelled: bool,
    pub facedancer_was_cancelled: bool,
    /// What revivals leave for the rest of the turn and later (revival.rs).
    pub revivals: Revivals,
    /// What the last bidding phase left behind (bidding.rs).
    pub last_bidding: BiddingLeftovers,
    /// The intrusions waiting for an answer, the next first. The one being answered is also
    /// shown by an `Interrupt::Intrusion`.
    pub intrusions: Vec<Intrusion>,
    /// Where the last shipment, move, revival or worm ride took forces, for the Bene Gesserit's
    /// reaction to it. Cleared when the shipment and move phase starts.
    pub last_shipment_or_movement: Option<ShipmentOrMovement>,
    /// The moves of the latest shipment and move phase, for the arrows on the board.
    pub recent_moves: Vec<RecentMove>,
    /// Who occupied Arrakeen or Carthag when the shipment and move phase began, so may move
    /// three territories.
    pub factions_with_ornithopters_at_start_of_movement: Vec<Faction>,
    /// Where the current shipper or mover took forces to a territory their ally is in.
    pub chosen_destinations_with_allies: Vec<Territory>,
    /// Bene Gesserit may accompany the latest shipment.
    pub bene_gesserit_may_accompany: bool,
    /// A Flight card played for the next move.
    pub current_flight_used: Option<FlightUse>,
    /// Who used the Flight discovery token for their next move.
    pub current_flight_discovery_used: Option<By>,
    /// The territories CHOAM closed to moves (`ChoamMovePrevention`), until the next shipment.
    pub current_blocked_territories: Vec<Territory>,
    /// The storm losses still to be taken, the first one next. The storm fills it in the storm
    /// phase (`StormStep::Losses`), and so does a shipment or move into the storm
    /// (`Interrupt::StormLosses`). Cleared when the storm phase starts and before each move.
    pub storm_losses_to_take: Vec<crate::game::LossToTake>,
}

impl TurnState {
    pub fn new() -> Self {
        TurnState {
            current_turn: Default::default(),
            current_main_phase: MainPhase::Started,
            current_moment: None,
            has_acted_or_passed: Vec::new(),
            recent_milestones: Vec::new(),
            recently_paid: Vec::new(),
            stored_recently_paid: Vec::new(),
            prevented_advantages: Vec::new(),
            latest_clairvoyance: None,
            latest_clairvoyance_qn_a: None,
            latest_clairvoyance_battle: None,
            resource_audited_factions: Vec::new(),
            current_juice: None,
            current_karma_prevention: None,
            current_planetology: None,
            bureaucrat_was_used_this_phase: Default::default(),
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
            revivals: Revivals::new(),
            last_bidding: BiddingLeftovers::default(),
            intrusions: Vec::new(),
            last_shipment_or_movement: None,
            recent_moves: Vec::new(),
            factions_with_ornithopters_at_start_of_movement: Vec::new(),
            chosen_destinations_with_allies: Vec::new(),
            bene_gesserit_may_accompany: false,
            storm_losses_to_take: Vec::new(),
            current_flight_used: None,
            current_flight_discovery_used: None,
            current_blocked_territories: Vec::new(),
        }
    }
}

/// The methods that read who played the card, for the state a played card leaves.
macro_rules! card_played_by {
    ($($t:ty),* $(,)?) => {$(
        impl $t {
            pub fn initiator(&self) -> Option<Faction> {
                self.submitter.faction
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

card_played_by!(JuiceUsed, NexusCardPlayed);

/// The Juice of Sapho in effect this main phase (`JuicePlayed`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JuiceUsed {
    pub submitter: By,
    pub type_: Option<JuiceType>,
}

/// A Nexus card whose effect lasts (`NexusPlayed`): a cunning or secret ally kept in one of the
/// `current_*` fields of the turn until it ends.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NexusCardPlayed {
    pub submitter: By,
    /// The battle plan element an Atreides Nexus card lets its player see.
    pub atreides_prescience_aspect: Option<PrescienceAspect>,
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
        self.turn.intrusions.clear();
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
        self.phase() == Phase::Bidding
            || self.phase() == Phase::GuildShip
            || self.phase() == Phase::GuildMove
            || self.phase() == Phase::NonGuildShip
            || self.phase() == Phase::NonGuildMove
            || self.phase() == Phase::BattlePhase
    }

    pub fn everyone_acted_or_passed(&self) -> bool {
        self.turn.has_acted_or_passed.len() == self.players.len()
    }

    pub fn stone(&mut self, m: Milestone) {
        self.turn.recent_milestones.push(m);
    }

    pub fn set_recent_payment(&mut self, amount: i32, by: Option<Faction>, to: Option<Faction>, reason: EventType) {
        if amount > 0 {
            self.turn.recently_paid.push(Payment::new_full(amount, by, to, Some(reason)));
        }
    }

    pub fn set_recent_payment_to_bank(&mut self, amount: i32, by: Option<Faction>, reason: EventType) {
        self.set_recent_payment(amount, by, None, reason);
    }

    pub fn clear_recent_payments(&mut self) {
        // StoredRecentlyPaid = RecentlyPaid (a reference swap); RecentlyPaid = []
        self.turn.stored_recently_paid = std::mem::take(&mut self.turn.recently_paid);
    }

    pub fn has_recent_payment_for(&self, t: EventType) -> bool {
        self.turn.recently_paid.iter().any(|p| p.reason == Some(t))
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
