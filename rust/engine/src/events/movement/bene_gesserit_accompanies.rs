use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct BeneGesseritAccompanies {
    pub location: Option<Location>,
    pub accompanies: bool,
    pub extra_advisor: bool,
}

impl Default for BeneGesseritAccompanies {
    fn default() -> Self {
        BeneGesseritAccompanies {
            location: Location::from_id(0),
            accompanies: Default::default(),
            extra_advisor: Default::default(),
        }
    }
}

impl BeneGesseritAccompanies {
    pub fn to(&self, _game: &Game) -> Option<Location> {
        self.location
    }

    pub fn total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        self.special_forces_added_to_location(game)
    }

    pub fn forces_added_to_location(&self, _game: &Game) -> i32 {
        0
    }

    pub fn special_forces_added_to_location(&self, _game: &Game) -> i32 {
        (if self.accompanies { 1 } else { 0 }) + (if self.extra_advisor { 1 } else { 0 })
    }

    pub fn bene_gesserit_may_accompany_to_shipment_location(g: &Game) -> bool {
        g.applicable(Rule::BeneGesseritAccompaniesToShipmentLocation) || g.applicable(Rule::BeneGesseritAdvisors)
    }

    pub fn valid_targets(g: &Game, p: PlayerId) -> Vec<Location> {
        let last_to = g.turn.last_shipment_or_movement.as_ref().and_then(|e| e.to_location(g));
        let Some(last_to) = last_to else {
            panic!("LastShipmentOrMovement is null");
        };

        let mut result = Vec::new();
        if last_to != g.board.map.polar_sink()
            && last_to.as_homeworld().is_none()
            && Some(last_to.territory()) != g.board.atomics_aftermath
            && !g.player(p).occupies_territory(g, Some(last_to.territory()))
            && BeneGesseritAccompanies::bene_gesserit_may_accompany_to_shipment_location(g)
            && !BeneGesseritAccompanies::ally_prevents_accompanying_to_shipment_location(g, p)
        {
            result.extend(last_to.territory().locations().iter().copied().filter(|l| {
                !Sector::in_storm(l.sector(&g.board.map), g.board.sector_in_storm) && (g.applicable(Rule::BeneGesseritAdvisors) || g.is_not_full(p, *l))
            }));
        }

        result.push(g.board.map.polar_sink());
        result
    }

    pub fn ally_prevents_accompanying_to_shipment_location(g: &Game, p: PlayerId) -> bool {
        let ally = g.get_player(g.player(p).ally);

        !g.applicable(Rule::AdvisorsDontConflictWithAlly)
            && ally.is_some_and(|a| {
                let t = g.turn.last_shipment_or_movement.as_ref().and_then(|e| e.to_location(g)).map(|l| l.territory());
                g.player(a).any_forces_in_territory(t) != 0
            })
    }

    pub fn may_send_extra_advisor(g: &Game, p: PlayerId, l: Location) -> bool {
        let pl = g.player(p);
        pl.has_any_high_threshold(g) && l == g.board.map.polar_sink() && pl.forces_in_reserve() >= 2
    }
}

impl EventRules for BeneGesseritAccompanies {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if self.accompanies {
            if game.player(by.player()).forces_in_reserve() <= 0 {
                return Err(Rejection(express!("You don't have forces in reserve")));
            }
            let location = self.location;
            if !location.is_some_and(|l| BeneGesseritAccompanies::valid_targets(game, by.player()).contains(&l)) {
                return Err(Rejection(express!("You can't accompany there")));
            }
            if !by.is(Faction::BeneGesserit) {
                return Err(Rejection(express!("You can't accompany shipments")));
            }
            let Some(location) = location else {
                return Err(Rejection(express!("To not selected")));
            };
            // Unreachable: the reserves check above already refused this.
            if self.accompanies && game.player(by.player()).forces_in_reserve() == 0 {
                return Err(Rejection(express!("No forces available")));
            }
            if self.extra_advisor && !BeneGesseritAccompanies::may_send_extra_advisor(game, by.player(), location) {
                return Err(Rejection(express!("You cannot send an extra advisor")));
            }
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        game.turn.bene_gesserit_may_accompany = false;

        if self.accompanies {
            let me = by.player();
            let location = self.location.required("location")?;
            let amount = self.special_forces_added_to_location(game);
            let as_fighters = game.player(me).occupies_territory(game, Some(location.territory()))
                || !game.is_occupied_territory(location.territory())
                || !game.applicable(Rule::BeneGesseritAdvisors);
            if as_fighters {
                Player::ship_forces(game, me, location, amount)?;
            } else {
                Player::ship_advisors(game, me, location, amount)?;
            }

            let m = express!(by.faction, " accompany to ", location)
                .with_route(by.required_faction()?, Vec::<Location>::new(), Some(location))
                .with_log(BeneGesseritReaction::new(BeneGesseritReactionKind::Accompany, location.into(), false));
            game.log(parts![m]);
            game.turn.last_shipment_or_movement = Some(ShipmentOrMovement { to: self.location });
            // Fighters trigger an ambassador, advisors don't (behavior-changes.md, "Bene
            // Gesserit fighters that accompany a shipment trigger an ambassador").
            let fighters = if as_fighters { amount } else { 0 };
            game.check_intrusion_with_fighters(&Event::from(self.clone()), by.faction.expect("faction"), fighters)?;
        } else {
            let m = express!(by.faction, " don't accompany")
                .with_log(BeneGesseritReaction::new(BeneGesseritReactionKind::DontAccompany, Option::<Location>::None.into(), false));
            game.log(parts![m]);
        }

        game.determine_next_shipment_and_move_sub_phase()?;
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        if self.accompanies {
            return express!(by.faction, " accompany shipment to ", self.location);
        }
        express!(by.faction, " don't accompany shipment")
    }
}

impl crate::events::location_event::LocationEvent for BeneGesseritAccompanies {
    fn to(&self, game: &Game) -> Option<Location> {
        BeneGesseritAccompanies::to(self, game)
    }

    fn total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        BeneGesseritAccompanies::total_amount_of_forces_added_to_location(self, game)
    }

    fn forces_added_to_location(&self, game: &Game) -> i32 {
        BeneGesseritAccompanies::forces_added_to_location(self, game)
    }

    fn special_forces_added_to_location(&self, game: &Game) -> i32 {
        BeneGesseritAccompanies::special_forces_added_to_location(self, game)
    }

}
