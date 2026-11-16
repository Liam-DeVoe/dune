//! Where the game is: the phase it is in, with that phase's own state, and the interrupts that
//! suspend it (rewrite-plan.md, section 1 "Game", and wave 5).
//!
//! `game.phase` is a [`PhaseState`]: the main phase being played and its step, holding the state
//! that only that phase uses, which is dropped when the phase ends. `game.interrupts` is a stack
//! of [`Interrupt`]s: things that suspend the phase until they are resolved (a player discards a
//! card, answers a Clairvoyance question, ...). The game waits on the top interrupt if there is
//! one, otherwise on the phase. [`Game::phase`] computes the flat [`Phase`] from both, because
//! views, the client and the bots' timing key on its names.
//!
//! Phases move over one at a time. Until a phase has its own variant, it is
//! `PhaseState::Legacy(phase)`: just the name, with its state in the old per-phase struct
//! (`game.storm`, `game.bidding`, ...), entered with [`Game::enter`].

use crate::prelude::*;

/// The phase the game is in, with that phase's state.
pub enum PhaseState {
    /// A phase that has no variant of its own yet: only its name. Its state is still in the
    /// per-phase struct on `Game`.
    Legacy(Phase),
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
            PhaseState::Legacy(phase) => *phase,
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

    /// Whether `phase` is a step of a phase that has its own variant. Such a phase is entered
    /// by its own functions (`Game::enter_collection`), never by name with [`Game::enter`],
    /// because entering it by name can't build its state.
    pub fn is_converted(phase: Phase) -> bool {
        BattleStep::of(phase).is_some()
            || CharityStep::of(phase).is_some()
            || BiddingStep::of(phase).is_some()
            || RevivalStep::of(phase).is_some()
            || ShipMoveStep::of(phase).is_some()
            || CollectionStep::of(phase).is_some()
            || ContemplateStep::of(phase).is_some()
            || phase == Phase::GameEnded
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
    /// A suspension not converted yet, by the phase it shows: storm losses taken during a
    /// shipment or move.
    Legacy(Phase),
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
            Interrupt::SkillAssignment => Phase::AssigningSkill,
            Interrupt::Intrusion { kind, cause } => cause.intrusion_phase(*kind),
            Interrupt::AllianceByTerror { .. } => Phase::AllianceByTerror,
            Interrupt::AllianceByAmbassador { .. } => Phase::AllianceByAmbassador,
            Interrupt::Legacy(phase) => *phase,
        }
    }

    /// The interrupt that a phase of that name is, if it is one that holds no data. The others
    /// are pushed by the code that knows their data, never by name: an intrusion or an alliance
    /// offer by its owner ([`Interrupt::intrusion`]), `PerformingKarmaHandSwap` by
    /// `KarmaHandSwapInitiated`, and `Discarding`, `DiscardingTraitor`, `TradingCards` and
    /// `Bureaucracy` where they start.
    pub fn of(phase: Phase) -> Option<Interrupt> {
        let i = match phase {
            Phase::SearchingDiscarded => Interrupt::SearchingDiscarded,
            Phase::Clairvoyance => Interrupt::Clairvoyance,
            Phase::AssigningSkill => Interrupt::SkillAssignment,
            Phase::StormLosses => Interrupt::Legacy(phase),
            _ => return None,
        };
        Some(i)
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

    /// The phase state under the interrupts.
    pub fn phase_state(&self) -> &PhaseState {
        &self.phase
    }

    /// Enters a phase by name. While phases move to [`PhaseState`] one at a time, this keeps
    /// the old behavior, where entering a phase replaced the one before and a suspended phase
    /// was resumed by entering it again:
    ///
    /// - a phase that is on the stack (an interrupt, or the phase state under them) is
    ///   resumed: the interrupts above it are dropped;
    /// - a phase that is an interrupt ([`Interrupt::of`]) is pushed;
    /// - any other phase replaces the phase state, and the interrupts are dropped.
    ///
    /// A phase that has its own variant ([`PhaseState::is_converted`]) can only be resumed
    /// this way; its owner enters it.
    ///
    /// `cleanup_deals` ends the deals that last until `phase`.
    pub fn enter(&mut self, phase: Phase, cleanup_deals: bool) {
        if let Some(i) = self.interrupts.iter().rposition(|i| i.phase() == phase) {
            self.interrupts.truncate(i + 1);
        } else if self.phase.phase() == phase {
            self.interrupts.clear();
        } else if let Some(i) = Interrupt::of(phase) {
            self.interrupts.push(i);
        } else {
            assert!(!PhaseState::is_converted(phase), "{phase} has its own phase state; enter it with its own function");
            assert!(
                !matches!(phase, Phase::Discarding | Phase::DiscardingTraitor | Phase::TradingCards | Phase::Bureaucracy),
                "{phase} is an interrupt with data; push it with Game::interrupt"
            );
            self.interrupts.clear();
            self.phase = PhaseState::Legacy(phase);
        }

        if cleanup_deals {
            self.remove_ended_deals(phase);
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
