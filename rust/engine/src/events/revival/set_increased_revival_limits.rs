use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct SetIncreasedRevivalLimits {
    pub factions: Option<Vec<Faction>>,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for SetIncreasedRevivalLimits {
    fn default() -> Self {
        SetIncreasedRevivalLimits {
            factions: Default::default(),
            base: Default::default(),
        }
    }
}

impl SetIncreasedRevivalLimits {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        self.try_validate(game).unwrap_or_else(|e| Some(express!(e.to_string())))
    }

    /// Fails without factions (C#: ArgumentNullException from Factions.Any(...) on a null array).
    pub fn try_validate(&self, game: &Game) -> GameResult<Option<Message>> {
        let factions = self.factions.as_ref().required("factions")?;
        // ValidTargets is re-run lazily per faction; `p.Faction` is only read when there are players.
        let p = self.player_opt();
        if factions.iter().any(|f| !game.players.iter().any(|x| x.faction != game.player(p.expect("player")).faction && x.faction == *f)) {
            return Ok(Some(express!("Invalid faction")));
        }

        Ok(None)
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_targets(g: &Game, p: PlayerIndex) -> impl Iterator<Item = Faction> {
        g.players.iter().filter(move |x| x.faction != g.player(p).faction).map(move |x| x.faction)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        // Stores the event's array, null included (executed without validation). The message
        // can't be built without factions, so the event fails before it is logged (C#: Log()
        // throws ArgumentNullException in GetMessage).
        game.factions_with_increased_revival_limits = self.factions.clone();
        let message = self.try_get_message(game)?;
        game.current_report_mut().express_event(message);
        Ok(())
    }

    pub fn try_get_message(&self, _game: &Game) -> GameResult<Message> {
        let factions = self.factions.as_ref().required("factions")?;
        if !factions.is_empty() {
            return Ok(express!(self.initiator(), " grant a revival limit of ", 5, " to ", ExpressionElement::array(factions.iter().copied())));
        }

        Ok(express!(self.initiator(), " don't grant increased revival limits"))
    }

    pub fn get_message(&self, game: &Game) -> Message {
        self.try_get_message(game).expect("the message of a SetIncreasedRevivalLimits event with factions")
    }

}

impl GameEventMethods for SetIncreasedRevivalLimits {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::SetIncreasedRevivalLimits
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::SetIncreasedRevivalLimits(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        SetIncreasedRevivalLimits::validate(self, game)
    }

    fn try_validate(&self, game: &Game) -> GameResult<Option<Message>> {
        SetIncreasedRevivalLimits::try_validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        SetIncreasedRevivalLimits::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        SetIncreasedRevivalLimits::get_message(self, game)
    }

    fn try_get_message(&self, game: &Game) -> GameResult<Message> {
        SetIncreasedRevivalLimits::try_get_message(self, game)
    }

}
