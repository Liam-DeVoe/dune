use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct PerformHmsMovement {
    pub target: Option<Location>,
    #[serde(skip)]
    pub passed: bool,
}

impl Default for PerformHmsMovement {
    fn default() -> Self {
        PerformHmsMovement {
            target: Location::from_id(0),
            passed: Default::default(),
        }
    }
}

impl PerformHmsMovement {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_locations(g: &Game) -> impl Iterator<Item = Location> {
        // AttachedToLocation is null only when the HMS was never placed; FindNeighboursForHmsMovement then throws.
        let start = g.board.map.hidden_mobile_stronghold().attached_to_location(&g.board.map).expect("attached to location");
        g.board.map
            .find_neighbours_for_hms_movement(start, 1, false, g.board.sector_in_storm)
            .into_iter()
            .filter(move |l| !l.is_stronghold())
    }
}

impl EventRules for PerformHmsMovement {
    fn validate(&self, game: &Game, _by: By) -> Result<(), Rejection> {
        if !self.passed {
            if !PerformHmsMovement::valid_locations(game).any(|v| Some(v) == self.target) {
                return Err(Rejection(express!("Invalid location")));
            }
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let me = by.player();
        let hms = game.board.map.hidden_mobile_stronghold();
        let collection_rate = game.player(me).any_forces_in(Some(hms.location())) * 2;
        let m = self.describe(game, by);
        game.log_event(m);

        let current_location = hms.attached_to_location(&game.board.map).required("the location of the HMS")?;
        game.collect_spice_from(by.faction.required("faction")?, current_location, collection_rate)?;

        if !self.passed {
            let target = self.target.required("target")?;
            hms.point_at(&mut game.board.map, target);
            game.collect_spice_from(by.faction.required("faction")?, target, collection_rate)?;
            game.storm.hms_moves_left -= 1;
            game.stone(Milestone::HmsMovement);
        }

        if self.passed || game.storm.hms_moves_left == 0 {
            game.determine_storm();
        }
        Ok(())
    }

    fn describe(&self, game: &Game, by: By) -> Message {
        let hms = game.board.map.hidden_mobile_stronghold();
        if !self.passed {
            return express!(by.faction, " move the ", hms, " to ", self.target);
        }
        express!(by.faction, " pass (further) movement of the ", hms)
    }
}
