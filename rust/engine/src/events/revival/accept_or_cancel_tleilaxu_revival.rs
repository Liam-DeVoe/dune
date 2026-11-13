use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct AcceptOrCancelTleilaxuRevival {
    pub cancel: bool,
    pub price: i32,
    pub hero_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for AcceptOrCancelTleilaxuRevival {
    fn default() -> Self {
        AcceptOrCancelTleilaxuRevival {
            cancel: Default::default(),
            price: Default::default(),
            hero_id: Default::default(),
            base: Default::default(),
        }
    }
}

impl AcceptOrCancelTleilaxuRevival {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn hero(&self) -> Option<Hero> {
        LeaderManager::hero_lookup().find(self.hero_id)
    }

    pub fn set_hero(&mut self, value: Option<Hero>) {
        self.hero_id = LeaderManager::hero_lookup().get_id(value);
    }

    pub fn is_denial(&self) -> bool {
        self.price == i32::MAX
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        self.log(game);

        match self.hero() {
            None => {
                let requests = game.current_revival_requests.clone();
                for r in requests.iter() {
                    let Some(h) = r.hero() else { game_error!("A revival request has no leader") };
                    game.early_revivals_offers.insert_new(h, i32::MAX);
                }

                game.current_revival_requests.clear();
            }
            Some(hero) => {
                game.early_revivals_offers.remove(&hero);

                if !self.cancel {
                    game.early_revivals_offers.insert_new(hero, self.price);
                }

                let request_to_remove =
                    game.current_revival_requests.iter().find(|r| r.hero() == Some(hero)).cloned();
                if let Some(request_to_remove) = request_to_remove {
                    // List.Remove: events compare by reference.
                    if let Some(i) = game.current_revival_requests.iter().position(|r| Rc::ptr_eq(r, &request_to_remove)) {
                        game.current_revival_requests.remove(i);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        let Some(hero) = self.hero() else {
            return express!(self.initiator(), " deny all outstanding requests for early revival");
        };

        if !self.cancel {
            if self.is_denial() {
                return express!(self.initiator(), " deny ", hero.faction(), " early revival of a leader");
            }
            return express!(self.initiator(), " offer ", hero.faction(), " revival of a leader for ", Payment::of(self.price));
        }

        express!(self.initiator(), " cancel their revival offer to ", hero.faction())
    }

}

impl GameEventMethods for AcceptOrCancelTleilaxuRevival {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::AcceptOrCancelTleilaxuRevival
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::AcceptOrCancelTleilaxuRevival(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        AcceptOrCancelTleilaxuRevival::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        AcceptOrCancelTleilaxuRevival::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        AcceptOrCancelTleilaxuRevival::get_message(self, game)
    }

}
