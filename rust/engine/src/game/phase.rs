//! Where the game is: the phase it is in, with that phase's own state, and the interrupts that
//! suspend it (rewrite-plan.md, section 1 "Game", and wave 5).
//!
//! `game.phase` is a [`PhaseState`]: the main phase being played and its step, holding the state
//! that only that phase uses, which is dropped when the phase ends. `game.interrupts` is a stack
//! of [`Interrupt`]s: things that suspend the phase until they are resolved (a player discards a
//! card, answers a Clairvoyance question, ...). The game waits on the top interrupt if there is
//! one, otherwise on the phase. [`Game::phase`] computes the flat [`Phase`] from both, because
//! views, the client and the bots' timing key on its names.

use crate::prelude::*;

/// The phase the game is in, with that phase's state.
pub enum PhaseState {
    /// Before the game starts: the host configures it and players join.
    AwaitingPlayers,
    Setup(SetupState),
    Storm(StormState),
    Blow(BlowState),
    Battle(BattlePhase),
    Charity(CharityState),
    Bidding(BiddingState),
    Revival(RevivalState),
    ShipMove(ShipMoveState),
    Collection(CollectionState),
    Contemplate(ContemplateState),
    /// The game is over (`Phase::GameEnded`).
    Ended(Ending),
}

impl PhaseState {
    /// The flat phase this state is in.
    pub fn phase(&self) -> Phase {
        match self {
            PhaseState::AwaitingPlayers => Phase::AwaitingPlayers,
            PhaseState::Setup(s) => s.step.phase(),
            PhaseState::Storm(s) => s.step.phase(),
            PhaseState::Blow(b) => b.step.phase(),
            PhaseState::Battle(b) => b.step.phase(),
            PhaseState::Charity(c) => c.step.phase(),
            PhaseState::Bidding(b) => b.step.phase(),
            PhaseState::Revival(r) => r.step.phase(),
            PhaseState::ShipMove(s) => s.step.phase(),
            PhaseState::Collection(c) => c.step.phase(),
            PhaseState::Contemplate(c) => c.step.phase(),
            PhaseState::Ended(_) => Phase::GameEnded,
        }
    }
}

/// Something that suspends the phase until it is resolved. When it is, the game goes back to
/// what it suspended ([`Game::resume`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Interrupt {
    /// `factions` must each discard a card, to their hand limit or by choice. A faction listed
    /// twice discards twice.
    Discarding { factions: Vec<Faction> },
    /// `faction` puts `remaining` traitors back after a Nexus card gave them new ones.
    DiscardingTraitor { faction: Option<Faction>, remaining: i32 },
    /// A player takes a card from the treachery discard pile.
    SearchingDiscarded,
    /// The target of a Clairvoyance question answers it.
    Clairvoyance,
    /// The ally answers a card trade offer.
    CardTrade { offer: CardTradeOffer },
    /// The Karama hand swap: Harkonnen return `number_of_cards` cards to `target`.
    KarmaHandSwap { number_of_cards: i32, target: Option<Faction> },
    /// The Bureaucrat decides whether `target`, who just received a payment, loses 2 spice.
    Bureaucracy { target: Option<Faction> },
    /// Fremen (or CHOAM) decide which forces a shipment or move into the storm lost
    /// (`storm_losses_to_take`). Storm losses in the storm phase are a storm step instead.
    StormLosses,
    /// A player chooses the skill of a revived leader.
    SkillAssignment,
    /// Forces entered a territory and the Bene Gesserit, a terror token or an ambassador there
    /// may react (`turn.intrusions` holds the ones waiting). Build it with
    /// [`Interrupt::intrusion`].
    Intrusion { kind: IntrusionType, cause: IntrusionCause },
    /// Moritani offered an alliance instead of terror, and `offered_to` decides.
    AllianceByTerror { offered_to: Option<Faction> },
    /// An Ecaz ambassador offered an alliance, and `offered_to` decides. `offered_by` activated
    /// the ambassador (Ecaz, or their ally), and the two flags are its Vidal options.
    AllianceByAmbassador { offered_to: Faction, offered_by: By, give_vidal_to_ally: bool, take_vidal: bool },
}

impl Interrupt {
    /// The flat phase the game shows while this interrupt is on top.
    pub fn phase(&self) -> Phase {
        match self {
            Interrupt::Discarding { .. } => Phase::Discarding,
            Interrupt::DiscardingTraitor { .. } => Phase::DiscardingTraitor,
            Interrupt::SearchingDiscarded => Phase::SearchingDiscarded,
            Interrupt::Clairvoyance => Phase::Clairvoyance,
            Interrupt::CardTrade { .. } => Phase::TradingCards,
            Interrupt::KarmaHandSwap { .. } => Phase::PerformingKarmaHandSwap,
            Interrupt::Bureaucracy { .. } => Phase::Bureaucracy,
            Interrupt::StormLosses => Phase::StormLosses,
            Interrupt::SkillAssignment => Phase::AssigningSkill,
            Interrupt::Intrusion { kind, cause } => cause.intrusion_phase(*kind),
            Interrupt::AllianceByTerror { .. } => Phase::AllianceByTerror,
            Interrupt::AllianceByAmbassador { .. } => Phase::AllianceByAmbassador,
        }
    }
}

impl Game {
    /// The flat phase the game is in: the top interrupt's, or else the phase state's.
    pub fn phase(&self) -> Phase {
        match self.interrupts.last() {
            Some(i) => i.phase(),
            None => self.phase.phase(),
        }
    }

    /// Replaces the phase state, dropping every interrupt, and ends the deals that last until
    /// the phase it shows.
    pub fn enter_state(&mut self, state: PhaseState) {
        self.interrupts.clear();
        self.phase = state;
        let phase = self.phase.phase();
        self.remove_ended_deals(phase);
    }

    /// Suspends the phase with `interrupt`, and ends the deals that last until it.
    pub fn interrupt(&mut self, interrupt: Interrupt) {
        let phase = interrupt.phase();
        self.interrupts.push(interrupt);
        self.remove_ended_deals(phase);
    }

    /// As [`Game::resume`], for the latest interrupt that shows `phase`, whatever its data.
    pub fn resume_phase(&mut self, phase: Phase, cleanup_deals: bool) -> GameResult {
        let Some(i) = self.interrupts.iter().rposition(|x| x.phase() == phase) else {
            game_error!("Nothing to resume after {}", phase);
        };
        let interrupt = self.interrupts[i].clone();
        self.resume(interrupt, cleanup_deals)
    }

    /// The latest interrupt `pick` accepts, as `pick` returns it.
    fn latest_interrupt<'a, T>(&'a self, pick: impl Fn(&'a Interrupt) -> Option<T>) -> Option<T> {
        self.interrupts.iter().rev().find_map(pick)
    }

    fn latest_interrupt_mut<'a, T>(&'a mut self, pick: impl Fn(&'a mut Interrupt) -> Option<T>) -> Option<T> {
        self.interrupts.iter_mut().rev().find_map(pick)
    }

    /// The factions that must still discard, while `Discarding` suspends the game.
    pub fn factions_that_must_discard(&self) -> Option<&Vec<Faction>> {
        self.latest_interrupt(|i| match i {
            Interrupt::Discarding { factions } => Some(factions),
            _ => None,
        })
    }

    pub fn factions_that_must_discard_mut(&mut self) -> Option<&mut Vec<Faction>> {
        self.latest_interrupt_mut(|i| match i {
            Interrupt::Discarding { factions } => Some(factions),
            _ => None,
        })
    }

    /// Who must put traitors back and how many are left, while `DiscardingTraitor` suspends
    /// the game.
    pub fn traitors_to_discard(&self) -> Option<(Option<Faction>, i32)> {
        self.latest_interrupt(|i| match i {
            Interrupt::DiscardingTraitor { faction, remaining } => Some((*faction, *remaining)),
            _ => None,
        })
    }

    pub fn traitors_to_discard_mut(&mut self) -> Option<&mut i32> {
        self.latest_interrupt_mut(|i| match i {
            Interrupt::DiscardingTraitor { remaining, .. } => Some(remaining),
            _ => None,
        })
    }

    /// The pending card trade offer, while its target is to answer it.
    pub fn card_trade_offer(&self) -> Option<&CardTradeOffer> {
        self.latest_interrupt(|i| match i {
            Interrupt::CardTrade { offer } => Some(offer),
            _ => None,
        })
    }

    /// The faction Bureaucracy would make lose spice, while the Bureaucrat decides.
    pub fn target_of_bureaucracy(&self) -> Option<Faction> {
        self.latest_interrupt(|i| match i {
            Interrupt::Bureaucracy { target } => Some(*target),
            _ => None,
        })
        .flatten()
    }

    pub fn target_of_bureaucracy_mut(&mut self) -> Option<&mut Option<Faction>> {
        self.latest_interrupt_mut(|i| match i {
            Interrupt::Bureaucracy { target } => Some(target),
            _ => None,
        })
    }

    /// Goes back to what the latest `interrupt` suspended, dropping it and anything pushed
    /// after it. With `cleanup_deals`, ends the deals that last until the phase it goes back to.
    pub fn resume(&mut self, interrupt: Interrupt, cleanup_deals: bool) -> GameResult {
        let Some(i) = self.interrupts.iter().rposition(|x| *x == interrupt) else {
            game_error!("Nothing to resume after {:?}", interrupt);
        };
        self.interrupts.truncate(i);
        if cleanup_deals {
            let phase = self.phase();
            self.remove_ended_deals(phase);
        }
        Ok(())
    }

    /// The phase the game would show if the latest `interrupt` were resolved.
    pub fn phase_under(&self, interrupt: Interrupt) -> Option<Phase> {
        let i = self.interrupts.iter().rposition(|x| *x == interrupt)?;
        Some(match i.checked_sub(1) {
            Some(below) => self.interrupts[below].phase(),
            None => self.phase.phase(),
        })
    }

    /// Whether `interrupt` is suspending the game, on top or under another.
    pub fn is_interrupted_by(&self, interrupt: Interrupt) -> bool {
        self.interrupts.contains(&interrupt)
    }
}

impl GameStatus {
    /// The status while a card or skill interrupt is on top, or `None`.
    pub(crate) fn interrupt_status(game: &Game, _me: Option<PlayerId>, phase: Phase) -> Option<GameStatus> {
        let g = game;
        Some(match phase {
            Phase::Clairvoyance => {
                let a = express!("Please answer a question from ", g.turn.latest_clairvoyance.as_ref().map(|c| c.initiator()), " by ", TreacheryCardType::Clairvoyance, ".");
                let w = express!("Waiting for an answer to a ", TreacheryCardType::Clairvoyance, " question...");
                let f = g.turn.latest_clairvoyance.as_ref().expect("latest clairvoyance").target;
                Self::faction_status(g, a, w, f, None)
            }

            Phase::SearchingDiscarded => {
                let p = g.owner_of_treachery_card_type(TreacheryCardType::SearchDiscarded);
                Self::player_status("Please take a card from the treachery discard pile.", "Waiting for a card to be taken from the treachery discard pile...", p, None)
            }

            Phase::TradingCards => {
                let offer_player = g.card_trade_offer().expect("current card trade offer").player();
                let f = g.player(offer_player).ally;
                Self::faction_status(g, "Please select which card to give to your ally.", "Waiting for a card to be returned...", f, None)
            }

            Phase::Bureaucracy => {
                let p = g.player_skilled_as(LeaderSkill::Bureaucrat);
                Self::player_status("Please decide whether to apply Bureaucracy to the latest payment.", "Waiting for Bureaucracy to be applied to the latest payment...", p, None)
            }

            Phase::Discarding => {
                Self::factions_status(g, "Please decide which card to discard.", "Waiting for factions to discard...", g.factions_that_must_discard().map_or(&[], |f| f), None)
            }

            Phase::PerformingKarmaHandSwap => {
                let (number_of_cards, target) = game.karma_hand_swap().expect("a Karama hand swap");
                let a = express!("Please decide which ", number_of_cards, " cards to return to ", target, ".");
                let w = express!(Faction::Harkonnen, " are deciding which ", number_of_cards, " cards to return to ", target, "...");
                Self::faction_status(game, a, w, Some(Faction::Harkonnen), None)
            }

            _ => return None,
        })
    }
}

impl Game {
    /// What `player` may do while a card or skill interrupt is on top (see `add_player_actions`).
    pub(crate) fn add_interrupt_actions(&self, player: PlayerId, _is_host: bool, result: &mut Vec<EventType>) {
        let faction = self.player(player).faction;
        match self.phase() {
            Phase::Discarding => {
                if self.factions_that_must_discard().is_some_and(|f| f.contains_opt(faction)) {
                    result.push(EventType::Discarded);
                }
            }

            Phase::DiscardingTraitor => {
                if self.traitors_to_discard().is_some_and(|(f, _)| f == faction) {
                    result.push(EventType::TraitorDiscarded);
                }
            }

            Phase::Bureaucracy => {
                if Some(player) == self.player_skilled_as(LeaderSkill::Bureaucrat) {
                    result.push(EventType::Bureaucracy);
                }
            }

            Phase::PerformingKarmaHandSwap => {
                if faction == Some(Faction::Harkonnen) && !self.karma_prevented(faction) {
                    result.push(EventType::KarmaHandSwap);
                }
            }

            Phase::TradingCards => {
                if Some(faction) == self.card_trade_offer().map(|o| o.target) {
                    result.push(EventType::CardTraded);
                }
            }

            Phase::Clairvoyance => {
                if Some(faction) == self.turn.latest_clairvoyance.as_ref().map(|c| c.target) {
                    result.push(EventType::ClairVoyanceAnswered);
                }
            }

            Phase::SearchingDiscarded => {
                if DiscardedSearched::can_be_played(self, player) {
                    result.push(EventType::DiscardedSearched);
                }
            }

            _ => {}
        }
    }
}
