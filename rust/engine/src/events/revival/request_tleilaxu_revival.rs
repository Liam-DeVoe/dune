use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct RequestTleilaxuRevival {
    #[canon(name = "_heroId")]
    pub hero_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for RequestTleilaxuRevival {
    fn default() -> Self {
        RequestTleilaxuRevival {
            hero_id: Default::default(),
            base: Default::default(),
        }
    }
}

impl RequestTleilaxuRevival {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn hero(&self) -> Option<Hero> {
        LeaderManager::hero_lookup().find(self.hero_id)
    }

    pub fn set_hero(&mut self, value: Option<Hero>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.hero_id = LeaderManager::hero_lookup().get_id(value);
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if !Self::valid_targets(game, self.player()).any(|h| Some(h) == self.hero()) {
            return Some(express!(self.hero(), " can't be revived this way"));
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_targets(g: &Game, p: PlayerIndex) -> impl Iterator<Item = Hero> {
        use either::Either;
        use std::iter::empty;
        let tleilaxu = g.get_player(Some(Faction::Tleilaxu));
        // The gholas query is built first and runs lazily; its predicate reads p.Faction per element.
        let gholas = match tleilaxu {
            Some(tleilaxu) => Either::Left(g.player(tleilaxu).leaders.iter().filter(move |l| l.faction() == g.player(p).faction).map(|l| Hero::Leader(*l))),
            None => Either::Right(empty()),
        };
        // KilledHeroes builds its list at the call; the Union is lazy.
        let killed = g.killed_heroes(p);
        crate::iter::unique(killed.into_iter().chain(gholas))
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        let hero = self.hero();
        let existing_request = game.current_revival_requests.iter().find(|r| r.hero() == hero).cloned();
        if let Some(existing_request) = existing_request {
            // List.Remove: events compare by reference.
            if let Some(i) = game.current_revival_requests.iter().position(|r| Rc::ptr_eq(r, &existing_request)) {
                game.current_revival_requests.remove(i);
            }
        }

        self.log(game);
        game.current_revival_requests.push(self.clone());
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " request ", Faction::Tleilaxu, " revival of a leader")
    }

}

impl GameEventMethods for RequestTleilaxuRevival {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::RequestTleilaxuRevival
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::RequestTleilaxuRevival(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        RequestTleilaxuRevival::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        RequestTleilaxuRevival::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        RequestTleilaxuRevival::get_message(self, game)
    }

}
