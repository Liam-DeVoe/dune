use crate::log::charity::CharityClaim;
use crate::prelude::*;

/// The charity phase (`PhaseState::Charity`): its step, and whether a claim earns the Spice tech
/// token its income. It is dropped when the phase ends. The CHOAM economics token, which lasts
/// the game, is `board.choam_economics_status`.
#[derive(Debug)]
pub struct CharityState {
    pub step: CharityStep,
    pub resource_tech_token_income: bool,
}

impl CharityState {
    pub fn new(step: CharityStep) -> Self {
        CharityState { step, resource_tech_token_income: false }
    }
}

/// Where the charity phase is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharityStep {
    /// Before charity is given, while the host may still wait for players.
    Beginning,
    /// Factions claim charity.
    Claiming,
    Report,
}

impl CharityStep {
    pub fn phase(self) -> Phase {
        match self {
            CharityStep::Beginning => Phase::BeginningOfCharity,
            CharityStep::Claiming => Phase::ClaimingCharity,
            CharityStep::Report => Phase::CharityReport,
        }
    }

}

/// `params object?[]` element list for `Log(...)`.
impl Game {
    /// The charity phase's state, while the game is in it (an interrupt may suspend it).
    pub fn charity(&self) -> Option<&CharityState> {
        match &self.phase {
            PhaseState::Charity(c) => Some(c),
            _ => None,
        }
    }

    pub fn charity_mut(&mut self) -> Option<&mut CharityState> {
        match &mut self.phase {
            PhaseState::Charity(c) => Some(c),
            _ => None,
        }
    }

    /// Starts the charity phase (`EndPhase::enter_charity_phase` does the rest).
    pub fn begin_charity(&mut self) {
        self.enter_state(PhaseState::Charity(CharityState::new(CharityStep::Beginning)));
    }

    /// Moves the charity phase to `step`, dropping any interrupt, as entering a phase did.
    pub fn enter_charity(&mut self, step: CharityStep) {
        match self.charity_mut() {
            Some(c) => {
                c.step = step;
                self.interrupts.clear();
                self.remove_ended_deals(step.phase());
            }
            None => self.enter_state(PhaseState::Charity(CharityState::new(step))),
        }
    }

    pub fn charity_is_cancelled(&self) -> bool {
        matches!(self.board.choam_economics_status, Some(ChoamEconomicsStatus::Cancel) | Some(ChoamEconomicsStatus::CancelFlipped))
    }

    pub fn receive_tech_income(&mut self, token: TechToken) {
        let tech_token_owner = self.player_indices().find(|p| self.player(*p).tech_tokens.contains(&token));
        if let Some(owner) = tech_token_owner {
            let amount = self.player(owner).tech_tokens.len() as i32;
            crate::game::amounts::gain(&mut self.player_mut(owner).resources, amount, "spice").expect("tech token income");
            let faction = self.player(owner).faction;
            self.log(parts![faction, " receive ", Payment::of(amount), " from ", token]);
        }
    }

    pub fn current_charity_multiplier(&self) -> i32 {
        if matches!(self.board.choam_economics_status, Some(ChoamEconomicsStatus::Double) | Some(ChoamEconomicsStatus::DoubleFlipped)) {
            return 2;
        }
        if matches!(self.board.choam_economics_status, Some(ChoamEconomicsStatus::Cancel) | Some(ChoamEconomicsStatus::CancelFlipped)) {
            return 0;
        }
        1
    }

    pub fn claim_charity(&mut self, player: PlayerId) {
        let faction = self.player(player).faction;
        self.turn.has_acted_or_passed.push(faction.expect("faction"));

        let basic_amount = (2i32 - i32::from(self.player(player).resources)) * self.current_charity_multiplier();
        self.give_charity(player, basic_amount);

        if !self.player(player).is(Faction::BeneGesserit) {
            self.charity_mut().expect("in the charity phase").resource_tech_token_income = true;
        }

        self.stone(Milestone::CharityClaimed);
    }

    pub fn give_charity(&mut self, to: PlayerId, basic_amount: i32) {
        let mut homeworld_bonus = 0;
        if self.gets_extra_charity_and_free_revival_due_to_low_threshold(to) {
            homeworld_bonus = 1;
        }

        let choam = self.get_player(Some(Faction::Choam));

        let to_faction = self.player(to).faction;
        // The homeworld bonus comes from the bank. The basic amount comes from CHOAM when they
        // control charity, and only if they can pay all of it.
        crate::game::amounts::gain(&mut self.player_mut(to).resources, homeworld_bonus, "spice").expect("charity");
        match choam {
            Some(choam) if !self.prevented(FactionAdvantage::ChoamControllingCharity) => {
                if i32::from(self.player(choam).resources) >= basic_amount {
                    crate::game::amounts::spend(&mut self.player_mut(choam).resources, basic_amount, "spice").expect("charity paid by CHOAM");
                    crate::game::amounts::gain(&mut self.player_mut(to).resources, basic_amount, "spice").expect("charity");
                    self.log(parts![
                        to_faction,
                        " get ",
                        Payment::of(basic_amount),
                        " from ",
                        Faction::Choam,
                        express_part_if!(homeworld_bonus > 0; " and ", Payment::of(homeworld_bonus), " from the bank")
                    ]);
                } else {
                    self.log(parts![
                        to_faction,
                        " are unable to claim ",
                        Payment::of(basic_amount),
                        " from ",
                        Faction::Choam,
                        express_part_if!(homeworld_bonus > 0; " but get ", Payment::of(homeworld_bonus), " from the bank")
                    ]);
                }
            }
            _ => {
                crate::game::amounts::gain(&mut self.player_mut(to).resources, basic_amount, "spice").expect("charity");
                let amount = basic_amount + homeworld_bonus;
                let msg = express!(to_faction, " claim ", Payment::of(amount)).with_log(CharityClaim::new(to_faction.expect("faction"), amount));
                self.log(parts![msg]);
            }
        }
    }
}

impl GameStatus {
    /// The status in the charity phase, or `None` in another phase.
    pub(crate) fn charity_status(_game: &Game, _me: Option<PlayerId>, phase: Phase) -> Option<GameStatus> {
        Some(match phase {
            Phase::ClaimingCharity => Self::status(express!("Factions may now claim charity if eligible."), None),

            _ => return None,
        })
    }
}

impl Game {
    /// What `player` may do in the charity phase (see `add_player_actions`).
    pub(crate) fn add_charity_actions(&self, player: PlayerId, is_host: bool, result: &mut Vec<EventType>) {
        let faction = self.player(player).faction;
        match self.phase() {
            Phase::ClaimingCharity => {
                if !is_host && faction == Some(Faction::Atreides) {
                    result.push(EventType::EndPhase);
                }
                if CharityClaimed::can_be_played(self, player) {
                    result.push(EventType::CharityClaimed);
                }
            }

            _ => {}
        }
    }
}
