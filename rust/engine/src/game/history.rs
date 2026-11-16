use crate::prelude::*;

/// The events played so far, the events undone, and what each event's moment and timing were.
pub struct History {
    pub entries: Vec<Event>,
    pub moments: Vec<Moment>,
    /// Keyed by the initiator, `None` for the host.
    pub timers: IndexMap<Option<Faction>, Timer<MainPhase>>,
    pub recently_undone_event_count: i32,
    pub recently_undone_events: Vec<Event>,
    /// The game as it was before the last `undo`, which `restore_recently_undone` returns to.
    pub previous_game_state: Option<Box<Game>>,
}

impl History {
    pub fn new() -> Self {
        History {
            entries: Vec::new(),
            moments: Vec::new(),
            timers: Default::default(),
            recently_undone_event_count: Default::default(),
            recently_undone_events: Vec::new(),
            previous_game_state: None,
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

        let events: Vec<Event> = self.history.entries.iter().chain(self.history.recently_undone_events.iter()).cloned().collect();
        *self = self.replay(&events);
    }

    /// Forgets the undone events and the game before the undo: the history is replayed
    /// into a new game.
    pub fn dismiss_recently_undone(&mut self) {
        let history = self.history.entries.clone();
        *self = self.replay(&history);
    }

    /// A new game of the same version and human seats that has executed `events`.
    fn replay(&self, events: &[Event]) -> Game {
        let mut result = self.new_like();
        for e in events {
            e.initialize_game(&result);
            e.execute(&mut result, false, false);
        }
        result
    }

    pub fn update_timers(&mut self, e: &Event) {
        if e.time() != DateTimeOffset::default() && self.history.entries.len() > 0 && self.in_timed_phase() {
            let previous_event = match e.event_type() {
                EventType::Battle => self.find_most_recent_event(vec![EventType::BattleInitiated]),
                EventType::Move => self.find_most_recent_event(vec![EventType::Shipment, EventType::EndPhase]),
                EventType::Shipment => self.find_most_recent_event(vec![EventType::Move, EventType::EndPhase]),
                _ => self.find_most_recent_event(vec![]),
            };

            let Some(previous_event) = previous_event else { return };

            // DateTimeOffset.Subtract compares UTC instants.
            let elapsed_time = TimeSpan::from_ticks(e.time().utc_ticks() - previous_event.time().utc_ticks());
            if elapsed_time >= TimeSpan::ONE_HOUR {
                return;
            }

            let initiator = e.initiator();
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
            TimeSpan::from_ticks(last.time().utc_ticks() - self.history.entries[0].time().utc_ticks())
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
            self.history.entries[0].time()
        } else {
            DateTimeOffset::default()
        }
    }

    pub fn latest_event(&self, event_type: EventType) -> Option<Event> {
        // List<T>.LastOrDefault(pred) walks backwards; the predicate has no side effects.
        self.history.entries.iter().rev().find(|e| e.event_type() == event_type).cloned()
    }

    pub fn latest_event_0(&self) -> Option<Event> {
        if self.history.entries.len() > 0 {
            Some(self.history.entries[self.history.entries.len() - 1].clone())
        } else {
            None
        }
    }

    pub fn event_count(&self) -> i32 {
        self.history.entries.len() as i32
    }

    pub fn find_most_recent_event(&self, types: Vec<EventType>) -> Option<Event> {
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
