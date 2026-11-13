use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct AlliancePassed {
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for AlliancePassed {
    fn default() -> Self {
        AlliancePassed {
            base: Default::default(),
        }
    }
}

impl AlliancePassed {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        self.log(game);

        let initiator = self.initiator();
        loop {
            let withdrawn_offer = game.current_alliance_offers.iter().position(|x| x.initiator() == initiator || x.target == initiator);
            let Some(i) = withdrawn_offer else { break };
            // List.Remove(withdrawnOffer): the first occurrence of that object, which is at i.
            game.current_alliance_offers.remove(i);
        }

        game.has_acted_or_passed.push(initiator);
        game.end_nexus_if_everyone_is_done();
    }

    pub fn get_message(&self, game: &Game) -> Message {
        let player = game.player(self.player());
        if player.has_ally() {
            express!(self.initiator(), " keep their alliance with ", player.ally)
        } else {
            express!(self.initiator(), " will not make an alliance")
        }
    }
}

impl GameEventMethods for AlliancePassed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::AlliancePassed
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::AlliancePassed(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        AlliancePassed::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        AlliancePassed::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        AlliancePassed::get_message(self, game)
    }
}
