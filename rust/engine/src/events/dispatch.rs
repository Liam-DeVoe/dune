//! How `Event` calls the event inside it, converted or `@legacy`. Not exported, so `.validate()`
//! on an `Rc<X>` still means the event's own method.

use crate::prelude::*;

/// How the dispatch in `events/mod.rs` calls an event, converted or not. Converted events get it
/// from [`EventRules`]; `@legacy` ones (`Rc<X>` with `X: GameEventMethods`) through their base.
pub(crate) trait EventImpl {
    /// Before validation: a legacy event takes its initiator and player from the submitter.
    fn prepare(&self, _game: &Game, _by: Option<Faction>) {}
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection>;
    fn try_validate(&self, game: &Game, by: By) -> GameResult<Result<(), Rejection>>;
    fn apply(&self, game: &mut Game, by: By) -> GameResult;
    fn describe(&self, game: &Game, by: By) -> Message;
    fn try_describe(&self, game: &Game, by: By) -> GameResult<Message>;
    fn describe_short(&self, game: &Game, by: By) -> Message;
}

impl<T: EventRules> EventImpl for T {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        EventRules::validate(self, game, by)
    }
    fn try_validate(&self, game: &Game, by: By) -> GameResult<Result<(), Rejection>> {
        EventRules::try_validate(self, game, by)
    }
    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        EventRules::apply(self, game, by)
    }
    fn describe(&self, game: &Game, by: By) -> Message {
        EventRules::describe(self, game, by)
    }
    fn try_describe(&self, game: &Game, by: By) -> GameResult<Message> {
        EventRules::try_describe(self, game, by)
    }
    fn describe_short(&self, game: &Game, by: By) -> Message {
        EventRules::describe_short(self, game, by)
    }
}

fn rejection(m: Option<Message>) -> Result<(), Rejection> {
    match m {
        None => Ok(()),
        Some(m) => Err(Rejection(m)),
    }
}

/// A legacy event reads its initiator and player from its base, which [`EventImpl::prepare`]
/// filled from the submitter, so the `by` argument is unused.
impl<T: GameEventMethods> EventImpl for Rc<T> {
    fn prepare(&self, game: &Game, by: Option<Faction>) {
        self.initialize(game, by);
    }
    fn validate(&self, game: &Game, _by: By) -> Result<(), Rejection> {
        rejection(GameEventMethods::validate(&**self, game))
    }
    fn try_validate(&self, game: &Game, _by: By) -> GameResult<Result<(), Rejection>> {
        GameEventMethods::try_validate(&**self, game).map(rejection)
    }
    fn apply(&self, game: &mut Game, _by: By) -> GameResult {
        GameEventMethods::execute_concrete_event(self, game)
    }
    fn describe(&self, game: &Game, _by: By) -> Message {
        GameEventMethods::get_message(&**self, game)
    }
    fn try_describe(&self, game: &Game, _by: By) -> GameResult<Message> {
        GameEventMethods::try_get_message(&**self, game)
    }
    fn describe_short(&self, game: &Game, _by: By) -> Message {
        GameEventMethods::get_short_message(&**self, game)
    }
}
