use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct EmperorBidSupport {
    pub amounts: Option<IndexMap<Faction, i32>>,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for EmperorBidSupport {
    fn default() -> Self {
        EmperorBidSupport {
            amounts: Default::default(),
            base: Default::default(),
        }
    }
}

impl EmperorBidSupport {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        // C# assigns the event's own dictionary (aliased, hazards 5.3); nothing mutates it afterwards, so a copy is equivalent.
        // Without amounts (only without validation) the message can't be built: the event fails
        // before it is logged (C#: Log() throws ArgumentNullException in GetMessage).
        game.permitted_use_of_emperor_spice = self.amounts.clone();
        let message = self.try_get_message(game)?;
        game.current_report_mut().express_event(message);
        Ok(())
    }

    pub fn try_get_message(&self, game: &Game) -> GameResult<Message> {
        if self.amounts.is_none() {
            game_error!("Emperor bid support has no amounts");
        }
        Ok(self.get_message(game))
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        // `Amounts.Sum(...)` on a null Amounts: ArgumentNullException (Parameter 'source').
        let amounts = self.amounts.as_ref().expect("amounts");
        if amounts.iter().map(|(_, v)| *v).sum::<i32>() > 0 {
            // One element: the lazy Where/Select query, whose runtime type .NET 10 prints as below.
            let parts = ExpressionElement::collection(
                "System.Linq.Enumerable+IEnumerableWhereSelectIterator`2[System.Collections.Generic.KeyValuePair`2[Treachery.Shared.Faction,System.Int32],Treachery.Shared.MessagePart]",
                amounts.iter().filter(|(_, v)| **v > 0).map(|(k, v)| express_part!(*k, ":", Payment::of(*v), " ")),
            );
            return express!(self.initiator(), " supports ", parts);
        }
        express!(self.initiator(), " doesn't support bids by other factions")
    }

}

impl GameEventMethods for EmperorBidSupport {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::EmperorBidSupport
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::EmperorBidSupport(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        EmperorBidSupport::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        EmperorBidSupport::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        EmperorBidSupport::get_message(self, game)
    }

    fn try_get_message(&self, game: &Game) -> GameResult<Message> {
        EmperorBidSupport::try_get_message(self, game)
    }

}
