use crate::prelude::*;

/// The events played so far, the events undone, and what each event's moment and timing were.
#[derive(Debug)]
pub struct History {
    pub entries: Vec<HistoryEntry>,
    pub moments: Vec<Moment>,
    /// Keyed by the initiator, `None` for the host.
    pub timers: IndexMap<Option<Faction>, Timer<MainPhase>>,
    pub recently_undone_event_count: i32,
    pub recently_undone_events: Vec<HistoryEntry>,
    /// The game as it was before the last `undo`, which `restore_recently_undone` returns to.
    pub previous_game_state: Option<Box<Game>>,
    /// The id of the entry being played, which `BattleInitiated` gives its battle.
    pub playing: EntryId,
}

impl History {
    /// Sets the time of the last event played, which the server knows only after it was applied.
    pub fn stamp_last(&mut self, at: DateTimeOffset) {
        if let Some(e) = self.entries.last_mut() {
            e.at = at;
        }
    }

    pub fn new() -> Self {
        History {
            entries: Vec::new(),
            moments: Vec::new(),
            timers: Default::default(),
            recently_undone_event_count: Default::default(),
            recently_undone_events: Vec::new(),
            previous_game_state: None,
            playing: EntryId::default(),
        }
    }
}

impl Game {
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
    /// with undone events), replays the history followed by the undone events. The human seats
    /// stay as they are now: they aren't part of the history.
    pub fn restore_recently_undone(&mut self) {
        if let Some(previous) = self.history.previous_game_state.take() {
            let human_seats = std::mem::take(&mut self.human_seats);
            *self = *previous;
            self.human_seats = human_seats;
            return;
        }

        if self.history.recently_undone_event_count == 0 || self.history.recently_undone_events.is_empty() {
            return;
        }

        let events: Vec<HistoryEntry> = self.history.entries.iter().chain(self.history.recently_undone_events.iter()).cloned().collect();
        *self = self.replay(&events);
    }

    /// Forgets the undone events and the game before the undo: the history is replayed
    /// into a new game.
    pub fn dismiss_recently_undone(&mut self) {
        let history = self.history.entries.clone();
        *self = self.replay(&history);
    }

    /// A new game of the same version and human seats that has played `events` without
    /// validating them. An event that fails is left out and its changes rolled back, as when it
    /// was first played.
    fn replay(&self, events: &[HistoryEntry]) -> Game {
        let mut result = self.new_like();
        for e in events {
            let _ = result.play(e.clone(), false, false);
        }
        result
    }

    /// Plays `event` for the submitter `by` (`None` for the host), as of `at`: checks that `by`
    /// may play this kind of event now (`is_host` adds the host's events), validates it, applies
    /// it and adds it to the history. On an error, the game doesn't change.
    ///
    /// `at` is the time the event sees while it is applied (the timers read it). The server
    /// submits with no time and stamps the entry afterwards ([`History::stamp_last`]).
    pub fn submit(&mut self, by: Option<Faction>, event: Event, at: DateTimeOffset, is_host: bool) -> Result<(), SubmitError> {
        self.play(HistoryEntry { by, at, event, id: EntryId::fresh() }, true, is_host)
    }

    /// [`play`](Game::play), with an error as the message the player sees. For tests and tools that compare those messages.
    pub fn play_for_message(&mut self, entry: HistoryEntry, validate: bool, is_host: bool) -> Option<Message> {
        self.play(entry, validate, is_host).err().map(|e| e.message())
    }

    /// [`submit`](Game::submit) of a whole entry, validated or not. Unvalidated play is for
    /// replaying a history and for tests.
    ///
    /// An event that fails while it is applied leaves the game as it was before
    /// ([`SubmitError::Failed`]): the game is rebuilt by replaying its history, as an undo
    /// does. A refused event changes nothing in the first place.
    pub fn play(&mut self, mut entry: HistoryEntry, validate: bool, is_host: bool) -> Result<(), SubmitError> {
        // A pass carries none of its other fields, as in the savegame.
        entry.event = entry.event.normalized();
        let by = By::new(self, entry.by);
        let establishing = matches!(entry.event, Event::EstablishPlayers(_));

        if validate {
            if establishing {
                // Starting the game skips the check for applicable events.
                entry.event.validate(self, by).map_err(SubmitError::Rejected)?;
            } else {
                if !self.get_applicable_events(by.player, is_host).contains(&entry.event_type()) {
                    return Err(SubmitError::NotApplicable(express!("Event '", entry.event.try_describe(self, by)?, "' is not applicable")));
                }
                entry.event.try_validate(self, by)?.map_err(SubmitError::Rejected)?;
            }
        }

        // From here on the game changes. Only the timers and `playing` aren't rebuilt by a
        // replay of the history: the timers count the times the events had when they were played,
        // which the server stamps afterwards. `update_timers` changes them only for an event
        // with a time, so they are copied only then.
        let playing = self.history.playing;
        let timers = (entry.at != DateTimeOffset::default()).then(|| self.history.timers.clone());
        self.history.playing = entry.id;
        if let Err(e) = self.apply_entry(entry, by, establishing) {
            self.roll_back(playing, timers);
            return Err(SubmitError::Failed(e));
        }
        Ok(())
    }

    /// Applies an accepted entry and adds it to the history. On an error, the changes made so far
    /// stay and the entry isn't added; [`play`](Game::play) then rolls the game back.
    fn apply_entry(&mut self, entry: HistoryEntry, by: By, establishing: bool) -> GameResult {
        if establishing {
            // Starting the game keeps the recent milestones, and always counts as entering the
            // start of a phase.
            self.perform_pre_event_tasks(&entry);
            entry.event.apply(self, by)?;
            self.perform_post_event_tasks(entry, true);
            return Ok(());
        }

        let moment_just_before_event = self.turn.current_moment;
        self.turn.recent_milestones.clear();
        self.perform_pre_event_tasks(&entry);
        entry.event.apply(self, by)?;
        let just_entered_start_of_phase = moment_just_before_event != Some(MainPhaseMoment::Start) && self.turn.current_moment == Some(MainPhaseMoment::Start);
        self.perform_post_event_tasks(entry, just_entered_start_of_phase);
        self.settle_battle_sequence();
        Ok(())
    }

    /// Puts the game back as it was before an event that failed halfway: replays the history
    /// into a new game, and carries over what the history doesn't hold. That is the human seats
    /// (`new_like`), the undo state, the timers and the id of the entry played last. `timers`
    /// holds the timers from before the event when it could have changed them.
    ///
    /// The events in the history all succeeded in this same sequence, so none fails in the
    /// replay. Errors are rare, so this costs nothing on the events that succeed.
    fn roll_back(&mut self, playing: EntryId, timers: Option<IndexMap<Option<Faction>, Timer<MainPhase>>>) {
        let mut result = self.replay(&self.history.entries);
        let history = &mut self.history;
        result.history.recently_undone_event_count = history.recently_undone_event_count;
        result.history.recently_undone_events = std::mem::take(&mut history.recently_undone_events);
        result.history.previous_game_state = history.previous_game_state.take();
        result.history.timers = timers.unwrap_or_else(|| std::mem::take(&mut history.timers));
        result.history.playing = playing;
        *self = result;
    }

    /// While a player is to initiate a battle, or the battle report is shown, moves the battle
    /// sequence on to the next player with a battle ([`Game::advance_battle_sequence`]). Queries
    /// read the sequence without changing it, so this is where the passed-over players are
    /// recorded, once per event.
    fn settle_battle_sequence(&mut self) {
        let choosing_battle = self.phase() == Phase::BattlePhase && self.current_battle().is_none();
        if choosing_battle || self.phase() == Phase::BattleReport {
            self.advance_battle_sequence();
        }
    }

    pub fn update_timers(&mut self, e: &HistoryEntry) {
        if e.at != DateTimeOffset::default() && self.history.entries.len() > 0 && self.in_timed_phase() {
            let previous_event = match e.event_type() {
                EventType::Battle => self.find_most_recent_event(vec![EventType::BattleInitiated]),
                EventType::Move => self.find_most_recent_event(vec![EventType::Shipment, EventType::EndPhase]),
                EventType::Shipment => self.find_most_recent_event(vec![EventType::Move, EventType::EndPhase]),
                _ => self.find_most_recent_event(vec![]),
            };

            let Some(previous_event) = previous_event else { return };

            // Compares UTC instants.
            let elapsed_time = TimeSpan::from_ticks(e.at.utc_ticks() - previous_event.at.utc_ticks());
            if elapsed_time >= TimeSpan::ONE_HOUR {
                return;
            }

            let initiator = e.by;
            if !self.history.timers.contains_key(&initiator) {
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
            TimeSpan::from_ticks(last.at.utc_ticks() - self.history.entries[0].at.utc_ticks())
        } else {
            TimeSpan::default()
        }
    }

    pub fn time_spent(&self, faction: Option<Faction>, phase: MainPhase) -> TimeSpan {
        if let Some(timer) = self.history.timers.get(&faction) {
            return timer.time_spent(phase);
        }
        TimeSpan::default()
    }

    pub fn started(&self) -> DateTimeOffset {
        if self.history.entries.len() > 0 {
            self.history.entries[0].at
        } else {
            DateTimeOffset::default()
        }
    }

    pub fn latest_event(&self, event_type: EventType) -> Option<HistoryEntry> {
        // List<T>.LastOrDefault(pred) walks backwards; the predicate has no side effects.
        self.history.entries.iter().rev().find(|e| e.event_type() == event_type).cloned()
    }

    pub fn latest_event_0(&self) -> Option<HistoryEntry> {
        if self.history.entries.len() > 0 {
            Some(self.history.entries[self.history.entries.len() - 1].clone())
        } else {
            None
        }
    }

    pub fn event_count(&self) -> i32 {
        self.history.entries.len() as i32
    }

    pub fn find_most_recent_event(&self, types: Vec<EventType>) -> Option<HistoryEntry> {
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
}
