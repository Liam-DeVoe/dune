use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct PerformBeneGesseritPlacement {
    pub target: Option<Location>,
}

impl Default for PerformBeneGesseritPlacement {
    fn default() -> Self {
        PerformBeneGesseritPlacement {
            target: Location::from_id(0),
        }
    }
}

impl PerformBeneGesseritPlacement {
    pub fn bene_gesserit_may_place_first_force_in_any_territory(g: &Game) -> bool {
        g.applicable(Rule::BeneGesseritFirstForceInAnyTerritory) || g.applicable(Rule::BeneGesseritAdvisors)
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_locations(g: &Game) -> impl Iterator<Item = Location> {
        use either::Either;
        if Self::bene_gesserit_may_place_first_force_in_any_territory(g) {
            let hms = g.board.map.hidden_mobile_stronghold().location();
            return Either::Left(g.board.map.locations(false).filter(move |l| *l != hms));
        }
        Either::Right(std::iter::once(g.board.map.polar_sink()))
    }
}

impl EventRules for PerformBeneGesseritPlacement {
    fn validate(&self, game: &Game, _by: By) -> Result<(), Rejection> {
        let target = self.target;
        if !Self::valid_locations(game).any(|l| Some(l) == target) {
            return Err(Rejection(express!("Invalid location")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        // Only reached outside the phase when events are replayed without validation.
        if game.setup().is_none() {
            game_error!("Not in the setup phase");
        }
        // A null Target (only without validation) is passed on as in C#: IsOccupied(null)
        // is false, ShipForces(null, 1) finds no homeworld to ship from, and `Target.Territory` throws.
        let condition = game.is_occupied_territory(self.target.required("target")?.territory());
        if condition {
            // Player is read before Target.
            let me = by.player();
            Player::ship_advisors(game, me, self.target.required("target")?, 1)?;
        } else {
            let me = by.player();
            Player::ship_forces(game, me, self.target.required("target")?, 1)?;
        }

        let m = self.describe(game, by);
        game.log_event(m);
        let c1 = game.is_playing(Faction::Moritani);
        let c2 = game.treachery_cards_before_traitors();
        if c1 {
            game.enter_setup(SetupStep::MoritaniSettingUp);
            Ok(())
        } else if c2 {
            game.enter_storm_phase()
        } else {
            game.deal_starting_treachery_cards()
        }
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " position themselves in ", self.target)
    }
}
