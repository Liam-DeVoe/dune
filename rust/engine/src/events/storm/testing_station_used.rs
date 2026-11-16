use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct TestingStationUsed {
    pub value_added: i32,
}

impl TestingStationUsed {
    pub fn can_be_played(g: &Game, p: PlayerId) -> bool {
        g.turn.current_phase == Phase::MetheorAndStormSpell
            && g.player(p).occupies(g, Some(g.board.map.testing_station().location()))
            && !g.storm.current_testing_station_used
    }
}

impl EventRules for TestingStationUsed {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        if self.value_added != -1 && self.value_added != 1 {
            return Err(Rejection(express!("Invalid amount")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        game.stone(Milestone::WeatherControlled);
        game.storm.current_testing_station_used = true;
        game.board.next_storm_moves += self.value_added;
        let next = game.board.next_storm_moves;
        game.log(parts![by.faction, " use ", DiscoveryToken::TestingStation, ": storm ", if self.value_added > 0 { "increases" } else { "weakens" }, " to ", next],
        );
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " use ", DiscoveryToken::TestingStation, " to influence Storm movement")
    }
}
