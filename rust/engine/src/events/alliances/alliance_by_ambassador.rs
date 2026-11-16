use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct AllianceByAmbassador {
    #[serde(skip)]
    pub passed: bool,
}

impl EventRules for AllianceByAmbassador {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let Some(&offer @ Interrupt::AllianceByAmbassador { offered_by, give_vidal_to_ally, take_vidal, .. }) = game.interrupts.last() else {
            game_error!("No alliance offered by an ambassador");
        };
        game.resume(offer, true)?;

        if !self.passed {
            game.make_alliance(by.faction.required("initiator")?, offered_by.faction.required("ambassador owner")?);

            if give_vidal_to_ally {
                game.take_vidal(by.player(), VidalMoment::EndOfTurn);
            }

            if game.turn.has_acted_or_passed.contains_opt(by.faction)
                && game.turn.has_acted_or_passed.contains_opt(offered_by.faction)
            {
                game.check_if_forces_should_be_destroyed_by_ally_presence(by.player())?;
            }
            game.flip_bene_gesserit_advisors_when_alone()?;
        } else {
            game.log(parts![by.faction, " don't ally with ", offered_by.faction]);
            if take_vidal {
                game.take_vidal(offered_by.player(), VidalMoment::AfterUsedInBattle);
            }
        }

        game.determine_next_shipment_and_move_sub_phase()?;
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, if !self.passed { "" } else { " don't" }, " agree to ally")
    }
}
