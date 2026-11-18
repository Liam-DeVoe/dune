use crate::prelude::*;
use crate::messages::part::Part;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct SetShipmentPermission {
    pub factions: Option<Vec<Faction>>,
    pub permission: ShipmentPermission,
}

impl SetShipmentPermission {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_targets(g: &Game, p: PlayerId) -> impl Iterator<Item = Faction> {
        // `p.Faction` is read inside the predicate, per element.
        g.players.iter().filter(move |x| x.faction != g.player(p).faction).filter_map(|x| x.faction)
    }

    pub fn is_applicable(g: &Game, p: PlayerId) -> bool {
        g.turn.current_main_phase == MainPhase::ShipmentAndMove && g.player(p).is(Faction::Guild) && g.player(p).has_any_high_threshold(g)
    }
}

impl EventRules for SetShipmentPermission {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        self.try_validate(game, by).unwrap_or_else(|e| Err(Rejection(express!(e.to_string()))))
    }

    /// Fails without factions.
    fn try_validate(&self, game: &Game, by: By) -> GameResult<Result<(), Rejection>> {
        let factions = self.factions.as_ref().required("factions")?;
        if factions.is_empty() {
            return Ok(Err(Rejection(express!("Select one or more factions"))));
        }
        // ValidTargets is re-evaluated for every faction.
        if factions.iter().any(|f| !SetShipmentPermission::valid_targets(game, by.player()).any(|v| v == *f)) {
            return Ok(Err(Rejection(express!("Invalid faction"))));
        }

        Ok(Ok(()))
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);

        let factions = self.factions.as_ref().required("factions")?;
        for f in factions {
            game.alliances.shipment_permissions.shift_remove(f);
        }

        if self.permission != ShipmentPermission::NONE {
            for f in factions {
                // Dictionary.Add: ArgumentException on a faction listed twice.
                game.alliances.shipment_permissions.insert_new(*f, self.permission);
            }
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        // A Faction[] is one message element (a value-type array is not spread).
        let factions: Part = match &self.factions {
            Some(f) => Part::list(f.iter().copied()),
            // A null array, which the log prints as nothing.
            None => Part::Null,
        };
        if self.permission == ShipmentPermission::NONE {
            return express!(by.faction, " → ", factions, ": all permissions revoked");
        }
        express!(
            by.faction,
            " → ",
            factions,
            ": ",
            "cross shipping: ",
            if self.permission.cross { "yes" } else { "no" },
            ", to (own) world: ",
            if self.permission.to_homeworld { "yes" } else { "no" },
            ", discount: ",
            if self.permission.guild_rate { "yes" } else { "no" }
        )
    }
}
