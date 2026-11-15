//! The `Game` struct and its rules. Each submodule holds the `impl Game` block for one phase or
//! topic, plus the state types it uses; see docs/engine.md.

mod alliances;
pub use alliances::Alliances;
mod applicable_events;
mod battalion;
pub use battalion::*;
mod battle;
pub use battle::*;
mod battle_outcome;
pub use battle_outcome::*;
mod bidding;
pub use bidding::BiddingState;
pub(crate) use bidding::*;
mod blow;
pub use blow::*;
mod board;
pub use board::Board;
mod charity;
pub use charity::CharityState;
mod collection;
pub use collection::*;
mod contemplate;
pub use contemplate::ContemplateState;
mod counters;
pub use counters::*;
mod deals;
pub use deals::*;
mod decks;
pub use decks::Decks;
mod history;
pub use history::History;
mod info;
pub use info::*;
mod leaders;
pub use leaders::*;
mod log;
pub use log::Log;
mod moment;
pub use moment::*;
mod movement;
pub use movement::*;
mod other;
pub use other::*;
mod participation;
pub use participation::*;
mod player;
pub use player::*;
mod player_sequence;
pub use player_sequence::*;
mod revival;
pub use revival::*;
mod rules;
pub use rules::Rules;
mod settings;
pub use settings::*;
mod setup;
pub use setup::SetupState;
mod state;
pub use state::*;
mod statistics;
pub use statistics::*;
mod status;
pub use status::*;
mod storm;
pub use storm::StormState;
mod turn;
pub use turn::TurnState;

use crate::prelude::*;

/// What the game does next when a phase ends: enter a phase, or run a method that decides.
#[derive(Clone, Copy)]
pub enum Step {
    Enter(Phase),
    Run(fn(&mut Game)),
}

/// A game: its rules, players and board, and the state of each phase. Each field's type is
/// defined in the file of its topic under `game/` (see docs/engine.md).
pub struct Game {
    pub seed: i32,
    pub random: Option<SharedRandom>,
    pub rules: Rules,
    pub players: Vec<Player>,
    /// Shared with the caller of `new Game` and with every game `Undo`,
    /// `RestoreRecentlyUndone` and `DismissRecentlyUndone` make from this one.
    pub participation: Shared<Participation>,
    pub board: Board,
    pub decks: Decks,
    pub leaders: Leaders,
    pub alliances: Alliances,
    pub deals: Deals,
    pub turn: TurnState,
    pub history: History,
    pub log: Log,

    // The state of each phase. They are all kept side by side, whatever the current phase.
    pub setup: SetupState,
    pub storm: StormState,
    pub blow: BlowState,
    pub charity: CharityState,
    pub bidding: BiddingState,
    pub revival: RevivalState,
    pub ship_move: ShipMoveState,
    pub battle: BattleState,
    pub collection: CollectionState,
    pub contemplate: ContemplateState,
}

impl Game {
    pub const LATEST_VERSION: i32 = 189;

    pub fn maximum_turns(&self) -> i32 {
        self.rules.settings.as_ref().expect("settings").borrow().maximum_turns
    }

    pub fn maximum_players(&self) -> i32 {
        self.rules.settings.as_ref().expect("settings").borrow().number_of_players
    }

    pub fn reports(&self) -> &[Report] {
        &self.log.report_list
    }

    /// The live report; use [`Game::current_report_mut`] to change it.
    pub fn current_report(&self) -> &Report {
        if self.log.report_list.len() > 0 {
            &self.log.report_list[self.log.report_list.len() - 1]
        } else {
            &self.log.pre_game_report
        }
    }

    /// `CurrentReport`, for mutation.
    pub fn current_report_mut(&mut self) -> &mut Report {
        if self.log.report_list.len() > 0 {
            let last = self.log.report_list.len() - 1;
            &mut self.log.report_list[last]
        } else {
            &mut self.log.pre_game_report
        }
    }

    pub fn new() -> Self {
        Self::with_participation(Self::LATEST_VERSION, &Shared::new(Participation::new()))
    }

    /// Only `LATEST_VERSION` games can be played, so savegames and event logs of older versions
    /// fail to load instead of replaying.
    pub fn check_version(version: i32) -> Result<(), String> {
        if version != Self::LATEST_VERSION {
            return Err(format!("Game version {} is not supported. Only version {} is supported.", version, Self::LATEST_VERSION));
        }
        Ok(())
    }

    /// The game keeps the caller's `Participation` object: changes made
    /// through either are seen by both.
    ///
    /// Only `LATEST_VERSION` is supported (see [`Game::check_version`]); other versions panic.
    pub fn with_participation(version: i32, participation: &Shared<Participation>) -> Self {
        if let Err(e) = Self::check_version(version) {
            panic!("{e}");
        }

        let mut game = Game {
            seed: -1,
            random: None,
            rules: Rules::new(version),
            players: Vec::new(),
            participation: participation.clone(),
            board: Board::new(),
            decks: Decks::new(),
            leaders: Leaders::new(),
            alliances: Alliances::new(),
            deals: Deals::new(),
            turn: TurnState::new(),
            history: History::new(),
            log: Log::new(),
            setup: SetupState::new(),
            storm: StormState::new(),
            blow: BlowState::new(),
            charity: CharityState::new(),
            bidding: BiddingState::new(),
            revival: RevivalState::new(),
            ship_move: ShipMoveState::new(),
            battle: BattleState::new(),
            collection: CollectionState::new(),
            contemplate: ContemplateState::new(),
        };

        game.initialize_leader_state();
        game.enter_phase_awaiting_players();
        game
    }

    pub fn initialize_leader_state(&mut self) {
        for l in LeaderManager::leaders() {
            self.leaders.leader_state.insert_new(Hero::Leader(*l), LeaderState { death_counter: 0, current_territory: None, skill: LeaderSkill::None, in_front_of_shield: false, ..Default::default() });
        }

        self.leaders.leader_state.insert_new(LeaderManager::messiah(), LeaderState { death_counter: 0, current_territory: None, ..Default::default() });
    }

    pub fn perform_pre_event_tasks(&mut self, e: &GameEvent) {
        self.update_timers(e);

        // e is AllyPermission or PlayerReplaced or NexusPlayed { IsBetrayal: true, Faction: Faction.Richese }
        let skip = match e {
            GameEvent::AllyPermission(_) | GameEvent::PlayerReplaced(_) => true,
            GameEvent::NexusPlayed(np) => np.is_betrayal(self) && np.faction == Faction::Richese,
            _ => false,
        };
        if skip {
            return;
        }

        self.clear_recent_payments();

        if !matches!(e, GameEvent::DealOffered(_) | GameEvent::DealAccepted(_) | GameEvent::DealRejected(_)) {
            self.decks.recently_discarded.clear();
        }
    }

    pub fn perform_post_event_tasks(&mut self, e: &GameEvent, just_entered_start_of_phase: bool) {
        if !just_entered_start_of_phase
            && !matches!(
                e,
                GameEvent::AllyPermission(_) | GameEvent::DealOffered(_) | GameEvent::DealAccepted(_) | GameEvent::DealRejected(_)
            )
        {
            self.main_phase_middle();
        }

        self.history.entries.push(e.clone());
        self.history.moments.push(Moment::new(self.turn.current_turn, self.turn.current_main_phase));
    }

    /// Goes back to the state after the first `until_event_nr` events: a new game replays
    /// them, and the events after them become `recently_undone_events`. The current game
    /// is kept as `previous_game_state`, so `restore_recently_undone` can return to it. A
    /// negative `until_event_nr` throws and leaves this game unchanged.
    pub fn undo(&mut self, until_event_nr: i32) {
        let history = self.history.entries.clone();
        let history_count = history.len() as i32;
        let max_event_nr = std::cmp::min(until_event_nr, history_count);
        let mut result = self.replay(&history[..max_event_nr.max(0) as usize]);

        result.history.recently_undone_event_count = history_count - max_event_nr;
        for i in max_event_nr..history_count {
            // A negative event number panics here.
            result.history.recently_undone_events.push(history[i as usize].clone());
        }

        let previous = std::mem::replace(self, result);
        self.history.previous_game_state = Some(Box::new(previous));
    }

    /// Returns to the game as it was before the last `undo`. Without one (a game loaded
    /// with undone events), replays the history followed by the undone events.
    pub fn restore_recently_undone(&mut self) {
        if let Some(previous) = self.history.previous_game_state.take() {
            *self = *previous;
            return;
        }

        if self.history.recently_undone_event_count == 0 || self.history.recently_undone_events.is_empty() {
            return;
        }

        let events: Vec<GameEvent> = self.history.entries.iter().chain(self.history.recently_undone_events.iter()).cloned().collect();
        *self = self.replay(&events);
    }

    /// Forgets the undone events and the game before the undo: the history is replayed
    /// into a new game.
    pub fn dismiss_recently_undone(&mut self) {
        let history = self.history.entries.clone();
        *self = self.replay(&history);
    }

    /// A new game of the same version and participation that has executed `events`.
    fn replay(&self, events: &[GameEvent]) -> Game {
        let mut result = Game::with_participation(self.rules.version, &self.participation);
        for e in events {
            e.initialize_game(&result);
            e.execute(&mut result, false, false);
        }
        result
    }

    pub fn update_timers(&mut self, e: &GameEvent) {
        if e.time() != DateTimeOffset::default() && self.history.entries.len() > 0 && self.in_timed_phase() {
            let previous_event = match e.event_type() {
                GameEventType::Battle => self.find_most_recent_event(vec![GameEventType::BattleInitiated]),
                GameEventType::Move => self.find_most_recent_event(vec![GameEventType::Shipment, GameEventType::EndPhase]),
                GameEventType::Shipment => self.find_most_recent_event(vec![GameEventType::Move, GameEventType::EndPhase]),
                _ => self.find_most_recent_event(vec![]),
            };

            let Some(previous_event) = previous_event else { return };

            // DateTimeOffset.Subtract compares UTC instants.
            let elapsed_time = TimeSpan::from_ticks(e.time().utc_ticks() - previous_event.time().utc_ticks());
            if !(elapsed_time.total_hours() < 1.0) {
                return;
            }

            let initiator = e.initiator();
            if self.history.timers.get(&initiator).is_none() {
                let timer = Timer::<MainPhase>::new();
                self.history.timers.insert_new(initiator, timer);
            }

            let phase = self.turn.current_main_phase;
            self.history.timers.get_mut(&initiator).expect("a timer for this faction").add(phase, elapsed_time);
        }
    }

    pub fn duration(&self) -> TimeSpan {
        if self.history.entries.len() > 0 {
            let last = &self.history.entries[self.history.entries.len() - 1];
            TimeSpan::from_ticks(last.time().utc_ticks() - self.history.entries[0].time().utc_ticks())
        } else {
            TimeSpan::default()
        }
    }

    pub fn time_spent(&self, faction: Faction, phase: MainPhase) -> TimeSpan {
        if let Some(timer) = self.history.timers.get(&faction) {
            return timer.time_spent(phase);
        }
        TimeSpan::default()
    }

    pub fn started(&self) -> DateTimeOffset {
        if self.history.entries.len() > 0 {
            self.history.entries[0].time()
        } else {
            DateTimeOffset::default()
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

    pub fn latest_event(&self, event_type: GameEventType) -> Option<GameEvent> {
        // List<T>.LastOrDefault(pred) walks backwards; the predicate has no side effects.
        self.history.entries.iter().rev().find(|e| e.event_type() == event_type).cloned()
    }

    pub fn latest_event_0(&self) -> Option<GameEvent> {
        if self.history.entries.len() > 0 {
            Some(self.history.entries[self.history.entries.len() - 1].clone())
        } else {
            None
        }
    }

    pub fn event_count(&self) -> i32 {
        self.history.entries.len() as i32
    }

    pub fn find_most_recent_event(&self, types: Vec<GameEventType>) -> Option<GameEvent> {
        if types.len() == 0 {
            return Some(self.history.entries.last().expect("an event in the history").clone());
        }

        let mut i = self.history.entries.len() as i32 - 1;
        while i >= 0 {
            if types.contains(&self.history.entries[i as usize].event_type()) {
                return Some(self.history.entries[i as usize].clone());
            }
            i -= 1;
        }

        None
    }

    /// Returns the live report, not a copy.
    pub fn start_report(&mut self, about: MainPhase) -> &mut Report {
        // Avoid leaving an empty duplicate behind if a report for the same phase is started twice in a row
        let current_turn = self.turn.current_turn;
        let has_reports = self.log.report_list.len() > 0;
        let current = self.current_report();
        if has_reports && current.about == about && current.turn == current_turn && !current.is_battle() && current.messages.len() == 0 {
            return self.current_report_mut();
        }

        let result = Report::new(about, self.turn.current_turn);
        self.log.report_list.push(result);
        let last = self.log.report_list.len() - 1;
        &mut self.log.report_list[last]
    }

    pub fn main_phase_start(&mut self, phase: MainPhase, start_report: bool) {
        self.turn.current_main_phase = phase;
        self.turn.current_moment = MainPhaseMoment::Start;
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
        if self.turn.current_moment == MainPhaseMoment::Start {
            self.turn.current_moment = MainPhaseMoment::Middle;
        }
    }

    pub fn main_phase_end(&mut self) {
        self.turn.current_moment = MainPhaseMoment::End;

        let mut exceptions_to_allowing: Vec<FactionAdvantage> = Vec::new();

        if self.turn.current_main_phase == MainPhase::Bidding && self.prevented(FactionAdvantage::TleilaxuIncreasingRevivalLimits) {
            exceptions_to_allowing.push(FactionAdvantage::TleilaxuIncreasingRevivalLimits);
        }

        if self.turn.current_main_phase == MainPhase::Resurrection && self.prevented(FactionAdvantage::AtreidesSpiceBlowPrescience) {
            exceptions_to_allowing.push(FactionAdvantage::AtreidesSpiceBlowPrescience);
        }

        self.turn.was_victim_of_bureaucracy = Faction::None;

        self.allow_all_prevented_faction_advantages(&exceptions_to_allowing);
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

    /// The game is passed to the actions.
    pub fn run_either(game: &mut Game, condition: bool, method_if_true: fn(&mut Game), method_otherwise: fn(&mut Game)) {
        if condition {
            method_if_true(game);
        } else {
            method_otherwise(game);
        }
    }

    /// Takes the step of the first choice whose condition holds, or `otherwise`.
    pub fn enter_first(&mut self, choices: &[(bool, Step)], otherwise: Step) {
        let step = choices.iter().find(|(condition, _)| *condition).map_or(otherwise, |(_, step)| *step);
        match step {
            Step::Enter(phase) => self.enter(phase, true),
            Step::Run(method) => method(self),
        }
    }

    pub fn known_cards(&self, p: PlayerIndex) -> Vec<TreacheryCard> {
        let player = self.player(p);
        let mut result: Vec<TreacheryCard> = player.treachery_cards.clone();
        result.extend(player.known_cards.iter().copied());

        if player.has_ally() {
            let ally = self.player(player.allied_player(self).expect("allied player"));
            result.extend(ally.treachery_cards.iter().copied());
            result.extend(ally.known_cards.iter().copied());
        }

        result.into_iter().distinct()
    }

    pub fn register_known(&mut self, c: TreacheryCard) {
        for i in 0..self.players.len() {
            Game::register_known_by_player(self, self.player_at(i), Some(c));
        }
    }

    pub fn register_known_by_player(game: &mut Game, p: PlayerIndex, c: Option<TreacheryCard>) {
        if let Some(c) = c {
            game.player_mut(p).known_cards.insert(c);
        }
    }

    pub fn register_known_by_faction(&mut self, f: Faction, c: TreacheryCard) {
        let p = self.get_player(Some(f));
        if let Some(p) = p {
            Game::register_known_by_player(self, p, Some(c));
        }
    }

    pub fn unregister_known(&mut self, c: TreacheryCard) {
        for i in 0..self.players.len() {
            Game::unregister_known_by_player(self, self.player_at(i), c);
        }
    }

    pub fn unregister_known_by_player(game: &mut Game, p: PlayerIndex, c: TreacheryCard) {
        game.player_mut(p).known_cards.remove(&c);
    }

    pub fn unregister_all_known_by_player(game: &mut Game, p: PlayerIndex, cards: &[TreacheryCard]) {
        for c in cards {
            Game::unregister_known_by_player(game, p, *c);
        }
    }

    /// The lists hold copies of the players' battalions. No caller mutates them.
    pub fn forces(&self, include_homeworlds: bool) -> IndexMap<Location, Vec<Battalion>> {
        let mut result: IndexMap<Location, Vec<Battalion>> = IndexMap::new();

        for l in self.board.map.locations(include_homeworlds) {
            result.insert_new(l, Vec::new());
        }

        for p in &self.players {
            if include_homeworlds {
                // C# parity: Map.Locations(true) already holds every homeworld, so this
                // Add throws ArgumentException for the first homeworld (no caller passes true).
                for w in &p.home_worlds {
                    result.insert_new(w.location(), Vec::new());
                }
            }

            let forces = if include_homeworlds { p.forces_in_locations.iter().map(|(k, v)| (*k, v.clone())).collect() } else { p.forces_on_planet() };

            for (key, value) in forces.iter() {
                match result.get_mut(key) {
                    Some(list) => list.push(value.clone()),
                    None => panic!("{key:?} is not a location of the map"),
                }
            }
        }

        result
    }

    pub fn forces_on_planet_excluding_empty_locations(&self, include_homeworlds: bool) -> IndexMap<Location, Vec<Battalion>> {
        let mut result: IndexMap<Location, Vec<Battalion>> = IndexMap::new();

        for p in &self.players {
            let forces = if include_homeworlds { p.forces_in_locations.iter().map(|(k, v)| (*k, v.clone())).collect() } else { p.forces_on_planet() };

            for (key, value) in forces.iter() {
                if !result.contains_key(key) {
                    result.insert_new(*key, Vec::new());
                }

                match result.get_mut(key) {
                    Some(list) => list.push(value.clone()),
                    None => panic!("{key:?} is not a location of the map"),
                }
            }
        }

        result
    }

    pub fn any_forces_in(&self, t: Territory) -> bool {
        for p in &self.players {
            for l in t.locations() {
                if p.forces_in_locations.contains_key(l) {
                    return true;
                }
            }
        }

        false
    }

    pub fn battalions_in(&self, l: Location) -> Vec<Battalion> {
        let mut result = Vec::new();
        for p in &self.players {
            let mut battalion: Option<Battalion> = None;
            if p.battalion_in(l, &mut battalion) {
                if let Some(battalion) = battalion {
                    result.push(battalion);
                }
            }
        }

        result
    }

    pub fn occupying_forces_on_planet(&self) -> IndexMap<Location, Vec<Battalion>> {
        let mut result: IndexMap<Location, Vec<Battalion>> = IndexMap::new();

        for p in &self.players {
            for (key, value) in p.forces_in_locations.iter().filter(|kvp| p.occupies(self, Some(*kvp.0))) {
                if !result.contains_key(key) {
                    result.insert_new(*key, Vec::new());
                }

                match result.get_mut(key) {
                    Some(list) => list.push(value.clone()),
                    None => panic!("{key:?} is not a location of the map"),
                }
            }
        }

        result
    }

    pub fn resources_in(&self, l: Location) -> i32 {
        self.board.resources_on_planet.get(&l).copied().unwrap_or(0)
    }

    pub fn change_resources_on_planet(&mut self, location: Location, amount: i32) {
        if self.board.resources_on_planet.contains_key(&location) {
            let current = self.board.resources_on_planet[&location];
            if (current + amount) == 0 {
                self.board.resources_on_planet.remove(&location);
            } else {
                self.board.resources_on_planet.insert(location, current + amount);
            }
        } else if amount != 0 {
            self.board.resources_on_planet.insert_new(location, amount);
        }
    }

    pub fn remove_resources(&mut self, location: Location) -> i32 {
        let mut result = 0;
        if self.board.resources_on_planet.contains_key(&location) {
            result = self.board.resources_on_planet[&location];
            self.board.resources_on_planet.remove(&location);
        }

        result
    }

    pub fn remove_resources_territory(&mut self, territory: Territory) -> i32 {
        let mut result: i32 = 0;
        for l in territory.locations() {
            result += self.remove_resources(*l);
        }

        result
    }

    pub fn flip_bene_gesserit_advisors_when_alone(&mut self) {
        if self.turn.current_main_phase >= MainPhase::Battle && self.applicable(Rule::DisableNovaFlipping) {
            return;
        }

        let bg = self.get_player(Some(Faction::BeneGesserit));
        if let Some(bg) = bg {
            // A lazy Where enumerated by foreach: the filter is evaluated for each
            // territory right before its body runs, after earlier flips.
            for t in self.board.map.territories(true) {
                let matches = self.player(bg).special_forces_in_territory(Some(t)) > 0
                    && (t == self.board.map.polar_sink().territory()
                        || !self.players.iter().any(|p| p.faction != Faction::BeneGesserit && p.any_forces_in_territory(Some(t)) > 0));
                if !matches {
                    continue;
                }

                Player::flip_forces_in_territory(self, bg, t, false);
                self.log(vec![express!(Faction::BeneGesserit, " are alone and flip to ", FactionForce::BeneGesserit, " in ", t)
                    .with_log(BeneGesseritFlipped::new(t))
                    .into()]);
            }
        }
    }

    pub fn allow_all_prevented_faction_advantages(&mut self, exceptions: &[FactionAdvantage]) {
        for adv in get_values_except_default(FactionAdvantage::None) {
            if !exceptions.contains(&adv) {
                self.allow(adv);
            }
        }
    }

    pub fn determine_occupation_at_start_or_end_of_turn(&mut self) {
        let current_occupier_of_ecaz_homeworld = self.occupier_of(World::Ecaz);
        let mut updated_occupation: NetDictionary<Homeworld, Faction> = NetDictionary::new();

        for hw in self.board.map.homeworlds() {
            for i in 0..self.players.len() {
                let player = self.player(self.player_at(i));
                let faction = player.faction;
                if player.controls(self, hw.location(), false) && !player.is_native_homeworld(hw) {
                    updated_occupation.insert_new(hw, faction);

                    if !self.occupies(faction, hw.world()) {
                        self.log(vec![faction.into(), " now occupy ".into(), hw.location().into()]);
                    }
                }
            }
        }

        let previous: Vec<(Homeworld, Faction)> = self.board.homeworld_occupation.iter().map(|(k, v)| (*k, *v)).collect();
        for (key, value) in previous {
            // ICollection<KeyValuePair>.Contains: the key is present with an equal value
            if updated_occupation.get(&key) != Some(&value) {
                self.log(vec![value.into(), " no longer occupies ".into(), key.location().into()]);
            }
        }

        self.board.homeworld_occupation = updated_occupation;

        self.check_if_shipment_permissions_should_be_revoked();
        self.check_if_occupier_takes_vidal(current_occupier_of_ecaz_homeworld);
        self.let_factions_discard_surplus_cards();
    }

    pub fn check_if_shipment_permissions_should_be_revoked(&mut self) {
        if !self.has_high_threshold(Faction::Guild) && self.ship_move.shipment_permissions.len() > 0 {
            self.ship_move.shipment_permissions.clear();
            self.log(vec!["Only ".into(), Faction::Guild.into(), " can ship cross/from planet now".into()]);
        }
    }

    pub fn let_factions_discard_surplus_cards(&mut self) {
        let factions: Vec<Faction> = self.players.iter().filter(|p| p.hand_size_exceeded(self)).map(|p| p.faction).collect();
        self.turn.factions_that_must_discard.extend(factions);
        if self.turn.factions_that_must_discard.len() > 0 {
            self.turn.phase_before_discarding = self.turn.current_phase;
            self.enter(Phase::Discarding, true);
        }
    }

    pub fn check_if_occupier_takes_vidal(&mut self, previous_occupier_of_ecaz_homeworld: Option<PlayerIndex>) {
        if self.vidal().is_none() {
            return;
        }

        let occupier_of_ecaz_homeworld = self.occupier_of(World::Ecaz);
        if let Some(occupier) = occupier_of_ecaz_homeworld {
            let vidal = self.vidal().expect("vidal");
            if !self.player(occupier).leaders.contains(&vidal) {
                self.take_vidal(occupier, VidalMoment::WhileEcazWorldIsOccupied);
            }
        } else if let Some(previous) = previous_occupier_of_ecaz_homeworld {
            let vidal = self.vidal();
            if vidal.map_or(false, |v| self.player(previous).leaders.contains(&v)) {
                let vidal = self.vidal().expect("vidal");
                // List.Remove: the first equal element
                let leaders = &mut self.player_mut(previous).leaders;
                if let Some(pos) = leaders.iter().position(|l| *l == vidal) {
                    leaders.remove(pos);
                }
                let faction = self.player(previous).faction;
                let vidal = self.vidal();
                self.log(vec![faction.into(), " lose ".into(), vidal.into()]);
            }
        }
    }

    pub fn set_aside_vidal(&mut self) {
        if self.vidal().is_none() || self.leaders.player_to_set_aside_vidal.is_none() {
            return;
        }

        let vidal = self.vidal().expect("vidal");
        let p = self.leaders.player_to_set_aside_vidal.expect("player to set aside vidal");
        if self.is_alive(Hero::Leader(vidal)) && self.player(p).leaders.contains(&vidal) {
            let leaders = &mut self.player_mut(p).leaders;
            if let Some(pos) = leaders.iter().position(|l| *l == vidal) {
                leaders.remove(pos);
            }
            let vidal = self.vidal();
            self.log(vec![vidal.into(), " is set aside".into()]);
        }

        self.leaders.player_to_set_aside_vidal = None;
        self.leaders.when_to_set_aside_vidal = VidalMoment::None;
    }

    /// Discards the card a player plays as Karama and logs the play as one aside:
    /// "X play [Karama]: <effect>". A card played as Karama that isn't one says
    /// so: "X play [Baliset] as a Karama card: <effect>".
    pub fn play_karma(&mut self, player: PlayerIndex, card: Option<TreacheryCard>, effect: Vec<ExpressionElement>) {
        self.discard_card_of(Some(player), card, false);
        let faction = self.player(player).faction;
        let m = Game::karma_played(faction, card, effect);
        self.log(vec![m.into()]);
    }

    pub fn karma_played(initiator: Faction, card: Option<TreacheryCard>, effect: Vec<ExpressionElement>) -> Message {
        let card_or_karma: ExpressionElement = match card {
            Some(c) => c.into(),
            None => TreacheryCardType::Karma.into(),
        };
        let has_effect = effect.len() > 0;
        express!(
            initiator,
            " play ",
            card_or_karma,
            express_part_if!(card.is_some() && card.expect("card").type_() != TreacheryCardType::Karma; " as a ", TreacheryCardType::Karma, " card"),
            express_part_if!(has_effect; ": ", MessagePart::express(effect))
        )
        .with_log(AsideLog::aside())
    }

    /// Discards the first card of `card_type` in the player's hand (for Bene Gesserit, a Useless
    /// card can stand in for Karma). Fails if they have none.
    pub fn discard(&mut self, player: PlayerIndex, card_type: TreacheryCardType) -> GameResult {
        let bene_gesserit_karma = card_type == TreacheryCardType::Karma && self.player(player).is(Faction::BeneGesserit);
        let card = self
            .player(player)
            .treachery_cards
            .iter()
            .copied()
            .find(|x| if bene_gesserit_karma { x.type_() == TreacheryCardType::Karma || x.type_() == TreacheryCardType::Useless } else { x.type_() == card_type })
            .ok_or_else(|| GameError::new(format!("{} have no {} card to discard", self.player(player).faction, card_type)))?;

        self.discard_card_of(Some(player), Some(card), true);
        Ok(())
    }

    /// `log` is false when the caller logs the card's play itself, such as PlayKarma.
    /// Discards `card` from the hand that holds it. Fails if more than one hand holds it.
    pub fn discard_card(&mut self, card: Option<TreacheryCard>, log: bool) -> GameResult {
        let Some(c) = card else { return Ok(()) };

        let mut holders = self.player_indices().filter(|p| self.player(*p).has(Some(c)));
        let player = holders.next();
        if holders.next().is_some() {
            game_error!("Card {} is in more than one hand", c.id());
        }
        self.discard_card_of(player, Some(c), log);
        self.register_known(c);
        Ok(())
    }

    pub fn discard_card_of(&mut self, player: Option<PlayerIndex>, card: Option<TreacheryCard>, log: bool) {
        if let (Some(player), Some(card)) = (player, card) {
            let faction = self.player(player).faction;
            if log {
                let mut fact = BattleFact::new(BattleFactKind::Discard, faction);
                fact.card = Some(card);
                self.log(vec![express!(faction, " discard ", card).with_fact(fact).into()]);
            }
            // List.Remove: the first equal element
            let cards = &mut self.player_mut(player).treachery_cards;
            if let Some(pos) = cards.iter().position(|c| *c == card) {
                cards.remove(pos);
            }
            self.decks.treachery_discard_pile.as_mut().expect("treachery discard pile").put_on_top(card);
            self.register_known(card);
            self.decks.recently_discarded.insert_new(card, faction);
            self.stone(Milestone::Discard);

            if !matches!(
                card.type_(),
                TreacheryCardType::Poison | TreacheryCardType::ProjectileAndPoison | TreacheryCardType::PoisonTooth
            ) {
                return;
            }

            let ecaz = self.get_player(Some(Faction::Ecaz));
            let Some(ecaz) = ecaz else { return };
            if !self.player(ecaz).has_any_high_threshold(self) {
                return;
            }
            self.log(vec![
                Faction::Ecaz.into(),
                " get ".into(),
                Payment::of(3).into(),
                " from the discarded ".into(),
                card.into(),
            ]);
            let p = self.player_mut(ecaz);
            p.resources += 3;
        }
    }

    /// Replays a savegame's events into a new game, or says why it can't.
    pub fn load(state: &GameState, participation: &Shared<Participation>, perform_validation: bool, is_host: bool) -> Result<Game, Message> {
        Game::check_version(state.version).map_err(|e| express!(e))?;
        let mut game = Game::with_participation(state.version, participation);
        let events = state.events.as_ref().ok_or_else(|| express!("The savegame has no events"))?;
        for (nr, e) in events.iter().enumerate() {
            e.initialize_game(&game);
            if let Some(message) = e.execute(&mut game, perform_validation, is_host) {
                return Err(express!(e.event_type().name(), "(", nr as i32, "):", message));
            }
        }
        Ok(game)
    }

    pub fn log(&mut self, expression: Vec<ExpressionElement>) {
        // A message logged on its own keeps what it carries, such as a route.
        if expression.len() == 1 {
            if let ExpressionElement::Message(_) = &expression[0] {
                let Some(ExpressionElement::Message(message)) = expression.into_iter().next() else { unreachable!() };
                self.current_report_mut().express_message(*message);
                return;
            }
        }
        self.current_report_mut().express(expression);
    }

    pub fn log_if(&mut self, condition: bool, expression: Vec<ExpressionElement>) {
        if condition {
            self.current_report_mut().express(expression);
        }
    }

    pub fn log_to(&mut self, faction: Faction, expression: Vec<ExpressionElement>) {
        self.current_report_mut().express_to(faction, expression);
    }

    pub fn stone(&mut self, m: Milestone) {
        self.turn.recent_milestones.push(m);
    }

    pub fn set_recent_payment(&mut self, amount: i32, by: Faction, to: Faction, reason: &GameEvent) {
        if amount > 0 {
            self.turn.recently_paid.push(Payment::new_full(amount, by, to, Some(reason.clone())));
        }
    }

    pub fn set_recent_payment_to_bank(&mut self, amount: i32, by: Faction, reason: &GameEvent) {
        self.set_recent_payment(amount, by, Faction::None, reason);
    }

    pub fn clear_recent_payments(&mut self) {
        // StoredRecentlyPaid = RecentlyPaid (a reference swap); RecentlyPaid = []
        self.turn.stored_recently_paid = std::mem::take(&mut self.turn.recently_paid);
    }

    pub fn has_recent_payment_for(&self, t: GameEventType) -> bool {
        self.turn.recently_paid.iter().any(|p| p.reason.as_ref().map_or(false, |r| r.event_type() == t))
    }

    pub fn has_quite_recent_payment_to(&self, f: Faction) -> bool {
        self.turn.recently_paid.iter().any(|p| p.to == f) || self.turn.stored_recently_paid.iter().any(|p| p.to == f)
    }

    pub fn quite_recent_payment_to(&self, f: Faction) -> Option<Payment> {
        let res = self
            .turn.recently_paid
            .iter()
            .find(|p| p.to == f)
            .or_else(|| self.turn.stored_recently_paid.iter().find(|p| p.to == f));
        res.cloned()
    }

    pub fn recently_paid_total_amount(&self) -> i32 {
        self.turn.recently_paid.iter().map(|p| p.amount).sum::<i32>()
    }

    pub fn occupies(&self, f: Faction, w: World) -> bool {
        if f != Faction::None {
            let hw_occupation = self.board.homeworld_occupation.keys().find(|hw| hw.world() == w).copied();
            if let Some(hw) = hw_occupation {
                return self.board.homeworld_occupation.get(&hw) == Some(&f);
            }
        }

        false
    }

    pub fn occupier_of(&self, w: World) -> Option<PlayerIndex> {
        let hw_occupation = self.board.homeworld_occupation.keys().find(|hw| hw.world() == w).copied();
        match hw_occupation {
            Some(hw) => self.get_player(self.board.homeworld_occupation.get(&hw).copied()),
            None => None,
        }
    }

    pub fn is_in_storm(&self, l: Location) -> bool {
        l.sector(&self.board.map) == self.board.sector_in_storm
    }

    pub fn is_in_storm_territory(&self, t: Territory) -> bool {
        t.locations().iter().any(|l| self.is_in_storm(*l))
    }

    pub fn is_occupied_territory(&self, t: Territory) -> bool {
        self.players.iter().any(|p| p.occupies_territory(self, Some(t)))
    }

    pub fn is_occupied_world(&self, world: World) -> bool {
        self.occupier_of(world).is_some()
    }

    pub fn is_occupied_by_player_or_their_ally(&self, world: World, p: PlayerIndex) -> bool {
        let occupier = self.occupier_of(world);
        match occupier {
            Some(o) => o == p || self.player(o).ally == self.player(p).faction,
            None => false,
        }
    }

    pub fn is_occupied_by_faction_or_their_ally(&self, world: World, f: Faction) -> bool {
        let occupier = self.occupier_of(world);
        match occupier {
            Some(o) => self.player(o).is(f) || self.player(o).ally == f,
            None => false,
        }
    }

    pub fn contains_conflicting_ally(&self, initiator: PlayerIndex, to: Location) -> bool {
        let ini = self.player(initiator);
        if !ini.has_ally() || self.board.map.polar_sink() == to || ini.faction == Faction::Ecaz || ini.ally == Faction::Ecaz {
            return false;
        }

        let ally = ini.allied_player(self);

        if ini.ally == Faction::BeneGesserit && self.applicable(Rule::AdvisorsDontConflictWithAlly) {
            return self.player(ally.expect("ally")).forces_in_territory(Some(to.territory())) > 0;
        }

        self.player(ally.expect("ally")).any_forces_in_territory(Some(to.territory())) > 0
    }

    pub fn owner_of(&self, hero: Hero) -> Option<PlayerIndex> {
        // Enumerable.Contains<IHero> over a List<Leader>: Equals on each leader
        self.players.iter().position(|p| p.leaders.iter().any(|l| Hero::Leader(*l) == hero)).map(|i| self.player_at(i))
    }

    pub fn owner_of_treachery_card(&self, card: TreacheryCard) -> Option<PlayerIndex> {
        self.players.iter().position(|p| p.has(Some(card))).map(|i| self.player_at(i))
    }

    pub fn owner_of_treachery_card_type(&self, card_type: TreacheryCardType) -> Option<PlayerIndex> {
        self.players.iter().position(|p| p.treachery_cards.iter().any(|c| c.type_() == card_type)).map(|i| self.player_at(i))
    }

    pub fn owner_of_location(&self, stronghold: Location) -> Option<PlayerIndex> {
        match self.board.stronghold_ownership.get(&stronghold) {
            Some(value) => self.get_player(Some(*value)),
            None => None,
        }
    }

    pub fn nr_of_occupants_excluding_faction(&self, l: Location, to_exclude: Faction) -> i32 {
        let factions: Vec<Faction> = self.battalions_in(l).iter().filter(|b| b.can_occupy()).map(|b| b.faction).collect();
        self.count_different_factions(&factions, to_exclude)
    }

    pub fn nr_of_occupants_of_territory_excluding(&self, t: Territory, to_exclude: Faction) -> i32 {
        let factions: Vec<Faction> = t
            .locations()
            .iter()
            .flat_map(|l| self.battalions_in(*l).into_iter().filter(|b| b.can_occupy()).map(|b| b.faction))
            .collect();
        self.count_different_factions(&factions, to_exclude)
    }

    pub fn count_different_factions(&self, factions: &[Faction], to_exclude: Faction) -> i32 {
        let mut result = 0;
        let ecaz_ally = self.get_ally(Some(Faction::Ecaz));
        let mut ecaz_or_ecaz_ally_counted = false;

        for faction in factions.iter().copied().distinct() {
            if faction == Faction::Ecaz || Some(faction) == ecaz_ally {
                if !ecaz_or_ecaz_ally_counted && to_exclude != Faction::Ecaz && Some(to_exclude) != ecaz_ally {
                    result += 1;
                }

                ecaz_or_ecaz_ally_counted = true;
            } else if faction != to_exclude {
                result += 1;
            }
        }

        result
    }

    pub fn has_ornithopters(&self, p: PlayerIndex) -> bool {
        (if self.applicable(Rule::MovementBonusRequiresOccupationBeforeMovement) {
            self.ship_move.factions_with_ornithopters_at_start_of_movement.contains(&self.player(p).faction)
        } else {
            self.occupies_arrakeen_or_carthag(p)
        }) || (self.ship_move.current_flight_used.as_ref().map_or(false, |f| f.move_three_territories)
            && self.ship_move.current_flight_used.as_ref().expect("current flight used").player_opt() == Some(p))
            || (self.ship_move.current_flight_discovery_used.is_some() && self.ship_move.current_flight_discovery_used.as_ref().expect("current flight discovery used").player_opt() == Some(p))
    }

    pub fn determine_maximum_move_distance(&self, p: PlayerIndex, moved: &[Battalion]) -> i32 {
        let player = self.player(p);
        let choam_extra_move_bonus = if player.faction == Faction::Choam && self.ship_move.choam_has_extra_move { 1 } else { 0 };

        let planetology_bonus = if self.turn.current_planetology.as_ref().map_or(false, |pl| pl.add_one_to_movement)
            && self.turn.current_planetology.as_ref().expect("current planetology").initiator() == player.faction
        {
            1
        } else {
            0
        };

        let mut result = 1 + planetology_bonus;

        if self.has_ornithopters(p) {
            result = 3;
        } else if player.is(Faction::Fremen) && !self.prevented(FactionAdvantage::FremenExtraMove) {
            result = 2 + planetology_bonus;
        } else if player.is(Faction::Ixian)
            && !self.prevented(FactionAdvantage::IxianCyborgExtraMove)
            && moved.iter().any(|b| b.amount_of_special_forces > 0)
        {
            result = 2 + planetology_bonus;
        }

        result + choam_extra_move_bonus
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn locations_with_any_forces_not_in_storm(&self, p: PlayerIndex) -> impl Iterator<Item = Location> {
        // `p` is dereferenced inside the predicate, per element.
        self.board.map
            .locations(true)
            .filter(move |l| l.sector(&self.board.map) != self.board.sector_in_storm && self.player(p).any_forces_in(Some(*l)) > 0)
    }

    pub fn is_not_full(&self, p: PlayerIndex, l: Location) -> bool {
        // C# dereferences `p` only after the first clause, so a null player passes for
        // locations outside strongholds and homeworlds (boundary.rs lazy cases).
        if !(l.territory().is_stronghold() || l.territory().is_homeworld()) {
            return true;
        }
        let player = self.player(p);
        (player.is(Faction::BeneGesserit) && player.special_forces_in(Some(l)) > 0)
            //Looks at occupying forces
            || (player.is(Faction::Ecaz)
                && player.has_ally()
                && self.player(player.allied_player(self).expect("allied player")).occupying_forces_in(Some(l.territory())) > 0)
            //Looks at occupying forces
            || (player.has_ally()
                && player.ally == Faction::Ecaz
                && self.player(player.allied_player(self).expect("allied player")).occupying_forces_in(Some(l.territory())) > 0)
            || self.nr_of_occupants_excluding_faction(l, player.faction) < 2
    }

    pub fn everyone_acted_or_passed(&self) -> bool {
        self.turn.has_acted_or_passed.len() == self.players.len()
    }

    pub fn assisted_note_keeping_enabled(&self, p: Option<PlayerIndex>) -> bool {
        match p {
            Some(p) => {
                self.applicable(Rule::AssistedNotekeeping)
                    || (self.player(p).is(Faction::Atreides) && self.applicable(Rule::AssistedNotekeepingForAtreides))
            }
            None => false,
        }
    }

    pub fn has_storm_prescience(&self, p: Option<PlayerIndex>) -> bool {
        let Some(p) = p else { return false };
        let player = self.player(p);
        self.applicable(Rule::FremenDeterminesStorm)
            && !self.prevented(FactionAdvantage::FremenStormPrescience)
            && (player.faction == Faction::Fremen
                || (player.ally == Faction::Fremen && self.alliances.fremen_shares_prescience)
                || self.has_deal(player.faction, DealType::ShareStormPrescience))
    }

    pub fn has_high_threshold(&self, f: Faction) -> bool {
        let player = self.get_player(Some(f));
        player.map_or(false, |p| self.player(p).has_any_high_threshold(self))
    }

    pub fn has_high_threshold_in(&self, f: Faction, w: World) -> bool {
        let player = self.get_player(Some(f));
        player.map_or(false, |p| self.player(p).has_high_threshold(self, w))
    }

    pub fn has_low_threshold(&self, f: Faction) -> bool {
        let player = self.get_player(Some(f));
        player.map_or(false, |p| self.player(p).has_any_low_threshold(self))
    }

    pub fn has_resource_deck_prescience(&self, p: Option<PlayerIndex>) -> bool {
        let Some(p) = p else { return false };
        let player = self.player(p);
        !self.prevented(FactionAdvantage::AtreidesSpiceBlowPrescience)
            && !self.has_low_threshold(Faction::Atreides)
            && (player.faction == Faction::Atreides
                || (player.ally == Faction::Atreides && self.alliances.atreides_shares_prescience)
                || self.has_deal(player.faction, DealType::ShareResourceDeckPrescience))
    }

    pub fn has_bidding_prescience(&self, p: PlayerIndex) -> bool {
        let is_publicly_known =
            self.bidding.current_auction_type == AuctionType::RicheseOnceAround || self.bidding.current_auction_type == AuctionType::RicheseSilent;
        let player = self.player(p);

        is_publicly_known
            || self.occupies(player.faction, World::Atreides)
            || self.occupies(player.ally, World::Atreides)
            || (!self.prevented(FactionAdvantage::AtreidesBiddingPrescience)
                && (player.faction == Faction::Atreides
                    || (player.ally == Faction::Atreides && self.alliances.atreides_shares_prescience)
                    || self.has_deal(player.faction, DealType::ShareBiddingPrescience)))
    }

    pub fn traitors_in_play(&self) -> Vec<Hero> {
        let mut result: Vec<Hero> = Vec::new();

        // `var factionsInPlay = Players.Select(p => p.Faction)` is lazy: each Contains below
        // runs the Select again, up to the first match.
        let factions_in_play = self.players.iter().map(|p| p.faction);
        result.extend(
            LeaderManager::leaders()
                .iter()
                .filter(|l| {
                    factions_in_play.clone().any(|f| f == l.faction())
                        && l.hero_type() != HeroType::Vidal
                        && (l.hero_type() != HeroType::Auditor || self.applicable(Rule::ChoamAuditor))
                })
                .map(|l| Hero::Leader(*l)),
        );

        if self.applicable(Rule::CheapHeroTraitor) {
            let card = TreacheryCardManager::get_cards_in_play(self)
                .into_iter()
                .find(|c| c.type_() == TreacheryCardType::Mercenary)
                .expect("a mercenary card in play");
            result.push(Hero::TreacheryCard(card));
        }

        result
    }

    /// A `DateTime` (`DateTimeOffset.DateTime`: the clock time, offset dropped),
    /// returned as a `DateTimeOffset` with offset 0. `DateTime.MinValue` is tick 0.
    pub fn first_action(&self) -> DateTimeOffset {
        match self.history.entries.first() {
            Some(e) => DateTimeOffset::new(e.time().ticks, 0),
            None => DateTimeOffset::default(),
        }
    }

    pub fn last_action(&self) -> DateTimeOffset {
        match self.history.entries.last() {
            Some(e) => DateTimeOffset::new(e.time().ticks, 0),
            None => DateTimeOffset::default(),
        }
    }
}

impl Game {
    /// The player at position `index` of `Players`.
    pub fn player_at(&self, index: usize) -> PlayerIndex {
        PlayerIndex { index }
    }

    /// Every player of `Players`, in order.
    pub fn player_indices(&self) -> impl Iterator<Item = PlayerIndex> + Clone + 'static {
        (0..self.players.len()).map(|index| PlayerIndex { index })
    }

    /// The player `p` refers to.
    pub fn player(&self, p: PlayerIndex) -> &Player {
        &self.players[p.index]
    }

    /// `p`, for mutation.
    pub fn player_mut(&mut self, p: PlayerIndex) -> &mut Player {
        &mut self.players[p.index]
    }

    /// The index of a player object of this game (`this` inside a `Player` method).
    pub fn index_of(&self, p: &Player) -> Option<PlayerIndex> {
        self.players.iter().position(|x| std::ptr::eq(x, p)).map(|index| PlayerIndex { index })
    }
}

/// What `Map` needs to know about the game (see map.rs).
impl crate::map::MapGame for Game {
    fn sector_in_storm(&self) -> i32 {
        self.board.sector_in_storm
    }

    fn history_count(&self) -> i32 {
        self.history.entries.len() as i32
    }

    fn applicable(&self, rule: Rule) -> bool {
        Game::applicable(self, rule)
    }

    fn forces_battalion_factions(&self) -> Vec<(Location, Vec<Faction>)> {
        // game.Forces(): every location (in Map.Locations order), with the factions of its battalions
        self.forces(false).iter().map(|(l, bs)| (*l, bs.iter().map(|b| b.faction).collect())).collect()
    }

    fn player_ally(&self, f: Faction) -> Option<Faction> {
        self.get_player(Some(f)).map(|p| self.player(p).ally)
    }

    fn nr_of_occupants_excluding_faction(&self, l: Location, f: Faction) -> i32 {
        Game::nr_of_occupants_excluding_faction(self, l, f)
    }

    fn current_blocked_territories(&self) -> Vec<Territory> {
        self.ship_move.current_blocked_territories.clone()
    }
}
