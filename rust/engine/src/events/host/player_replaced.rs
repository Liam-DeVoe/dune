use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayerReplaced {
    pub to_replace: Option<Faction>,
}

impl PlayerReplaced {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_targets(g: &Game) -> impl Iterator<Item = Faction> {
        g.players.iter().filter_map(move |p| p.faction)
    }
}

impl EventRules for PlayerReplaced {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, _by: By) -> GameResult {
        let player = game.get_player(self.to_replace);
        // IsBot(null) reads p.Seat: NullReferenceException.
        let is_bot = game.is_bot(player.required("player")?);
        let message = express!(self.to_replace, " will now be played by a ", if is_bot { "Bot" } else { "Human" })
            .with_log(AsideLog::aside());
        game.log(vec![message.into()]);
        Ok(())
    }

    fn describe(&self, _game: &Game, _by: By) -> Message {
        express!(self.to_replace, " player has been replaced")
    }
}
