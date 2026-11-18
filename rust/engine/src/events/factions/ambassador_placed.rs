use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct AmbassadorPlaced {
    pub ambassador: Option<Ambassador>,
    pub stronghold: Option<Territory>,
}

impl Default for AmbassadorPlaced {
    fn default() -> Self {
        AmbassadorPlaced {
            ambassador: Default::default(),
            stronghold: Territory::from_id(0),
        }
    }
}

impl AmbassadorPlaced {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_strongholds(g: &Game, _p: PlayerId) -> impl Iterator<Item = Territory> {
        g.board.map
            .territories(false)
            .filter(move |t| {
                t.is_visible(&g.board.map) && t.is_stronghold() && !g.is_in_storm_territory(*t) && g.ambassador_in(*t) == None
            })
    }

    pub fn valid_ambassadors(game: &Game, p: PlayerId) -> Vec<Ambassador> {
        game.player(p).ambassadors.clone()
    }

    pub fn is_applicable(g: &Game, p: PlayerId) -> bool {
        i32::from(g.player(p).resources) > g.turn.revivals.ambassadors_placed_this_turn
            && g.phase() == Phase::ResurrectionReport
            && !g.prevented(FactionAdvantage::EcazAmbassadors)
            && !AmbassadorPlaced::valid_ambassadors(g, p).is_empty()
            && !AmbassadorPlaced::valid_strongholds(g, p).next().is_none()
    }
}

impl EventRules for AmbassadorPlaced {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        let stronghold = self.stronghold;
        if !AmbassadorPlaced::valid_strongholds(game, by.player()).into_iter().any(|t| Some(t) == stronghold) {
            return Err(Rejection(express!("Invalid stronghold")));
        }
        if !AmbassadorPlaced::valid_ambassadors(game, by.player()).iter().any(|a| Some(*a) == self.ambassador) {
            return Err(Rejection(express!("Ambassador not available")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let Some(stronghold) = self.stronghold else {
            game_error!("Stronghold is null");
        };

        game.turn.revivals.ambassadors_placed_this_turn += 1;
        let me = by.player();
        let placed = game.turn.revivals.ambassadors_placed_this_turn;
        let p = game.player_mut(me);
        crate::game::amounts::spend(&mut p.resources, placed, "spice")?;
        game.log(parts![by.faction, " station the ", self.ambassador, " ambassador in ", stronghold, " for ", Payment::of(placed)]);
        if game.board.ambassadors_on_planet.contains_key(&stronghold) {
            return Err(GameError::express(express!("There already is an ambassador in ", stronghold)));
        }
        let ambassador = self.ambassador.required("ambassador")?;
        game.board.ambassadors_on_planet.insert_new(stronghold, ambassador);
        // List.Remove: the first equal element
        let ambassadors = &mut game.player_mut(me).ambassadors;
        if let Some(pos) = ambassadors.iter().position(|a| *a == ambassador) {
            ambassadors.remove(pos);
        }
        game.stone(Milestone::AmbassadorPlaced);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " station the ", self.ambassador, " ambassador in ", self.stronghold)
    }
}
