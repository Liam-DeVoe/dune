use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct FlightDiscoveryUsed {}

impl FlightDiscoveryUsed {
    pub fn is_available(g: &Game, p: PlayerId) -> bool {
        g.board.owner_of_flight_discovery == Some(g.player(p).faction)
    }
}

impl EventRules for FlightDiscoveryUsed {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);
        game.board.owner_of_flight_discovery = Some(None);
        game.turn.current_flight_discovery_used = Some(by);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " use the ", DiscoveryToken::Flight, " discovery token to gain movement speed")
    }
}
