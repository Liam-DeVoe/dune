use crate::prelude::*;
use crate::messages::part::Part;

#[derive(Clone, Debug)]
pub struct EmperorBidSupport {
    pub amounts: Option<IndexMap<Faction, i32>>,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
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
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        // Not in C#, which accepted any amounts. Support for the Emperor's own bids let the
        // Emperor bid their spice twice, and paying left them with less than zero
        // (behavior-changes.md, "Turns, sectors and amounts out of range").
        if let Some(amounts) = &self.amounts {
            if amounts.get_opt(self.initiator()).is_some() {
                return Some(express!("You can't support your own bids"));
            }
            if amounts.values().any(|v| *v < 0) {
                return Some(express!("Invalid amount"));
            }
        }
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        // Without amounts (only without validation) the message can't be built: the event fails
        // before it is logged.
        game.alliances.permitted_use_of_emperor_spice = self.amounts.clone();
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
            let parts = Part::list(
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
