use crate::prelude::*;
use crate::messages::part::Part;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EmperorBidSupport {
    pub amounts: Option<IndexMap<Faction, i32>>,
}

impl EventRules for EmperorBidSupport {
    fn validate(&self, _game: &Game, by: By) -> Result<(), Rejection> {
        // Not in C#, which accepted any amounts. Support for the Emperor's own bids let the
        // Emperor bid their spice twice, and paying left them with less than zero
        // (behavior-changes.md, "Turns, sectors and amounts out of range").
        if let Some(amounts) = &self.amounts {
            if amounts.get_opt(by.faction).is_some() {
                return Err(Rejection(express!("You can't support your own bids")));
            }
            if amounts.values().any(|v| *v < 0) {
                return Err(Rejection(express!("Invalid amount")));
            }
        }
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        // Without amounts (only without validation) the message can't be built: the event fails
        // before it is logged.
        game.alliances.permitted_use_of_emperor_spice = self.amounts.clone();
        let message = self.try_describe(game, by)?;
        game.current_report_mut().express_event(message);
        Ok(())
    }

    fn try_describe(&self, game: &Game, by: By) -> GameResult<Message> {
        if self.amounts.is_none() {
            game_error!("Emperor bid support has no amounts");
        }
        Ok(self.describe(game, by))
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        // `Amounts.Sum(...)` on a null Amounts: ArgumentNullException (Parameter 'source').
        let amounts = self.amounts.as_ref().expect("amounts");
        if amounts.iter().map(|(_, v)| *v).sum::<i32>() > 0 {
            let parts = Part::list(
                amounts.iter().filter(|(_, v)| **v > 0).map(|(k, v)| express_part!(*k, ":", Payment::of(*v), " ")),
            );
            return express!(by.faction, " supports ", parts);
        }
        express!(by.faction, " doesn't support bids by other factions")
    }
}
