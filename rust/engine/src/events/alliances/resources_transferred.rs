use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct ResourcesTransferred {
    pub resources: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for ResourcesTransferred {
    fn default() -> Self {
        ResourcesTransferred {
            resources: Default::default(),
            base: Default::default(),
        }
    }
}

impl ResourcesTransferred {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if self.resources > ResourcesTransferred::max_amount(game, self.player()) {
            return Some(express!("You can't transfer more than ", Payment::of(ResourcesTransferred::max_amount(game, self.player()))));
        }
        if !ResourcesTransferred::may_transfer(game, self.player()) {
            return Some(express!("You currently can't transfer ", Concept::Resource));
        }

        None
    }

    pub fn can_be_played(g: &Game, p: PlayerIndex) -> bool {
        g.player(p).has_ally() && ResourcesTransferred::max_amount(g, p) > 0 && ResourcesTransferred::may_transfer(g, p)
    }

    pub fn may_transfer(g: &Game, p: PlayerIndex) -> bool {
        !(g.has_bid_to_pay(p) || g.current_phase_cannot_be_interrupted())
    }

    pub fn max_amount(game: &Game, p: PlayerIndex) -> i32 {
        let pl = game.player(p);
        pl.transferable_resources.min(pl.resources)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        self.log(game);
        let me = self.player();
        {
            let p = game.player_mut(me);
            p.resources -= self.resources;
            p.transferable_resources -= self.resources;
        }
        let ally = game.player(me).allied_player(game).required("allied player")?;
        let a = game.player_mut(ally);
        a.resources += self.resources;
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " transfer ", Payment::of(self.resources), " to their ally")
    }

}

impl GameEventMethods for ResourcesTransferred {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::ResourcesTransferred
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::ResourcesTransferred(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        ResourcesTransferred::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        ResourcesTransferred::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        ResourcesTransferred::get_message(self, game)
    }

}
