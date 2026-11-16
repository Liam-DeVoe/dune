use crate::prelude::*;
use crate::messages::part::Part;

/// State of the shipment and movement phase, including the intrusions and the alliances that terror
/// and ambassadors offer during it, and the Guild's shipment permissions.
pub struct ShipMoveState {
    pub shipment_and_move_sequence: Option<PlayerSequence>,
    /// Where the last shipment or move took forces, for the Bene Gesserit's reaction to it.
    pub last_shipment_or_movement: Option<ShipmentOrMovement>,
    pub intrusions: Vec<Intrusion>,
    pub ships_tech_token_income: bool,
    /// The moves of this phase, for the arrows on the board.
    pub recent_moves: Vec<RecentMove>,
    pub current_no_field_value: i32,
    pub latest_revealed_no_field_value: i32,
    // In the order the Guild granted them.
    pub shipment_permissions: IndexMap<Faction, ShipmentPermission>,
    pub factions_with_ornithopters_at_start_of_movement: Vec<Faction>,
    pub beginning_of_shipment_and_move_phase: bool,
    pub current_karma_shipment_prevention: Option<ShipmentPrevention>,
    pub chosen_destinations_with_allies: Vec<Territory>,
    pub current_player_may_perform_extra_move: bool,
    pub bene_gesserit_may_accompany: bool,
    pub phase_before_caravan_caused_intrusion: Option<Phase>,
    pub phase_before_revival_caused_intrusion: Option<Phase>,
    pub current_flight_used: Option<FlightUse>,
    /// Who used the Flight discovery token for their next move.
    pub current_flight_discovery_used: Option<By>,
    pub current_ambassador_activated: Option<AmbassadorActivation>,
    pub alliance_by_ambassador_offered_to: Option<Faction>,
    pub paused_ambassador_phase: Option<Phase>,
    pub paused_terror_phase: Option<Phase>,
    pub alliance_by_terror_was_offered: bool,
    pub alliance_by_terror_offered_to: Option<Faction>,
    pub in_guild_cunning_shipment: bool,
    pub current_blocked_territories: Vec<Territory>,
    pub choam_has_extra_move: bool,
    pub moves_logged: i32,
}

impl ShipMoveState {
    pub fn new() -> Self {
        ShipMoveState {
            shipment_and_move_sequence: None,
            last_shipment_or_movement: None,
            intrusions: Vec::new(),
            ships_tech_token_income: Default::default(),
            recent_moves: Vec::new(),
            current_no_field_value: -1,
            latest_revealed_no_field_value: -1,
            shipment_permissions: Default::default(),
            factions_with_ornithopters_at_start_of_movement: Vec::new(),
            beginning_of_shipment_and_move_phase: Default::default(),
            current_karma_shipment_prevention: None,
            chosen_destinations_with_allies: Vec::new(),
            current_player_may_perform_extra_move: Default::default(),
            bene_gesserit_may_accompany: Default::default(),
            phase_before_caravan_caused_intrusion: Default::default(),
            phase_before_revival_caused_intrusion: Default::default(),
            current_flight_used: None,
            current_flight_discovery_used: None,
            current_ambassador_activated: None,
            alliance_by_ambassador_offered_to: Default::default(),
            paused_ambassador_phase: Default::default(),
            paused_terror_phase: Default::default(),
            alliance_by_terror_was_offered: Default::default(),
            alliance_by_terror_offered_to: Default::default(),
            in_guild_cunning_shipment: Default::default(),
            current_blocked_territories: Vec::new(),
            choam_has_extra_move: Default::default(),
            moves_logged: Default::default(),
        }
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

/// The Ambassador activation that the current intrusion led to (`AmbassadorActivated`), as
/// `AllianceByAmbassador` reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AmbassadorActivation {
    pub submitter: By,
    pub ecaz_give_vidal_to_ally: bool,
    pub ecaz_take_vidal: bool,
}

impl AmbassadorActivation {
    pub fn initiator(&self) -> Option<Faction> {
        self.submitter.faction
    }

    #[track_caller]
    pub fn player(&self) -> PlayerId {
        self.submitter.player()
    }
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
        let current_is_guild = match &self.ship_move.shipment_and_move_sequence {
            Some(seq) => seq.current_faction(self) == Faction::Guild,
            None => false,
        };
        if current_is_guild && self.guild_may_ship_out_of_turn_order() {
            let mut seq = self.ship_move.shipment_and_move_sequence.take().expect("shipment and move sequence");
            seq.next_player(self);
            self.ship_move.shipment_and_move_sequence = Some(seq);
        }

        // Every argument is evaluated before Enter runs, including the call that may log.
        let condition1 = self.juice_forces_first_player() && self.turn.current_juice.clone().expect("current juice").initiator() != Some(Faction::Guild);
        let condition2 = self.check_if_guild_acts_or_auto_delays();
        self.enter_first(&[(condition1, Step::Enter(Phase::NonGuildShip)), (condition2, Step::Enter(Phase::GuildShip))], Step::Enter(Phase::NonGuildShip))?;
        Ok(())
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

        self.ship_move.last_shipment_or_movement = Some(ShipmentOrMovement { to: Some(to) });
        let was_occupied_before_move = self.is_occupied_territory(to.territory());
        let values: Vec<Battalion> = force_locations.values().cloned().collect();
        let dist = self.determine_maximum_move_distance(initiator, &values);
        let move_number = if matches!(evt, Event::Move(_) | Event::Caravan(_)) {
            self.ship_move.moves_logged += 1;
            Some(self.ship_move.moves_logged)
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
        self.turn.phase_before_discarding = Some(self.turn.current_phase);
        self.turn.factions_that_must_discard.push(f);
        self.enter(Phase::Discarding, true);
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

        if self.ship_move.bene_gesserit_may_accompany {
            if self.prevented(FactionAdvantage::BeneGesseritAccompanies) {
                self.log_prevention_by_karma(FactionAdvantage::BeneGesseritAccompanies);
                self.ship_move.bene_gesserit_may_accompany = false;
                if !self.applicable(Rule::FullPhaseKarma) {
                    self.allow(FactionAdvantage::BeneGesseritAccompanies);
                }
            } else if self.has_low_threshold(Faction::BeneGesserit) {
                self.ship_move.bene_gesserit_may_accompany = false;
            }
        }

        if !self.storm.storm_losses_to_take.is_empty() {
            self.storm.phase_before_storm_loss = Some(self.turn.current_phase);
            self.enter(Phase::StormLosses, true);
        } else if !self.ship_move.intrusions.is_empty() {
            match self.ship_move.intrusions[0].type_ {
                IntrusionType::BeneGesseritIntrusion => self.determine_next_shipment_and_move_sub_phase_on_bene_gesserit_intruded()?,
                IntrusionType::Terror => self.determine_next_shipment_and_move_sub_phase_on_terror_triggered()?,
                IntrusionType::Ambassador => self.determine_next_shipment_and_move_sub_phase_on_ambassador_triggered()?,
            }
        } else {
            self.determine_next_shipment_and_move_sub_phase_on_no_intrusion()?;
        }

        self.skip_move_if_nothing_to_move()?;
        Ok(())
    }

    pub fn skip_move_if_nothing_to_move(&mut self) -> GameResult {
        if !matches!(self.turn.current_phase, Phase::GuildMove | Phase::NonGuildMove) {
            return Ok(());
        }

        let player = if self.turn.current_phase == Phase::GuildMove {
            self.get_player(Some(Faction::Guild))
        } else {
            self.ship_move.shipment_and_move_sequence.as_ref().required("shipment and move sequence")?.current_player(self)
        };

        let Some(player) = player else { return Ok(()) };

        if !PlacementEvent::valid_movement_sources(self, player).is_empty() {
            return Ok(());
        }

        self.storm.storm_losses_to_take.clear();
        let pf = self.player(player).faction;
        self.turn.has_acted_or_passed.push(pf.required("faction")?);

        if self.turn.current_phase == Phase::NonGuildMove {
            let mut seq = self.ship_move.shipment_and_move_sequence.take().required("shipment and move sequence")?;
            seq.next_player(self);
            self.ship_move.shipment_and_move_sequence = Some(seq);
            let current_is_guild = self.ship_move.shipment_and_move_sequence.as_ref().required("shipment and move sequence")?.current_faction(self) == Faction::Guild;
            if current_is_guild && self.guild_may_ship_out_of_turn_order() {
                let mut seq = self.ship_move.shipment_and_move_sequence.take().required("shipment and move sequence")?;
                seq.next_player(self);
                self.ship_move.shipment_and_move_sequence = Some(seq);
            }
        }

        if self.ship_move.in_guild_cunning_shipment {
            self.turn.current_guild_cunning = None;
            self.ship_move.in_guild_cunning_shipment = false;
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

        self.ship_move.current_flight_used = None;
        self.ship_move.current_flight_discovery_used = None;
        self.turn.current_planetology = None;
        Ok(())
    }

    pub fn cleanup_obsolete_intrusions(&mut self) -> GameResult {
        if !self.ship_move.intrusions.is_empty() {
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

    pub fn determine_next_shipment_and_move_sub_phase_on_no_intrusion(&mut self) -> GameResult {
        match self.turn.current_phase {
            Phase::FremenRidingMonsterA
            | Phase::BeneGesseritIntrudedByFremenRidingMonsterA
            | Phase::TerrorTriggeredByFremenRidingMonsterA
            | Phase::AmbassadorTriggeredByFremenRidingMonsterA => {
                self.end_worm_ride_during_phase(Phase::FremenRidingMonsterA)?;
            }

            Phase::FremenRidingMonsterB
            | Phase::BeneGesseritIntrudedByFremenRidingMonsterB
            | Phase::TerrorTriggeredByFremenRidingMonsterB
            | Phase::AmbassadorTriggeredByFremenRidingMonsterB => {
                self.end_worm_ride_during_phase(Phase::FremenRidingMonsterB)?;
            }

            Phase::GuildShip | Phase::BeneGesseritIntrudedByGuildShip | Phase::TerrorTriggeredByGuildShip | Phase::AmbassadorTriggeredByGuildShip => {
                let c1 = self.is_playing(Faction::BeneGesserit) && self.ship_move.bene_gesserit_may_accompany;
                let c2 = !self.ship_move.in_guild_cunning_shipment;
                self.enter_first(&[(c1, Step::Enter(Phase::BeneGesseritAccompaniesGuildShip)), (c2, Step::Enter(Phase::GuildMove))], Step::Enter(Phase::NonGuildMove))?;
            }

            Phase::BeneGesseritAccompaniesGuildShip | Phase::TerrorTriggeredByBeneGesseritAccompaniesGuildShip | Phase::AmbassadorTriggeredByBeneGesseritAccompaniesGuildShip => {
                let c = !self.ship_move.in_guild_cunning_shipment;
                self.enter_either(c, Phase::GuildMove, Phase::NonGuildMove);
            }

            Phase::NonGuildShip | Phase::BeneGesseritIntrudedByNonGuildShip | Phase::TerrorTriggeredByNonGuildShip | Phase::AmbassadorTriggeredByNonGuildShip => {
                let c = self.is_playing(Faction::BeneGesserit) && self.ship_move.bene_gesserit_may_accompany;
                self.enter_either(c, Phase::BeneGesseritAccompaniesNonGuildShip, Phase::NonGuildMove);
            }

            Phase::BeneGesseritAccompaniesNonGuildShip | Phase::TerrorTriggeredByBeneGesseritAccompaniesNonGuildShip | Phase::AmbassadorTriggeredByBeneGesseritAccompaniesNonGuildShip => {
                self.enter(Phase::NonGuildMove, true);
            }

            Phase::GuildMove | Phase::BeneGesseritIntrudedByGuildMove | Phase::TerrorTriggeredByGuildMove | Phase::AmbassadorTriggeredByGuildMove => {
                self.determine_next_sub_phase_after_guild_ship_and_move()?;
            }

            Phase::NonGuildMove | Phase::BeneGesseritIntrudedByNonGuildMove | Phase::TerrorTriggeredByNonGuildMove | Phase::AmbassadorTriggeredByNonGuildMove => {
                self.determine_next_sub_phase_after_non_guild_move()?;
            }

            Phase::BeneGesseritIntrudedByCaravan | Phase::TerrorTriggeredByCaravan | Phase::AmbassadorTriggeredByCaravan => {
                let p = self.ship_move.phase_before_caravan_caused_intrusion.required("phase before caravan caused intrusion")?;
                self.enter(p, true);
            }

            Phase::BeneGesseritIntrudedByRevival | Phase::TerrorTriggeredByRevival | Phase::AmbassadorTriggeredByRevival => {
                let mut next_phase = self.ship_move.phase_before_revival_caused_intrusion.required("phase before revival caused intrusion")?;
                if next_phase == Phase::AssigningSkill && !SkillAssigned::players_must_choose_leader_skills(self) {
                    next_phase = self.setup.phase_before_skill_assignment.required("phase before skill assignment")?;
                    if next_phase == self.turn.current_phase {
                        next_phase = Phase::Resurrection;
                    }
                }

                self.enter(next_phase, true);
            }

            _ => {}
        }
        Ok(())
    }

    pub fn determine_next_shipment_and_move_sub_phase_on_bene_gesserit_intruded(&mut self) -> GameResult {
        let next = match self.turn.current_phase {
            Phase::FremenRidingMonsterA
            | Phase::BeneGesseritIntrudedByFremenRidingMonsterA
            | Phase::TerrorTriggeredByFremenRidingMonsterA
            | Phase::AmbassadorTriggeredByFremenRidingMonsterA => Phase::BeneGesseritIntrudedByFremenRidingMonsterA,

            Phase::FremenRidingMonsterB
            | Phase::BeneGesseritIntrudedByFremenRidingMonsterB
            | Phase::TerrorTriggeredByFremenRidingMonsterB
            | Phase::AmbassadorTriggeredByFremenRidingMonsterB => Phase::BeneGesseritIntrudedByFremenRidingMonsterB,

            Phase::GuildShip
            | Phase::BeneGesseritIntrudedByGuildShip
            | Phase::TerrorTriggeredByGuildShip
            | Phase::AmbassadorTriggeredByGuildShip
            | Phase::TerrorTriggeredByBeneGesseritAccompaniesGuildShip
            | Phase::AmbassadorTriggeredByBeneGesseritAccompaniesGuildShip => Phase::BeneGesseritIntrudedByGuildShip,

            Phase::NonGuildShip
            | Phase::BeneGesseritIntrudedByNonGuildShip
            | Phase::TerrorTriggeredByNonGuildShip
            | Phase::AmbassadorTriggeredByNonGuildShip
            | Phase::TerrorTriggeredByBeneGesseritAccompaniesNonGuildShip
            | Phase::AmbassadorTriggeredByBeneGesseritAccompaniesNonGuildShip => Phase::BeneGesseritIntrudedByNonGuildShip,

            Phase::GuildMove | Phase::BeneGesseritIntrudedByGuildMove | Phase::TerrorTriggeredByGuildMove | Phase::AmbassadorTriggeredByGuildMove => Phase::BeneGesseritIntrudedByGuildMove,

            Phase::NonGuildMove | Phase::BeneGesseritIntrudedByNonGuildMove | Phase::TerrorTriggeredByNonGuildMove | Phase::AmbassadorTriggeredByNonGuildMove => Phase::BeneGesseritIntrudedByNonGuildMove,

            Phase::BeneGesseritIntrudedByCaravan | Phase::TerrorTriggeredByCaravan | Phase::AmbassadorTriggeredByCaravan => Phase::BeneGesseritIntrudedByCaravan,

            Phase::BeneGesseritIntrudedByRevival | Phase::TerrorTriggeredByRevival | Phase::AmbassadorTriggeredByRevival => Phase::BeneGesseritIntrudedByRevival,

            _ => game_error!("BeneGesserit intrusion triggered during undefined phase: {}", self.turn.current_phase),
        };
        self.enter(next, true);
        Ok(())
    }

    pub fn determine_next_shipment_and_move_sub_phase_on_terror_triggered(&mut self) -> GameResult {
        let next = match self.turn.current_phase {
            Phase::FremenRidingMonsterA
            | Phase::BeneGesseritIntrudedByFremenRidingMonsterA
            | Phase::TerrorTriggeredByFremenRidingMonsterA
            | Phase::AmbassadorTriggeredByFremenRidingMonsterA => Phase::TerrorTriggeredByFremenRidingMonsterA,

            Phase::FremenRidingMonsterB
            | Phase::BeneGesseritIntrudedByFremenRidingMonsterB
            | Phase::TerrorTriggeredByFremenRidingMonsterB
            | Phase::AmbassadorTriggeredByFremenRidingMonsterB => Phase::TerrorTriggeredByFremenRidingMonsterB,

            Phase::GuildShip | Phase::BeneGesseritIntrudedByGuildShip | Phase::TerrorTriggeredByGuildShip | Phase::AmbassadorTriggeredByGuildShip => Phase::TerrorTriggeredByGuildShip,

            // C# parity: unlike the Ambassador mapping, BeneGesseritAccompanies phases go back to the plain ship phase.
            Phase::BeneGesseritAccompaniesGuildShip | Phase::TerrorTriggeredByBeneGesseritAccompaniesGuildShip | Phase::AmbassadorTriggeredByBeneGesseritAccompaniesGuildShip => Phase::TerrorTriggeredByGuildShip,

            Phase::NonGuildShip | Phase::BeneGesseritIntrudedByNonGuildShip | Phase::TerrorTriggeredByNonGuildShip | Phase::AmbassadorTriggeredByNonGuildShip => Phase::TerrorTriggeredByNonGuildShip,

            Phase::BeneGesseritAccompaniesNonGuildShip | Phase::TerrorTriggeredByBeneGesseritAccompaniesNonGuildShip | Phase::AmbassadorTriggeredByBeneGesseritAccompaniesNonGuildShip => Phase::TerrorTriggeredByNonGuildShip,

            Phase::GuildMove | Phase::BeneGesseritIntrudedByGuildMove | Phase::TerrorTriggeredByGuildMove | Phase::AmbassadorTriggeredByGuildMove => Phase::TerrorTriggeredByGuildMove,

            Phase::NonGuildMove | Phase::BeneGesseritIntrudedByNonGuildMove | Phase::TerrorTriggeredByNonGuildMove | Phase::AmbassadorTriggeredByNonGuildMove => Phase::TerrorTriggeredByNonGuildMove,

            Phase::BeneGesseritIntrudedByCaravan | Phase::TerrorTriggeredByCaravan | Phase::AmbassadorTriggeredByCaravan => Phase::TerrorTriggeredByCaravan,

            Phase::BeneGesseritIntrudedByRevival | Phase::TerrorTriggeredByRevival | Phase::AmbassadorTriggeredByRevival => Phase::TerrorTriggeredByRevival,

            _ => game_error!("Terror triggered during undefined phase: {}", self.turn.current_phase),
        };
        self.enter(next, true);
        Ok(())
    }

    pub fn determine_next_shipment_and_move_sub_phase_on_ambassador_triggered(&mut self) -> GameResult {
        let next = match self.turn.current_phase {
            Phase::FremenRidingMonsterA
            | Phase::BeneGesseritIntrudedByFremenRidingMonsterA
            | Phase::TerrorTriggeredByFremenRidingMonsterA
            | Phase::AmbassadorTriggeredByFremenRidingMonsterA => Phase::AmbassadorTriggeredByFremenRidingMonsterA,

            Phase::FremenRidingMonsterB
            | Phase::BeneGesseritIntrudedByFremenRidingMonsterB
            | Phase::TerrorTriggeredByFremenRidingMonsterB
            | Phase::AmbassadorTriggeredByFremenRidingMonsterB => Phase::AmbassadorTriggeredByFremenRidingMonsterB,

            Phase::GuildShip | Phase::BeneGesseritIntrudedByGuildShip | Phase::TerrorTriggeredByGuildShip | Phase::AmbassadorTriggeredByGuildShip => Phase::AmbassadorTriggeredByGuildShip,

            Phase::BeneGesseritAccompaniesGuildShip | Phase::TerrorTriggeredByBeneGesseritAccompaniesGuildShip | Phase::AmbassadorTriggeredByBeneGesseritAccompaniesGuildShip => Phase::AmbassadorTriggeredByBeneGesseritAccompaniesGuildShip,

            Phase::NonGuildShip | Phase::BeneGesseritIntrudedByNonGuildShip | Phase::TerrorTriggeredByNonGuildShip | Phase::AmbassadorTriggeredByNonGuildShip => Phase::AmbassadorTriggeredByNonGuildShip,

            Phase::BeneGesseritAccompaniesNonGuildShip | Phase::TerrorTriggeredByBeneGesseritAccompaniesNonGuildShip | Phase::AmbassadorTriggeredByBeneGesseritAccompaniesNonGuildShip => Phase::AmbassadorTriggeredByBeneGesseritAccompaniesNonGuildShip,

            Phase::GuildMove | Phase::BeneGesseritIntrudedByGuildMove | Phase::TerrorTriggeredByGuildMove | Phase::AmbassadorTriggeredByGuildMove => Phase::AmbassadorTriggeredByGuildMove,

            Phase::NonGuildMove | Phase::BeneGesseritIntrudedByNonGuildMove | Phase::TerrorTriggeredByNonGuildMove | Phase::AmbassadorTriggeredByNonGuildMove => Phase::AmbassadorTriggeredByNonGuildMove,

            Phase::BeneGesseritIntrudedByCaravan | Phase::TerrorTriggeredByCaravan | Phase::AmbassadorTriggeredByCaravan => Phase::AmbassadorTriggeredByCaravan,

            Phase::BeneGesseritIntrudedByRevival | Phase::TerrorTriggeredByRevival | Phase::AmbassadorTriggeredByRevival => Phase::AmbassadorTriggeredByRevival,

            _ => game_error!("Ambassador triggered during undefined phase: {}", self.turn.current_phase),
        };
        self.enter(next, true);
        Ok(())
    }

    pub fn check_if_forces_should_be_destroyed_by_ally_presence(&mut self, p: PlayerId) -> GameResult {
        let (p_ally, p_faction) = (self.player(p).ally, self.player(p).faction);
        if p_ally != None && p_faction != Some(Faction::Ecaz) && p_ally != Some(Faction::Ecaz) {
            let special_forces_are_allowed = self.player(p).is(Faction::BeneGesserit) && self.applicable(Rule::AdvisorsDontConflictWithAlly);

            //Forces that must be destroyed because moves ended where allies are
            let mut i = 0;
            while i < self.ship_move.chosen_destinations_with_allies.len() {
                let t = self.ship_move.chosen_destinations_with_allies[i];
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

    pub fn determine_next_sub_phase_after_non_guild_move(&mut self) -> GameResult {
        if self.ship_move.current_player_may_perform_extra_move {
            self.ship_move.current_player_may_perform_extra_move = false;
            self.enter(Phase::NonGuildMove, true);
        } else {
            let c1 = self.everyone_acted_or_passed();
            let c2 = self.check_if_guild_acts_or_auto_delays();
            self.enter_first(&[(c1, Step::Run(Game::conclude_shipment_and_move)), (c2, Step::Enter(Phase::GuildShip))], Step::Enter(Phase::NonGuildShip))?;
        }
        Ok(())
    }

    pub fn determine_next_sub_phase_after_guild_ship_and_move(&mut self) -> GameResult {
        if self.ship_move.current_player_may_perform_extra_move {
            self.ship_move.current_player_may_perform_extra_move = false;
            self.enter(Phase::GuildMove, true);
        } else if !self.ship_move.in_guild_cunning_shipment && self.turn.current_guild_cunning.as_ref().is_some_and(|c| c.by(Faction::Guild)) {
            self.ship_move.in_guild_cunning_shipment = true;
            self.enter(Phase::GuildShip, true);
        } else {
            let c = !self.everyone_acted_or_passed();
            self.enter_or_else_try(c, Phase::NonGuildShip, Game::conclude_shipment_and_move)?;
        }
        Ok(())
    }

    pub fn conclude_shipment_and_move(&mut self) -> GameResult {
        self.check_if_moritani_gains_vidal();
        self.ship_move.current_karma_shipment_prevention = None;
        self.main_phase_end();
        self.enter(Phase::ShipmentAndMoveConcluded, true);
        self.receive_ships_tech_income();
        self.ship_move.choam_has_extra_move = false;
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
        if self.ship_move.ships_tech_token_income {
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
        let territory = e.to_location(self).required("destination")?.territory();
        self.check_bene_gesserit_intrusion(e, initiator, territory);
        self.check_ambassador_triggered(e, initiator);
        self.check_terror_triggered(e, initiator);
        Ok(!self.ship_move.intrusions.is_empty())
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

    pub fn check_ambassador_triggered(&mut self, e: &Event, initiator: Faction) {
        let ecaz_player = self.get_player(Some(Faction::Ecaz));
        if let Some(ecaz) = ecaz_player {
            if (initiator != Faction::BeneGesserit || e.location_event_forces_added_to_location(self) > 0)
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
        let intrusion = Intrusion { initiator, to: e.to_location(self), type_ };
        self.ship_move.intrusions.push(intrusion);
    }

    pub fn dequeue_intrusion(&mut self, type_: IntrusionType) -> GameResult {
        if self.ship_move.intrusions.is_empty() {
            game_error!("No intrusions to dequeue");
        }
        if self.ship_move.intrusions[0].type_ != type_ {
            game_error!("Wrong intrusion type: actual:{} vs expected:{}", type_, self.ship_move.intrusions[0].type_);
        }

        self.ship_move.intrusions.remove(0);
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
        match &self.ship_move.current_karma_shipment_prevention {
            Some(k) => k.target == f,
            None => false,
        }
    }

    pub fn has_shipment_permission(&self, p: PlayerId, permission: ShipmentPermission) -> bool {
        match self.ship_move.shipment_permissions.get_opt(self.player(p).faction) {
            Some(permissions) => permissions.contains(permission),
            None => false,
        }
    }

    pub fn last_bene_gesserit_intrusion(&self) -> Option<Intrusion> {
        match self.ship_move.intrusions.first() {
            Some(i) if i.type_ == IntrusionType::BeneGesseritIntrusion => Some(i.clone()),
            _ => None,
        }
    }

    pub fn last_terror_trigger(&self) -> Option<Intrusion> {
        match self.ship_move.intrusions.first() {
            Some(i) if i.type_ == IntrusionType::Terror => Some(i.clone()),
            _ => None,
        }
    }

    pub fn last_ambassador_trigger(&self) -> Option<Intrusion> {
        match self.ship_move.intrusions.first() {
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
}

impl Intrusion {
    pub fn territory(&self, _game: &Game) -> Territory {
        self.to.expect("to location").territory()
    }

    pub fn initiator(&self) -> Faction {
        self.initiator
    }
}
