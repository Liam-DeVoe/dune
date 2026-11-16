use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HideSecrets {
}

impl EventRules for HideSecrets {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        game.rules.secrets_remain_hidden.push(by.faction.required("faction")?);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " hide their secrets at end of game")
    }
}
