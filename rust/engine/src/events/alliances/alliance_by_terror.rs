use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AllianceByTerror {
    pub passed: bool,
}

impl EventRules for AllianceByTerror {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let paused = game.ship_move.paused_terror_phase.required("paused terror phase")?;
        game.enter(paused, true);

        if !self.passed {
            if game.player(by.player()).has_ally() {
                let ally = game.player(by.player()).ally;
                game.log(parts![by.faction, " and ", ally, " end their alliance"]);
                game.break_alliance(by.faction.required("faction")?);
            }

            let moritani = game.get_player(Some(Faction::Moritani));
            if let Some(moritani) = moritani {
                if game.player(moritani).has_ally() {
                    let ally = game.player(moritani).ally;
                    game.log(parts![Faction::Moritani, " and ", ally, " end their alliance"]);
                    game.break_alliance(Faction::Moritani);
                }
            }

            game.make_alliance(by.faction.required("faction")?, Faction::Moritani);

            if game.turn.has_acted_or_passed.contains_opt(by.faction) && game.turn.has_acted_or_passed.contains(&Faction::Moritani) {
                game.check_if_forces_should_be_destroyed_by_ally_presence(by.player())?;
            }

            let territory = game.last_terror_trigger().required("last terror trigger")?.territory(game);
            game.log(parts!["Terror in ", territory, " is returned to supplies"]);
            // ToList(): a snapshot before the dictionary changes
            for t in game.terror_in(territory).collect::<Vec<_>>() {
                game.board.terror_on_planet.shift_remove(&t);
                game.board.unplaced_terror_tokens.push(t);
            }

            game.ship_move.alliance_by_terror_was_offered = false;
            game.dequeue_intrusion(IntrusionType::Terror)?;
            game.determine_next_shipment_and_move_sub_phase()?;
        } else {
            game.log(parts![by.faction, " don't ally with ", Faction::Moritani]);
        }

        game.let_factions_discard_surplus_cards();
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, if !self.passed { "" } else { " don't" }, " agree to ally")
    }
}
