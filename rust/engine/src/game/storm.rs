use crate::events::TakeLosses;
use crate::log::storm::{StormDestroyedSpice, StormForecast, StormKilled};
use crate::game::LossToTake;
use crate::prelude::*;
use crate::random::Rng;

/// State of the storm phase: the dials, the Hidden Mobile Stronghold's moves and storm losses.
pub struct StormState {
    pub hms_moves_left: i32,
    pub karma_hms_moves_left: i32,
    pub has_battle_wheel: Vec<Faction>,
    pub dials: Vec<i32>,
    pub current_testing_station_used: bool,
    pub phase_before_storm_loss: Option<Phase>,
    pub storm_losses_to_take: Vec<LossToTake>,
}

impl StormState {
    pub fn new() -> Self {
        StormState {
            hms_moves_left: Default::default(),
            karma_hms_moves_left: 2,
            has_battle_wheel: Vec::new(),
            dials: Vec::new(),
            current_testing_station_used: Default::default(),
            phase_before_storm_loss: Default::default(),
            storm_losses_to_take: Vec::new(),
        }
    }
}

/// `params object?[]` element list for `Log(...)`.
macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(ExpressionElement::from($e)),*] };
}

impl Game {
    pub fn enter_storm_phase(&mut self) {
        self.turn.current_turn += 1;
        self.main_phase_start(MainPhase::Storm, true);
        self.revival.factions_that_revived_special_forces_this_turn.clear();
        self.storm.storm_losses_to_take.clear();
        self.storm.current_testing_station_used = false;

        if self.turn.current_turn == 1 {
            self.storm.dials.clear();
            self.turn.has_acted_or_passed.clear();
            self.enter(Phase::DiallingStorm, true);
        } else if !self.collection.just_revealed_discovery_strongholds.is_empty() {
            self.enter(Phase::BeginningOfStorm, true);
        } else {
            self.move_hms_before_dialling_storm();
        }

        self.determine_occupation_at_start_or_end_of_turn();
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
                self.storm.hms_moves_left = 3;
                self.enter(Phase::HmsMovement, true);
            }
        } else {
            self.determine_storm();
        }
    }

    pub fn determine_later_storm_with_storm_deck(&self) -> i32 {
        self.random.as_ref().expect("random").lock().random_range(1..=6)
    }

    pub fn determine_storm(&mut self) {
        if self.use_storm_deck() {
            self.reveal_storm();
        } else {
            self.storm.dials.clear();
            self.turn.has_acted_or_passed.clear();
            self.enter(Phase::DiallingStorm, true);
        }
    }

    pub fn reveal_storm(&mut self) {
        self.turn.has_acted_or_passed.clear();
        let msg = express!("The storm will move ", self.board.next_storm_moves, " sectors...")
            .with_log(StormForecast::new(self.board.next_storm_moves));
        self.log(elems![msg]);
        self.enter(Phase::MetheorAndStormSpell, true);
    }

    pub fn collect_spice_from(&mut self, faction: Faction, l: Location, maximum_amount: i32) {
        let Some(value) = self.board.resources_on_planet.get(&l).copied() else { return };

        let collected = value.min(maximum_amount);
        self.change_resources_on_planet(l, -collected);
        let Some(collector) = self.get_player(Some(faction)) else { return };
        let r = &mut self.player_mut(collector).resources;
        *r += collected;
        self.log(elems![faction, " collect ", Payment::of(collected), " from ", l]);
    }

    pub fn move_storm_and_determine_next(&mut self, amount: i32) {
        if amount == 0 {
            self.perform_storm();
        }

        for _ in 0..amount.max(0) {
            self.board.sector_in_storm = (self.board.sector_in_storm + 1) % Map::NUMBER_OF_SECTORS;
            self.perform_storm();
        }

        if self.use_storm_deck() {
            self.board.next_storm_moves = self.determine_later_storm_with_storm_deck();
        }

        self.stone(Milestone::Storm);
    }

    pub fn perform_storm(&mut self) {
        // `Forces()` builds a new dictionary, so it is a snapshot of locations. Its
        // battalions are the players' live objects, but every field read
        // from one below is read before anything changes that battalion (only its
        // own player's forces are killed, after the reads), so copies behave the same.
        let forces = self.forces(false);
        for (key, value) in forces.iter() {
            let key = *key;
            // The Where predicate is checked lazily per element; SectorInStorm does not
            // change inside the loop.
            if !(key.sector(&self.board.map) == self.board.sector_in_storm && !self.is_protected_from_storm(key)) {
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
                        loss.amount = TakeLosses::half_of(battalion.amount_of_forces, battalion.amount_of_special_forces);
                        loss.faction = Some(battalion.faction);
                        self.storm.storm_losses_to_take.push(loss);
                    } else {
                        kill_count += self.player(player).any_forces_in(Some(key));
                        killed_forces += self.player(player).forces_in(Some(key));
                        killed_special_forces += self.player(player).special_forces_in(Some(key));
                        Player::kill_all_forces(self, player, key, false);
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
                    self.storm.storm_losses_to_take.push(loss);
                } else {
                    self.reveal_current_no_field(player, Some(key));
                    kill_count += self.player(player).any_forces_in(Some(key));
                    killed_forces += self.player(player).forces_in(Some(key));
                    killed_special_forces += self.player(player).special_forces_in(Some(key));
                    Player::kill_all_forces(self, player, key, false);
                }

                if kill_count > 0 {
                    let msg = express!("The storm kills ", kill_count, battalion.faction, " forces in ", key)
                        .with_log(StormKilled::new(battalion.faction, key, killed_forces, killed_special_forces));
                    self.log(elems![msg]);
                }
            }
        }

        self.flip_bene_gesserit_advisors_when_alone();

        // `Strongholds` and the predicate only read static data, SectorInStorm and
        // ShieldWallDestroyed, none of which the body changes.
        let strongholds: Vec<Territory> = self
            .strongholds()
            .into_iter()
            .filter(|s| s.locations().iter().any(|l| l.sector(&self.board.map) == self.board.sector_in_storm && !self.is_protected_from_storm(*l)))
            .collect();
        for t in strongholds {
            if let Some(ambassador) = self.ambassador_in(t) {
                let Some(ecaz) = self.get_player(Some(Faction::Ecaz)) else { continue };
                self.board.ambassadors_on_planet.shift_remove(&t);
                self.player_mut(ecaz).ambassadors.push(ambassador);
                self.log(elems!["The ambassador in ", t, " returns to ", Faction::Ecaz]);
            }
        }

        let locations: Vec<Location> = self.board.map.locations(false).filter(|l| l.sector(&self.board.map) == self.board.sector_in_storm).collect();
        for l in locations {
            let removed = self.remove_resources(l);
            if removed > 0 {
                let msg = express!("The storm destroys ", Payment::of(removed), " in ", l)
                    .with_log(StormDestroyedSpice::new(l, removed));
                self.log(elems![msg]);
            }
        }
    }

    pub fn end_storm_phase(&mut self) {
        self.collection.just_revealed_discovery_strongholds.clear();

        self.main_phase_end();

        if !self.storm.storm_losses_to_take.is_empty() {
            // BlowA marks losses taken in the storm phase: once they are taken, TakeLosses calls
            // this again, which ends the phase (again) and enters the storm report.
            self.storm.phase_before_storm_loss = Some(Phase::BlowA);
            self.enter(Phase::StormLosses, true);
        } else {
            self.enter(Phase::StormReport, true);
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
