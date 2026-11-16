use crate::prelude::*;

/// A player pays 3 spice so that Moritani doesn't get its Extortion token back.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExtortionPrevented {}

impl ExtortionPrevented {
    pub fn can_be_played(g: &Game, p: PlayerId) -> bool {
        g.player(p).faction != Some(Faction::Moritani)
            && i32::from(g.player(p).resources) >= 3
            && g.contemplate.extortion_to_be_returned
    }
}

impl EventRules for ExtortionPrevented {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);
        game.contemplate.extortion_to_be_returned = false;
        let p = game.player_mut(by.player());
        p.resources -= 3;
        let moritani = game.get_player(Some(Faction::Moritani)).required("get player")?;
        let c = game.player_mut(moritani);
        c.resources += 3;
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(
            by.faction,
            " pay ",
            Payment::of(3),
            " to prevent ",
            Faction::Moritani,
            " from regaining ",
            TerrorType::Extortion
        )
    }
}
