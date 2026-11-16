use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct BeneGesseritBattleAnnouncement {
    pub territory: Option<Territory>,
}

impl Default for BeneGesseritBattleAnnouncement {
    fn default() -> Self {
        BeneGesseritBattleAnnouncement {
            territory: Territory::from_id(0),
        }
    }
}

impl BeneGesseritBattleAnnouncement {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_territories(g: &Game, p: PlayerId) -> impl Iterator<Item = Territory> {
        let player = g.player(p);
        g.board.map
            .territories(g.applicable(Rule::Homeworlds))
            .filter(move |t| {
                let t = *t;
                (!player.has_ally() || g.player(player.allied_player(g).expect("allied player")).any_forces_in_territory(Some(t)) == 0)
                    && player.special_forces_in_territory(Some(t)) > 0
                    && (!t.is_stronghold() || g.nr_of_occupants_of_territory_excluding(t, player.faction) <= 1)
                    && !t.locations().iter().any(|l| Sector::in_storm(l.sector(&g.board.map), g.board.sector_in_storm) && player.special_forces_in(Some(*l)) > 0)
            })
    }
}

impl EventRules for BeneGesseritBattleAnnouncement {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if by.faction != Some(Faction::BeneGesserit) {
            return Err(Rejection(express!("Your faction can't announce battle")));
        }
        // p.SpecialForcesIn(Territory): the argument (a lookup that may throw) is evaluated
        // before the call dereferences a null p.
        let territory = self.territory;
        let p = by.player();
        if game.player(p).special_forces_in_territory(territory) == 0 {
            return Err(Rejection(express!("You don't have advisors there")));
        }
        let ally = game.get_player(game.player(p).ally);
        if let Some(ally) = ally {
            if game.player(ally).any_forces_in_territory(self.territory) > 0 {
                return Err(Rejection(express!("Can't announce battle due to ally presence")));
            }
        }
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let initiator = game.get_player(by.faction);
        let Some(initiator) = initiator else { return Ok(()) };
        let Some(territory) = self.territory else { return Ok(()) };
        Player::flip_forces_in_territory(game, initiator, territory, false)?;
        let m = self.describe(game, by);
        game.log_event(m);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " flip to ", FactionForce::BeneGesserit, " in ", self.territory)
    }
}
