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
        // Factions.Any(...) on a null array: ArgumentNullException.
        let Some(factions) = self.factions.as_ref() else {
            crate::compat::cs::throw("ArgumentNullException", crate::compat::cs::with_parameter("Value cannot be null.", "source"))
        };
        // ValidTargets is re-run lazily per faction; `p.Faction` is only read when there are players.
        let p = self.player_opt();
        if factions.iter().any(|f| !game.players.iter().any(|x| x.faction != game.player(p.nre()).faction && x.faction == *f)) {
            return Some(express!("Invalid faction"));
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_targets(g: &Game, p: PlayerIndex) -> impl Iterator<Item = Faction> {
        g.players.iter().filter(move |x| x.faction != g.player(p).faction).map(move |x| x.faction)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        // Stores the event's array, null included (executed without validation);
        // Log() then throws ArgumentNullException in GetMessage.
        game.factions_with_increased_revival_limits = self.factions.clone();
        self.log(game);
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        let Some(factions) = self.factions.as_ref() else {
            crate::compat::cs::throw("ArgumentNullException", crate::compat::cs::with_parameter("Value cannot be null.", "source"))
        };
        if !factions.is_empty() {
            return express!(self.initiator(), " grant a revival limit of ", 5, " to ", ExpressionElement::array(factions.iter().copied()));
        }

        express!(self.initiator(), " don't grant increased revival limits")
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

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        SetIncreasedRevivalLimits::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        SetIncreasedRevivalLimits::get_message(self, game)
    }

}
