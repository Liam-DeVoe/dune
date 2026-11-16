use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AmalPlayed {}

impl EventRules for AmalPlayed {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        game.discard(by.player(), TreacheryCardType::Amal)?;
        let m = self.describe(game, by);
        game.log_event(m);

        for i in 0..game.players.len() {
            let p = game.player_at(i);
            // Half, rounded up.
            let resources_paid = game.player(p).resources.div_ceil(2);
            let pl = game.player_mut(p);
            pl.resources -= resources_paid;
            let resources_paid = i32::from(resources_paid);
            let faction = game.player(p).faction;
            game.log(vec![faction.into(), " lose ".into(), Payment::of(resources_paid).into()]);
        }

        game.stone(Milestone::Amal);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " perform ", TreacheryCardType::Amal)
    }
}
