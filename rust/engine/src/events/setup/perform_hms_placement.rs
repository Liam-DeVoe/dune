use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct PerformHmsPlacement {
    pub target: Option<Location>,
}

impl Default for PerformHmsPlacement {
    fn default() -> Self {
        PerformHmsPlacement {
            target: Location::from_id(0),
        }
    }
}

impl PerformHmsPlacement {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_locations(g: &Game, p: PlayerId) -> impl Iterator<Item = Location> {
        use either::Either;
        if g.player(p).faction != Some(Faction::Ixian) {
            return Either::Left(g.board.map.locations(false).filter(|l| !l.territory().is_stronghold()));
        }
        Either::Right(g.board.map.locations(false).filter(move |l| !l.territory().is_stronghold() && !Sector::in_storm(l.sector(&g.board.map), g.board.sector_in_storm)))
    }
}

impl EventRules for PerformHmsPlacement {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        // A null Player throws NullReferenceException in ValidLocations.
        if !Self::valid_locations(game, by.player()).any(|l| Some(l) == self.target) {
            return Err(Rejection(express!("Invalid location")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        // Only reached outside the phase when events are replayed without validation.
        if game.storm().is_none() {
            game_error!("Not in the storm phase");
        }
        let target = self.target.required("target")?;
        let hms = game.board.map.hidden_mobile_stronghold();
        hms.point_at(&mut game.board.map, target);
        let m = self.describe(game, by);
        game.log_event(m);
        game.end_storm_phase();
        game.stone(Milestone::HmsMovement);
        Ok(())
    }

    fn describe(&self, game: &Game, by: By) -> Message {
        express!(by.faction, " position the ", game.board.map.hidden_mobile_stronghold(), " above ", self.target)
    }
}
