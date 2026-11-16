use crate::events::TakeLosses;
use crate::log::storm::{StormDestroyedSpice, StormForecast, StormKilled};
use crate::game::LossToTake;
use crate::prelude::*;
use crate::messages::part::Part;
use crate::random::Rng;

/// The storm phase (`PhaseState::Storm`): its step, the dials and whether the Testing Station
/// was used. It is dropped when the phase ends.
pub struct StormState {
    pub step: StormStep,
    pub dials: Vec<i32>,
    pub current_testing_station_used: bool,
}

impl StormState {
    pub fn new(step: StormStep) -> Self {
        StormState { step, dials: Vec::new(), current_testing_station_used: false }
    }
}

/// Where the storm phase is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StormStep {
    /// Before the storm moves, while forces may enter a just revealed discovery stronghold.
    Beginning,
    /// The Ixian moves the Hidden Mobile Stronghold.
    HmsMovement,
    /// The two factions with a battle wheel dial the storm.
    Dialling,
    /// The first turn: the Ixian places the Hidden Mobile Stronghold.
    HmsPlacement,
    /// Factions may play Weather Control, Family Atomics or the Testing Station.
    MetheorAndStormSpell,
    /// Fremen (or CHOAM) decide which forces the storm killed (`storm_losses_to_take`).
    Losses,
    Report,
}

impl StormStep {
    pub fn phase(self) -> Phase {
        match self {
            StormStep::Beginning => Phase::BeginningOfStorm,
            StormStep::HmsMovement => Phase::HmsMovement,
            StormStep::Dialling => Phase::DiallingStorm,
            StormStep::HmsPlacement => Phase::HmsPlacement,
            StormStep::MetheorAndStormSpell => Phase::MetheorAndStormSpell,
            StormStep::Losses => Phase::StormLosses,
            StormStep::Report => Phase::StormReport,
        }
    }

    pub fn of(phase: Phase) -> Option<StormStep> {
        let step = match phase {
            Phase::BeginningOfStorm => StormStep::Beginning,
            Phase::HmsMovement => StormStep::HmsMovement,
            Phase::DiallingStorm => StormStep::Dialling,
            Phase::HmsPlacement => StormStep::HmsPlacement,
            Phase::MetheorAndStormSpell => StormStep::MetheorAndStormSpell,
            Phase::StormLosses => StormStep::Losses,
            Phase::StormReport => StormStep::Report,
            _ => return None,
        };
        Some(step)
    }
}

/// `params object?[]` element list for `Log(...)`.
macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(Part::from($e)),*] };
}

impl Game {
    /// The storm phase's state, while the game is in it (an interrupt may suspend it).
    pub fn storm(&self) -> Option<&StormState> {
        match &self.phase {
            PhaseState::Storm(s) => Some(s),
            _ => None,
        }
    }

    pub fn storm_mut(&mut self) -> Option<&mut StormState> {
        match &mut self.phase {
            PhaseState::Storm(s) => Some(s),
            _ => None,
        }
    }

    /// The storm phase's state, for code that only runs in the storm phase.
    pub(crate) fn storm_state(&mut self) -> &mut StormState {
        self.storm_mut().expect("the storm phase")
    }

    /// Moves the storm phase to `step`, dropping any interrupt, as entering a phase did. Starts
    /// the storm phase if the game isn't in it yet: [`Game::enter_storm_phase`] enters its first
    /// step only once it knows which one that is, so no deals end at a step it skips.
    pub fn enter_storm(&mut self, step: StormStep) {
        match self.storm_mut() {
            Some(s) => {
                s.step = step;
                self.interrupts.clear();
                self.remove_ended_deals(step.phase());
            }
            None => self.enter_state(PhaseState::Storm(StormState::new(step))),
        }
    }

    pub fn enter_storm_phase(&mut self) -> GameResult {
        self.turn.current_turn = self.turn.current_turn.next().expect("a game has at most 255 turns");
        self.main_phase_start(MainPhase::Storm, true);
        self.turn.revivals.factions_that_revived_special_forces_this_turn.clear();
        self.turn.storm_losses_to_take.clear();

        // The new storm state has no dials yet and the Testing Station unused.
        if self.turn.current_turn == Turn::FIRST {
            self.turn.has_acted_or_passed.clear();
            self.enter_storm(StormStep::Dialling);
        } else if !self.board.just_revealed_discovery_strongholds.is_empty() {
            self.enter_storm(StormStep::Beginning);
        } else {
            self.move_hms_before_dialling_storm();
        }

        self.determine_occupation_at_start_or_end_of_turn();
        Ok(())
    }

    pub fn move_hms_before_dialling_storm(&mut self) {
        let ixian = self.get_player(Some(Faction::Ixian));
        let hms = self.board.map.hidden_mobile_stronghold();
        if let Some(_ixian) = ixian.filter(|g| self.player(*g).any_forces_in(Some(hms.location())) > 0) {
            if self.prevented(FactionAdvantage::IxianMovingHms) {
                self.log_prevention_by_karma(FactionAdvantage::IxianMovingHms);
                if !self.applicable(Rule::FullPhaseKarma) {
                    self.allow(FactionAdvantage::IxianMovingHms);
                }
                self.determine_storm();
            } else if self.has_low_threshold(Faction::Ixian) {
                self.log_prevention_by_low_threshold(FactionAdvantage::IxianMovingHms);
                self.determine_storm();
            } else if hms.attached_to_location(&self.board.map).map(|l| l.sector(&self.board.map)) == Some(self.board.sector_in_storm) {
                self.log(elems!["The storm prevents ", hms, " from moving"]);
                self.determine_storm();
            } else {
                self.enter_storm(StormStep::HmsMovement);
                self.board.hms_moves_left = 3;
            }
        } else {
            self.determine_storm();
        }
    }

    pub fn determine_later_storm_with_storm_deck(&mut self) -> i32 {
        self.rng.random_range(1..=6)
    }

    pub fn determine_storm(&mut self) {
        if self.use_storm_deck() {
            self.reveal_storm();
        } else {
            self.turn.has_acted_or_passed.clear();
            self.enter_storm(StormStep::Dialling);
            self.storm_state().dials.clear();
        }
    }

    pub fn reveal_storm(&mut self) {
        self.turn.has_acted_or_passed.clear();
        let msg = express!("The storm will move ", self.board.next_storm_moves, " sectors...")
            .with_log(StormForecast::new(self.board.next_storm_moves));
        self.log(elems![msg]);
        self.enter_storm(StormStep::MetheorAndStormSpell);
    }

    pub fn collect_spice_from(&mut self, faction: Faction, l: Location, maximum_amount: i32) -> GameResult {
        let Some(value) = self.board.resources_on_planet.get(&l).copied() else { return Ok(()) };

        let collected = i32::from(value).min(maximum_amount);
        self.change_resources_on_planet(l, -collected)?;
        let Some(collector) = self.get_player(Some(faction)) else { return Ok(()) };
        crate::game::amounts::gain(&mut self.player_mut(collector).resources, collected, "spice")?;
        self.log(elems![faction, " collect ", Payment::of(collected), " from ", l]);
        Ok(())
    }

    pub fn move_storm_and_determine_next(&mut self, amount: i32) -> GameResult {
        if amount == 0 {
            self.perform_storm()?;
        }

        for _ in 0..amount.max(0) {
            // Before the first storm (which the rules never move) the storm starts from sector 0.
            self.board.sector_in_storm = Some(self.board.sector_in_storm.map_or(Sector::new(0), |s| s.moved(1)));
            self.perform_storm()?;
        }

        if self.use_storm_deck() {
            self.board.next_storm_moves = self.determine_later_storm_with_storm_deck();
        }

        self.stone(Milestone::Storm);
        Ok(())
    }

    pub fn perform_storm(&mut self) -> GameResult {
        // `Forces()` builds a new dictionary, so it is a snapshot of locations. Its
        // battalions are the players' live objects, but every field read
        // from one below is read before anything changes that battalion (only its
        // own player's forces are killed, after the reads), so copies behave the same.
        let forces = self.forces(false);
        for (key, value) in forces.iter() {
            let key = *key;
            // The Where predicate is checked lazily per element; SectorInStorm does not
            // change inside the loop.
            if !(Sector::in_storm(key.sector(&self.board.map), self.board.sector_in_storm) && !self.is_protected_from_storm(key)) {
                continue;
            }

            for battalion in value.iter() {
                let Some(player) = self.get_player(Some(battalion.faction)) else { continue };

                let mut kill_count = 0;
                let mut killed_forces = 0;
                let mut killed_special_forces = 0;
                if battalion.is(Faction::Fremen) && self.applicable(Rule::FremenStormLosses) {
                    if !self.prevented(FactionAdvantage::FremenProtectedFromStorm) {
                        let mut loss = LossToTake::new();
                        loss.location = Some(key);
                        loss.amount = TakeLosses::half_of(battalion.forces(), battalion.special_forces());
                        loss.faction = Some(battalion.faction);
                        self.turn.storm_losses_to_take.push(loss);
                    } else {
                        kill_count += self.player(player).any_forces_in(Some(key));
                        killed_forces += self.player(player).forces_in(Some(key));
                        killed_special_forces += self.player(player).special_forces_in(Some(key));
                        Player::kill_all_forces(self, player, key, false)?;
                        if !self.applicable(Rule::FullPhaseKarma) {
                            self.allow(FactionAdvantage::FremenProtectedFromStorm);
                        }
                    }
                } else if battalion.is(Faction::Choam)
                    && (!self.prevented(FactionAdvantage::ChoamDiscarding) || self.applicable(Rule::NexusCards))
                {
                    let mut loss = LossToTake::new();
                    loss.location = Some(key);
                    loss.amount = battalion.total_amount_of_forces();
                    loss.faction = Some(battalion.faction);
                    self.turn.storm_losses_to_take.push(loss);
                } else {
                    self.reveal_current_no_field(player, Some(key))?;
                    kill_count += self.player(player).any_forces_in(Some(key));
                    killed_forces += self.player(player).forces_in(Some(key));
                    killed_special_forces += self.player(player).special_forces_in(Some(key));
                    Player::kill_all_forces(self, player, key, false)?;
                }

                if kill_count > 0 {
                    let msg = express!("The storm kills ", kill_count, battalion.faction, " forces in ", key)
                        .with_log(StormKilled::new(battalion.faction, key, killed_forces, killed_special_forces));
                    self.log(elems![msg]);
                }
            }
        }

        self.flip_bene_gesserit_advisors_when_alone()?;

        // `Strongholds` and the predicate only read static data, SectorInStorm and
        // ShieldWallDestroyed, none of which the body changes.
        let strongholds: Vec<Territory> = self
            .strongholds()
            .into_iter()
            .filter(|s| s.locations().iter().any(|l| Sector::in_storm(l.sector(&self.board.map), self.board.sector_in_storm) && !self.is_protected_from_storm(*l)))
            .collect();
        for t in strongholds {
            if let Some(ambassador) = self.ambassador_in(t) {
                let Some(ecaz) = self.get_player(Some(Faction::Ecaz)) else { continue };
                self.board.ambassadors_on_planet.shift_remove(&t);
                self.player_mut(ecaz).ambassadors.push(ambassador);
                self.log(elems!["The ambassador in ", t, " returns to ", Faction::Ecaz]);
            }
        }

        let locations: Vec<Location> = self.board.map.locations(false).filter(|l| Sector::in_storm(l.sector(&self.board.map), self.board.sector_in_storm)).collect();
        for l in locations {
            let removed = self.remove_resources(l);
            if removed > 0 {
                let msg = express!("The storm destroys ", Payment::of(removed), " in ", l)
                    .with_log(StormDestroyedSpice::new(l, removed));
                self.log(elems![msg]);
            }
        }
        Ok(())
    }

    pub fn end_storm_phase(&mut self) {
        self.board.just_revealed_discovery_strongholds.clear();

        self.main_phase_end();

        if !self.turn.storm_losses_to_take.is_empty() {
            // Once they are taken, TakeLosses calls this again, which ends the phase (again) and
            // enters the storm report.
            self.enter_storm(StormStep::Losses);
        } else {
            self.enter_storm(StormStep::Report);
        }
    }

    pub fn use_storm_deck(&self) -> bool {
        self.applicable(Rule::StormDeckWithoutFremen) || (self.is_playing(Faction::Fremen) && self.applicable(Rule::FremenDeterminesStorm))
    }

    pub fn is_protected_from_storm(&self, l: Location) -> bool {
        if !self.board.shield_wall_destroyed {
            return l.territory().is_protected_from_storm();
        }
        l.territory().is_protected_from_storm()
            && !(l.id() == self.board.map.arrakeen().id() || l.territory() == self.board.map.imperial_basin() || l.id() == self.board.map.carthag().id())
    }
}

impl GameStatus {
    /// The status in the storm phase, or `None` in another phase.
    pub(crate) fn storm_status(game: &Game, _me: Option<PlayerId>, phase: Phase) -> Option<GameStatus> {
        let g = game;
        Some(match phase {
            Phase::HmsMovement => {
                let a = express!("You may move the Hidden Mobile Stronghold to a sector in an adjacent territory, or pass.");
                let w = express!(Faction::Ixian, " are moving the Hidden Mobile Stronghold...");
                Self::faction_status(game, a, w, Some(Faction::Ixian), None)
            }

            Phase::DiallingStorm => {
                let factions: Vec<Faction> = g.rules.factions_in_play.iter().copied().filter(|f| g.board.has_battle_wheel.contains(f) && !g.turn.has_acted_or_passed.contains(f)).collect();
                Self::factions_status(g, "Please dial a number to determine storm movement.", "Storm movement is being determined...", &factions, None)
            }

            Phase::MetheorAndStormSpell => Self::status(express!("Factions may now use ", TreacheryCardType::Metheor, " or ", TreacheryCardType::StormSpell, "..."), None),

            Phase::StormLosses => {
                let a = express!("Please decide which forces were killed by the storm in ", TakeLosses::losses_to_take(g).location, ".");
                let w = express!(TakeLosses::losses_to_take(g).faction, " are deciding which forces were killed by the storm in ", TakeLosses::losses_to_take(g).location, "...");
                let f = g.turn.storm_losses_to_take[0].faction;
                Self::faction_status(g, a, w, f, None)
            }

            _ => return None,
        })
    }
}

impl Game {
    /// What `player` may do in the storm phase (see `add_player_actions`).
    pub(crate) fn add_storm_actions(&self, player: PlayerId, _is_host: bool, result: &mut Vec<EventType>) {
        let faction = self.player(player).faction;
        match self.phase() {
            Phase::DiallingStorm => {
                let f = self.player(player).faction;
                if self.board.has_battle_wheel.contains_opt(f) && !self.turn.has_acted_or_passed.contains_opt(f) {
                    result.push(EventType::StormDialled);
                }
            }

            Phase::BeginningOfStorm => {
                if !DiscoveryEntered::valid_targets(self, player).next().is_none() {
                    result.push(EventType::DiscoveryEntered);
                }
            }

            Phase::HmsMovement => {
                if faction == Some(Faction::Ixian) {
                    result.push(EventType::PerformHmsMovement);
                }
            }

            Phase::MetheorAndStormSpell => {
                if self.player(player).has_treachery_card_type(TreacheryCardType::StormSpell) && self.turn.current_turn > Turn::FIRST {
                    result.push(EventType::StormSpellPlayed);
                }
                if MetheorPlayed::may_play_metheor(self, player) {
                    result.push(EventType::MetheorPlayed);
                }
                if TestingStationUsed::can_be_played(self, player) {
                    result.push(EventType::TestingStationUsed);
                }
            }

            Phase::StormLosses => {
                if faction == TakeLosses::losses_to_take(self).faction {
                    result.push(EventType::TakeLosses);
                }
            }

            _ => {}
        }
    }
}
