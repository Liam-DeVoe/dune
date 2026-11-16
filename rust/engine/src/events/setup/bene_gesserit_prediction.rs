use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct BeneGesseritPrediction {
    pub to_win: Option<Faction>,
    pub turn: Turn,
}

impl BeneGesseritPrediction {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_targets(g: &Game, p: PlayerId) -> impl Iterator<Item = Faction> {
        g.players_other_than(p)
    }

    pub fn valid_turns(g: &Game) -> Vec<Turn> {
        (1..=g.maximum_turns().number()).map(Turn::new).collect()
    }
}

impl EventRules for BeneGesseritPrediction {
    fn validate(&self, game: &Game, _by: By) -> Result<(), Rejection> {
        if !game.is_playing(self.to_win) {
            return Err(Rejection(express!("Invalid target")));
        }
        if self.turn < Turn::FIRST || self.turn > game.maximum_turns() {
            return Err(Rejection(express!("Invalid turn")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let me = by.player();
        game.player_mut(me).predicted_faction = self.to_win;
        game.player_mut(me).predicted_turn = Some(self.turn);
        let m = self.describe(game, by);
        game.log_event(m);
        let condition = game.treachery_cards_before_traitors();
        if condition { game.deal_starting_treachery_cards() } else { game.deal_traitors() }
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " predict who will win and when")
    }
}
