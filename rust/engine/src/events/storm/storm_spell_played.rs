use crate::log::storm::StormMoved;
use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct StormSpellPlayed {
    pub move_amount: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for StormSpellPlayed {
    fn default() -> Self {
        StormSpellPlayed {
            move_amount: Default::default(),
            base: Default::default(),
        }
    }
}

impl StormSpellPlayed {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        if self.move_amount < 0 || self.move_amount > 10 {
            return Some(express!("Invalid number of sectors"));
        }

        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let card = game.player(self.player()).card(TreacheryCardType::StormSpell);
        game.discard(self.player(), TreacheryCardType::StormSpell)?;
        let msg = self.get_message(game).with_log(StormMoved::new(game.sector_in_storm, self.move_amount, self.initiator(), card));
        self.log_objects(game, vec![ExpressionElement::from(msg)]);
        game.move_storm_and_determine_next(self.move_amount);
        game.end_storm_phase();
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " use ", TreacheryCardType::StormSpell, " to move the storm ", self.move_amount, " sectors")
    }

}

impl GameEventMethods for StormSpellPlayed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::StormSpellPlayed
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::StormSpellPlayed(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        StormSpellPlayed::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        StormSpellPlayed::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        StormSpellPlayed::get_message(self, game)
    }

}
