use crate::prelude::*;
use crate::messages::part::Part;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SetIncreasedRevivalLimits {
    pub factions: Option<Vec<Faction>>,
}

impl SetIncreasedRevivalLimits {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_targets(g: &Game, p: PlayerId) -> impl Iterator<Item = Faction> {
        g.players.iter().filter(move |x| x.faction != g.player(p).faction).filter_map(move |x| x.faction)
    }
}

impl EventRules for SetIncreasedRevivalLimits {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        self.try_validate(game, by).unwrap_or_else(|e| Err(Rejection(express!(e.to_string()))))
    }

    /// Fails without factions (C#: ArgumentNullException from Factions.Any(...) on a null array).
    fn try_validate(&self, game: &Game, by: By) -> GameResult<Result<(), Rejection>> {
        let factions = self.factions.as_ref().required("factions")?;
        // ValidTargets is re-run lazily per faction; `p.Faction` is only read when there are players.
        let p = by.player;
        if factions.iter().any(|f| !game.players.iter().any(|x| x.faction != game.player(p.expect("player")).faction && x.faction == Some(*f))) {
            return Ok(Err(Rejection(express!("Invalid faction"))));
        }

        Ok(Ok(()))
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        // Stores the event's array, null included (executed without validation). The message
        // can't be built without factions, so the event fails before it is logged (C#: Log()
        // throws ArgumentNullException in GetMessage).
        game.revival.factions_with_increased_revival_limits = self.factions.clone();
        let message = self.try_describe(game, by)?;
        game.current_report_mut().express_event(message);
        Ok(())
    }

    fn describe(&self, game: &Game, by: By) -> Message {
        self.try_describe(game, by).expect("the message of a SetIncreasedRevivalLimits event with factions")
    }

    fn try_describe(&self, _game: &Game, by: By) -> GameResult<Message> {
        let factions = self.factions.as_ref().required("factions")?;
        if !factions.is_empty() {
            return Ok(express!(by.faction, " grant a revival limit of ", 5, " to ", Part::list(factions.iter().copied())));
        }

        Ok(express!(by.faction, " don't grant increased revival limits"))
    }
}
