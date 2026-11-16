use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct KarmaHmsMovement {
    #[serde(skip)]
    pub passed: bool,
    pub target: Option<Location>,
}

impl Default for KarmaHmsMovement {
    fn default() -> Self {
        KarmaHmsMovement {
            passed: Default::default(),
            target: Location::from_id(0),
        }
    }
}

impl KarmaHmsMovement {
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

impl EventRules for KarmaHmsMovement {
    fn validate(&self, game: &Game, _by: By) -> Result<(), Rejection> {
        if !KarmaHmsMovement::valid_locations(game).any(|v| Some(v) == self.target) {
            return Err(Rejection(express!("Invalid location")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let me = by.player();
        let hms = game.board.map.hidden_mobile_stronghold();
        let collection_rate = game.player(me).any_forces_in(Some(hms.location())) * 2;
        if !game.player(me).special_karma_power_used {
            let card = Karma::valid_karma_cards(game, me).into_iter().next();
            if self.passed {
                game.play_karma(me, card, vec!["pass movement of the ".into(), hms.into()]);
            } else {
                let target = self.target;
                game.play_karma(me, card, vec!["move the ".into(), hms.into(), " to ".into(), target.into()]);
            }

            game.player_mut(me).special_karma_power_used = true;
        } else {
            let m = self.describe(game, by);
        game.log_event(m);
        }

        let current_location = hms.attached_to_location(&game.board.map).required("the location of the HMS")?;
        game.collect_spice_from(by.faction.required("faction")?, current_location, collection_rate)?;

        if !self.passed {
            let target = self.target.required("target")?;
            hms.point_at(&mut game.board.map, target);
            game.collect_spice_from(by.faction.required("faction")?, target, collection_rate)?;
            game.board.karma_hms_moves_left -= 1;
            game.stone(Milestone::HmsMovement);
        }

        if self.passed {
            game.board.karma_hms_moves_left = 0;
        }
        Ok(())
    }

    fn describe(&self, game: &Game, by: By) -> Message {
        let hms = game.board.map.hidden_mobile_stronghold();
        if !self.passed {
            return express!("Using ", TreacheryCardType::Karma, ", ", by.faction, " move the ", hms, " to ", self.target);
        }
        express!(by.faction, " pass (further) movement of the ", hms, " using ", TreacheryCardType::Karma)
    }
}
