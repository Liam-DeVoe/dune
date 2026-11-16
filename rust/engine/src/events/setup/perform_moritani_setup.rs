use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct PerformMoritaniSetup {
    pub target: Option<Location>,
}

impl Default for PerformMoritaniSetup {
    fn default() -> Self {
        PerformMoritaniSetup {
            target: Location::from_id(0),
        }
    }
}

impl PerformMoritaniSetup {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_locations(g: &Game) -> impl Iterator<Item = Location> {
        let hms = g.board.map.hidden_mobile_stronghold().location();
        g.board.map.locations(false).filter(move |l| *l != hms && !g.any_forces_in(l.territory()))
    }
}

impl EventRules for PerformMoritaniSetup {
    fn validate(&self, game: &Game, _by: By) -> Result<(), Rejection> {
        let target = self.target;
        if !Self::valid_locations(game).any(|l| Some(l) == target) {
            return Err(Rejection(express!("Invalid location")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        // Player is read before Target.
        let me = by.player();
        // `Target` may be null without validation: ShipForces passes it on.
        let target = self.target.required("target")?;
        Player::ship_forces(game, me, target, 6)?;
        let m = self.describe(game, by);
        game.log_event(m);
        if game.treachery_cards_before_traitors() {
            game.enter_storm_phase()?;
        } else {
            game.deal_starting_treachery_cards()?;
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " have set up forces")
    }
}
