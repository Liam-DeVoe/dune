use crate::prelude::*;
use crate::messages::part::Part;

/// The shipment and movement phase (`PhaseState::ShipMove`): its step, the order of play, and
/// what only matters until the phase ends. The intrusions and alliance offers that suspend it
/// are interrupts. What outlives the phase is elsewhere: the moves on the board, the flights and
/// blocked territories in `turn`, the Guild's shipment permissions in `alliances`, the Richese
/// No-Field values on `board`.
pub struct ShipMoveState {
    pub step: ShipMoveStep,
    /// Who ships and moves, in turn.
    pub sequence: PlayerSequence,
    /// Some faction shipped, so the owner of the Ships tech token gets income at the end.
    pub ships_tech_token_income: bool,
    /// The Guild's Karama, preventing one faction from shipping this phase.
    pub karma_shipment_prevention: Option<ShipmentPrevention>,
    /// The player who just moved may move again (a Flight card).
    pub current_player_may_perform_extra_move: bool,
    /// The Guild ship and move a second time by their Nexus cunning.
    pub in_guild_cunning_shipment: bool,
    /// CHOAM may move one territory further (`ChoamExtraMove`).
    pub choam_has_extra_move: bool,
}

impl ShipMoveState {
    pub fn new(step: ShipMoveStep, sequence: PlayerSequence) -> Self {
        ShipMoveState {
            step,
            sequence,
            ships_tech_token_income: false,
            karma_shipment_prevention: None,
            current_player_may_perform_extra_move: false,
            in_guild_cunning_shipment: false,
            choam_has_extra_move: false,
        }
    }
}

/// Where the shipment and movement phase is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipMoveStep {
    /// Before the first shipment, while the host may still wait for players.
    Beginning,
    /// A faction other than the Guild ships, in turn.
    NonGuildShip,
    /// The Guild ship, or decide to wait.
    GuildShip,
    /// Bene Gesserit may send an advisor along with the latest shipment.
    BeneGesseritAccompaniesNonGuildShip,
    BeneGesseritAccompaniesGuildShip,
    NonGuildMove,
    GuildMove,
    Concluded,
}

impl ShipMoveStep {
    pub fn phase(self) -> Phase {
        match self {
            ShipMoveStep::Beginning => Phase::BeginningOfShipAndMove,
            ShipMoveStep::NonGuildShip => Phase::NonGuildShip,
            ShipMoveStep::GuildShip => Phase::GuildShip,
            ShipMoveStep::BeneGesseritAccompaniesNonGuildShip => Phase::BeneGesseritAccompaniesNonGuildShip,
            ShipMoveStep::BeneGesseritAccompaniesGuildShip => Phase::BeneGesseritAccompaniesGuildShip,
            ShipMoveStep::NonGuildMove => Phase::NonGuildMove,
            ShipMoveStep::GuildMove => Phase::GuildMove,
            ShipMoveStep::Concluded => Phase::ShipmentAndMoveConcluded,
        }
    }

    pub fn of(phase: Phase) -> Option<ShipMoveStep> {
        Some(match phase {
            Phase::BeginningOfShipAndMove => ShipMoveStep::Beginning,
            Phase::NonGuildShip => ShipMoveStep::NonGuildShip,
            Phase::GuildShip => ShipMoveStep::GuildShip,
            Phase::BeneGesseritAccompaniesNonGuildShip => ShipMoveStep::BeneGesseritAccompaniesNonGuildShip,
            Phase::BeneGesseritAccompaniesGuildShip => ShipMoveStep::BeneGesseritAccompaniesGuildShip,
            Phase::NonGuildMove => ShipMoveStep::NonGuildMove,
            Phase::GuildMove => ShipMoveStep::GuildMove,
            Phase::ShipmentAndMoveConcluded => ShipMoveStep::Concluded,
            _ => return None,
        })
    }
}

impl Game {
    /// The shipment and movement phase's state, while the game is in it (an interrupt may
    /// suspend it).
    pub fn ship_move(&self) -> Option<&ShipMoveState> {
        match &self.phase {
            PhaseState::ShipMove(s) => Some(s),
            _ => None,
        }
    }

    pub fn ship_move_mut(&mut self) -> Option<&mut ShipMoveState> {
        match &mut self.phase {
            PhaseState::ShipMove(s) => Some(s),
            _ => None,
        }
    }

    /// The shipment and movement phase's state, for code that only runs in it.
    #[track_caller]
    fn ship_move_state(&mut self) -> &mut ShipMoveState {
        self.ship_move_mut().expect("in the shipment and move phase")
    }

    /// Who ships and moves, in turn, during the phase.
    pub fn shipment_and_move_sequence(&self) -> Option<&PlayerSequence> {
        self.ship_move().map(|s| &s.sequence)
    }

    /// The player whose turn it is to ship and move, during the phase.
    pub fn current_shipment_and_move_player(&self) -> Option<PlayerId> {
        self.shipment_and_move_sequence().and_then(|s| s.current_player(self))
    }

    /// Moves on to the next player in the shipment and move order.
    pub(crate) fn next_shipment_and_move_player(&mut self) {
        let mut s = self.ship_move_state().sequence.clone();
        s.next_player(self);
        self.ship_move_state().sequence = s;
    }

    /// Whether the Guild ship and move a second time by their Nexus cunning.
    pub fn in_guild_cunning_shipment(&self) -> bool {
        self.ship_move().is_some_and(|s| s.in_guild_cunning_shipment)
    }

    /// Starts the shipment and movement phase (`enter_shipment_and_move_phase` in end_phase.rs
    /// does the rest).
    pub fn begin_ship_move(&mut self) {
        let sequence = PlayerSequence::starting_with_first_player(self);
        self.enter_state(PhaseState::ShipMove(ShipMoveState::new(ShipMoveStep::Beginning, sequence)));
    }

    /// Moves the phase to `step`, dropping any interrupt, as entering a phase did.
    #[track_caller]
    pub fn enter_ship_move(&mut self, step: ShipMoveStep) {
        self.ship_move_state().step = step;
        self.interrupts.clear();
        self.remove_ended_deals(step.phase());
    }

    /// Moves to `if_true` or `if_false`.
    fn enter_ship_move_either(&mut self, condition: bool, if_true: ShipMoveStep, if_false: ShipMoveStep) {
        self.enter_ship_move(if condition { if_true } else { if_false });
    }
}

/// Where the last shipment or move took forces (`LocationEvent.To` of the event C# kept).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShipmentOrMovement {
    pub to: Option<Location>,
}

impl ShipmentOrMovement {
    pub fn to_location(&self, _game: &Game) -> Option<Location> {
        self.to
    }
}

/// A move of this phase: who moved which forces where.
#[derive(Clone, Debug)]
pub struct RecentMove {
    pub initiator: Faction,
    pub to: Option<Location>,
    pub force_locations: IndexMap<Location, Battalion>,
}

impl RecentMove {
    pub fn placement_to(&self, _game: &Game) -> Option<Location> {
        self.to
    }

    pub fn placement_force_locations(&self, _game: &Game) -> IndexMap<Location, Battalion> {
        self.force_locations.clone()
    }
}

/// A Karama that prevents a faction from shipping this phase.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShipmentPrevention {
    pub target: Option<Faction>,
}

/// A Flight card played for the next move.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlightUse {
    pub by: By,
    pub move_three_territories: bool,
}

impl FlightUse {
    pub fn extra_move(&self) -> bool {
        !self.move_three_territories
    }
}

/// A Planetologist used for the next move.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanetologyUse {
    pub initiator: Option<Faction>,
    pub add_one_to_movement: bool,
}

impl PlanetologyUse {
    pub fn move_from_two_territories(&self) -> bool {
        !self.add_one_to_movement
    }
}

/// Builds a list of message elements from mixed arguments.
macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(Part::from($e)),*] };
}

impl Game {
    pub fn start_ship_and_move_sequence(&mut self) -> GameResult {
        self.skip_guild_in_turn_order();

        // Both are evaluated before entering, including the call that may log.
        let juice_first = self.juice_forces_first_player() && self.turn.current_juice.clone().expect("current juice").initiator() != Some(Faction::Guild);
        let guild_acts = self.check_if_guild_acts_or_auto_delays();
        self.enter_ship_move_either(guild_acts && !juice_first, ShipMoveStep::GuildShip, ShipMoveStep::NonGuildShip);
        Ok(())
    }

    /// When the Guild's turn in the shipment order comes and they may ship whenever they like,
    /// the next faction goes instead.
    pub fn skip_guild_in_turn_order(&mut self) {
        let current_is_guild = self.shipment_and_move_sequence().is_some_and(|s| s.current_faction(self) == Faction::Guild);
        if current_is_guild && self.guild_may_ship_out_of_turn_order() {
            self.next_shipment_and_move_player();
        }
    }

    pub fn check_if_guild_acts_or_auto_delays(&mut self) -> bool {
        let guild = self.get_player(Some(Faction::Guild));
        let guild_may_delay = self.guild_may_delay();
        let guild_has_to_act = self.is_playing(Faction::Guild)
            && self.turn.has_acted_or_passed.iter().all(|p| *p != Faction::Guild)
            && self.guild_may_ship_out_of_turn_order();
        let guild_is_on_auto_delay = match guild {
            Some(o) => self.is_automated(AutomationRuleType::ShipmentGuildAutoDelay, o),
            None => false,
        };
        if guild_has_to_act && guild_is_on_auto_delay && guild_may_delay {
            let m = express!(Faction::Guild, " delay their shipment").with_log(GuildDelayed::default());
            self.log(elems![m]);
        }

        guild_has_to_act && !(guild_is_on_auto_delay && guild_may_delay)
    }

    pub fn perform_move_from_locations(&mut self, initiator: PlayerId, force_locations: &IndexMap<Location, Battalion>, evt: &Event, as_advisors: bool, by_caravan: bool) -> GameResult {
        let Some(to) = evt.to_location(self) else { return Ok(()) };

        self.turn.last_shipment_or_movement = Some(ShipmentOrMovement { to: Some(to) });
        let was_occupied_before_move = self.is_occupied_territory(to.territory());
        let values: Vec<Battalion> = force_locations.values().cloned().collect();
        let dist = self.determine_maximum_move_distance(initiator, &values);
        let move_number = if matches!(evt, Event::Move(_) | Event::Caravan(_)) {
            self.counters.moves_logged += 1;
            Some(self.counters.moves_logged)
        } else {
            None
        };

        let from_territories = force_locations.keys().map(|l| l.territory()).distinct();
        for from_territory in from_territories {
            let mut total_number_of_forces = 0;
            let mut total_number_of_special_forces = 0;

            // forceLocations is a local dictionary that nothing modifies, so the lazy Where can be collected.
            let in_territory: Vec<(Location, Battalion)> = force_locations
                .iter()
                .filter(|(k, _)| k.territory() == from_territory)
                .map(|(k, v)| (*k, v.clone()))
                .collect();
            for fl in in_territory {
                if fl.1.total_amount_of_forces() > 0 {
                    self.perform_move_from_location(initiator, fl.0, &fl.1, to, &mut total_number_of_forces, &mut total_number_of_special_forces, false)?;
                    // C# parity: Sandmaster is applied per source location.
                    self.check_sandmaster(initiator, to, dist, fl)?;
                }
            }

            if self.player(initiator).is(Faction::BeneGesserit) {
                Player::flip_forces(self, initiator, to, was_occupied_before_move && as_advisors)?;
            }

            if total_number_of_forces > 0 || total_number_of_special_forces > 0 {
                let from_locations: Vec<Location> = force_locations
                    .iter()
                    .filter(|(k, v)| k.territory() == from_territory && v.total_amount_of_forces() > 0)
                    .map(|(k, _)| *k)
                    .collect();
                self.log_move(initiator, from_territory, &from_locations, to, total_number_of_forces, total_number_of_special_forces, was_occupied_before_move && as_advisors, by_caravan, move_number);
            }
        }

        self.stone(Milestone::Move);
        self.flip_bene_gesserit_advisors_when_alone()?;
        Ok(())
    }

    pub fn check_sandmaster(&mut self, initiator: PlayerId, to: Location, dist: i32, fl: (Location, Battalion)) -> GameResult {
        if self.has_skill_in_front_of_shield(initiator, LeaderSkill::Sandmaster) {
            let faction = self.player(initiator).faction;
            let ignore_storm = faction == Some(Faction::Fremen) && self.applicable(Rule::FremenMayMoveIntoStorm);
            let paths = self.board.map.find_paths(fl.0, to, dist, ignore_storm, faction.expect("faction"), &*self);
            let mut most_spice = 0;
            let mut path_with_most_spice: Option<Vec<Location>> = None;
            for p in paths {
                let amount_of_spice_locations = p.iter().filter(|l| self.board.resources_on_planet.contains_key(*l)).distinct().len() as i32;
                if amount_of_spice_locations > most_spice {
                    most_spice = amount_of_spice_locations;
                    path_with_most_spice = Some(p);
                }
            }

            if let Some(path) = path_with_most_spice {
                if most_spice > 0 {
                    for loc in path {
                        if self.board.resources_on_planet.contains_key(&loc) {
                            self.change_resources_on_planet(loc, -1)?;
                        }
                    }

                    let pl = self.player_mut(initiator);
                    crate::game::amounts::gain(&mut pl.resources, most_spice, "spice").expect("spice income");
                    let f = self.player(initiator).faction;
                    self.log(elems![f, " ", LeaderSkill::Sandmaster, " collects ", Payment::of(most_spice), " along the way"]);
                }
            }
        }
        Ok(())
    }

    pub fn perform_move_from_location(&mut self, initiator: PlayerId, from: Location, battalion: &Battalion, to: Location, total_number_of_forces: &mut i32, total_number_of_special_forces: &mut i32, ignore_storm: bool) -> GameResult {
        let must_move_through_storm = self.must_move_through_storm(initiator, from, to, battalion);
        if !ignore_storm && (self.is_in_storm(to) || must_move_through_storm) {
            let kill_count;
            if self.player(initiator).is(Faction::Fremen) && !self.prevented(FactionAdvantage::FremenProtectedFromStorm) && self.applicable(Rule::FremenStormLosses) {
                kill_count = 0;
                Player::move_forces(self, initiator, from, to, battalion.forces())?;
                Player::move_special_forces(self, initiator, from, to, battalion.special_forces())?;
                let amount = TakeLosses::half_of(battalion.forces(), battalion.special_forces());
                self.storm.storm_losses_to_take.push(LossToTake { location: Some(to), amount, faction: Some(battalion.faction) });
            } else {
                kill_count = battalion.total_amount_of_forces();
                Player::kill_forces(self, initiator, from, battalion.forces(), battalion.special_forces(), false)?;
            }

            if kill_count > 0 {
                let f = self.player(initiator).faction;
                let m = express!(kill_count, f, " forces are killed by the storm while travelling")
                    .with_log(ShipMoveLoss::new(f.expect("faction"), true, true, kill_count, None));
                self.log(elems![m]);
            }
        } else {
            Player::move_forces(self, initiator, from, to, battalion.forces())?;
            Player::move_special_forces(self, initiator, from, to, battalion.special_forces())?;
        }

        *total_number_of_forces += battalion.forces();
        *total_number_of_special_forces += battalion.special_forces();
        Ok(())
    }

    pub fn must_move_through_storm(&self, initiator: PlayerId, from: Location, to: Location, moved: &Battalion) -> bool {
        let max = self.determine_maximum_move_distance(initiator, std::slice::from_ref(moved));
        let faction = self.player(initiator).faction;
        let targets_avoiding_storm = self.board.map.find_neighbours(from, max, false, faction.expect("faction"), self, true);
        let targets_ignoring_storm = self.board.map.find_neighbours(from, max, true, faction.expect("faction"), self, true);
        !targets_avoiding_storm.contains(&to) && targets_ignoring_storm.contains(&to)
    }

    pub fn log_move(&mut self, initiator: PlayerId, from: Territory, from_locations: &[Location], to: Location, force_amount: i32, special_force_amount: i32, as_advisors: bool, by_caravan: bool, move_number: Option<i32>) {
        let bene_gesserit_as = self.player(initiator).is(Faction::BeneGesserit) && self.applicable(Rule::BeneGesseritAdvisors);
        let pl = self.player(initiator);
        let faction = pl.faction;
        let force = pl.force();
        let special_force = pl.special_force();
        let as_what: Part = if as_advisors { special_force.into() } else { force.into() };
        let mut message = express!(
            Game::caravan_message(by_caravan),
            faction,
            " move ",
            express_part_if!(force_amount > 0; force_amount, force),
            express_part_if!(special_force_amount > 0; special_force_amount, special_force),
            " from ",
            from,
            " to ",
            to,
            express_part_if!(bene_gesserit_as; " as ", as_what)
        )
        .with_route(faction.expect("faction"), from_locations.iter().copied(), Some(to));

        if let Some(nr) = move_number {
            message = message.with_log(MoveLogged::new(faction.expect("faction"), nr, from, force_amount, special_force_amount, to, by_caravan, if bene_gesserit_as { Some(as_advisors) } else { None }));
        }

        self.log(elems![message]);
    }

    pub fn caravan_message(by_caravan: bool) -> MessagePart {
        express_part_if!(by_caravan; "By ", TreacheryCardType::Caravan, ", ")
    }

    pub fn take_vidal(&mut self, p: PlayerId, when_to_set_aside: VidalMoment) {
        let Some(vidal) = self.vidal() else { return };

        let current_owner = self.owner_of(Hero::from(vidal));
        if let Some(owner) = current_owner {
            let leaders = &mut self.player_mut(owner).leaders;
            if let Some(pos) = leaders.iter().position(|l| *l == vidal) {
                leaders.remove(pos);
            }

            if self.is_alive(Hero::from(vidal)) {
                let f = self.player(owner).faction;
                self.log(elems![f, " lose ", vidal]);
            }

            self.leaders.captured_leaders.shift_remove(&vidal);
        }

        self.player_mut(p).leaders.push(vidal);
        self.leaders.player_to_set_aside_vidal = Some(p);
        self.leaders.when_to_set_aside_vidal = Some(when_to_set_aside);

        let f = self.player(p).faction;
        self.log(elems![f, " take ", vidal]);

        self.set_in_front_of_shield(vidal, false);
    }

    pub fn let_player_discard_treachery_card_of_choice(&mut self, f: Faction) {
        self.turn.factions_that_must_discard.push(f);
        self.interrupt(Interrupt::Discarding);
    }

    pub fn kill_ambassador_in(&mut self, territory: Territory) {
        if let Some(ambassador) = self.ambassador_in(territory) {
            let ecaz = self.get_player(Some(Faction::Ecaz));
            self.board.ambassadors_on_planet.shift_remove(&territory);
            self.player_mut(ecaz.expect("ecaz")).ambassadors.push(ambassador);
            self.log(elems!["The ambassador in ", territory, " returns to ", Faction::Ecaz]);
        }
    }

    pub fn determine_next_shipment_and_move_sub_phase(&mut self) -> GameResult {
        self.cleanup_obsolete_intrusions()?;

        if self.turn.bene_gesserit_may_accompany {
            if self.prevented(FactionAdvantage::BeneGesseritAccompanies) {
                self.log_prevention_by_karma(FactionAdvantage::BeneGesseritAccompanies);
                self.turn.bene_gesserit_may_accompany = false;
                if !self.applicable(Rule::FullPhaseKarma) {
                    self.allow(FactionAdvantage::BeneGesseritAccompanies);
                }
            } else if self.has_low_threshold(Faction::BeneGesserit) {
                self.turn.bene_gesserit_may_accompany = false;
            }
        }

        if !self.storm.storm_losses_to_take.is_empty() {
            self.interrupt(Interrupt::StormLosses);
        } else if let Some(next) = self.turn.intrusions.first() {
            let kind = next.type_;
            let Some(cause) = self.current_intrusion_cause() else {
                game_error!("{} triggered during undefined phase: {}", kind, self.phase());
            };
            self.show_intrusion(kind, cause);
        } else {
            self.continue_after_intrusions()?;
        }

        self.skip_move_if_nothing_to_move()?;
        Ok(())
    }

    pub fn skip_move_if_nothing_to_move(&mut self) -> GameResult {
        if !matches!(self.phase(), Phase::GuildMove | Phase::NonGuildMove) {
            return Ok(());
        }

        let player = if self.phase() == Phase::GuildMove {
            self.get_player(Some(Faction::Guild))
        } else {
            self.shipment_and_move_sequence().required("shipment and move sequence")?.current_player(self)
        };

        let Some(player) = player else { return Ok(()) };

        if !PlacementEvent::valid_movement_sources(self, player).is_empty() {
            return Ok(());
        }

        self.storm.storm_losses_to_take.clear();
        let pf = self.player(player).faction;
        self.turn.has_acted_or_passed.push(pf.required("faction")?);

        if self.phase() == Phase::NonGuildMove {
            self.next_shipment_and_move_player();
            self.skip_guild_in_turn_order();
        }

        if self.in_guild_cunning_shipment() {
            self.turn.current_guild_cunning = None;
            self.ship_move_state().in_guild_cunning_shipment = false;
        }

        let m = express!(pf, " pass movement").with_log(MovePassed::new(pf.required("faction")?));
        self.log(elems![m]);

        self.determine_next_shipment_and_move_sub_phase()?;
        self.check_if_forces_should_be_destroyed_by_ally_presence(player)?;
        self.flip_bene_gesserit_advisors_when_alone()?;

        if !self.applicable(Rule::FullPhaseKarma) {
            self.allow(FactionAdvantage::FremenExtraMove);
        }
        if !self.applicable(Rule::FullPhaseKarma) {
            self.allow(FactionAdvantage::IxianCyborgExtraMove);
        }

        self.turn.current_flight_used = None;
        self.turn.current_flight_discovery_used = None;
        self.turn.current_planetology = None;
        Ok(())
    }

    pub fn cleanup_obsolete_intrusions(&mut self) -> GameResult {
        if !self.turn.intrusions.is_empty() {
            let mut i = 0;
            let mut search = true;
            while search {
                i += 1;
                if i > 100 {
                    game_error!("Stuck");
                }

                let bene_gesserit_intrusion_is_obsolete = match self.last_bene_gesserit_intrusion() {
                    Some(bi) => {
                        let bene_gesserit = self.get_player(Some(Faction::BeneGesserit)).required("The Bene Gesserit player")?;
                        self.player(bene_gesserit).forces_in_territory(Some(bi.territory(self))) == 0
                    }
                    None => false,
                };
                if bene_gesserit_intrusion_is_obsolete {
                    self.dequeue_intrusion(IntrusionType::BeneGesseritIntrusion)?;
                } else if self.last_terror_trigger().filter(|tt| self.terror_in(tt.territory(self)).next().is_none()).is_some() {
                    self.dequeue_intrusion(IntrusionType::Terror)?;
                } else if self.last_ambassador_trigger().filter(|at| self.ambassador_in(at.territory(self)) == None).is_some() {
                    self.dequeue_intrusion(IntrusionType::Ambassador)?;
                } else {
                    search = false;
                }
            }
        }
        Ok(())
    }

    /// What caused the intrusion shown, or else the step of the shipment and move phase or the
    /// worm ride that the next intrusion would interrupt. `None` anywhere else.
    pub fn current_intrusion_cause(&self) -> Option<IntrusionCause> {
        if let Some(Interrupt::Intrusion { cause, .. }) = self.interrupts.last() {
            return Some(*cause);
        }
        IntrusionCause::of_step(self.phase())
    }

    /// Shows the next intrusion, in place of the one shown before.
    pub fn show_intrusion(&mut self, kind: IntrusionType, cause: IntrusionCause) {
        self.drop_intrusion();
        self.interrupt(Interrupt::intrusion(kind, cause));
    }

    /// After a Caravan or a revival queued intrusions: shows the first.
    pub fn show_first_intrusion(&mut self, cause: IntrusionCause) -> GameResult {
        let kind = self.turn.intrusions.first().required("an intrusion")?.type_;
        self.show_intrusion(kind, cause);
        Ok(())
    }

    /// Every intrusion is answered: the game goes on from what caused them.
    pub fn continue_after_intrusions(&mut self) -> GameResult {
        let Some(cause) = self.current_intrusion_cause() else { return Ok(()) };
        match cause {
            IntrusionCause::WormRideA => self.end_worm_ride_during_phase(Phase::FremenRidingMonsterA)?,
            IntrusionCause::WormRideB => self.end_worm_ride_during_phase(Phase::FremenRidingMonsterB)?,

            IntrusionCause::GuildShip => {
                let accompany = self.is_playing(Faction::BeneGesserit) && self.turn.bene_gesserit_may_accompany;
                if accompany {
                    self.enter_ship_move(ShipMoveStep::BeneGesseritAccompaniesGuildShip);
                } else {
                    self.enter_guild_move_unless_cunning();
                }
            }

            IntrusionCause::BeneGesseritAccompaniesGuildShip => self.enter_guild_move_unless_cunning(),

            IntrusionCause::NonGuildShip => {
                let accompany = self.is_playing(Faction::BeneGesserit) && self.turn.bene_gesserit_may_accompany;
                self.enter_ship_move_either(accompany, ShipMoveStep::BeneGesseritAccompaniesNonGuildShip, ShipMoveStep::NonGuildMove);
            }

            IntrusionCause::BeneGesseritAccompaniesNonGuildShip => self.enter_ship_move(ShipMoveStep::NonGuildMove),
            IntrusionCause::GuildMove => self.determine_next_sub_phase_after_guild_ship_and_move()?,
            IntrusionCause::NonGuildMove => self.determine_next_sub_phase_after_non_guild_move()?,

            // Back to the shipment step the Caravan was played in.
            IntrusionCause::Caravan => {
                self.drop_intrusion();
                let phase = self.phase();
                self.remove_ended_deals(phase);
            }

            // Back to the revival, or to the skill assignment it started if that still waits.
            IntrusionCause::Revival => {
                self.drop_intrusion();
                if self.interrupts.last() == Some(&Interrupt::SkillAssignment) && !SkillAssigned::players_must_choose_leader_skills(self) {
                    self.interrupts.pop();
                }
                let phase = self.phase();
                self.remove_ended_deals(phase);
            }
        }
        Ok(())
    }

    /// After the Guild's shipment: their move, or, after a second shipment by their Nexus
    /// cunning, the next faction's move.
    fn enter_guild_move_unless_cunning(&mut self) {
        let cunning = self.in_guild_cunning_shipment();
        self.enter_ship_move_either(cunning, ShipMoveStep::NonGuildMove, ShipMoveStep::GuildMove);
    }

    /// Drops the intrusion shown, if one is on top, without ending deals.
    fn drop_intrusion(&mut self) {
        if let Some(Interrupt::Intrusion { .. }) = self.interrupts.last() {
            self.interrupts.pop();
        }
    }

    pub fn check_if_forces_should_be_destroyed_by_ally_presence(&mut self, p: PlayerId) -> GameResult {
        let (p_ally, p_faction) = (self.player(p).ally, self.player(p).faction);
        if p_ally != None && p_faction != Some(Faction::Ecaz) && p_ally != Some(Faction::Ecaz) {
            let special_forces_are_allowed = self.player(p).is(Faction::BeneGesserit) && self.applicable(Rule::AdvisorsDontConflictWithAlly);

            //Forces that must be destroyed because moves ended where allies are
            let mut i = 0;
            while i < self.turn.chosen_destinations_with_allies.len() {
                let t = self.turn.chosen_destinations_with_allies[i];
                i += 1;
                if !(special_forces_are_allowed && self.player(p).special_forces_in_territory(Some(t)) > 0) && self.player(p).any_forces_in_territory(Some(t)) > 0 {
                    let m = express!("All ", p_faction, " forces in ", t, " were killed due to ally presence")
                        .with_log(ShipMoveLoss::new(p_faction.expect("faction"), false, false, 0, Some(t)));
                    self.log(elems![m]);
                    self.reveal_current_no_field_in_territory(p, t)?;
                    Player::kill_all_forces_in_territory(self, p, t, false)?;
                }
            }

            if self.turn.has_acted_or_passed.contains_opt(p_ally) {
                //Forces that must be destroyed if both the player and his ally have moved

                let player_territories = if self.applicable(Rule::AdvisorsDontConflictWithAlly) {
                    self.player(p).occupied_territories(self)
                } else {
                    self.player(p).territories_with_forces(self).collect()
                };
                let ally_player = self.get_player(p_ally);
                let ally_territories = if self.applicable(Rule::AdvisorsDontConflictWithAlly) {
                    ally_player.map(|a| self.player(a).occupied_territories(self))
                } else {
                    ally_player.map(|a| self.player(a).territories_with_forces(self).collect())
                }
                .unwrap_or_default();

                let polar_sink = self.board.map.polar_sink().territory();
                let to_kill: Vec<Territory> = player_territories
                    .iter()
                    .copied()
                    .distinct()
                    .into_iter()
                    .filter(|t| ally_territories.contains(t))
                    .filter(|x| *x != polar_sink)
                    .collect();
                for t in to_kill {
                    let m = express!("All ", p_faction, " forces in ", t, " were killed due to ally presence")
                        .with_log(ShipMoveLoss::new(p_faction.expect("faction"), false, false, 0, Some(t)));
                    self.log(elems![m]);
                    self.reveal_current_no_field_in_territory(p, t)?;
                    Player::kill_all_forces_in_territory(self, p, t, false)?;
                }
            }
        }
        Ok(())
    }

    pub fn threatened_by_ally_presence(&self, p: PlayerId, t: Territory) -> bool {
        if t == self.board.map.polar_sink().territory() {
            return false;
        }

        let pl = self.player(p);
        let Some(ally) = self.get_player(pl.ally) else { return false };

        if pl.faction == Some(Faction::Ecaz) || pl.ally == Some(Faction::Ecaz) {
            return false;
        }

        if self.applicable(Rule::AdvisorsDontConflictWithAlly) {
            if pl.is(Faction::BeneGesserit) && !pl.occupies_territory(self, Some(t)) {
                return false;
            }

            if pl.ally == Some(Faction::BeneGesserit) && !self.player(ally).occupies_territory(self, Some(t)) {
                return false;
            }
        }

        self.player(ally).any_forces_in_territory(Some(t)) > 0
    }

    /// Takes the extra move a Flight card gave the player who just moved, if there is one.
    fn take_extra_move(&mut self) -> bool {
        std::mem::take(&mut self.ship_move_state().current_player_may_perform_extra_move)
    }

    pub fn determine_next_sub_phase_after_non_guild_move(&mut self) -> GameResult {
        if self.take_extra_move() {
            self.enter_ship_move(ShipMoveStep::NonGuildMove);
        } else {
            // Both are evaluated first, including the call that may log.
            let everyone_acted = self.everyone_acted_or_passed();
            let guild_acts = self.check_if_guild_acts_or_auto_delays();
            if everyone_acted {
                self.conclude_shipment_and_move()?;
            } else {
                self.enter_ship_move_either(guild_acts, ShipMoveStep::GuildShip, ShipMoveStep::NonGuildShip);
            }
        }
        Ok(())
    }

    pub fn determine_next_sub_phase_after_guild_ship_and_move(&mut self) -> GameResult {
        if self.take_extra_move() {
            self.enter_ship_move(ShipMoveStep::GuildMove);
        } else if !self.in_guild_cunning_shipment() && self.turn.current_guild_cunning.as_ref().is_some_and(|c| c.by(Faction::Guild)) {
            self.ship_move_state().in_guild_cunning_shipment = true;
            self.enter_ship_move(ShipMoveStep::GuildShip);
        } else if !self.everyone_acted_or_passed() {
            self.enter_ship_move(ShipMoveStep::NonGuildShip);
        } else {
            self.conclude_shipment_and_move()?;
        }
        Ok(())
    }

    pub fn conclude_shipment_and_move(&mut self) -> GameResult {
        self.check_if_moritani_gains_vidal();
        self.ship_move_state().karma_shipment_prevention = None;
        self.main_phase_end();
        self.enter_ship_move(ShipMoveStep::Concluded);
        self.receive_ships_tech_income();
        self.ship_move_state().choam_has_extra_move = false;
        self.turn.current_bene_gesserit_cunning = None;
        Ok(())
    }

    pub fn check_if_moritani_gains_vidal(&mut self) {
        let moritani = self.get_player(Some(Faction::Moritani));
        let Some(moritani) = moritani else { return };
        if self.prevented(FactionAdvantage::MoritaniGainingVidal) {
            return;
        }

        if !self.vidal_is_alive() {
            return;
        }

        let ecaz = self.get_player(Some(Faction::Ecaz));
        let nr_of_battles_in_strongholds = Battle::battles_to_be_fought(self, moritani, false)
            .iter()
            .map(|b| b.territory)
            .filter(|t| (ecaz.is_none() || self.player(ecaz.expect("ecaz")).any_forces_in_territory(Some(*t)) == 0) && (t.is_stronghold() || self.is_special_stronghold(*t)))
            .distinct()
            .len();

        if nr_of_battles_in_strongholds >= 2 && !self.vidal_is_captured_or_ghola() && self.occupier_of(World::Ecaz).is_none() {
            self.take_vidal(moritani, VidalMoment::EndOfTurn);
        }
    }

    pub fn receive_ships_tech_income(&mut self) {
        if self.ship_move().is_some_and(|s| s.ships_tech_token_income) {
            let tech_token_owner = self.player_indices().find(|p| self.player(*p).tech_tokens.contains(&TechToken::Ships));
            if let Some(owner) = tech_token_owner {
                let amount = self.player(owner).tech_tokens.len() as i32;
                let pl = self.player_mut(owner);
                crate::game::amounts::gain(&mut pl.resources, amount, "spice").expect("spice income");
                let f = self.player(owner).faction;
                self.log(elems![f, " receive ", Payment::of(amount), " from ", TechToken::Ships]);
            }
        }
    }

    /// Queues the intrusions that `e`, a location event by `initiator`, causes.
    pub fn check_intrusion(&mut self, e: &Event, initiator: Faction) -> GameResult<bool> {
        let fighters = e.location_event_forces_added_to_location(self);
        self.check_intrusion_with_fighters(e, initiator, fighters)
    }

    /// As [`Game::check_intrusion`], for an event whose forces arrived as `fighters` fighters
    /// (not advisors): Bene Gesserit accompanying a shipment arrive as either, which their
    /// event alone doesn't say.
    pub fn check_intrusion_with_fighters(&mut self, e: &Event, initiator: Faction, fighters: i32) -> GameResult<bool> {
        let territory = e.to_location(self).required("destination")?.territory();
        self.check_bene_gesserit_intrusion(e, initiator, territory);
        self.check_ambassador_triggered(e, initiator, fighters);
        self.check_terror_triggered(e, initiator);
        Ok(!self.turn.intrusions.is_empty())
    }

    pub fn check_bene_gesserit_intrusion(&mut self, e: &Event, initiator: Faction, territory: Territory) {
        let bg_player = self.get_player(Some(Faction::BeneGesserit));
        if let Some(bg) = bg_player {
            if territory != self.board.map.polar_sink().territory()
                && !territory.is_homeworld()
                && self.applicable(Rule::BeneGesseritAdvisors)
                && Some(initiator) != self.player(bg).ally
                && initiator != Faction::BeneGesserit
                && self.player(bg).forces_in_territory(Some(territory)) > 0
            {
                if self.prevented(FactionAdvantage::BeneGesseritIntrusion) {
                    self.log_prevention_by_karma(FactionAdvantage::BeneGesseritIntrusion);
                    if !self.applicable(Rule::FullPhaseKarma) {
                        self.allow(FactionAdvantage::BeneGesseritIntrusion);
                    }
                } else {
                    self.queue_intrusion(e, initiator, IntrusionType::BeneGesseritIntrusion);
                }
            }
        }
    }

    /// An ambassador triggers when forces of another faction enter its territory, other than
    /// Ecaz's ally, the faction of the ambassador, or Bene Gesserit advisors.
    pub fn check_ambassador_triggered(&mut self, e: &Event, initiator: Faction, fighters: i32) {
        let ecaz_player = self.get_player(Some(Faction::Ecaz));
        if let Some(ecaz) = ecaz_player {
            if (initiator != Faction::BeneGesserit || fighters > 0)
                && initiator != Faction::Ecaz
                && Some(initiator) != self.player(ecaz).ally
                && self.ambassador_in(e.to_location(self).expect("to location").territory()) != None
                && self.ambassador_in(e.to_location(self).expect("to location").territory()) != Game::ambassador_of(initiator)
            {
                self.queue_intrusion(e, initiator, IntrusionType::Ambassador);
            }
        }
    }

    pub fn check_terror_triggered(&mut self, e: &Event, initiator: Faction) {
        let moritani_player = self.get_player(Some(Faction::Moritani));
        if let Some(moritani) = moritani_player {
            if (e.location_event_total_amount_of_forces_added_to_location(self) >= 3 || !self.player(moritani).has_any_low_threshold(self))
                && initiator != Faction::Moritani
                && Some(initiator) != self.player(moritani).ally
                && !self.is_occupied_by_faction_or_their_ally(World::Moritani, initiator)
                && !self.terror_in(e.to_location(self).expect("to location").territory()).next().is_none()
            {
                self.queue_intrusion(e, initiator, IntrusionType::Terror);
            }
        }
    }

    pub fn queue_intrusion(&mut self, e: &Event, initiator: Faction, type_: IntrusionType) {
        let intrusion = Intrusion { initiator, to: e.to_location(self), type_, alliance_offered: false };
        self.turn.intrusions.push(intrusion);
    }

    pub fn dequeue_intrusion(&mut self, type_: IntrusionType) -> GameResult {
        if self.turn.intrusions.is_empty() {
            game_error!("No intrusions to dequeue");
        }
        if self.turn.intrusions[0].type_ != type_ {
            game_error!("Wrong intrusion type: actual:{} vs expected:{}", type_, self.turn.intrusions[0].type_);
        }

        self.turn.intrusions.remove(0);
        Ok(())
    }

    pub fn vidal(&self) -> Option<Leader> {
        self.leaders.leader_state.keys().copied().find(|h| h.hero_type() == HeroType::Vidal).and_then(|h| h.as_leader())
    }

    pub fn vidal_is_alive(&self) -> bool {
        match self.vidal() {
            Some(v) => self.is_alive(Hero::from(v)),
            None => false,
        }
    }

    pub fn vidal_is_captured_or_ghola(&self) -> bool {
        let player_with_vidal = self.players.iter().find(|p| p.leaders.iter().any(|l| l.hero_type() == HeroType::Vidal));
        match player_with_vidal {
            Some(pwv) => self.leaders.captured_leaders.keys().any(|h| h.hero_type() == HeroType::Vidal) || pwv.is(Faction::Tleilaxu),
            None => false,
        }
    }

    pub fn prevented_from_shipping(&self, f: impl Into<Option<Faction>>) -> bool {
        let f: Option<Faction> = f.into();
        match self.ship_move().and_then(|s| s.karma_shipment_prevention) {
            Some(k) => k.target == f,
            None => false,
        }
    }

    pub fn has_shipment_permission(&self, p: PlayerId, permission: ShipmentPermission) -> bool {
        match self.alliances.shipment_permissions.get_opt(self.player(p).faction) {
            Some(permissions) => permissions.contains(permission),
            None => false,
        }
    }

    /// Marks the terror intrusion being answered as one where Moritani offered an alliance.
    pub fn set_terror_alliance_offered(&mut self, offered: bool) {
        if let Some(i) = self.turn.intrusions.first_mut().filter(|i| i.type_ == IntrusionType::Terror) {
            i.alliance_offered = offered;
        }
    }

    pub fn last_bene_gesserit_intrusion(&self) -> Option<Intrusion> {
        match self.turn.intrusions.first() {
            Some(i) if i.type_ == IntrusionType::BeneGesseritIntrusion => Some(i.clone()),
            _ => None,
        }
    }

    pub fn last_terror_trigger(&self) -> Option<Intrusion> {
        match self.turn.intrusions.first() {
            Some(i) if i.type_ == IntrusionType::Terror => Some(i.clone()),
            _ => None,
        }
    }

    pub fn last_ambassador_trigger(&self) -> Option<Intrusion> {
        match self.turn.intrusions.first() {
            Some(i) if i.type_ == IntrusionType::Ambassador => Some(i.clone()),
            _ => None,
        }
    }

    pub fn occupies_arrakeen_or_carthag(&self, p: PlayerId) -> bool {
        self.player(p).occupies(self, Some(self.board.map.arrakeen())) || self.player(p).occupies(self, Some(self.board.map.carthag()))
    }

    pub fn ambassador_of(faction: Faction) -> Option<Ambassador> {
        <Ambassador as crate::enum_type::GameEnum>::from_value(faction as i32)
    }

    pub fn guild_may_delay(&self) -> bool {
        self.guild_may_ship_out_of_turn_order()
            && ((self.players.len() as i32) - (self.turn.has_acted_or_passed.len() as i32))
                > (if self.juice_forces_last_player() && !self.turn.has_acted_or_passed.contains_opt(self.turn.current_juice.as_ref().expect("current juice").initiator()) { 2 } else { 1 })
    }

    pub fn guild_may_ship_out_of_turn_order(&self) -> bool {
        self.applicable(Rule::GuildDetermineShipment) && !self.prevented(FactionAdvantage::GuildDetermineMoveMoment)
    }
}

/// An intrusion waiting for its answer: who moved forces where, from the event that caused it.
#[derive(Clone, Debug, PartialEq)]
pub struct Intrusion {
    pub initiator: Faction,
    /// Where the forces went (`LocationEvent.To` of the event).
    pub to: Option<Location>,
    pub type_: IntrusionType,
    /// For a terror intrusion: Moritani offered an alliance instead and it was refused, so they
    /// may neither pass nor offer again.
    pub alliance_offered: bool,
}

/// What made forces enter the territory of an intrusion: a step of the shipment and move
/// phase, a Caravan, a revival to the planet, or a worm ride.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntrusionCause {
    NonGuildShip,
    GuildShip,
    BeneGesseritAccompaniesNonGuildShip,
    BeneGesseritAccompaniesGuildShip,
    NonGuildMove,
    GuildMove,
    Caravan,
    Revival,
    WormRideA,
    WormRideB,
}

impl IntrusionCause {
    /// The cause of an intrusion during the step `phase`, if forces enter territories in it.
    pub fn of_step(phase: Phase) -> Option<IntrusionCause> {
        Some(match phase {
            Phase::NonGuildShip => IntrusionCause::NonGuildShip,
            Phase::GuildShip => IntrusionCause::GuildShip,
            Phase::BeneGesseritAccompaniesNonGuildShip => IntrusionCause::BeneGesseritAccompaniesNonGuildShip,
            Phase::BeneGesseritAccompaniesGuildShip => IntrusionCause::BeneGesseritAccompaniesGuildShip,
            Phase::NonGuildMove => IntrusionCause::NonGuildMove,
            Phase::GuildMove => IntrusionCause::GuildMove,
            Phase::FremenRidingMonsterA => IntrusionCause::WormRideA,
            Phase::FremenRidingMonsterB => IntrusionCause::WormRideB,
            _ => return None,
        })
    }

    /// The phase shown while an intrusion of `kind` with this cause waits.
    /// [`Interrupt::intrusion`] never builds a Bene Gesserit or terror intrusion caused by the
    /// Bene Gesserit accompanying a shipment, so `TerrorTriggeredByBeneGesseritAccompanies*`
    /// are never shown.
    pub fn intrusion_phase(self, kind: IntrusionType) -> Phase {
        use IntrusionCause as C;
        match kind {
            IntrusionType::BeneGesseritIntrusion => match self {
                C::NonGuildShip | C::BeneGesseritAccompaniesNonGuildShip => Phase::BeneGesseritIntrudedByNonGuildShip,
                C::GuildShip | C::BeneGesseritAccompaniesGuildShip => Phase::BeneGesseritIntrudedByGuildShip,
                C::NonGuildMove => Phase::BeneGesseritIntrudedByNonGuildMove,
                C::GuildMove => Phase::BeneGesseritIntrudedByGuildMove,
                C::Caravan => Phase::BeneGesseritIntrudedByCaravan,
                C::Revival => Phase::BeneGesseritIntrudedByRevival,
                C::WormRideA => Phase::BeneGesseritIntrudedByFremenRidingMonsterA,
                C::WormRideB => Phase::BeneGesseritIntrudedByFremenRidingMonsterB,
            },
            IntrusionType::Terror => match self {
                C::NonGuildShip => Phase::TerrorTriggeredByNonGuildShip,
                C::GuildShip => Phase::TerrorTriggeredByGuildShip,
                C::BeneGesseritAccompaniesNonGuildShip => Phase::TerrorTriggeredByBeneGesseritAccompaniesNonGuildShip,
                C::BeneGesseritAccompaniesGuildShip => Phase::TerrorTriggeredByBeneGesseritAccompaniesGuildShip,
                C::NonGuildMove => Phase::TerrorTriggeredByNonGuildMove,
                C::GuildMove => Phase::TerrorTriggeredByGuildMove,
                C::Caravan => Phase::TerrorTriggeredByCaravan,
                C::Revival => Phase::TerrorTriggeredByRevival,
                C::WormRideA => Phase::TerrorTriggeredByFremenRidingMonsterA,
                C::WormRideB => Phase::TerrorTriggeredByFremenRidingMonsterB,
            },
            IntrusionType::Ambassador => match self {
                C::NonGuildShip => Phase::AmbassadorTriggeredByNonGuildShip,
                C::GuildShip => Phase::AmbassadorTriggeredByGuildShip,
                C::BeneGesseritAccompaniesNonGuildShip => Phase::AmbassadorTriggeredByBeneGesseritAccompaniesNonGuildShip,
                C::BeneGesseritAccompaniesGuildShip => Phase::AmbassadorTriggeredByBeneGesseritAccompaniesGuildShip,
                C::NonGuildMove => Phase::AmbassadorTriggeredByNonGuildMove,
                C::GuildMove => Phase::AmbassadorTriggeredByGuildMove,
                C::Caravan => Phase::AmbassadorTriggeredByCaravan,
                C::Revival => Phase::AmbassadorTriggeredByRevival,
                C::WormRideA => Phase::AmbassadorTriggeredByFremenRidingMonsterA,
                C::WormRideB => Phase::AmbassadorTriggeredByFremenRidingMonsterB,
            },
        }
    }
}

impl Interrupt {
    /// The intrusion of `kind` caused by `cause`. Only an ambassador keeps the Bene Gesserit
    /// accompanying a shipment as its cause. A Bene Gesserit intrusion or a terror token counts
    /// as caused by the shipment itself, so the game goes on from the shipment step when it is
    /// answered (C# parity).
    pub fn intrusion(kind: IntrusionType, cause: IntrusionCause) -> Interrupt {
        let cause = match (kind, cause) {
            (IntrusionType::Ambassador, _) => cause,
            (_, IntrusionCause::BeneGesseritAccompaniesNonGuildShip) => IntrusionCause::NonGuildShip,
            (_, IntrusionCause::BeneGesseritAccompaniesGuildShip) => IntrusionCause::GuildShip,
            _ => cause,
        };
        Interrupt::Intrusion { kind, cause }
    }
}

impl Intrusion {
    pub fn territory(&self, _game: &Game) -> Territory {
        self.to.expect("to location").territory()
    }

    pub fn initiator(&self) -> Faction {
        self.initiator
    }
}

impl GameStatus {
    /// The status in the shipment and move phase, during an intrusion or an alliance offered
    /// through one, or `None` anywhere else.
    pub(crate) fn ship_move_status(game: &Game, _me: Option<PlayerId>, phase: Phase) -> Option<GameStatus> {
        match game.interrupts.last() {
            Some(Interrupt::Intrusion { kind, cause }) => Some(Self::intrusion_status(game, *kind, *cause)),
            Some(Interrupt::AllianceByTerror { offered_to }) => {
                let a = express!("Please decide about entering an alliance offered to you by ", Faction::Moritani);
                let w = express!(*offered_to, " are considering entering an alliance with ", Faction::Moritani, "...");
                Some(Self::faction_status(game, a, w, *offered_to, None))
            }
            Some(Interrupt::AllianceByAmbassador { offered_to, .. }) => {
                let a = express!("Please decide about entering an alliance offered to you by ", Faction::Ecaz);
                let w = express!(*offered_to, " are considering entering an alliance with ", Faction::Ecaz, "...");
                Some(Self::faction_status(game, a, w, Some(*offered_to), None))
            }
            Some(_) => None,
            None => Self::ship_move_step_status(game, ShipMoveStep::of(phase)?),
        }
    }

    /// The status while an intrusion waits for its answer.
    fn intrusion_status(g: &Game, kind: IntrusionType, cause: IntrusionCause) -> GameStatus {
        match kind {
            // C# parity: after a worm ride, the territory comes from `LastShipmentOrMovement!`,
            // which must be there.
            IntrusionType::BeneGesseritIntrusion if matches!(cause, IntrusionCause::WormRideA | IntrusionCause::WormRideB) => {
                let t = g.turn.last_shipment_or_movement.as_ref().expect("last shipment or movement").to_location(g).map(|l| l.territory());
                let a = express!("Please decide what to do in response to an intrusion of ", t, "; be fighters or advisors?");
                let w = express!(Faction::BeneGesserit, " are deciding what to do in response to an intrusion of ", t, "...");
                Self::faction_status(g, a, w, Some(Faction::BeneGesserit), None)
            }

            IntrusionType::BeneGesseritIntrusion => {
                let t = g.turn.last_shipment_or_movement.as_ref().and_then(|e| e.to_location(g)).map(|l| l.territory());
                let a = express!("Please decide how to respond to the intrusion in ", t, "; be fighters or advisors?");
                let w = express!(Faction::BeneGesserit, " are deciding how to respond to the intrusion in ", t, "...");
                Self::faction_status(g, a, w, Some(Faction::BeneGesserit), None)
            }

            IntrusionType::Terror => {
                let a = express!("Do you wish to respond to this intrusion with terror?");
                let w = express!(Faction::Moritani, " are thinking about resorting to terror...");
                Self::faction_status(g, a, w, Some(Faction::Moritani), None)
            }

            IntrusionType::Ambassador => {
                let a = express!("Do you wish to activate your ambassador?");
                let w = express!(Faction::Ecaz, Self::ally(g, Faction::Ecaz), " are thinking about activating their ambassador...");
                let players = Self::player_and_ally(g, Faction::Ecaz);
                Self::players_status(a, w, players, None)
            }
        }
    }

    /// The status at a step of the shipment and move phase, or `None` at its end, which has the
    /// status of every phase's end.
    fn ship_move_step_status(g: &Game, step: ShipMoveStep) -> Option<GameStatus> {
        let current_faction = || g.shipment_and_move_sequence().expect("shipment and move sequence").current_faction(g);
        Some(match step {
            ShipMoveStep::Beginning => Self::host_status(
                express!("You may now start the Ship & Move sequence..."),
                express!("Waiting for the host to start the Ship & Move sequence..."),
                None,
            ),

            ShipMoveStep::NonGuildShip if current_faction() == Faction::Fremen => {
                let a = express!("Please decide to rally forces or pass.");
                let w = express!(Faction::Fremen, " are thinking about rallying forces...");
                Self::status_ship_and_move(g, a, w)
            }

            ShipMoveStep::NonGuildShip => {
                let a = express!("Please decide to ship forces or pass.");
                let w = express!(current_faction(), " are thinking about shipping forces...");
                Self::status_ship_and_move(g, a, w)
            }

            ShipMoveStep::GuildShip if g.guild_may_delay() => {
                let a = express!("Please decide to ship now or delay your turn and let other factions go first.");
                let w = express!(Faction::Guild, " are deciding about taking their turn now...");
                Self::status_ship_and_move(g, a, w)
            }

            ShipMoveStep::GuildShip => {
                let a = express!("Please decide to ship forces or pass.");
                let w = express!(Faction::Guild, " are thinking about shipping forces...");
                Self::status_ship_and_move(g, a, w)
            }

            ShipMoveStep::BeneGesseritAccompaniesNonGuildShip | ShipMoveStep::BeneGesseritAccompaniesGuildShip => {
                let a = express!("Do you wish to accompany the latest shipment?");
                let w = express!(Faction::BeneGesserit, " are thinking about accompanying the latest shipment...");
                Self::faction_status(g, a, w, Some(Faction::BeneGesserit), None)
            }

            ShipMoveStep::NonGuildMove => {
                let a = express!("Please decide to move forces or pass.");
                let w = express!(current_faction(), " are thinking about moving forces.");
                Self::status_ship_and_move(g, a, w)
            }

            ShipMoveStep::GuildMove => {
                let a = express!("Please decide to move forces or pass.");
                let w = express!(Faction::Guild, " are thinking about moving forces.");
                Self::status_ship_and_move(g, a, w)
            }

            ShipMoveStep::Concluded => return None,
        })
    }
}

impl Game {
    /// What `player` may do in the shipment and move phase, during an intrusion or an alliance
    /// offered through one (see `add_player_actions`).
    pub(crate) fn add_ship_move_actions(&self, player: PlayerId, _is_host: bool, result: &mut Vec<EventType>) {
        let faction = self.player(player).faction;
        match self.interrupts.last() {
            Some(Interrupt::Intrusion { kind, .. }) => self.add_intrusion_actions(player, *kind, result),
            Some(Interrupt::AllianceByTerror { offered_to }) => {
                if faction == *offered_to {
                    result.push(EventType::AllianceByTerror);
                }
            }
            Some(Interrupt::AllianceByAmbassador { offered_to, .. }) => {
                if faction == Some(*offered_to) {
                    result.push(EventType::AllianceByAmbassador);
                }
            }
            Some(_) => {}
            None => {
                if let Some(s) = self.ship_move() {
                    self.add_ship_move_step_actions(player, s.step, result);
                }
            }
        }
    }

    /// Who may answer an intrusion: Bene Gesserit, Moritani, or Ecaz and an ally they share
    /// ambassadors with.
    fn add_intrusion_actions(&self, player: PlayerId, kind: IntrusionType, result: &mut Vec<EventType>) {
        let faction = self.player(player).faction;
        match kind {
            IntrusionType::BeneGesseritIntrusion => {
                if faction == Some(Faction::BeneGesserit) {
                    result.push(EventType::BeneGesseritFlip);
                }
            }
            IntrusionType::Terror => {
                if faction == Some(Faction::Moritani) {
                    result.push(EventType::TerrorRevealed);
                }
            }
            IntrusionType::Ambassador => {
                if faction == Some(Faction::Ecaz) || (self.player(player).ally == Some(Faction::Ecaz) && self.alliances.ecaz_shares_ambassadors) {
                    result.push(EventType::AmbassadorActivated);
                }
            }
        }
    }

    fn add_ship_move_step_actions(&self, player: PlayerId, step: ShipMoveStep, result: &mut Vec<EventType>) {
        let faction = self.player(player).faction;
        let has_caravan = || self.player(player).treachery_cards.iter().any(|c| c.type_() == TreacheryCardType::Caravan);
        match step {
            ShipMoveStep::GuildShip => {
                if faction == Some(Faction::Guild) {
                    if self.guild_may_delay() {
                        result.push(EventType::GuildDelay);
                    }
                    result.push(EventType::Shipment);
                    if has_caravan() && !PlacementEvent::valid_movement_sources(self, player).is_empty() {
                        result.push(EventType::Caravan);
                    }
                }
            }

            ShipMoveStep::BeneGesseritAccompaniesGuildShip | ShipMoveStep::BeneGesseritAccompaniesNonGuildShip => {
                if faction == Some(Faction::BeneGesserit) {
                    result.push(EventType::BeneGesseritAccompanies);
                }
            }

            ShipMoveStep::NonGuildShip => {
                if self.is_current_shipment_and_move_player(player) {
                    result.push(EventType::Shipment);
                    if has_caravan() {
                        result.push(EventType::Caravan);
                    }
                }
            }

            ShipMoveStep::GuildMove => {
                if faction == Some(Faction::Guild) {
                    self.add_move_actions(player, result);
                }
            }

            ShipMoveStep::NonGuildMove => {
                if self.is_current_shipment_and_move_player(player) {
                    self.add_move_actions(player, result);
                }
            }

            ShipMoveStep::Beginning | ShipMoveStep::Concluded => {}
        }
    }
}
