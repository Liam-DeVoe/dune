use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AllianceBroken {
}

impl EventRules for AllianceBroken {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if !game.player(by.player()).has_ally() {
            return Err(Rejection(express!("You currently have no ally")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let ally = game.player(by.player()).ally;
        let m = self.describe(game, by)
            .with_log(NexusLog::new(None, Some(NexusAlliance::new(NexusAllianceChange::Ended, by.faction, ally))));
        game.log(vec![m.into()]);

        // The former ally gets to act again in the alliance phase, even if they already passed.
        if matches!(game.turn.current_phase, Phase::AllianceA | Phase::AllianceB) {
            // List.Remove: the first occurrence, if any.
            if let Some(i) = game.turn.has_acted_or_passed.iter().position(|f| Some(*f) == ally) {
                game.turn.has_acted_or_passed.remove(i);
            }
        }

        game.break_alliance(by.faction.required("faction")?);
        game.let_factions_discard_surplus_cards();
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " end their alliance")
    }
}
