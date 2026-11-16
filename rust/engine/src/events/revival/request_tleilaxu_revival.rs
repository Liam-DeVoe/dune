use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct RequestTleilaxuRevival {
    pub hero: Option<Hero>,
    /// The fields of `Event` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for RequestTleilaxuRevival {
    fn default() -> Self {
        RequestTleilaxuRevival {
            hero: TreacheryCardManager::get(0).map(Hero::TreacheryCard),
            base: Default::default(),
        }
    }
}

impl RequestTleilaxuRevival {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if !Self::valid_targets(game, self.player()).any(|h| Some(h) == self.hero) {
            return Some(express!(self.hero, " can't be revived this way"));
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_targets(g: &Game, p: PlayerId) -> impl Iterator<Item = Hero> {
        use either::Either;
        use std::iter::empty;
        let tleilaxu = g.get_player(Some(Faction::Tleilaxu));
        // The gholas query is built first and runs lazily; its predicate reads p.Faction per element.
        let gholas = match tleilaxu {
            Some(tleilaxu) => Either::Left(g.player(tleilaxu).leaders.iter().filter(move |l| Some(l.faction()) == g.player(p).faction).map(|l| Hero::Leader(*l))),
            None => Either::Right(empty()),
        };
        // KilledHeroes builds its list at the call; the Union is lazy.
        let killed = g.killed_heroes(p);
        crate::iter::unique(killed.into_iter().chain(gholas))
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let hero = self.hero;
        let existing_request = game.revival.current_revival_requests.iter().find(|r| r.hero == hero).cloned();
        if let Some(existing_request) = existing_request {
            // List.Remove: events compare by reference.
            if let Some(i) = game.revival.current_revival_requests.iter().position(|r| Rc::ptr_eq(r, &existing_request)) {
                game.revival.current_revival_requests.remove(i);
            }
        }

        self.log(game);
        game.revival.current_revival_requests.push(self.clone());
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " request ", Faction::Tleilaxu, " revival of a leader")
    }

}

impl GameEventMethods for RequestTleilaxuRevival {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> EventType {
        EventType::RequestTleilaxuRevival
    }

    fn into_event(self: Rc<Self>) -> Event {
        Event::RequestTleilaxuRevival(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        RequestTleilaxuRevival::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        RequestTleilaxuRevival::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        RequestTleilaxuRevival::get_message(self, game)
    }

}
